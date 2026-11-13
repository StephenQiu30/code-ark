import { Play, LoaderCircle } from "lucide-react";
import { useState, useRef, useEffect, useId } from "react";
import { Button } from "./ui/button";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogDescription,
  DialogFooter,
} from "./ui/dialog";
import { BindingDialog } from "./BindingDialog";
import { errorCode } from "../api/tauri";
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
  const { api, state, run, feedback } = useWorkspace();
  const t = useT();
  const [busy, setBusy] = useState(false);
  const lock = useRef(false);
  const notificationScope = useId();
  const checkSequence = useRef(0);
  const [bindingService, setBindingService] = useState<string | null>(null);
  const [reports, setReports] = useState<PreflightReport[] | null>(null);
  const [reportScope, setReportScope] = useState("");
  const currentScope = JSON.stringify([
    state.snapshot?.workspace.workspaceId,
    state.snapshot?.target?.targetId,
    serviceIds,
    sceneId,
  ]);
  const scopeChanged = !!reports && reportScope !== currentScope;
  useEffect(() => {
    if (scopeChanged) feedback.error({ code: "StaleOperation" });
  }, [scopeChanged, feedback]);
  const blocked =
    !state.snapshot?.target ||
    !!state.snapshot?.environmentError ||
    !!state.error ||
    state.snapshot.operations.some(
      (operation) =>
        ["Queued", "Running"].includes(operation.phase) &&
        operation.allowedActions.includes("cancel"),
    );
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
    const notice = `start-preflight:${notificationScope}:${++checkSequence.current}`;
    feedback.loading("正在检查启动条件…", { id: notice });
    try {
      const result = await run(async () => {
        const reports: PreflightReport[] = [];
        for (const id of serviceIds) {
          try {
            reports.push(await api.preflightService(id));
          } catch (error) {
            if (
              [
                "DependencyBindingRequired",
                "ExistingDatabaseBindingRequired",
                "InitializationGrantOutOfScope",
                "BindingReverificationRequired",
              ].includes(errorCode(error))
            )
              setBindingService(id);
            throw error;
          }
        }
        return reports;
      });
      if (!result) return;
      if (
        result.some((r) =>
          r.issues.some((i) => i.severity.toLowerCase() === "error"),
        )
      ) {
        result.forEach((report) => feedback.preflight(report));
        return;
      }
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
      feedback.dismiss(notice);
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
      {bindingService && (
        <BindingDialog
          serviceId={bindingService}
          onClose={() => setBindingService(null)}
          onAuthorized={() => void start()}
        />
      )}
      <Button
        disabled={blocked || busy || !serviceIds.length}
        onClick={() => void start()}
      >
        {busy ? (
          <LoaderCircle data-icon="inline-start" className="animate-spin" />
        ) : (
          <Play data-icon="inline-start" />
        )}
        {t(label)}
        {busy ? t(" · 正在检查…") : ""}
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
          {reports?.map((report, i) => (
            <p key={i}>
              所需内存 {(report.requiredMemoryBytes / 1024 ** 3).toFixed(1)} GiB
              · Docker 可用 {(report.daemonMemoryBytes / 1024 ** 3).toFixed(1)}{" "}
              GiB · 容量 {report.capacity.daemonStatus}
            </p>
          ))}
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
