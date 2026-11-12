import { useState } from "react";
import {
  Card,
  CardHeader,
  CardTitle,
  CardDescription,
  CardContent,
  CardFooter,
} from "../components/ui/card";
import { Button } from "../components/ui/button";
import {
  Empty,
  EmptyHeader,
  EmptyTitle,
  EmptyDescription,
} from "../components/ui/empty";
import { Badge } from "../components/ui/badge";
import { useWorkspace } from "../lib/workspace";
import { useT } from "../lib/i18n";
export function OperationsView() {
  const { state, api, run } = useWorkspace();
  const t = useT();
  const [busy, setBusy] = useState<string | null>(null);
  const operations = state.snapshot?.operations ?? [];
  return (
    <div className="flex flex-col gap-5">
      <h1>{t("运行任务")}</h1>
      <p className="muted">
        {t(
          "重开应用会恢复后端任务。取消不会删除已存在数据，保留资源见任务详情。",
        )}
      </p>
      {operations.map((operation) => (
        <Card key={operation.operationId}>
          <CardHeader>
            <div className="flex justify-between gap-2">
              <CardTitle>{operation.operationId}</CardTitle>
              <Badge variant="secondary">{operation.phase}</Badge>
            </div>
            <CardDescription>{operation.errorCode}</CardDescription>
          </CardHeader>
          <CardContent>
            <ol className="flex flex-col gap-3">
              {operation.steps.map((step) => (
                <li key={step.stepId}>
                  <strong>{step.stepId}</strong> · {step.phase} ·{" "}
                  {step.messageCode !== operation.errorCode && step.messageCode}
                  <p className="muted">
                    已耗时 {(step.elapsedMs / 1000).toFixed(1)} 秒
                    {step.deadlineAt
                      ? ` · 截止 ${new Date(step.deadlineAt).toLocaleTimeString()}`
                      : ""}
                  </p>
                </li>
              ))}
            </ol>
            <p>{t("本次登记的资源")}</p>
            <ul>
              {operation.createdResourceIds.map((id) => (
                <li key={id}>{id}</li>
              ))}
            </ul>
            <p className="muted">
              应用 generation：{operation.appliedGeneration ?? "尚未应用"} ·
              最后就绪：{operation.lastReadyGeneration ?? "尚未验证"}
            </p>
          </CardContent>
          <CardFooter>
            {operation.allowedActions?.includes("retry") && (
              <Button
                disabled={busy === operation.operationId}
                onClick={async () => {
                  setBusy(operation.operationId);
                  try {
                    await run(() => api.retryOperation(operation.operationId));
                  } finally {
                    setBusy(null);
                  }
                }}
              >
                {t("重试")}
              </Button>
            )}
            {operation.allowedActions?.includes("cancel") && (
              <Button
                variant="outline"
                disabled={busy === operation.operationId}
                onClick={async () => {
                  setBusy(operation.operationId);
                  try {
                    await run(() => api.cancelOperation(operation.operationId));
                  } finally {
                    setBusy(null);
                  }
                }}
              >
                {t("取消")}
              </Button>
            )}
          </CardFooter>
        </Card>
      ))}
      {!operations.length && (
        <Empty>
          <EmptyHeader>
            <EmptyTitle>{t("没有运行任务")}</EmptyTitle>
            <EmptyDescription>
              {t("启动服务后的步骤、耗时与恢复操作会显示在这里。")}
            </EmptyDescription>
          </EmptyHeader>
        </Empty>
      )}
    </div>
  );
}
