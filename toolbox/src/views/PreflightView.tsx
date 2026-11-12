import {
  NativeSelect,
  NativeSelectOption,
} from "../components/ui/native-select";
import { useState } from "react";
import { Button } from "../components/ui/button";
import {
  Card,
  CardHeader,
  CardTitle,
  CardDescription,
  CardContent,
} from "../components/ui/card";
import { useWorkspace } from "../lib/workspace";
import { useT } from "../lib/i18n";
import type { PreflightReport } from "../types/toolbox";
export function PreflightView() {
  const { api, state, run } = useWorkspace();
  const t = useT();
  const [report, setReport] = useState<PreflightReport | null>(null);
  const [serviceId, setServiceId] = useState("redis");
  const [busy, setBusy] = useState(false);
  return (
    <div className="flex flex-col gap-5">
      <h1>{t("环境体检")}</h1>
      <Card>
        <CardHeader>
          <CardTitle>{t("当前 Docker 目标")}</CardTitle>
          <CardDescription>
            {t("体检不会启动服务或更改运行时配置。")}
          </CardDescription>
        </CardHeader>
        <CardContent>
          <dl className="details">
            <dt>Context</dt>
            <dd>{state.snapshot?.target?.contextName ?? "未检测"}</dd>
            <dt>{t("系统 / CPU")}</dt>
            <dd>
              {state.snapshot?.target
                ? `${state.snapshot.target.osType} / ${state.snapshot.target.architecture}`
                : "未知"}
            </dd>
          </dl>
          <div className="flex flex-wrap items-center gap-3">
            <label htmlFor="preflight-service">{t("检查服务")}</label>
            <NativeSelect
              id="preflight-service"
              value={serviceId}
              onChange={(e) => setServiceId(e.target.value)}
            >
              {state.catalog?.services.map((s) => (
                <NativeSelectOption value={s.id} key={s.id}>
                  {s.name}
                </NativeSelectOption>
              ))}
            </NativeSelect>
            <Button
              disabled={busy || !state.snapshot?.target}
              onClick={async () => {
                setBusy(true);
                try {
                  const value = await run(() =>
                    api.preflightService(serviceId),
                  );
                  if (value) setReport(value);
                } finally {
                  setBusy(false);
                }
              }}
            >
              {t("执行体检")}
            </Button>
            <Button
              variant="outline"
              onClick={() => void run(() => api.detectEnvironment())}
            >
              {t("重新检测")}
            </Button>
          </div>
        </CardContent>
      </Card>
      {report && (
        <Card>
          <CardHeader>
            <CardTitle>{t("体检结果")}</CardTitle>
            <CardDescription>
              {report.requiresCapacityConsent
                ? "容量未知，启动前将再次请求单次确认。"
                : "以启动时重新检查为准。"}
            </CardDescription>
          </CardHeader>
          <CardContent>
            <p>
              内存需求 {(report.requiredMemoryBytes / 1024 ** 3).toFixed(2)} GiB
              · Docker 内存 {(report.daemonMemoryBytes / 1024 ** 3).toFixed(2)}{" "}
              GiB
            </p>
            <p>
              工作区可用：
              {report.capacity.workspaceFreeBytes === null
                ? "未知"
                : `${(report.capacity.workspaceFreeBytes / 1024 ** 3).toFixed(1)} GiB`}
            </p>
            <p>Docker 存储：{report.capacity.daemonStatus}</p>
            {report.issues.map((issue, i) => (
              <p key={i}>
                {issue.severity} · {issue.code} {issue.field}
              </p>
            ))}
          </CardContent>
        </Card>
      )}
    </div>
  );
}
