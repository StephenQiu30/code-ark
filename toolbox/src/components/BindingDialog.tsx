import { NativeSelect, NativeSelectOption } from "./ui/native-select";
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
import { FieldGroup, Field, FieldLabel, FieldDescription } from "./ui/field";
import { Input } from "./ui/input";
import { useWorkspace } from "../lib/workspace";
import type { DatabaseCandidate, BindingGrantPreview } from "../types/toolbox";
export function BindingDialog({
  serviceId,
  onClose,
}: {
  serviceId: string;
  onClose: () => void;
}) {
  const { api, run } = useWorkspace();
  const t = useT();
  const [candidates, setCandidates] = useState<DatabaseCandidate[]>([]);
  const [candidateId, setCandidateId] = useState("");
  const [username, setUsername] = useState("");
  const [password, setPassword] = useState("");
  const [databases, setDatabases] = useState("");
  const [preview, setPreview] = useState<BindingGrantPreview | null>(null);
  const [busy, setBusy] = useState(false);
  useEffect(() => {
    let active = true;
    void run(() => api.databaseCandidates()).then((result) => {
      if (active && result) setCandidates(result);
    });
    return () => {
      active = false;
    };
  }, [api, run]);
  return (
    <Dialog
      open
      onOpenChange={(open) => {
        if (!open) onClose();
      }}
    >
      <DialogContent>
        <DialogHeader>
          <DialogTitle>{t("连接并授权已有数据库")}</DialogTitle>
          <DialogDescription>
            {t(
              "只支持后端发现并验证的容器数据库。不会接管生命周期或静默创建替代实例。",
            )}
          </DialogDescription>
        </DialogHeader>
        {!preview ? (
          <FieldGroup>
            <Field>
              <FieldLabel htmlFor="binding-candidate">
                {t("已有数据库")}
              </FieldLabel>
              <NativeSelect
                id="binding-candidate"
                value={candidateId}
                onChange={(e) => setCandidateId(e.target.value)}
              >
                <NativeSelectOption value="">
                  {t("选择已发现的容器")}
                </NativeSelectOption>
                {candidates.map((c) => (
                  <NativeSelectOption value={c.candidateId} key={c.candidateId}>
                    {c.name} · {c.engine} · {c.networkNames.join(", ")}
                  </NativeSelectOption>
                ))}
              </NativeSelect>
              {!candidates.length && (
                <FieldDescription>
                  {t(
                    "未发现可复用容器。原生进程或远程数据库拓扑暂不支持，请检查目标与网络。",
                  )}
                </FieldDescription>
              )}
            </Field>
            <Field>
              <FieldLabel htmlFor="binding-user">{t("用户名")}</FieldLabel>
              <Input
                id="binding-user"
                value={username}
                onChange={(e) => setUsername(e.target.value)}
                autoComplete="off"
              />
            </Field>
            <Field>
              <FieldLabel htmlFor="binding-password">
                {t("连接密码")}
              </FieldLabel>
              <Input
                id="binding-password"
                value={password}
                type="password"
                onChange={(e) => setPassword(e.target.value)}
                autoComplete="off"
              />
            </Field>
            <Field>
              <FieldLabel htmlFor="binding-databases">
                {t("允许的专属数据库（逗号分隔）")}
              </FieldLabel>
              <Input
                id="binding-databases"
                value={databases}
                onChange={(e) => setDatabases(e.target.value)}
              />
            </Field>
          </FieldGroup>
        ) : (
          <>
            <p>{preview.scopeNotice}</p>
            <dl className="details">
              <dt>{t("授权数据库")}</dt>
              <dd>{preview.grant.allowedDatabases.join("、")}</dd>
              <dt>{t("账号范围")}</dt>
              <dd>{preview.grant.allowedAccounts.join("、")}</dd>
              <dt>{t("操作")}</dt>
              <dd>{preview.grant.operations.join("、")}</dd>
              <dt>{t("服务器指纹")}</dt>
              <dd>{preview.grant.serverFingerprint}</dd>
            </dl>
            <p>{t("外部数据库仅连接/观察，不提供停止、删除或整库重置。")}</p>
          </>
        )}
        <DialogFooter>
          <Button variant="outline" onClick={onClose}>
            {t("取消")}
          </Button>
          {!preview ? (
            <Button
              disabled={busy || !candidateId || !username || !databases.trim()}
              onClick={async () => {
                setBusy(true);
                try {
                  const binding = await run(() =>
                    api.verifyDatabaseBinding({
                      candidateId,
                      username,
                      password,
                      allowedDatabases: databases
                        .split(",")
                        .map((s) => s.trim())
                        .filter(Boolean),
                    }),
                  );
                  setPassword("");
                  if (binding) {
                    const result = await run(() =>
                      api.bindingGrantPreview(binding.bindingId, serviceId),
                    );
                    if (result) setPreview(result);
                  }
                } finally {
                  setBusy(false);
                }
              }}
            >
              {t("验证连接并预览授权")}
            </Button>
          ) : (
            <Button
              disabled={busy}
              onClick={async () => {
                setBusy(true);
                try {
                  const result = await run(() =>
                    api.authorizeDatabaseBinding(preview.previewId),
                  );
                  if (result) onClose();
                } finally {
                  setBusy(false);
                }
              }}
            >
              {t("确认以上专属初始化授权")}
            </Button>
          )}
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
