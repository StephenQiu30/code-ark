import { useState, useRef } from "react";
import { Button } from "./ui/button";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogDescription,
  DialogFooter,
} from "./ui/dialog";
import { Alert, AlertTitle, AlertDescription } from "./ui/alert";
import { useWorkspace } from "../lib/workspace";
import { useT } from "../lib/i18n";
import type { PreflightReport } from "../types/toolbox";
export function StartAction({
  serviceIds,
  sceneId = null,
  label = "启动",
}: {
  serviceIds: string[];
  sceneId?: string | null;
  label?: string;
}) {
  const { api, state, run } = useWorkspace();
  const t = useT();
  const [busy, setBusy] = useState(false);
  const lock = useRef(false);
  const [reports, setReports] = useState<PreflightReport[] | null>(null);
  const [reportScope, setReportScope] = useState("");
  const currentScope = JSON.stringify([
    state.snapshot?.workspace.workspaceId,
    state.snapshot?.target?.targetId,
    serviceIds,
    sceneId,
  ]);
  const scopeChanged = !!reports && reportScope !== currentScope;
  const blocked =
    !state.snapshot?.target ||
    !!state.snapshot?.environmentError ||
    !!state.error;
  const submit = async (consent: boolean) => {
    const snapshot = state.snapshot;
    if (!snapshot?.target) return;
    await run(() =>
      api.startServices({
        workspaceId: snapshot.workspace.workspaceId,
        targetId: snapshot.target!.targetId,
        serviceIds,
        sceneId,
        acceptUnknownCapacity: consent,
      }),
    );
  };
  const start = async () => {
    if (lock.current) return;
    lock.current = true;
    setBusy(true);
    try {
      const result = await run(() =>
        Promise.all(serviceIds.map((id) => api.preflightService(id))),
      );
      if (!result) return;
      if (
        result.some(
          (r) =>
            r.requiresCapacityConsent ||
            r.issues.some((i) => i.severity.toLowerCase() === "error"),
        )
      ) {
        setReportScope(currentScope);
        setReports(result);
        return;
      }
      await submit(false);
    } finally {
      lock.current = false;
      setBusy(false);
    }
  };
  const errors =
    reports?.flatMap((r) =>
      r.issues.filter((i) => i.severity.toLowerCase() === "error"),
    ) ?? [];
  return (
    <>
      <Button
        disabled={blocked || busy || !serviceIds.length}
        onClick={() => void start()}
      >
        {busy ? t("正在检查…") : t(label)}
      </Button>
      <Dialog
        open={!!reports}
        onOpenChange={(open) => {
          if (!open) setReports(null);
        }}
      >
        <DialogContent>
          <DialogHeader>
            <DialogTitle>
              {t(errors.length ? "启动前检查未通过" : "无法确认可用磁盘容量")}
            </DialogTitle>
            <DialogDescription>
              {t(
                "预检只针对本次选择。未知容量需要您单独确认，其他错误必须先修复。",
              )}
            </DialogDescription>
          </DialogHeader>
          {errors.map((issue, i) => (
            <Alert key={i} variant="destructive">
              <AlertTitle>{issue.code}</AlertTitle>
              <AlertDescription>{issue.field}</AlertDescription>
            </Alert>
          ))}
          {reports?.map((report, i) => (
            <p key={i}>
              所需内存 {(report.requiredMemoryBytes / 1024 ** 3).toFixed(1)} GiB
              · Docker 可用 {(report.daemonMemoryBytes / 1024 ** 3).toFixed(1)}{" "}
              GiB · 容量 {report.capacity.daemonStatus}
            </p>
          ))}
          {scopeChanged && (
            <p role="alert">目标、工作区或服务已变化，请取消并重新检查。</p>
          )}
          <DialogFooter>
            <Button variant="outline" onClick={() => setReports(null)}>
              {t("取消")}
            </Button>
            {!errors.length && (
              <Button
                disabled={busy || scopeChanged || blocked}
                onClick={async () => {
                  if (lock.current || scopeChanged || blocked) return;
                  lock.current = true;
                  setBusy(true);
                  try {
                    await submit(true);
                    setReports(null);
                  } finally {
                    lock.current = false;
                    setBusy(false);
                  }
                }}
              >
                {t("仅本次继续")}
              </Button>
            )}
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </>
  );
}
