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
import { FieldGroup, Field, FieldLabel, FieldDescription } from "./ui/field";
import { Input } from "./ui/input";
import { useWorkspace } from "../lib/workspace";
import type {
  ConfigDraftView,
  DashboardSnapshot,
  OperationRecord,
} from "../types/toolbox";
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
  disabled = false,
}: {
  draft: ConfigDraftView;
  onApplied: (operation: OperationRecord) => void;
  disabled?: boolean;
}) {
  const { api, run, state, feedback } = useWorkspace();
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
    !!live && (live.appliedGeneration ?? "unapplied") === draft.baseGeneration;
  const service = state.catalog?.services.find(
    (spec) => spec.id === live?.serviceId,
  );
  const fieldLabel = (key: string) => {
    const port = /^port:(.+):(\d+)$/.exec(key);
    if (!port) return key;
    const name = service?.name ?? live?.serviceId ?? port[1];
    return `${name} 宿主机端口${service?.ports.length === 1 ? "" : `（容器 ${port[2]}）`}`;
  };
  const identityMatches =
    validationScope === scope && generationMatches && !!state.snapshot?.target;
  const canApply =
    validated &&
    identityMatches &&
    !!report &&
    !errors.length &&
    !draft.validationErrors.length &&
    !busy &&
    !disabled &&
    !state.error &&
    (!needsConsent || consented);
  useEffect(() => {
    setConsentScope(null);
    setConfirm(false);
  }, [scope]);
  useEffect(() => {
    if (validated && !identityMatches) feedback.error({ code: "StaleDraft" });
  }, [validated, identityMatches, feedback]);
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
      <p>修改后先保存并验证，再确认应用。</p>
      <FieldGroup>
        {draft.fields.map((field) => (
          <Field
            key={field.key}
            data-disabled={!field.editable || busy || disabled}
            data-invalid={draft.validationErrors.length > 0}
          >
            <FieldLabel htmlFor={`draft-${field.key}`}>
              {fieldLabel(field.key)}
            </FieldLabel>
            <Input
              id={`draft-${field.key}`}
              disabled={!field.editable || busy || disabled}
              aria-invalid={draft.validationErrors.length > 0}
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
        </div>
      ) : null}
      {needsConsent && (
        <Field>
          <FieldLabel htmlFor="config-capacity-consent">
            <Checkbox
              id="config-capacity-consent"
              checked={consented}
              disabled={busy || disabled}
              onCheckedChange={(checked) =>
                setConsentScope(checked === true ? scope : null)
              }
            />
            {t("我理解容量未知，仅同意本次应用")}
          </FieldLabel>
          <FieldDescription>
            {t(
              "此确认仅适用于本次修改和当前 Docker 环境；重新验证或切换环境后需要再次确认。",
            )}
          </FieldDescription>
        </Field>
      )}
      <div className="flex flex-wrap gap-2">
        <Button
          variant="outline"
          disabled={busy || disabled}
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
                "应用会重新检查端口、凭据和依赖，保留持久化数据。完成后更新连接信息。",
              )}
            </DialogDescription>
          </DialogHeader>
          <ul>
            {draft.applyImpact.map((impact, index) => (
              <li key={index}>{impact}</li>
            ))}
          </ul>
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
                    onApplied(result);
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
