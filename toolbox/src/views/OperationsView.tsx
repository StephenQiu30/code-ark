import { useState } from "react";
import { CircleCheck, CircleAlert, LoaderCircle } from "lucide-react";
import { PageHeader } from "../components/PageHeader";
import { ResourceList } from "../components/ResourceList";
import { Button } from "../components/ui/button";
import { Badge } from "../components/ui/badge";
import { Progress } from "../components/ui/progress";
import { Tabs, TabsList, TabsTrigger } from "../components/ui/tabs";
import {
  Empty,
  EmptyHeader,
  EmptyTitle,
  EmptyDescription,
} from "../components/ui/empty";
import { useWorkspace } from "../lib/workspace";
import { useT } from "../lib/i18n";

export function OperationsView() {
  const { state, api, run } = useWorkspace();
  const t = useT();
  const [busy, setBusy] = useState<string | null>(null);
  const [filter, setFilter] = useState("all");
  const operations = (state.snapshot?.operations ?? []).filter(
    (o) =>
      filter === "all" ||
      (filter === "running"
        ? o.allowedActions.includes("cancel")
        : filter === "failed"
          ? o.phase === "Failed" || o.allowedActions.includes("retry")
          : ["Completed", "Succeeded", "Ready"].includes(o.phase)),
  );
  return (
    <div className="flex flex-col gap-8">
      <PageHeader
        title={t("运行任务")}
        description="离开页面，任务仍会继续。"
      />
      <Tabs value={filter} onValueChange={setFilter}>
        <TabsList variant="line">
          <TabsTrigger value="all">全部</TabsTrigger>
          <TabsTrigger value="running">进行中</TabsTrigger>
          <TabsTrigger value="failed">需要处理</TabsTrigger>
          <TabsTrigger value="done">已完成</TabsTrigger>
        </TabsList>
      </Tabs>
      <ResourceList items={operations} getKey={(o) => o.operationId}>
        {(operation) => {
          const completed = operation.steps.filter((s) =>
            ["Completed", "Succeeded", "Ready"].includes(s.phase),
          ).length;
          const names = operation.instanceIds
            .map(
              (id) =>
                state.snapshot?.services.find((s) => s.instanceId === id)
                  ?.serviceId,
            )
            .filter(Boolean)
            .map(
              (id) =>
                state.catalog?.services.find((s) => s.id === id)?.name ?? id,
            );
          return (
            <article className="flex flex-col gap-5 py-6">
              <div className="flex items-center justify-between gap-4">
                <h2 className="section-title">
                  {names.length ? names.join(" / ") : "服务操作"}
                </h2>
                <Badge
                  variant={
                    operation.phase === "Failed" ? "status-error" : "status"
                  }
                >
                  {operation.phase}
                </Badge>
              </div>
              {!!operation.steps.length && (
                <>
                  <Progress
                    value={(completed / operation.steps.length) * 100}
                    aria-label="任务进度"
                  />
                  <ol className="flex flex-col gap-4">
                    {operation.steps.map((step) => {
                      const done = ["Completed", "Succeeded", "Ready"].includes(
                        step.phase,
                      );
                      const Icon = done
                        ? CircleCheck
                        : step.phase === "Failed"
                          ? CircleAlert
                          : LoaderCircle;
                      return (
                        <li
                          key={step.stepId}
                          className="flex items-center gap-3"
                        >
                          <Icon className="size-4 shrink-0" />
                          <div className="min-w-0 flex-1">
                            <h3>{step.stepId}</h3>
                            <p className="text-xs text-muted-foreground">
                              已耗时 {(step.elapsedMs / 1000).toFixed(1)} 秒
                              {step.deadlineAt
                                ? ` · 截止 ${new Date(step.deadlineAt).toLocaleTimeString()}`
                                : ""}
                            </p>
                          </div>
                          <Badge variant="status">{step.phase}</Badge>
                        </li>
                      );
                    })}
                  </ol>
                </>
              )}
              {!!operation.createdResourceIds.length && (
                <div className="flex flex-col gap-2">
                  <h3 className="text-xs text-muted-foreground">
                    {t("本次登记的资源")}
                  </h3>
                  <ul className="flex flex-col gap-1 font-mono text-xs">
                    {operation.createdResourceIds.map((id) => (
                      <li key={id} className="break-all">
                        {id}
                      </li>
                    ))}
                  </ul>
                </div>
              )}
              <div className="flex flex-wrap gap-2">
                {operation.allowedActions.includes("retry") && (
                  <Button
                    disabled={busy === operation.operationId}
                    onClick={async () => {
                      setBusy(operation.operationId);
                      try {
                        await run(() =>
                          api.retryOperation(operation.operationId),
                        );
                      } finally {
                        setBusy(null);
                      }
                    }}
                  >
                    {t("重试")}
                  </Button>
                )}
                {operation.allowedActions.includes("cancel") && (
                  <Button
                    variant="outline"
                    disabled={busy === operation.operationId}
                    onClick={async () => {
                      setBusy(operation.operationId);
                      try {
                        await run(() =>
                          api.cancelOperation(operation.operationId),
                        );
                      } finally {
                        setBusy(null);
                      }
                    }}
                  >
                    {t("取消")}
                  </Button>
                )}
              </div>
            </article>
          );
        }}
      </ResourceList>
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
