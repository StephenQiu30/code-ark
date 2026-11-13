import userEvent from "@testing-library/user-event";
import {
  act,
  render,
  screen,
  waitFor,
  fireEvent,
} from "@testing-library/react";
import { describe, it, expect, vi } from "vitest";
import App from "../../App";
import {
  createTestApi,
  fixtureCatalog,
  fixtureSnapshot,
} from "../../test-fixtures";

describe("workspace", () => {
  it("explains an unaccepted native runtime without reporting backend disconnection", async () => {
    const api = createTestApi();
    api.preflightService = vi
      .fn()
      .mockRejectedValue({ code: "NativeRuntimeAcceptanceRequired" });
    render(<App api={api} />);
    await userEvent.click(
      await screen.findByRole("button", { name: "启动 Redis" }),
    );
    expect(
      await screen.findByText("当前平台尚未完成运行验收"),
    ).toBeInTheDocument();
    expect(screen.queryByText("连接工作箱后端失败")).not.toBeInTheDocument();
    expect(api.startServices).not.toHaveBeenCalled();
  });
  it("shows Redis first success without starting anything on selection", async () => {
    const api = createTestApi();
    render(<App api={api} />);
    expect(
      await screen.findByRole("button", { name: "启动 Redis" }),
    ).toBeEnabled();
    expect(
      screen.getByRole("heading", { name: "按项目开始" }),
    ).toBeInTheDocument();
    await userEvent.click(
      screen.getByRole("button", { name: "查看 Backend MySQL" }),
    );
    expect(await screen.findByRole("dialog")).toBeInTheDocument();
    expect(api.startServices).not.toHaveBeenCalled();
  });
  it("requires explicit unknown-capacity consent and uses authoritative unverified phase", async () => {
    const api = createTestApi();
    api.preflightService = vi.fn().mockResolvedValue({
      issues: [],
      capacity: {
        workspaceFreeBytes: null,
        daemonFreeBytes: null,
        daemonStatus: "Unknown",
      },
      requiredMemoryBytes: 0,
      daemonMemoryBytes: 0,
      requiresCapacityConsent: true,
    });
    render(<App api={api} />);
    await userEvent.click(
      await screen.findByRole("button", { name: "启动 Redis" }),
    );
    expect(await screen.findByText("无法确认可用磁盘容量")).toBeInTheDocument();
    expect(api.startServices).not.toHaveBeenCalled();
    await userEvent.click(screen.getByRole("button", { name: "仅本次继续" }));
    await waitFor(() =>
      expect(api.startServices).toHaveBeenCalledWith(
        expect.objectContaining({
          serviceIds: ["redis"],
          acceptUnknownCapacity: true,
        }),
      ),
    );
    expect(screen.queryByText("已就绪")).not.toBeInTheDocument();
  });
  it("displays missing Docker repair and rechecks without starting a service", async () => {
    const api = createTestApi();
    api.dashboardSnapshot = vi.fn().mockResolvedValue({
      ...fixtureSnapshot,
      target: null,
      environmentError: "DockerNotInstalled",
    });
    render(<App api={api} />);
    expect(await screen.findByText("尚未安装 Docker")).toBeInTheDocument();
    await userEvent.click(
      screen.getByRole("button", { name: "Docker 官方安装指引" }),
    );
    await waitFor(() =>
      expect(api.openOfficial).toHaveBeenCalledWith("docker-install"),
    );
    await userEvent.click(screen.getByRole("button", { name: "设置" }));
    await userEvent.click(screen.getByRole("button", { name: "检测连接" }));
    await waitFor(() => expect(api.detectEnvironment).toHaveBeenCalledTimes(1));
    expect(api.startServices).not.toHaveBeenCalled();
  });
  it("restores operations from snapshot and ignores duplicated events", async () => {
    const api = createTestApi();
    api.dashboardSnapshot = vi.fn().mockResolvedValue({
      ...fixtureSnapshot,
      operations: [
        {
          operationId: "op1",
          targetId: "target",
          instanceIds: ["redis-1"],
          requestDigest: "d",
          desiredGeneration: null,
          appliedGeneration: null,
          lastReadyGeneration: null,
          phase: "Failed",
          steps: [
            {
              stepId: "pull",
              phase: "Failed",
              messageCode: "DownloadFailed",
              elapsedMs: 2000,
              deadlineAt: null,
            },
          ],
          createdResourceIds: ["volume-1"],
          errorCode: "DownloadFailed",
          createdAt: 1,
          seq: 8,
          allowedActions: ["retry"],
        },
      ],
    });
    render(<App api={api} />);
    await userEvent.click(
      await screen.findByRole("button", { name: "运行任务" }),
    );
    expect(await screen.findByText(/DownloadFailed/)).toBeInTheDocument();
    expect(screen.getByText("volume-1")).toBeInTheDocument();
    await userEvent.click(screen.getByRole("button", { name: "重试" }));
    await waitFor(() => expect(api.retryOperation).toHaveBeenCalledWith("op1"));
  });
  it("does not permit starts when disconnected from desktop backend", async () => {
    const api = createTestApi();
    api.dashboardSnapshot = vi
      .fn()
      .mockRejectedValue(new Error("raw secret must not display"));
    render(<App api={api} />);
    expect(await screen.findByText("连接工作箱后端失败")).toBeInTheDocument();
    expect(document.querySelector('main [role="alert"]')).toBeNull();
    expect(
      screen.queryByText("raw secret must not display"),
    ).not.toBeInTheDocument();
  });
});

describe("scene alternatives", () => {
  it("launches exactly the explicitly selected queue rather than all alternatives", async () => {
    const api = createTestApi();
    api.serviceCatalog = vi.fn().mockResolvedValue({
      ...fixtureCatalog,
      scenes: [
        {
          id: "cdc-messaging",
          name: "消息场景",
          services: ["mysql", "canal", "kafka"],
        },
      ],
    });
    render(<App api={api} />);
    await userEvent.click(
      await screen.findByRole("button", { name: "查看 消息场景" }),
    );
    await userEvent.click(screen.getByRole("radio", { name: "RabbitMQ" }));
    await userEvent.click(screen.getByRole("button", { name: "启动" }));
    await waitFor(() =>
      expect(api.startServices).toHaveBeenCalledWith(
        expect.objectContaining({
          serviceIds: ["mysql", "canal", "rabbitmq"],
          sceneId: "cdc-messaging",
        }),
      ),
    );
  });
});

it("ignores events older than the loaded operation snapshot and never submits from events", async () => {
  const api = createTestApi();
  let event!: (value: import("../../types/toolbox").StepProgressEvent) => void;
  api.subscribeProgress = vi.fn().mockImplementation(async (callback) => {
    event = callback;
    return () => {};
  });
  api.dashboardSnapshot = vi.fn().mockResolvedValue({
    ...fixtureSnapshot,
    operations: [
      {
        operationId: "operation",
        targetId: "target",
        instanceIds: [],
        requestDigest: "d",
        desiredGeneration: null,
        appliedGeneration: null,
        lastReadyGeneration: null,
        phase: "Running",
        steps: [],
        createdResourceIds: [],
        errorCode: null,
        createdAt: 1,
        seq: 8,
        allowedActions: ["cancel"],
      },
    ],
  });
  render(<App api={api} />);
  await screen.findByRole("button", { name: "启动 Redis" });
  await act(async () => {
    event({
      operationId: "operation",
      seq: 7,
      serviceId: "redis",
      stepId: "pull",
      phase: "Running",
      messageCode: "Pulling",
    });
  });
  expect(api.dashboardSnapshot).toHaveBeenCalledTimes(1);
  expect(api.startServices).not.toHaveBeenCalled();
});

it("switches navigation and settings explanations to English without storing secrets or language", async () => {
  const api = createTestApi();
  render(<App api={api} />);
  await userEvent.click(await screen.findByRole("button", { name: "设置" }));
  await userEvent.click(screen.getByRole("radio", { name: "English" }));
  expect(screen.getByRole("button", { name: "Overview" })).toBeInTheDocument();
  expect(
    screen.getByText(
      "Switching workspaces does not move data or stop existing instances.",
    ),
  ).toBeInTheDocument();
  expect(localStorage.length).toBe(0);
});

it("recovers the catalog after the first backend connection failed", async () => {
  const api = createTestApi();
  api.serviceCatalog = vi
    .fn()
    .mockRejectedValueOnce({ code: "CatalogUnavailable" })
    .mockResolvedValue(fixtureCatalog);
  render(<App api={api} />);
  await screen.findByText(/CatalogUnavailable/);
  await userEvent.click(screen.getByRole("button", { name: "重新检测" }));
  expect(
    await screen.findByRole("heading", { name: "按项目开始" }),
  ).toBeInTheDocument();
});

it("stops a scene explicitly and passes only registered instances while keeping data", async () => {
  const api = createTestApi();
  render(<App api={api} />);
  await userEvent.click(
    await screen.findByRole("button", { name: "查看 Backend MySQL" }),
  );
  expect(api.stopServices).not.toHaveBeenCalled();
  await userEvent.click(
    screen.getByRole("button", { name: "停止场景（保留数据）" }),
  );
  await waitFor(() =>
    expect(api.stopServices).toHaveBeenCalledWith({
      workspaceId: "workspace",
      targetId: "target",
      instanceIds: ["redis-1"],
      sceneId: "backend-mysql",
    }),
  );
});

it("does not transfer one-operation capacity consent to a different Docker target", async () => {
  const api = createTestApi();
  api.preflightService = vi.fn().mockResolvedValue({
    issues: [],
    capacity: {
      workspaceFreeBytes: null,
      daemonFreeBytes: null,
      daemonStatus: "Unknown",
    },
    requiredMemoryBytes: 0,
    daemonMemoryBytes: 0,
    requiresCapacityConsent: true,
  });
  api.dashboardSnapshot = vi
    .fn()
    .mockResolvedValueOnce(fixtureSnapshot)
    .mockResolvedValue({
      ...fixtureSnapshot,
      target: { ...fixtureSnapshot.target!, targetId: "another-target" },
    });
  render(<App api={api} />);
  await userEvent.click(
    await screen.findByRole("button", { name: "启动 Redis" }),
  );
  expect(
    await screen.findByRole("button", { name: "仅本次继续" }),
  ).toBeDisabled();
  expect(api.startServices).not.toHaveBeenCalled();
});

it("offers a prominent Docker detection action on a cold launch with no target", async () => {
  const api = createTestApi();
  api.dashboardSnapshot = vi
    .fn()
    .mockResolvedValueOnce({
      ...fixtureSnapshot,
      target: null,
      environmentError: null,
    })
    .mockResolvedValue(fixtureSnapshot);
  render(<App api={api} />);
  expect(
    await screen.findByRole("button", { name: "启动 Redis" }),
  ).toBeDisabled();
  await userEvent.click(
    screen.getByRole("button", { name: "检测 Docker 环境" }),
  );
  await waitFor(() => expect(api.detectEnvironment).toHaveBeenCalledTimes(1));
  await waitFor(() =>
    expect(screen.getByRole("button", { name: "启动 Redis" })).toBeEnabled(),
  );
  expect(api.startServices).not.toHaveBeenCalled();
});
