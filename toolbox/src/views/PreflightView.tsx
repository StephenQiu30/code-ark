import { useEffect, useRef, useState } from "react";
import {
  Container,
  HardDrive,
  MemoryStick,
  Plug,
} from "lucide-react";
import type { LucideIcon } from "lucide-react";
import { EnvironmentConnectButton } from "../components/EnvironmentConnectButton";
import { PageHeader } from "../components/PageHeader";
import { ResourceList } from "../components/ResourceList";
import { Button } from "../components/ui/button";
import { Badge } from "../components/ui/badge";
import { Field, FieldGroup, FieldLabel } from "../components/ui/field";
import {
  Select,
  SelectContent,
  SelectGroup,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "../components/ui/select";
import { useWorkspace } from "../lib/workspace";
import { useT } from "../lib/i18n";
import type { PreflightReport } from "../types/toolbox";

type Check = {
  title: string;
  detail: string;
  icon: LucideIcon;
  state: "passed" | "pending" | "warning";
};
export function CheckRow({ title, detail, icon: Icon, state }: Check) {
  return (
    <div className="flex min-h-16 items-center gap-3 py-3">
      <Icon className="size-5 shrink-0" />
      <div className="flex min-w-0 flex-1 flex-col gap-1">
        <h3 className="text-[13px] font-medium">{title}</h3>
        <p className="text-xs text-muted-foreground">{detail}</p>
      </div>
      <Badge
        variant={
          state === "passed"
            ? "status-ready"
            : state === "warning"
              ? "status-warning"
              : "status"
        }
      >
        {state === "passed"
          ? "通过"
          : state === "warning"
            ? "待处理"
            : "待检查"}
      </Badge>
    </div>
  );
}
const gib = (n: number) => `${(n / 1024 ** 3).toFixed(1)} GiB`;
export function PreflightView() {
  const { api, state, run, feedback } = useWorkspace();
  const t = useT();
  const [report, setReport] = useState<PreflightReport | null>(null);
  const [serviceId, setServiceId] = useState("redis");
  const [busy, setBusy] = useState(false);
  const scope = JSON.stringify([
    serviceId,
    state.snapshot?.workspace.workspaceId,
    state.snapshot?.target?.targetId,
  ]);
  const current = useRef(scope);
  current.current = scope;
  useEffect(() => {
    setReport(null);
  }, [scope]);
  const target = state.snapshot?.target;
  const errors =
    report?.issues.filter((i) => i.severity.toLowerCase() === "error") ?? [];
  const passed = report && !errors.length;
  const checks: Check[] = [
    {
      title: "Docker Engine",
      detail: target
        ? `${target.contextName} · ${target.osType} / ${target.architecture}`
        : "尚未检测到运行时",
      icon: Container,
      state: target && !state.snapshot?.environmentError ? "passed" : "pending",
    },
    {
      title: "端口与依赖",
      detail: report
        ? errors.length
          ? "预检未通过"
          : "所选服务的启动检查已通过"
        : "执行所选服务预检后显示",
      icon: Plug,
      state: report ? (passed ? "passed" : "warning") : "pending",
    },
    {
      title: "内存",
      detail: report
        ? `Docker 可用 ${gib(report.daemonMemoryBytes)} · 需要 ${gib(report.requiredMemoryBytes)}`
        : "尚未检查",
      icon: MemoryStick,
      state: report
        ? report.daemonMemoryBytes >= report.requiredMemoryBytes
          ? "passed"
          : "warning"
        : "pending",
    },
    {
      title: "磁盘",
      detail: report
        ? `工作区可用 ${report.capacity.workspaceFreeBytes === null ? "未知" : gib(report.capacity.workspaceFreeBytes)} · Docker 存储 ${report.capacity.daemonStatus}`
        : "尚未检查",
      icon: HardDrive,
      state: report
        ? report.requiresCapacityConsent
          ? "warning"
          : "passed"
        : "pending",
    },
  ];
  return (
    <div className="flex flex-col gap-8">
      <PageHeader
        title={t("环境体检")}
        description="检查本机运行时与启动条件。"
        actions={
          <EnvironmentConnectButton variant="outline" label="重新检测" disabled={busy} />
        }
      />
      <FieldGroup>
        <Field orientation="horizontal">
          <FieldLabel htmlFor="preflight-service">{t("检查服务")}</FieldLabel>
          <Select value={serviceId} onValueChange={setServiceId}>
            <SelectTrigger id="preflight-service">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectGroup>
                {state.catalog?.services.map((s) => (
                  <SelectItem key={s.id} value={s.id}>
                    {s.name}
                  </SelectItem>
                ))}
              </SelectGroup>
            </SelectContent>
          </Select>
          <Button
            disabled={busy || !target}
            onClick={async () => {
              const requested = scope;
              setBusy(true);
              try {
                const value = await run(() => api.preflightService(serviceId));
                if (value && current.current === requested) {
                  setReport(value);
                  feedback.preflight(value, true);
                }
              } finally {
                setBusy(false);
              }
            }}
          >
            {busy ? "正在检查…" : t("执行体检")}
          </Button>
        </Field>
      </FieldGroup>
      <ResourceList items={checks} getKey={(c) => c.title}>
        {(c) => <CheckRow {...c} />}
      </ResourceList>
      <p className="text-xs text-muted-foreground">
        体检不会启动服务。实际启动前会按所选场景再次检查。
        {report?.requiresCapacityConsent && "容量未知，启动时需要单独确认。"}
      </p>
    </div>
  );
}
