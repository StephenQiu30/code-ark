import { useEffect, useState } from "react";
import { useT } from "../lib/i18n";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogDescription,
  DialogFooter,
} from "./ui/dialog";
import { Button } from "./ui/button";
import { Checkbox } from "./ui/checkbox";
import {
  FieldGroup,
  Field,
  FieldLabel,
  FieldError,
  FieldDescription,
} from "./ui/field";
import { Input } from "./ui/input";
import { useWorkspace } from "../lib/workspace";
import type { ConfigDraftView, DashboardSnapshot } from "../types/toolbox";
function draftScope(
  snapshot: DashboardSnapshot | null,
  draft: ConfigDraftView,
) {
  return JSON.stringify([
    snapshot?.workspace.workspaceId,
    snapshot?.target?.targetId,
    draft.instanceId,
    draft.draftId,
    draft.baseGeneration,
    snapshot?.services.find(
      (service) => service.instanceId === draft.instanceId,
    )?.appliedGeneration,
  ]);
}
export function ConfigEditor({
  draft: initial,
  onApplied,
}: {
  draft: ConfigDraftView;
  onApplied: () => void;
}) {
  const { api, run, state } = useWorkspace();
  const t = useT();
  const [draft, setDraft] = useState(initial);
  const [values, setValues] = useState<Record<string, string>>({});
  const [validated, setValidated] = useState(false);
  const [validationScope, setValidationScope] = useState<string | null>(null);
  const [consentScope, setConsentScope] = useState<string | null>(null);
  const [confirm, setConfirm] = useState(false);
  const [busy, setBusy] = useState(false);
  const scope = draftScope(state.snapshot, draft);
  const report = draft.preflight;
  const errors =
    report?.issues.filter(
      (issue) => issue.severity.toLowerCase() === "error",
    ) ?? [];
  const needsConsent = report?.requiresCapacityConsent ?? false;
  const consented = consentScope === scope;
  const live = state.snapshot?.services.find(
    (service) => service.instanceId === draft.instanceId,
  );
  const generationMatches =
    !!live && live.appliedGeneration === draft.baseGeneration;
  const identityMatches =
    validationScope === scope && generationMatches && !!state.snapshot?.target;
  const canApply =
    validated &&
    identityMatches &&
    !!report &&
    !errors.length &&
    !draft.validationErrors.length &&
    !busy &&
    !state.error &&
    (!needsConsent || consented);
  useEffect(() => {
    setConsentScope(null);
    setConfirm(false);
  }, [scope]);
  useEffect(() => {
    setDraft(initial);
    setValues({});
    setValidated(false);
    setValidationScope(null);
    setConsentScope(null);
    setConfirm(false);
  }, [initial.draftId, initial.instanceId, initial.baseGeneration]);
  return (
    <div className="flex flex-col gap-4">
      <p>草稿 · 基于 {draft.baseGeneration}。保存草稿不会更改当前连接。</p>
      <FieldGroup>
        {draft.fields.map((field) => (
          <Field key={field.key} data-disabled={!field.editable || busy}>
            <FieldLabel htmlFor={`draft-${field.key}`}>{field.key}</FieldLabel>
            <Input
              id={`draft-${field.key}`}
              disabled={!field.editable || busy}
              type={field.sensitive ? "password" : "text"}
              value={values[field.key] ?? field.value ?? ""}
              placeholder={field.sensitive ? "已保存的秘密保持不变" : ""}
              onChange={(event) => {
                setValues({ ...values, [field.key]: event.target.value });
                setValidated(false);
                setConsentScope(null);
                setConfirm(false);
              }}
            />
            <FieldDescription>
              {field.sensitive
                ? "秘密不回填；已初始化账号不提供随机生成或密码轮换。"
                : ""}
            </FieldDescription>
          </Field>
        ))}
      </FieldGroup>
      {draft.validationErrors.map((error, index) => (
        <FieldError key={index}>{error}</FieldError>
      ))}
      {report ? (
        <div className="flex flex-col gap-2">
          <h3>{t("本草稿的真实环境预检")}</h3>
          <p>
            所需内存 {(report.requiredMemoryBytes / 1024 ** 3).toFixed(2)} GiB ·
            Docker可用 {(report.daemonMemoryBytes / 1024 ** 3).toFixed(2)} GiB
          </p>
          <p>
            工作区可用：
            {report.capacity.workspaceFreeBytes === null
              ? "未知"
              : `${(report.capacity.workspaceFreeBytes / 1024 ** 3).toFixed(1)} GiB`}{" "}
            · Docker存储：
            {report.capacity.daemonFreeBytes === null
              ? "未知"
              : `${(report.capacity.daemonFreeBytes / 1024 ** 3).toFixed(1)} GiB`}
          </p>
          {(report.capacity.workspaceFreeBytes === null ||
            report.capacity.daemonFreeBytes === null) && (
            <p>{t("可用磁盘容量未知")}</p>
          )}
          {report.issues.map((issue, index) => (
            <FieldError key={index}>
              {issue.code}
              {issue.field ? ` · ${issue.field}` : ""}
            </FieldError>
          ))}
        </div>
      ) : (
        <p className="muted">{t("请先保存草稿并完成真实环境预检")}</p>
      )}
      {validated && !identityMatches && (
        <FieldError>
          {t("草稿所属工作区、目标或已应用版本已变化，请重新验证")}
        </FieldError>
      )}
      {needsConsent && (
        <Field>
          <FieldLabel htmlFor="config-capacity-consent">
            <Checkbox
              id="config-capacity-consent"
              checked={consented}
              disabled={busy}
              onCheckedChange={(checked) =>
                setConsentScope(checked ? scope : null)
              }
            />
            {t("我理解容量未知，仅同意本次应用")}
          </FieldLabel>
          <FieldDescription>
            {t(
              "此确认仅适用于当前草稿、目标与基础版本；重新验证或身份变化会清除。",
            )}
          </FieldDescription>
        </Field>
      )}
      <div className="flex flex-wrap gap-2">
        <Button
          variant="outline"
          disabled={busy}
          onClick={async () => {
            setBusy(true);
            setValidated(false);
            setValidationScope(null);
            setConsentScope(null);
            setConfirm(false);
            const snapshot = state.snapshot;
            try {
              const result = await run(() =>
                api.validateDraft({
                  draftId: draft.draftId,
                  instanceId: draft.instanceId,
                  baseGeneration: draft.baseGeneration,
                  values: {
                    ...Object.fromEntries(
                      draft.fields
                        .filter(
                          (field) => !field.sensitive && field.value !== null,
                        )
                        .map((field) => [field.key, field.value!]),
                    ),
                    ...values,
                  },
                }),
              );
              if (result) {
                setDraft(result);
                setValidationScope(draftScope(snapshot, result));
                setValidated(result.validationErrors.length === 0);
                setValues({});
              }
            } finally {
              setBusy(false);
            }
          }}
        >
          {t("保存草稿并验证")}
        </Button>
        <Button disabled={!canApply} onClick={() => setConfirm(true)}>
          {t("预览并应用")}
        </Button>
      </div>
      <Dialog open={confirm} onOpenChange={setConfirm}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>
              {draft.requiresRebuild
                ? t("应用将重建受影响的托管组件")
                : t("应用已验证的配置")}
            </DialogTitle>
            <DialogDescription>
              {t(
                "当前连接仍是已应用版本。应用会重新验证端口、凭据、数据兼容和依赖；完成后以新的后端快照为准。",
              )}
            </DialogDescription>
          </DialogHeader>
          <ul>
            {draft.applyImpact.map((impact, index) => (
              <li key={index}>{impact}</li>
            ))}
          </ul>
          <p>
            草稿 {draft.draftId} · 基础版本 {draft.baseGeneration}
          </p>
          <DialogFooter>
            <Button variant="outline" onClick={() => setConfirm(false)}>
              {t("取消")}
            </Button>
            <Button
              disabled={!canApply}
              onClick={async () => {
                if (!canApply) return;
                setBusy(true);
                try {
                  const result = await run(() =>
                    api.applyDraft(draft.draftId, needsConsent && consented),
                  );
                  if (result) {
                    setValidated(false);
                    onApplied();
                  }
                } finally {
                  setConsentScope(null);
                  setConfirm(false);
                  setBusy(false);
                }
              }}
            >
              {t("确认应用")}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}
