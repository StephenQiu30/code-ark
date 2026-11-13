import userEvent from "@testing-library/user-event";
import {
  act,
  render,
  screen,
  fireEvent,
  waitFor,
  within,
} from "@testing-library/react";
import { describe, it, expect, vi } from "vitest";
import App from "../../App";
import { createTestApi } from "../../test-fixtures";
async function openRedis(api: ReturnType<typeof createTestApi>) {
  render(<App api={api} />);
  await userEvent.click(
    await screen.findByRole("button", { name: "全部服务" }),
  );
  await userEvent.click(
    await screen.findByRole("button", { name: "查看 Redis" }),
  );
  await screen.findByRole("region", { name: "服务详情 Redis" });
}
describe("service safety", () => {
  it("keeps configuration available before the first apply without requesting a connection or offering stop and logs", async () => {
    const api = createTestApi();
    const { fixtureSnapshot } = await import("../../test-fixtures");
    api.dashboardSnapshot = vi.fn().mockResolvedValue({
      ...fixtureSnapshot,
      services: fixtureSnapshot.services.map((service) =>
        service.serviceId === "redis"
          ? { ...service, phase: "Uninitialized", appliedGeneration: null }
          : service,
      ),
    });
    api.connectionView = vi
      .fn()
      .mockRejectedValue({ code: "NoAppliedConfiguration" });
    await openRedis(api);
    await screen.findAllByText("尚未应用配置");
    expect(api.connectionView).not.toHaveBeenCalled();
    expect(
      screen.queryByRole("button", { name: "停止服务（保留数据）" }),
    ).not.toBeInTheDocument();
    expect(screen.getByRole("tab", { name: "日志" })).toBeDisabled();
    await userEvent.click(screen.getByRole("tab", { name: "配置" }));
    expect(await screen.findByLabelText("PORT")).toBeEnabled();
    expect(api.configDraft).toHaveBeenCalledWith("redis-1");
    expect(api.startLogStream).not.toHaveBeenCalled();
  });
  it("shows applied endpoint and real network, reveals only explicitly, clears on close", async () => {
    const api = createTestApi();
    await openRedis(api);
    expect(
      await within(
        screen.getByRole("region", { name: "服务详情 Redis" }),
      ).findByText("127.0.0.1:16379"),
    ).toBeInTheDocument();
    await userEvent.click(screen.getByRole("radio", { name: "Docker 容器" }));
    expect(screen.getByText("real-network")).toBeInTheDocument();
    expect(screen.queryByText("26379")).not.toBeInTheDocument();
    expect(api.revealSecret).not.toHaveBeenCalled();
    await userEvent.click(screen.getByRole("button", { name: "显示 密码" }));
    expect(await screen.findByText("test-password")).toBeInTheDocument();
    await userEvent.click(screen.getByRole("button", { name: "关闭详情" }));
    await waitFor(() =>
      expect(screen.queryByText("test-password")).not.toBeInTheDocument(),
    );
    await userEvent.click(screen.getByRole("button", { name: "查看 Redis" }));
    expect(
      await within(
        screen.getByRole("region", { name: "服务详情 Redis" }),
      ).findByText("127.0.0.1:16379"),
    ).toBeInTheDocument();
    expect(screen.queryByText("test-password")).not.toBeInTheDocument();
  });
  it("requires draft validation and an explicit apply confirmation", async () => {
    const api = createTestApi();
    await openRedis(api);
    await userEvent.click(screen.getByRole("tab", { name: "配置" }));
    expect(await screen.findByLabelText("PORT")).toHaveValue("26379");
    fireEvent.change(screen.getByLabelText("PORT"), {
      target: { value: "26380" },
    });
    await userEvent.click(
      screen.getByRole("button", { name: "保存草稿并验证" }),
    );
    await waitFor(() =>
      expect(api.validateDraft).toHaveBeenCalledWith(
        expect.objectContaining({
          baseGeneration: "applied-1",
          values: { PORT: "26380" },
        }),
      ),
    );
    expect(api.applyDraft).not.toHaveBeenCalled();
    await userEvent.click(screen.getByRole("button", { name: "预览并应用" }));
    expect(
      await screen.findByText("应用将重建受影响的托管组件"),
    ).toBeInTheDocument();
    await userEvent.click(screen.getByRole("button", { name: "确认应用" }));
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
    await userEvent.click(screen.getByRole("tab", { name: "依赖与数据" }));
    await userEvent.click(screen.getByRole("button", { name: "预览重置计划" }));
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
    await userEvent.click(
      await screen.findByRole("button", { name: "端口与日志" }),
    );
    await userEvent.click(
      await screen.findByRole("button", { name: "预览诊断" }),
    );
    expect(await screen.findByText("已脱敏摘要")).toBeInTheDocument();
    expect(api.exportDiagnostics).not.toHaveBeenCalled();
    await userEvent.click(screen.getByRole("button", { name: "确认导出" }));
    await waitFor(() =>
      expect(api.exportDiagnostics).toHaveBeenCalledWith({
        previewId: "preview",
        excludedSections: [],
        includeLogs: false,
      }),
    );
    expect(await screen.findByText("诊断内容已导出")).toBeInTheDocument();
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
    await userEvent.click(
      await screen.findByRole("button", { name: "显示 密码" }),
    );
    await userEvent.click(screen.getByRole("tab", { name: "配置" }));
    await act(async () => {
      resolve("late-secret");
    });
    await userEvent.click(screen.getByRole("tab", { name: "连接与入口" }));
    await within(
      screen.getByRole("region", { name: "服务详情 Redis" }),
    ).findByText("127.0.0.1:16379");
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
    expect(screen.getByRole("tab", { name: "配置" })).toBeDisabled();
    expect(screen.getByRole("tab", { name: "日志" })).toBeDisabled();
    expect(api.configDraft).not.toHaveBeenCalled();
    expect(api.startLogStream).not.toHaveBeenCalled();
    expect(
      screen.queryByRole("button", { name: "重新启动并验证" }),
    ).not.toBeInTheDocument();
    await userEvent.click(screen.getByRole("tab", { name: "依赖与数据" }));
    expect(
      screen.queryByRole("button", { name: "预览重置计划" }),
    ).not.toBeInTheDocument();
    expect(
      screen.queryByRole("button", { name: "停止服务（保留数据）" }),
    ).not.toBeInTheDocument();
    expect(screen.getAllByText("外部 · 仅观察").length).toBeGreaterThan(0);
  });
  it("closes a pending stop confirmation when the instance becomes external", async () => {
    const api = createTestApi();
    const { fixtureSnapshot } = await import("../../test-fixtures");
    await openRedis(api);
    await userEvent.click(
      screen.getByRole("button", { name: "停止服务（保留数据）" }),
    );
    await screen.findByRole("button", { name: "确认停止" });
    vi.mocked(api.dashboardSnapshot).mockResolvedValue({
      ...fixtureSnapshot,
      services: fixtureSnapshot.services.map((service) =>
        service.serviceId === "redis"
          ? { ...service, ownership: "External", phase: "ExternalObserve" }
          : service,
      ),
    });
    await act(async () => {
      window.dispatchEvent(new Event("focus"));
    });
    await waitFor(() =>
      expect(
        screen.queryByRole("button", { name: "确认停止" }),
      ).not.toBeInTheDocument(),
    );
    expect(api.stopServices).not.toHaveBeenCalled();
  });
  it("disconnects an open log view when the instance becomes external", async () => {
    const api = createTestApi();
    const { fixtureSnapshot } = await import("../../test-fixtures");
    await openRedis(api);
    await userEvent.click(screen.getByRole("tab", { name: "日志" }));
    await waitFor(() => expect(api.startLogStream).toHaveBeenCalledTimes(1));
    vi.mocked(api.dashboardSnapshot).mockResolvedValue({
      ...fixtureSnapshot,
      services: fixtureSnapshot.services.map((service) =>
        service.serviceId === "redis"
          ? { ...service, ownership: "External", phase: "ExternalObserve" }
          : service,
      ),
    });
    await act(async () => {
      window.dispatchEvent(new Event("focus"));
    });
    await waitFor(() =>
      expect(screen.getByRole("tab", { name: "连接与入口" })).toHaveAttribute(
        "data-state",
        "active",
      ),
    );
    expect(screen.getByRole("tab", { name: "日志" })).toBeDisabled();
    await waitFor(() =>
      expect(api.stopLogStream).toHaveBeenCalledWith("stream-1"),
    );
    expect(api.startLogStream).toHaveBeenCalledTimes(1);
  });
  it("does not offer external instances in the global log view", async () => {
    const api = createTestApi();
    const { fixtureSnapshot } = await import("../../test-fixtures");
    api.dashboardSnapshot = vi.fn().mockResolvedValue({
      ...fixtureSnapshot,
      services: fixtureSnapshot.services.map((service) =>
        service.serviceId === "redis"
          ? {
              ...service,
              ownership: "ExternalConnected",
              phase: "ExternalObserve",
            }
          : service,
      ),
    });
    render(<App api={api} />);
    await userEvent.click(
      await screen.findByRole("button", { name: "端口与日志" }),
    );
    await screen.findByText("还没有可查看的实例");
    expect(api.startLogStream).not.toHaveBeenCalled();
    expect(
      screen.queryByRole("combobox", { name: "选择日志服务" }),
    ).not.toBeInTheDocument();
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
  await userEvent.click(screen.getByRole("tab", { name: "依赖与数据" }));
  await userEvent.click(screen.getByRole("button", { name: "预览重置计划" }));
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
  await screen.findByRole("region", { name: "服务详情 Redis" });
  await user.keyboard("{Escape}");
  await waitFor(() =>
    expect(
      screen.queryByRole("region", { name: "服务详情 Redis" }),
    ).not.toBeInTheDocument(),
  );
  await waitFor(() => expect(trigger).toHaveFocus());
});
