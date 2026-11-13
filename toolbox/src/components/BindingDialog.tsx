import {
  Select,
  SelectContent,
  SelectGroup,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "./ui/select";
import { useT } from "../lib/i18n";
import { useEffect, useRef, useState } from "react";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogDescription,
  DialogFooter,
} from "./ui/dialog";
import { Button } from "./ui/button";
import { FieldGroup, Field, FieldLabel } from "./ui/field";
import { Skeleton } from "./ui/skeleton";
import { Input } from "./ui/input";
import { useWorkspace } from "../lib/workspace";
import type {
  DatabaseCandidate,
  BindingGrantPreview,
  BindingRequirements,
} from "../types/toolbox";
export function BindingDialog({
  serviceId,
  onClose,
  onAuthorized,
}: {
  serviceId: string;
  onClose: () => void;
  onAuthorized?: () => void;
}) {
  const { api, run, feedback, state } = useWorkspace();
  const t = useT();
  const [candidates, setCandidates] = useState<DatabaseCandidate[]>([]);
  const [candidateId, setCandidateId] = useState("");
  const [username, setUsername] = useState("");
  const [password, setPassword] = useState("");
  const [databases, setDatabases] = useState("");
  const [preview, setPreview] = useState<BindingGrantPreview | null>(null);
  const [busy, setBusy] = useState(false);
  const active = useRef(true);
  const [requirements, setRequirements] = useState<BindingRequirements | null>(
    null,
  );
  const knownBinding = state.snapshot?.bindings.find(
    (binding) =>
      binding.engine ===
      (state.catalog?.services.find((s) => s.id === serviceId)
        ?.dependencies[0] ?? serviceId),
  );
  const close = () => {
    active.current = false;
    onClose();
  };
  const finish = () => {
    if (!active.current) return;
    close();
    onAuthorized?.();
  };
  useEffect(() => {
    active.current = true;
    return () => {
      active.current = false;
    };
  }, []);
  useEffect(() => {
    let active = true;
    setBusy(true);
    void run(() =>
      Promise.all([
        api.databaseCandidates(),
        api.bindingRequirements(serviceId),
      ]),
    )
      .then(async (result) => {
        if (!active || !result) return;
        const [all, required] = result;
        setRequirements(required);
        setDatabases(required.databaseNames.join(", "));
        const candidates = all.filter(
          (candidate) => candidate.engine === required.engine,
        );
        setCandidates(candidates);
        if (candidates.length === 1) setCandidateId(candidates[0].candidateId);
        if (knownBinding && !["mysql", "pgsql", "canal"].includes(serviceId)) {
          const preview = await run(() =>
            api.bindingGrantPreview(knownBinding.bindingId, serviceId),
          );
          if (active && preview) setPreview(preview);
        } else if (!candidates.length)
          feedback.info("未发现可复用数据库，请检查目标与网络。");
      })
      .finally(() => {
        if (active) setBusy(false);
      });
    return () => {
      active = false;
    };
  }, [api, run, feedback, serviceId, knownBinding?.bindingId]);
  return (
    <Dialog
      open
      onOpenChange={(open) => {
        if (!open) close();
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
        {!requirements ? (
          <Skeleton className="h-40" />
        ) : !preview ? (
          <FieldGroup>
            <Field>
              <FieldLabel htmlFor="binding-candidate">
                {t("已有数据库")}
              </FieldLabel>
              <Select value={candidateId} onValueChange={setCandidateId}>
                <SelectTrigger id="binding-candidate">
                  <SelectValue placeholder={t("选择已发现的容器")} />
                </SelectTrigger>
                <SelectContent>
                  <SelectGroup>
                    {candidates.map((c) => (
                      <SelectItem key={c.candidateId} value={c.candidateId}>
                        {c.name} · {c.engine} · {c.networkNames.join(", ")}
                      </SelectItem>
                    ))}
                  </SelectGroup>
                </SelectContent>
              </Select>
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
            {(requirements.editableDatabases ||
              requirements.databaseNames.length > 0) && (
              <Field>
                <FieldLabel htmlFor="binding-databases">
                  {t("允许的专属数据库（逗号分隔）")}
                </FieldLabel>
                <Input
                  id="binding-databases"
                  value={databases}
                  readOnly={!requirements.editableDatabases}
                  onChange={(e) => setDatabases(e.target.value)}
                />
              </Field>
            )}
          </FieldGroup>
        ) : (
          <>
            <p>{preview.scopeNotice}</p>
            <dl className="details">
              <dt>{t("授权数据库")}</dt>
              <dd>{preview.grant.allowedDatabases.join("、")}</dd>
              <dt>{t("账号范围")}</dt>
              <dd>
                {preview.grant.allowedAccounts.join("、") ||
                  t("沿用已验证的连接账号")}
              </dd>
              <dt>{t("操作")}</dt>
              <dd>{preview.grant.operations.join("、")}</dd>
              <dt>{t("服务器指纹")}</dt>
              <dd>{preview.grant.serverFingerprint}</dd>
            </dl>
            <p>{t("外部数据库仅连接/观察，不提供停止、删除或整库重置。")}</p>
          </>
        )}
        <DialogFooter>
          <Button variant="outline" onClick={close}>
            {t("取消")}
          </Button>
          {!preview ? (
            <Button
              disabled={
                busy ||
                !requirements ||
                !candidateId ||
                !username ||
                (requirements.editableDatabases && !databases.trim())
              }
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
                  if (!active.current) return;
                  setPassword("");
                  if (binding && ["mysql", "pgsql"].includes(serviceId)) {
                    feedback.success("数据库连接已确认");
                    finish();
                  } else if (binding) {
                    const result = await run(() =>
                      api.bindingGrantPreview(binding.bindingId, serviceId),
                    );
                    if (active.current && result) setPreview(result);
                  }
                } finally {
                  if (active.current) setBusy(false);
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
                  if (result) finish();
                } finally {
                  if (active.current) setBusy(false);
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
