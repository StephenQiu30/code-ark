import { useT } from "../lib/i18n";
import { useEffect, useState } from "react";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogDescription,
  DialogFooter,
} from "./ui/dialog";
import { Button } from "./ui/button";
import { Field, FieldLabel } from "./ui/field";
import { Input } from "./ui/input";
import { useWorkspace } from "../lib/workspace";
import type { ResetPlan } from "../types/toolbox";
export function ResetPlanDialog({
  plan,
  onClose,
}: {
  plan: ResetPlan;
  onClose: () => void;
}) {
  const { api, run, state, feedback } = useWorkspace();
  const t = useT();
  const [typed, setTyped] = useState("");
  const [now, setNow] = useState(Date.now());
  const [busy, setBusy] = useState(false);
  useEffect(() => {
    const timer = setInterval(() => setNow(Date.now()), 1000);
    return () => clearInterval(timer);
  }, []);
  const expired = now >= plan.expiresAt;
  const scopeChanged =
    state.snapshot?.target?.targetId !== plan.targetId ||
    state.snapshot?.services.find(
      (service) => service.serviceId === plan.serviceId,
    )?.instanceId !== plan.instanceId;
  useEffect(() => {
    if (expired || scopeChanged) feedback.error({ code: "StaleResetPlan" });
  }, [expired, scopeChanged, feedback]);
  useEffect(() => {
    setTyped("");
  }, [plan.planId, plan.fingerprint]);
  return (
    <Dialog
      open
      onOpenChange={(open) => {
        if (!open) onClose();
      }}
    >
      <DialogContent>
        <DialogHeader>
          <DialogTitle>精确重置计划 · {plan.serviceId}</DialogTitle>
          <DialogDescription>
            {t(
              "只请求删除下列后端登记资源。资源或归属变化时需重新计划，服务名不是唯一校验。",
            )}{" "}
            {t("此操作可能永久删除列出的数据")}。{plan.impact.join("；")}
          </DialogDescription>
        </DialogHeader>
        <ul>
          {plan.exactResourceIds.map((id) => (
            <li key={id}>{id}</li>
          ))}
        </ul>
        {plan.databaseClaims.map((claim) => (
          <p key={claim.claimId}>
            数据库：{claim.databaseNames.join("、")} · {claim.ownerInstanceId}
          </p>
        ))}
        <p>
          有效至 {new Date(plan.expiresAt).toLocaleString()} · {plan.planId}
        </p>
        <Field>
          <FieldLabel htmlFor="reset-name">{t("输入服务 ID 确认")}</FieldLabel>
          <Input
            id="reset-name"
            value={typed}
            onChange={(e) => setTyped(e.target.value)}
            autoComplete="off"
          />
        </Field>
        <DialogFooter>
          <Button variant="outline" onClick={onClose}>
            {t("取消")}
          </Button>
          <Button
            variant="destructive"
            disabled={
              expired || scopeChanged || typed !== plan.serviceId || busy
            }
            onClick={async () => {
              if (Date.now() >= plan.expiresAt || scopeChanged) return;
              setBusy(true);
              try {
                const result = await run(() =>
                  api.executeReset({
                    planId: plan.planId,
                    fingerprint: plan.fingerprint,
                    typedServiceId: typed,
                  }),
                );
                if (result) onClose();
              } finally {
                setBusy(false);
              }
            }}
          >
            {t("执行精确重置")}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
