import {
  act,
  render,
  screen,
  fireEvent,
  waitFor,
} from "@testing-library/react";
import { describe, it, expect, vi } from "vitest";
import App from "../../App";
import { createTestApi } from "../../test-fixtures";
async function openRedis(api: ReturnType<typeof createTestApi>) {
  render(<App api={api} />);
  fireEvent.click(await screen.findByRole("button", { name: "全部服务" }));
  fireEvent.click(await screen.findByRole("button", { name: "查看 Redis" }));
  await screen.findByRole("dialog");
}
describe("service safety", () => {
  it("shows applied endpoint and real network, reveals only explicitly, clears on close", async () => {
    const api = createTestApi();
    await openRedis(api);
    expect(await screen.findByText("127.0.0.1:16379")).toBeInTheDocument();
    expect(screen.getByText("real-network")).toBeInTheDocument();
    expect(screen.queryByText("26379")).not.toBeInTheDocument();
    expect(api.revealSecret).not.toHaveBeenCalled();
    fireEvent.click(screen.getByRole("button", { name: "显示 密码" }));
    expect(await screen.findByText("test-password")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "关闭详情" }));
    await waitFor(() =>
      expect(screen.queryByText("test-password")).not.toBeInTheDocument(),
    );
    fireEvent.click(screen.getByRole("button", { name: "查看 Redis" }));
    expect(await screen.findByText("127.0.0.1:16379")).toBeInTheDocument();
    expect(screen.queryByText("test-password")).not.toBeInTheDocument();
  });
  it("requires draft validation and an explicit apply confirmation", async () => {
    const api = createTestApi();
    await openRedis(api);
    fireEvent.click(screen.getByRole("tab", { name: "配置" }));
    expect(await screen.findByLabelText("PORT")).toHaveValue("26379");
    fireEvent.change(screen.getByLabelText("PORT"), {
      target: { value: "26380" },
    });
    fireEvent.click(screen.getByRole("button", { name: "保存草稿并验证" }));
    await waitFor(() =>
      expect(api.validateDraft).toHaveBeenCalledWith(
        expect.objectContaining({
          baseGeneration: "applied-1",
          values: { PORT: "26380" },
        }),
      ),
    );
    expect(api.applyDraft).not.toHaveBeenCalled();
    fireEvent.click(screen.getByRole("button", { name: "预览并应用" }));
    expect(
      await screen.findByText("应用将重建受影响的托管组件"),
    ).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "确认应用" }));
    await waitFor(() =>
      expect(api.applyDraft).toHaveBeenCalledWith("draft-1", false),
    );
  });
  it("rejects an expired reset plan even when service name is correct", async () => {
    const api = createTestApi();
    api.planReset = vi.fn().mockResolvedValue({
      planId: "expired",
      targetId: "target",
      instanceId: "redis-1",
      serviceId: "redis",
      exactResourceIds: ["owned-volume"],
      databaseClaims: [],
      impact: ["data loss"],
      fingerprint: "fp",
      expiresAt: Date.now() - 1000,
    });
    await openRedis(api);
    fireEvent.click(screen.getByRole("tab", { name: "依赖与数据" }));
    fireEvent.click(screen.getByRole("button", { name: "预览重置计划" }));
    expect(await screen.findByText("owned-volume")).toBeInTheDocument();
    fireEvent.change(screen.getByLabelText("输入服务 ID 确认"), {
      target: { value: "redis" },
    });
    expect(screen.getByRole("button", { name: "执行精确重置" })).toBeDisabled();
    expect(api.executeReset).not.toHaveBeenCalled();
  });
  it("exports diagnostics only after preview and defaults to no logs", async () => {
    const api = createTestApi();
    render(<App api={api} />);
    fireEvent.click(await screen.findByRole("button", { name: "预览诊断" }));
    expect(await screen.findByText("已脱敏摘要")).toBeInTheDocument();
    expect(api.exportDiagnostics).not.toHaveBeenCalled();
    fireEvent.click(screen.getByRole("button", { name: "确认导出" }));
    await waitFor(() =>
      expect(api.exportDiagnostics).toHaveBeenCalledWith({
        previewId: "preview",
        excludedSections: [],
        includeLogs: false,
      }),
    );
    expect(
      await screen.findByText("/workspace/diagnostics.json"),
    ).toBeInTheDocument();
  });
});

describe("asynchronous view boundaries", () => {
  it("does not retain a reveal result that arrives after switching tabs", async () => {
    const api = createTestApi();
    let resolve!: (value: string) => void;
    api.revealSecret = vi.fn().mockImplementation(
      () =>
        new Promise<string>((r) => {
          resolve = r;
        }),
    );
    await openRedis(api);
    fireEvent.click(await screen.findByRole("button", { name: "显示 密码" }));
    fireEvent.click(screen.getByRole("tab", { name: "配置" }));
    await act(async () => {
      resolve("late-secret");
    });
    fireEvent.click(screen.getByRole("tab", { name: "连接与入口" }));
    await screen.findByText("127.0.0.1:16379");
    expect(screen.queryByText("late-secret")).not.toBeInTheDocument();
  });
  it("never exposes lifecycle controls for external instances", async () => {
    const api = createTestApi();
    const { fixtureSnapshot } = await import("../../test-fixtures");
    api.dashboardSnapshot = vi.fn().mockResolvedValue({
      ...fixtureSnapshot,
      services: fixtureSnapshot.services.map((s) =>
        s.serviceId === "redis"
          ? { ...s, ownership: "External", phase: "ExternalObserve" }
          : s,
      ),
    });
    await openRedis(api);
    fireEvent.click(screen.getByRole("tab", { name: "依赖与数据" }));
    expect(
      screen.queryByRole("button", { name: "预览重置计划" }),
    ).not.toBeInTheDocument();
    expect(
      screen.queryByRole("button", { name: "停止服务（保留数据）" }),
    ).not.toBeInTheDocument();
    expect(screen.getByText("外部服务 · 仅观察")).toBeInTheDocument();
  });
});

it("refuses a reset plan for a different instance even when its name and expiry look valid", async () => {
  const api = createTestApi();
  api.planReset = vi.fn().mockResolvedValue({
    planId: "other-instance",
    targetId: "target",
    instanceId: "redis-old",
    serviceId: "redis",
    exactResourceIds: ["old-volume"],
    databaseClaims: [],
    impact: ["data loss"],
    fingerprint: "fp",
    expiresAt: Date.now() + 60000,
  });
  await openRedis(api);
  fireEvent.click(screen.getByRole("tab", { name: "依赖与数据" }));
  fireEvent.click(screen.getByRole("button", { name: "预览重置计划" }));
  await screen.findByText("old-volume");
  fireEvent.change(screen.getByLabelText("输入服务 ID 确认"), {
    target: { value: "redis" },
  });
  expect(screen.getByRole("button", { name: "执行精确重置" })).toBeDisabled();
  expect(api.executeReset).not.toHaveBeenCalled();
});

it("supports keyboard dismissal and restores focus to the service action", async () => {
  const { default: userEvent } = await import("@testing-library/user-event");
  const user = userEvent.setup();
  const api = createTestApi();
  render(<App api={api} />);
  await user.click(await screen.findByRole("button", { name: "全部服务" }));
  const trigger = await screen.findByRole("button", { name: "查看 Redis" });
  await user.click(trigger);
  await screen.findByRole("dialog");
  await user.keyboard("{Escape}");
  await waitFor(() =>
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument(),
  );
  expect(trigger).toHaveFocus();
});
