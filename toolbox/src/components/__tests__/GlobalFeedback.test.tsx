import {
  act,
  fireEvent,
  render,
  screen,
  waitFor,
  within,
} from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { expect, it, vi } from "vitest";
import { toast } from "sonner";
import App from "../../App";
import { AppErrorBoundary, GlobalFeedback } from "../GlobalFeedback";
import { ThemeProvider } from "../../lib/theme";
import { WorkspaceProvider, useWorkspace } from "../../lib/workspace";
import { createTestApi, fixtureSnapshot } from "../../test-fixtures";

it("shows detection immediately, prevents duplicate requests, and replaces progress with the native result", async () => {
  const api = createTestApi();
  let finish!: (value: typeof fixtureSnapshot) => void;
  api.detectEnvironment = vi.fn().mockImplementation(
    () =>
      new Promise((resolve) => {
        finish = resolve;
      }),
  );
  render(<App api={api} />);
  await userEvent.click(await screen.findByRole("button", { name: "设置" }));
  const detect = screen.getByRole("button", { name: "检测连接" });
  await userEvent.click(detect);
  await screen.findByText("正在检测 Docker 连接…");
  expect(detect).toBeDisabled();
  fireEvent.click(detect);
  expect(api.detectEnvironment).toHaveBeenCalledTimes(1);
  await act(async () => finish(fixtureSnapshot));
  await screen.findByText("Docker 连接已确认");
  expect(screen.getByRole("button", { name: "检测连接" })).toBeEnabled();
  expect(document.querySelector('main [role="alert"]')).toBeNull();
});

it("announces repeated resolved detection failures even when a background incident is unchanged", async () => {
  const api = createTestApi();
  const missing = {
    ...fixtureSnapshot,
    target: null,
    environmentError: "DockerNotInstalled",
  };
  api.dashboardSnapshot = vi.fn().mockResolvedValue(missing);
  api.detectEnvironment = vi.fn().mockResolvedValue(missing);
  const reports = vi.spyOn(toast, "error");
  try {
    render(<App api={api} />);
    await screen.findByText("尚未安装 Docker");
    await userEvent.click(screen.getByRole("button", { name: "设置" }));
    await userEvent.click(screen.getByRole("button", { name: "检测连接" }));
    await waitFor(() => expect(api.detectEnvironment).toHaveBeenCalledTimes(1));
    await userEvent.click(screen.getByRole("button", { name: "检测连接" }));
    await waitFor(() => expect(api.detectEnvironment).toHaveBeenCalledTimes(2));
    expect(
      reports.mock.calls.filter((call) =>
        call[1]?.id?.toString().startsWith("environment-detection:"),
      ),
    ).toHaveLength(2);
    expect(screen.queryByText("Docker 连接已确认")).not.toBeInTheDocument();
  } finally {
    reports.mockRestore();
  }
});

it("starts a preset directly from the catalog after Docker is confirmed", async () => {
  const api = createTestApi();
  render(<App api={api} />);
  await userEvent.click(
    await screen.findByRole("button", { name: "全部服务" }),
  );
  const start = await screen.findByRole("button", { name: "启动 Redis" });
  await userEvent.click(start);
  await waitFor(() =>
    expect(api.startServices).toHaveBeenCalledWith(
      expect.objectContaining({ serviceIds: ["redis"] }),
    ),
  );
  expect(
    screen.queryByRole("region", { name: "服务详情 Redis" }),
  ).not.toBeInTheDocument();
});

function Driver() {
  const { refresh, state, feedback } = useWorkspace();
  return (
    <main>
      <button onClick={() => void refresh()}>刷新快照</button>
      <button onClick={() => void feedback.copy("private-value")}>
        复制数据
      </button>
      <output aria-label="连接状态">{state.error ?? "connected"}</output>
    </main>
  );
}

it("deduplicates a connection incident across refreshes, clears it on recovery, and reports a later incident", async () => {
  const api = createTestApi();
  const reject = {
    code: "BackendUnavailable",
    message: "secret=dont-render-this",
  };
  api.dashboardSnapshot = vi.fn().mockRejectedValue(reject);
  const report = vi.spyOn(toast, "error");
  try {
    render(
      <ThemeProvider>
        <GlobalFeedback />
        <WorkspaceProvider api={api}>
          <Driver />
        </WorkspaceProvider>
      </ThemeProvider>,
    );
    await screen.findByText("连接工作箱后端失败");
    expect(document.querySelector("main [data-sonner-toast]")).toBeNull();
    expect(document.querySelectorAll("[data-sonner-toaster]")).toHaveLength(1);
    await userEvent.click(screen.getByText("刷新快照"));
    await userEvent.click(screen.getByText("刷新快照"));
    expect(report).toHaveBeenCalledTimes(1);
    expect(screen.queryByText(/dont-render-this/)).not.toBeInTheDocument();
    vi.mocked(api.dashboardSnapshot).mockResolvedValue(fixtureSnapshot);
    await userEvent.click(screen.getByText("刷新快照"));
    await screen.findByText("已重新连接工作箱后端");
    await waitFor(() =>
      expect(screen.queryByText("连接工作箱后端失败")).not.toBeInTheDocument(),
    );
    expect(screen.getByLabelText("连接状态")).toHaveTextContent("connected");
    vi.mocked(api.dashboardSnapshot).mockRejectedValue(reject);
    await userEvent.click(screen.getByText("刷新快照"));
    await screen.findByText("连接工作箱后端失败");
    expect(report).toHaveBeenCalledTimes(2);
  } finally {
    report.mockRestore();
  }
});

it("reports a command rejection once without locking otherwise connected service actions", async () => {
  const api = createTestApi();
  const check = api.preflightService;
  api.preflightService = vi
    .fn()
    .mockRejectedValueOnce({
      code: "PortConflict",
      message: "password=do-not-show",
    })
    .mockImplementation(check);
  const report = vi.spyOn(toast, "error");
  try {
    render(<App api={api} />);
    const start = await screen.findByRole("button", { name: "启动 Redis" });
    await userEvent.click(start);
    await screen.findByText(/PortConflict/);
    expect(report).toHaveBeenCalledTimes(1);
    expect(screen.queryByText(/do-not-show/)).not.toBeInTheDocument();
    expect(document.querySelector('main [role="alert"]')).toBeNull();
    expect(start).toBeEnabled();
    await userEvent.click(start);
    await waitFor(() => expect(api.startServices).toHaveBeenCalledTimes(1));
    await screen.findByText("启动任务已提交");
    expect(screen.queryByText("服务操作已完成")).not.toBeInTheDocument();
  } finally {
    report.mockRestore();
  }
});

it("uses global notifications for clipboard success and failure without exposing copied content", async () => {
  const write = vi
    .fn()
    .mockResolvedValueOnce(undefined)
    .mockRejectedValueOnce(new Error("private-value"));
  Object.defineProperty(navigator, "clipboard", {
    configurable: true,
    value: { writeText: write },
  });
  const api = createTestApi();
  render(
    <ThemeProvider>
      <GlobalFeedback />
      <WorkspaceProvider api={api}>
        <Driver />
      </WorkspaceProvider>
    </ThemeProvider>,
  );
  fireEvent.click(screen.getByText("复制数据"));
  await screen.findByText("已复制");
  fireEvent.click(screen.getByText("复制数据"));
  await screen.findByText("无法访问剪贴板");
  expect(write).toHaveBeenCalledWith("private-value");
  expect(screen.queryByText("private-value")).not.toBeInTheDocument();
  expect(
    within(screen.getByRole("main")).queryByText("已复制"),
  ).not.toBeInTheDocument();
});

it("handles unhandled promises and runtime exceptions through the single global host", async () => {
  render(<App api={createTestApi()} />);
  await screen.findByRole("button", { name: "启动 Redis" });
  const rejection = new Event("unhandledrejection", { cancelable: true });
  Object.defineProperty(rejection, "reason", {
    value: new Error("unsafe-runtime-payload"),
  });
  act(() => window.dispatchEvent(rejection));
  await screen.findByText("界面发生异常");
  const exception = new ErrorEvent("error", {
    cancelable: true,
    message: "unsafe-runtime-payload",
    error: new Error("unsafe-runtime-payload"),
  });
  act(() => window.dispatchEvent(exception));
  await waitFor(() =>
    expect(screen.getAllByText("界面发生异常")).toHaveLength(1),
  );
  expect(rejection.defaultPrevented).toBe(true);
  expect(exception.defaultPrevented).toBe(true);
  expect(screen.queryByText(/unsafe-runtime-payload/)).not.toBeInTheDocument();
});

it("keeps the global host available after a React render failure", async () => {
  function Broken() {
    throw new Error("render-secret");
    return null;
  }
  const consoleError = vi.spyOn(console, "error").mockImplementation(() => {});
  try {
    render(
      <ThemeProvider>
        <GlobalFeedback />
        <AppErrorBoundary>
          <Broken />
        </AppErrorBoundary>
      </ThemeProvider>,
    );
    await screen.findByText("页面加载失败");
    expect(
      screen.getAllByRole("button", { name: "重新加载页面" }).length,
    ).toBeGreaterThan(0);
    expect(document.querySelectorAll("[data-sonner-toaster]")).toHaveLength(1);
    expect(screen.queryByText(/render-secret/)).not.toBeInTheDocument();
  } finally {
    consoleError.mockRestore();
  }
});

it("presents failed preflight results globally and never submits a blocked start", async () => {
  const api = createTestApi();
  const report = await api.preflightService("redis");
  api.preflightService = vi.fn().mockResolvedValue({
    ...report,
    issues: [
      {
        severity: "error",
        code: "PortConflict",
        field: "secret-field-value",
      },
    ],
  });
  render(<App api={api} />);
  await userEvent.click(
    await screen.findByRole("button", { name: "启动 Redis" }),
  );
  await screen.findByText(/PortConflict/);
  expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
  expect(screen.queryByText("secret-field-value")).not.toBeInTheDocument();
  expect(api.startServices).not.toHaveBeenCalled();
});
