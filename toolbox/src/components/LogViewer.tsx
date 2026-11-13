import { useT } from "../lib/i18n";
import { useCallback, useEffect, useRef, useState } from "react";
import { Button } from "./ui/button";
import { useWorkspace } from "../lib/workspace";
import { errorCode } from "../api/tauri";
import type { LogBatch, LogAck, LogStreamStateEvent } from "../types/toolbox";
type Ring = { lines: string[]; bytes: number; dropped: number; acks: LogAck[] };
const size = (value: unknown) =>
  new TextEncoder().encode(JSON.stringify(value)).byteLength;
export function appendLogBatch(current: Ring, batch: LogBatch): Ring {
  if (
    batch.lines.length > 100 ||
    size(batch) > 65536 ||
    batch.lines.some((line) => size(line) > 16384)
  )
    return current;
  let lines = [...current.lines, ...batch.lines];
  let bytes = size(lines);
  let dropped = current.dropped + batch.droppedCount;
  while (lines.length > 2000 || bytes > 2097152) {
    bytes -= size(lines.shift()!) + (lines.length ? 1 : 0);
    dropped++;
  }
  return {
    lines,
    bytes,
    dropped,
    acks: [
      ...current.acks.slice(-1),
      { streamId: batch.streamId, batchId: batch.batchId },
    ],
  };
}
export function LogViewer({ instanceId }: { instanceId: string }) {
  const { api, feedback } = useWorkspace();
  const t = useT();
  const [restart, setRestart] = useState(0);
  const [streamId, setStreamId] = useState<string | null>(null);
  const [phase, setPhase] = useState("Connecting");
  const [error, setError] = useState(false);
  const [messageCode, setMessageCode] = useState<string | null>(null);
  const [ring, setRing] = useState<Ring>({
    lines: [],
    bytes: 0,
    dropped: 0,
    acks: [],
  });
  const active = useRef<string | null>(null);
  const shutdown = useRef<Promise<void>>(Promise.resolve());
  const acknowledged = useRef(new Set<number>());
  const accepted = useRef(new Set<number>());
  const receiveTime = useRef(Date.now());
  const [idle, setIdle] = useState(0);
  const idleNotice = useRef(0);
  useEffect(() => {
    const previousShutdown = shutdown.current;
    let disposed = false;
    let handleId: string | null = null;
    let unlisten: (() => void) | undefined;
    let terminal = false;
    let stateOverflow = false;
    const early: LogBatch[] = [];
    const earlyStates = new Map<string, LogStreamStateEvent>();
    const closed = (value: string) =>
      ["Expired", "Failed", "Stopped"].includes(value);
    active.current = null;
    receiveTime.current = Date.now();
    setIdle(0);
    acknowledged.current.clear();
    accepted.current.clear();
    setStreamId(null);
    setError(false);
    setMessageCode(null);
    setPhase("Connecting");
    setRing({ lines: [], bytes: 0, dropped: 0, acks: [] });
    const receiveState = (event: LogStreamStateEvent) => {
      if (disposed) return;
      if (!handleId) {
        const previous = earlyStates.get(event.streamId);
        if (previous && closed(previous.phase)) return;
        if (!previous && earlyStates.size >= 64) {
          stateOverflow = true;
          return;
        }
        earlyStates.set(event.streamId, event);
        return;
      }
      if (event.streamId !== handleId || terminal) return;
      terminal = closed(event.phase);
      active.current = ["Active", "AwaitingConsumer"].includes(event.phase)
        ? handleId
        : null;
      setPhase(event.phase);
      setMessageCode(
        event.messageCode === null
          ? null
          : errorCode({ code: event.messageCode }),
      );
      setError(event.phase === "Expired" || event.phase === "Failed");
    };
    const receive = (batch: LogBatch) => {
      if (disposed) return;
      if (!handleId) {
        if (early.length < 2 && size(batch) <= 65536) early.push(batch);
        return;
      }
      if (
        active.current !== handleId ||
        batch.streamId !== handleId ||
        accepted.current.has(batch.batchId)
      )
        return;
      accepted.current.add(batch.batchId);
      if (accepted.current.size > 4000) {
        const first = accepted.current.values().next().value;
        if (first !== undefined) accepted.current.delete(first);
      }
      receiveTime.current = Date.now();
      setRing((previous) => appendLogBatch(previous, batch));
    };
    const start = async () => {
      const stop = await api.subscribeLogStreamState(receiveState);
      if (disposed) {
        stop();
        return;
      }
      unlisten = stop;
      await previousShutdown;
      if (disposed) return;
      const handle = await api.startLogStream(
        { instanceId, tailLines: 100 },
        receive,
      );
      handleId = handle.streamId;
      if (disposed) {
        await api.stopLogStream(handle.streamId).catch(() => {});
        return;
      }
      setStreamId(handle.streamId);
      const earlyState = earlyStates.get(handle.streamId);
      if (earlyState && (!closed(handle.phase) || closed(earlyState.phase))) {
        terminal = false;
        receiveState(earlyState);
      } else {
        terminal = closed(handle.phase);
        active.current = ["Active", "AwaitingConsumer"].includes(handle.phase)
          ? handle.streamId
          : null;
        setPhase(handle.phase);
        setError(handle.phase === "Expired" || handle.phase === "Failed");
      }
      if (!earlyState && stateOverflow) {
        terminal = true;
        active.current = null;
        setPhase("Unavailable");
        setMessageCode("LogStateUnavailable");
        setError(true);
        void api.stopLogStream(handle.streamId).catch(() => {});
      }
      earlyStates.clear();
      early.forEach(receive);
    };
    const starting = start().catch((error) => {
      if (disposed) return;
      active.current = null;
      setError(false);
      setMessageCode(errorCode(error));
      setPhase("Unavailable");
      unlisten?.();
      unlisten = undefined;
    });
    const timer = setInterval(
      () => setIdle(Date.now() - receiveTime.current),
      1000,
    );
    return () => {
      disposed = true;
      clearInterval(timer);
      active.current = null;
      unlisten?.();
      unlisten = undefined;
      shutdown.current = handleId
        ? api.stopLogStream(handleId).catch(() => {})
        : Promise.all([previousShutdown, starting]).then(() => {});
    };
  }, [api, instanceId, restart]);
  // useEffect runs after React commits this rendered ring. Never ACK on Channel receipt.
  useEffect(() => {
    if (!streamId || active.current !== streamId) return;
    for (const ack of ring.acks) {
      if (ack.streamId !== streamId || acknowledged.current.has(ack.batchId))
        continue;
      acknowledged.current.add(ack.batchId);
      if (acknowledged.current.size > 4000) {
        const first = acknowledged.current.values().next().value;
        if (first !== undefined) acknowledged.current.delete(first);
      }
      void api.ackLogBatch(ack).catch(() => {});
    }
  }, [api, ring, streamId]);
  const copy = useCallback(
    () => feedback.copy(ring.lines.join("\n")),
    [feedback, ring.lines],
  );
  useEffect(() => {
    if (error)
      feedback.error({
        code:
          messageCode ??
          (phase === "Expired" ? "LogStreamExpired" : "LogReadFailed"),
      });
    else if (phase === "AwaitingConsumer")
      feedback.info("日志已暂停，等待渲染确认");
    else if (phase === "Stopped") feedback.info("日志订阅已停止");
  }, [error, messageCode, phase, feedback]);
  useEffect(() => {
    const stage =
      phase === "Active" ? (idle >= 30000 ? 30 : idle >= 10000 ? 10 : 0) : 0;
    if (stage && stage !== idleNotice.current)
      feedback.info(
        stage === 30
          ? "30 秒未收到新批次；可重新连接读取最近日志。"
          : "10 秒未收到新批次，可能暂无日志或正在等待消费。",
        { id: "log-idle" },
      );
    if (!stage && idleNotice.current) feedback.dismiss("log-idle");
    idleNotice.current = stage;
  }, [idle, phase, feedback]);
  return (
    <div className="flex flex-col gap-3">
      <p className="muted">
        {t("后端脱敏日志")} ·{" "}
        <span role="status" aria-label="日志订阅状态">
          {phase}
        </span>{" "}
        · {ring.lines.length}/2000 {t("行")} · {(ring.bytes / 1024).toFixed(1)}{" "}
        KiB / 2048 KiB
      </p>
      <p role="status">
        {t("已丢弃 / 截断")} {ring.dropped} {t("行")}
      </p>
      <div className="flex flex-wrap gap-2">
        <Button variant="outline" onClick={() => setRestart((n) => n + 1)}>
          {t("重新连接日志")}
        </Button>
        <Button
          variant="outline"
          disabled={!ring.lines.length}
          onClick={() => void copy()}
        >
          {t("复制脱敏日志")}
        </Button>
      </div>
      <pre aria-label="实时脱敏日志" className="log-output">
        {ring.lines.length ? ring.lines.join("\n") : t("等待最近日志…")}
      </pre>
      <p className="muted">
        {t("只保留受限最近日志，不保证跨订阅完整重放。未知秘密仍需您复核。")}
      </p>
    </div>
  );
}
