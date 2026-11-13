import {
  act,
  fireEvent,
  render as renderView,
  screen,
  waitFor,
} from "@testing-library/react";
import type { ReactNode } from "react";
import { GlobalFeedback } from "../GlobalFeedback";
import { ThemeProvider } from "../../lib/theme";
import { toast } from "sonner";
import { it, expect, vi } from "vitest";
import { WorkspaceProvider } from "../../lib/workspace";
import { LogViewer, appendLogBatch } from "../LogViewer";
import { createTestApi } from "../../test-fixtures";
import type { LogBatch } from "../../types/toolbox";
function render(children: ReactNode) {
  return renderView(
    <ThemeProvider>
      <GlobalFeedback />
      {children}
    </ThemeProvider>,
  );
}
it("ACKs only the current stream after its lines are committed to the DOM", async () => {
  const api = createTestApi();
  let receive!: (batch: LogBatch) => void;
  let committed = false;
  api.startLogStream = vi.fn().mockImplementation(async (_request, onBatch) => {
    receive = onBatch;
    return { streamId: "stream-1", phase: "Active" };
  });
  api.ackLogBatch = vi.fn().mockImplementation(async () => {
    committed = !!screen.queryByText("render-before-ack");
  });
  const { unmount } = render(
    <WorkspaceProvider api={api}>
      <LogViewer instanceId="redis-1" />
    </WorkspaceProvider>,
  );
  await waitFor(() => expect(api.startLogStream).toHaveBeenCalled());
  await act(async () => {
    receive({
      streamId: "stream-1",
      batchId: 1,
      fromSeq: 1,
      lines: ["render-before-ack"],
      droppedCount: 3,
    });
    expect(api.ackLogBatch).not.toHaveBeenCalled();
  });
  expect(committed).toBe(true);
  expect(screen.getByText("已丢弃 / 截断 3 行")).toBeInTheDocument();
  expect(api.ackLogBatch).toHaveBeenCalledWith({
    streamId: "stream-1",
    batchId: 1,
  });
  await act(async () => {
    receive({
      streamId: "stream-1",
      batchId: 1,
      fromSeq: 1,
      lines: ["duplicate"],
      droppedCount: 0,
    });
    receive({
      streamId: "old-stream",
      batchId: 2,
      fromSeq: 2,
      lines: ["old-stream-line"],
      droppedCount: 0,
    });
  });
  expect(api.ackLogBatch).toHaveBeenCalledTimes(1);
  expect(screen.queryByText("duplicate")).not.toBeInTheDocument();
  expect(screen.queryByText("old-stream-line")).not.toBeInTheDocument();
  unmount();
  expect(api.stopLogStream).toHaveBeenCalledWith("stream-1");
});
it("bounds rendered lines and bytes, rejects oversize batches without ACK credit", () => {
  let ring = {
    lines: [] as string[],
    bytes: 0,
    dropped: 0,
    acks: [] as { streamId: string; batchId: number }[],
  };
  for (let i = 0; i < 25; i++)
    ring = appendLogBatch(ring, {
      streamId: "s",
      batchId: i,
      fromSeq: i * 100,
      lines: Array(100).fill("x".repeat(600)),
      droppedCount: 0,
    });
  expect(ring.lines).toHaveLength(2000);
  expect(ring.dropped).toBe(500);
  expect(ring.bytes).toBeLessThanOrEqual(2097152);
  expect(ring.acks).toHaveLength(2);
  let byteRing = {
    lines: [] as string[],
    bytes: 0,
    dropped: 0,
    acks: [] as { streamId: string; batchId: number }[],
  };
  for (let i = 0; i < 50; i++)
    byteRing = appendLogBatch(byteRing, {
      streamId: "bytes",
      batchId: i,
      fromSeq: i * 4,
      lines: Array(4).fill("你".repeat(5000)),
      droppedCount: 0,
    });
  expect(byteRing.lines.length).toBeLessThan(200);
  expect(byteRing.dropped).toBeGreaterThan(0);
  expect(byteRing.bytes).toBeLessThanOrEqual(2097152);
  const previous = ring;
  ring = appendLogBatch(ring, {
    streamId: "s",
    batchId: 26,
    fromSeq: 2500,
    lines: ["x".repeat(17000)],
    droppedCount: 0,
  });
  expect(ring).toBe(previous);
  for (let i = 30; i < 75; i++)
    ring = appendLogBatch(ring, {
      streamId: "s",
      batchId: i,
      fromSeq: i * 100,
      lines: Array(100).fill("你".repeat(200)),
      droppedCount: 0,
    });
  expect(ring.bytes).toBeLessThanOrEqual(2097152);
});
it("cleans up a stream whose startup finishes after the detail closes", async () => {
  const api = createTestApi();
  let resolve!: (value: { streamId: string; phase: string }) => void;
  api.startLogStream = vi.fn().mockImplementation(
    () =>
      new Promise((r) => {
        resolve = r;
      }),
  );
  const { unmount } = render(
    <WorkspaceProvider api={api}>
      <LogViewer instanceId="redis-1" />
    </WorkspaceProvider>,
  );
  await waitFor(() => expect(api.startLogStream).toHaveBeenCalledTimes(1));
  unmount();
  await act(async () => {
    resolve({ streamId: "late", phase: "Active" });
  });
  expect(api.stopLogStream).toHaveBeenCalledWith("late");
  expect(api.ackLogBatch).not.toHaveBeenCalled();
});

it("shows inactivity without claiming backend expiry, and resets the clock on reconnect", async () => {
  vi.useFakeTimers();
  try {
    const api = createTestApi();
    const { unmount } = render(
      <WorkspaceProvider api={api}>
        <LogViewer instanceId="redis-1" />
      </WorkspaceProvider>,
    );
    await act(async () => {});
    await act(async () => {
      vi.advanceTimersByTime(31000);
    });
    await act(async () => {
      await vi.advanceTimersByTimeAsync(1);
    });
    expect(
      screen.getByText("30 秒未收到新批次；可重新连接读取最近日志。"),
    ).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "重新连接日志" }));
    await act(async () => {});
    await act(async () => {
      await vi.advanceTimersByTimeAsync(400);
    });
    expect(toast.getToasts().some((item) => item.id === "log-idle")).toBe(
      false,
    );
    expect(
      screen.getByRole("status", { name: "日志订阅状态" }),
    ).toHaveTextContent("Active");
    unmount();
  } finally {
    vi.useRealTimers();
  }
});

it("subscribes before opening the stream and renders actual pause/resume state", async () => {
  const api = createTestApi();
  let emit!: (event: import("../../types/toolbox").LogStreamStateEvent) => void;
  let complete!: (stop: () => void) => void;
  const unlisten = vi.fn();
  api.subscribeLogStreamState = vi.fn().mockImplementation((callback) => {
    emit = callback;
    return new Promise((resolve) => {
      complete = resolve;
    });
  });
  const { unmount } = render(
    <WorkspaceProvider api={api}>
      <LogViewer instanceId="redis-1" />
    </WorkspaceProvider>,
  );
  await waitFor(() =>
    expect(api.subscribeLogStreamState).toHaveBeenCalledTimes(1),
  );
  expect(api.startLogStream).not.toHaveBeenCalled();
  await act(async () => {
    complete(unlisten);
  });
  expect(
    screen.getByRole("status", { name: "日志订阅状态" }),
  ).toHaveTextContent("Active");
  await act(async () => {
    emit({
      streamId: "stream-1",
      phase: "AwaitingConsumer",
      messageCode: null,
    });
  });
  expect(
    screen.getByRole("status", { name: "日志订阅状态" }),
  ).toHaveTextContent("AwaitingConsumer");
  expect(
    await screen.findByText("日志已暂停，等待渲染确认"),
  ).toBeInTheDocument();
  await act(async () => {
    emit({ streamId: "stream-1", phase: "Active", messageCode: null });
  });
  expect(
    screen.getByRole("status", { name: "日志订阅状态" }),
  ).toHaveTextContent("Active");
  unmount();
  expect(unlisten).toHaveBeenCalledTimes(1);
});
it("preserves terminal expiry received before the start response and refuses late ACK credit", async () => {
  const api = createTestApi();
  let emit!: (event: import("../../types/toolbox").LogStreamStateEvent) => void;
  let receive!: (batch: LogBatch) => void;
  api.subscribeLogStreamState = vi.fn().mockImplementation(async (callback) => {
    emit = callback;
    return () => {};
  });
  api.startLogStream = vi.fn().mockImplementation(async (_request, onBatch) => {
    receive = onBatch;
    emit({
      streamId: "stream-1",
      phase: "Expired",
      messageCode: "LogStreamExpired",
    });
    return { streamId: "stream-1", phase: "Active" };
  });
  render(
    <WorkspaceProvider api={api}>
      <LogViewer instanceId="redis-1" />
    </WorkspaceProvider>,
  );
  expect(await screen.findByText(/LogStreamExpired/)).toBeInTheDocument();
  expect(
    screen.getByRole("status", { name: "日志订阅状态" }),
  ).toHaveTextContent("Expired");
  await act(async () => {
    receive({
      streamId: "stream-1",
      batchId: 1,
      fromSeq: 1,
      lines: ["late unrendered batch"],
      droppedCount: 0,
    });
    emit({ streamId: "stream-1", phase: "Active", messageCode: null });
  });
  expect(api.ackLogBatch).not.toHaveBeenCalled();
  expect(screen.queryByText("late unrendered batch")).not.toBeInTheDocument();
  expect(
    screen.getByRole("status", { name: "日志订阅状态" }),
  ).toHaveTextContent("Expired");
});
it("shows backend read failure, ignores old streams and stops ACKs for batches not committed before expiry", async () => {
  const api = createTestApi();
  const callbacks: Array<
    (event: import("../../types/toolbox").LogStreamStateEvent) => void
  > = [];
  const unsubscribers = [vi.fn(), vi.fn()];
  const batches: Array<(batch: LogBatch) => void> = [];
  api.subscribeLogStreamState = vi.fn().mockImplementation(async (callback) => {
    callbacks.push(callback);
    return unsubscribers[callbacks.length - 1];
  });
  api.startLogStream = vi.fn().mockImplementation(async (_request, onBatch) => {
    batches.push(onBatch);
    return { streamId: `stream-${batches.length}`, phase: "Active" };
  });
  render(
    <WorkspaceProvider api={api}>
      <LogViewer instanceId="redis-1" />
    </WorkspaceProvider>,
  );
  await waitFor(() =>
    expect(api.subscribeLogStreamState).toHaveBeenCalledTimes(1),
  );
  await waitFor(() => expect(batches).toHaveLength(1));
  await act(async () => {
    batches[0]({
      streamId: "stream-1",
      batchId: 1,
      fromSeq: 1,
      lines: ["not-yet-committed"],
      droppedCount: 0,
    });
    callbacks[0]({
      streamId: "stream-1",
      phase: "Expired",
      messageCode: "LogStreamExpired",
    });
    expect(api.ackLogBatch).not.toHaveBeenCalled();
  });
  expect(api.ackLogBatch).not.toHaveBeenCalled();
  fireEvent.click(screen.getByRole("button", { name: "重新连接日志" }));
  await waitFor(() => expect(batches).toHaveLength(2));
  expect(unsubscribers[0]).toHaveBeenCalledTimes(1);
  await act(async () => {
    callbacks[1]({
      streamId: "stream-1",
      phase: "Failed",
      messageCode: "OldStreamFailure",
    });
    callbacks[0]({
      streamId: "stream-2",
      phase: "Expired",
      messageCode: "StaleListenerFailure",
    });
  });
  expect(
    screen.getByRole("status", { name: "日志订阅状态" }),
  ).toHaveTextContent("Active");
  expect(screen.queryByText("OldStreamFailure")).not.toBeInTheDocument();
  await act(async () => {
    callbacks[1]({
      streamId: "stream-2",
      phase: "Failed",
      messageCode: "LogReadFailed",
    });
  });
  expect(
    screen.getByRole("status", { name: "日志订阅状态" }),
  ).toHaveTextContent("Failed");
  expect(await screen.findByText(/LogReadFailed/)).toBeInTheDocument();
  expect(api.ackLogBatch).not.toHaveBeenCalled();
});
it("cleans up delayed state subscription without opening a closed view stream", async () => {
  const api = createTestApi();
  let resolve!: (stop: () => void) => void;
  const stop = vi.fn();
  api.subscribeLogStreamState = vi.fn().mockImplementation(
    () =>
      new Promise((r) => {
        resolve = r;
      }),
  );
  const { unmount } = render(
    <WorkspaceProvider api={api}>
      <LogViewer instanceId="redis-1" />
    </WorkspaceProvider>,
  );
  await waitFor(() =>
    expect(api.subscribeLogStreamState).toHaveBeenCalledTimes(1),
  );
  unmount();
  await act(async () => {
    resolve(stop);
  });
  expect(stop).toHaveBeenCalledTimes(1);
  expect(api.startLogStream).not.toHaveBeenCalled();
});

it("does not start an unobservable stream when the state listener cannot subscribe", async () => {
  const api = createTestApi();
  api.subscribeLogStreamState = vi.fn().mockRejectedValue({
    code: "LogSubscriptionUnavailable",
    retryable: true,
    redactedContext: "",
  });
  render(
    <WorkspaceProvider api={api}>
      <LogViewer instanceId="redis-1" />
    </WorkspaceProvider>,
  );
  expect(
    await screen.findByText(/LogSubscriptionUnavailable/),
  ).toBeInTheDocument();
  expect(
    screen.getByRole("status", { name: "日志订阅状态" }),
  ).toHaveTextContent("Unavailable");
  expect(api.startLogStream).not.toHaveBeenCalled();
});
it("accumulates dropped-count deltas only once and displays normal stopped state", async () => {
  const api = createTestApi();
  let receive!: (batch: LogBatch) => void;
  let emit!: (event: import("../../types/toolbox").LogStreamStateEvent) => void;
  api.subscribeLogStreamState = vi.fn().mockImplementation(async (callback) => {
    emit = callback;
    return () => {};
  });
  api.startLogStream = vi.fn().mockImplementation(async (_request, onBatch) => {
    receive = onBatch;
    return { streamId: "stream-1", phase: "Active" };
  });
  render(
    <WorkspaceProvider api={api}>
      <LogViewer instanceId="redis-1" />
    </WorkspaceProvider>,
  );
  await waitFor(() => expect(api.startLogStream).toHaveBeenCalledTimes(1));
  await act(async () => {
    receive({
      streamId: "stream-1",
      batchId: 1,
      fromSeq: 1,
      lines: ["first"],
      droppedCount: 2,
    });
  });
  await act(async () => {
    receive({
      streamId: "stream-1",
      batchId: 2,
      fromSeq: 2,
      lines: ["second"],
      droppedCount: 3,
    });
    receive({
      streamId: "stream-1",
      batchId: 2,
      fromSeq: 2,
      lines: ["duplicate"],
      droppedCount: 3,
    });
  });
  expect(screen.getByText("已丢弃 / 截断 5 行")).toBeInTheDocument();
  expect(api.ackLogBatch).toHaveBeenCalledTimes(2);
  await act(async () => {
    emit({ streamId: "stream-1", phase: "Stopped", messageCode: null });
  });
  expect(
    screen.getByRole("status", { name: "日志订阅状态" }),
  ).toHaveTextContent("Stopped");
  expect(await screen.findByText("日志订阅已停止")).toBeInTheDocument();
});

it("retains the backend failure code when both the early event and start handle are terminal", async () => {
  const api = createTestApi();
  let emit!: (event: import("../../types/toolbox").LogStreamStateEvent) => void;
  api.subscribeLogStreamState = vi.fn().mockImplementation(async (callback) => {
    emit = callback;
    return () => {};
  });
  api.startLogStream = vi.fn().mockImplementation(async () => {
    emit({
      streamId: "stream-1",
      phase: "Failed",
      messageCode: "LogReaderExited",
    });
    return { streamId: "stream-1", phase: "Failed" };
  });
  render(
    <WorkspaceProvider api={api}>
      <LogViewer instanceId="redis-1" />
    </WorkspaceProvider>,
  );
  expect(await screen.findByText(/LogReaderExited/)).toBeInTheDocument();
  expect(
    screen.getByRole("status", { name: "日志订阅状态" }),
  ).toHaveTextContent("Failed");
  expect(api.ackLogBatch).not.toHaveBeenCalled();
});

it("waits for the previous stream to stop before an explicit reconnect starts another", async () => {
  const api = createTestApi();
  let release!: (value: void) => void;
  api.stopLogStream = vi.fn().mockImplementation(
    () =>
      new Promise<void>((resolve) => {
        release = resolve;
      }),
  );
  const { unmount } = render(
    <WorkspaceProvider api={api}>
      <LogViewer instanceId="redis-1" />
    </WorkspaceProvider>,
  );
  await waitFor(() => expect(api.startLogStream).toHaveBeenCalledTimes(1));
  fireEvent.click(screen.getByRole("button", { name: "重新连接日志" }));
  await act(async () => {});
  expect(api.startLogStream).toHaveBeenCalledTimes(1);
  await act(async () => {
    release();
  });
  await waitFor(() => expect(api.startLogStream).toHaveBeenCalledTimes(2));
  unmount();
});
