import { useEffect, useRef, useState } from "react";
import { ArrowLeft, Copy, Eye, EyeOff, ExternalLink } from "lucide-react";
import { PageHeader } from "./PageHeader";
import { Button } from "./ui/button";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "./ui/tabs";
import { ToggleGroup, ToggleGroupItem } from "./ui/toggle-group";
import { Empty, EmptyDescription, EmptyHeader, EmptyTitle } from "./ui/empty";
import { Skeleton } from "./ui/skeleton";
import { Separator } from "./ui/separator";
import { Tooltip, TooltipContent, TooltipTrigger } from "./ui/tooltip";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "./ui/dialog";
import { StatusBadge } from "./StatusBadge";
import { ConfigEditor } from "./ConfigApplyDialog";
import { BindingDialog } from "./BindingDialog";
import { ResetPlanDialog } from "./ResetPlanDialog";
import { StartAction } from "./StartAction";
import { LogViewer } from "./LogViewer";
import { useWorkspace } from "../lib/workspace";
import { useT } from "../lib/i18n";
import { categoryLabel } from "../lib/services";
import type {
  ConfigDraftView,
  ConnectionView,
  ResetPlan,
} from "../types/toolbox";

export function ConnectionRow({
  label,
  value,
  children,
  onCopy,
}: {
  label: string;
  value: string;
  children?: React.ReactNode;
  onCopy?: () => void;
}) {
  return (
    <div className="flex min-h-19 min-w-0 items-center gap-4 py-3">
      <div className="flex min-w-0 flex-1 flex-col gap-2">
        <span className="text-xs text-muted-foreground">{label}</span>
        <code className="break-all">{value}</code>
      </div>
      {children}
      {onCopy && (
        <Tooltip>
          <TooltipTrigger asChild>
            <Button
              size="icon"
              variant="ghost"
              aria-label={`复制 ${label}`}
              onClick={onCopy}
            >
              <Copy />
            </Button>
          </TooltipTrigger>
          <TooltipContent>复制 {label}</TooltipContent>
        </Tooltip>
      )}
    </div>
  );
}

export function ServiceDetailView({
  serviceId,
  onClose,
}: {
  serviceId: string;
  onClose: () => void;
}) {
  const { state, api, run, feedback } = useWorkspace();
  const t = useT();
  const spec = state.catalog?.services.find((s) => s.id === serviceId);
  const live = state.snapshot?.services.find((s) => s.serviceId === serviceId);
  const instanceId = live?.instanceId;
  const external =
    live?.phase === "ExternalObserve" ||
    live?.ownership.toLowerCase().startsWith("external");
  const hasAppliedConfig = !!live?.appliedGeneration;
  const scope = useRef(0);
  const [tab, setTab] = useState("connection");
  const [target, setTarget] = useState("host");
  const [connection, setConnection] = useState<ConnectionView | null>(null);
  const [draft, setDraft] = useState<ConfigDraftView | null>(null);
  const [pendingApply, setPendingApply] = useState<string | null>(null);
  const instanceBusy =
    !!instanceId &&
    !!state.snapshot?.operations.some(
      (operation) =>
        operation.instanceIds.includes(instanceId) &&
        ["Queued", "Running"].includes(operation.phase),
    );
  const pendingApplyPhase = state.snapshot?.operations.find(
    (operation) => operation.operationId === pendingApply,
  )?.phase;
  const [secrets, setSecrets] = useState<Record<string, string>>({});
  const [binding, setBinding] = useState(false);
  const [reset, setReset] = useState<ResetPlan | null>(null);
  const [stopConfirm, setStopConfirm] = useState(false);
  const [busy, setBusy] = useState(false);
  const heading = useRef<HTMLDivElement>(null);
  useEffect(() => {
    heading.current?.querySelector<HTMLElement>("h1")?.focus();
  }, []);
  useEffect(() => {
    if (external || !hasAppliedConfig) {
      setTab((current) =>
        current === "logs" || (external && current === "config")
          ? "connection"
          : current,
      );
      setStopConfirm(false);
      setReset(null);
    }
  }, [external, hasAppliedConfig]);
  useEffect(() => {
    let alive = true;
    scope.current++;
    setConnection(null);
    setSecrets({});
    if (instanceId) {
      if (external || hasAppliedConfig)
        void api
          .connectionView(instanceId)
          .then((v) => {
            if (alive) setConnection(v);
          })
          .catch(() => {});
    }
    return () => {
      alive = false;
      scope.current++;
    };
  }, [api, instanceId, live?.appliedGeneration, external, hasAppliedConfig]);
  useEffect(() => {
    setDraft(null);
    setPendingApply(null);
  }, [instanceId, external]);
  useEffect(() => {
    if (!instanceId || external || instanceBusy) return;
    const finishedApply =
      pendingApply &&
      pendingApplyPhase &&
      !["Queued", "Running"].includes(pendingApplyPhase);
    if (draft?.instanceId === instanceId && !finishedApply) return;
    let alive = true;
    void api
      .configDraft(instanceId)
      .then((value) => {
        if (alive) {
          setDraft(value);
          setPendingApply(null);
        }
      })
      .catch(() => {});
    return () => {
      alive = false;
    };
  }, [
    api,
    instanceId,
    external,
    instanceBusy,
    draft,
    pendingApply,
    pendingApplyPhase,
  ]);
  const clearSecrets = () => {
    scope.current++;
    setSecrets({});
  };
  const copy = feedback.copy;
  const canBind =
    ["mysql", "pgsql"].includes(serviceId) ||
    spec?.dependencies.some((id) => ["mysql", "pgsql"].includes(id));
  return (
    <section
      aria-label={`服务详情 ${spec?.name ?? serviceId}`}
      className="flex min-w-0 flex-col gap-8"
    >
      <div ref={heading}>
        <PageHeader
          title={spec?.name ?? serviceId}
          description={categoryLabel(spec?.category ?? "", serviceId)}
          actions={
            <>
              <StatusBadge phase={live?.phase ?? "Uninitialized"} />
              {!external && hasAppliedConfig && instanceId && (
                <Button
                  variant="outline"
                  onClick={() => setStopConfirm(true)}
                  aria-label="停止服务（保留数据）"
                >
                  {t("停止服务")}
                </Button>
              )}
              <Button
                variant="ghost"
                aria-label="关闭详情"
                onClick={() => {
                  clearSecrets();
                  onClose();
                }}
              >
                <ArrowLeft data-icon="inline-start" />
                {t("返回")}
              </Button>
            </>
          }
        />
      </div>
      <Tabs
        value={tab}
        onValueChange={(v) => {
          clearSecrets();
          setTab(v);
        }}
        className="gap-8"
      >
        <TabsList variant="line" className="max-w-full flex-wrap justify-start">
          <TabsTrigger value="connection">{t("连接与入口")}</TabsTrigger>
          <TabsTrigger value="config" disabled={external}>
            {t("配置")}
          </TabsTrigger>
          <TabsTrigger value="logs" disabled={external || !hasAppliedConfig}>
            {t("日志")}
          </TabsTrigger>
          <TabsTrigger value="data">{t("依赖与数据")}</TabsTrigger>
        </TabsList>
        <TabsContent value="connection">
          <div className="grid min-w-0 gap-12 xl:grid-cols-[minmax(0,1fr)_240px]">
            <div className="flex min-w-0 flex-col gap-6">
              {connection ? (
                <>
                  <ToggleGroup
                    type="single"
                    value={target}
                    onValueChange={(v) => {
                      if (v) {
                        clearSecrets();
                        setTarget(v);
                      }
                    }}
                    aria-label="连接目标"
                  >
                    <ToggleGroupItem value="host">宿主机应用</ToggleGroupItem>
                    <ToggleGroupItem value="container">
                      Docker 容器
                    </ToggleGroupItem>
                  </ToggleGroup>
                  <div>
                    {(target === "host"
                      ? connection.hostEndpoints
                      : connection.containerEndpoints
                    ).map((endpoint) => (
                      <ConnectionRow
                        key={endpoint.label}
                        label={endpoint.label}
                        value={`${endpoint.host}:${endpoint.port}`}
                        onCopy={() =>
                          void copy(`${endpoint.host}:${endpoint.port}`)
                        }
                      >
                        {["http", "https"].includes(endpoint.protocol) && (
                          <Button
                            variant="ghost"
                            size="sm"
                            onClick={() =>
                              void run(() =>
                                api.openServiceConsole(
                                  instanceId!,
                                  endpoint.label,
                                ),
                              )
                            }
                          >
                            <ExternalLink data-icon="inline-start" />
                            {live?.phase === "Ready"
                              ? "打开控制台"
                              : "打开控制台（后台尚未连接）"}
                          </Button>
                        )}
                      </ConnectionRow>
                    ))}
                    {target === "container" && (
                      <>
                        <ConnectionRow
                          label="容器连接与网络"
                          value={
                            connection.requiredNetworks.join("、") ||
                            "未提供容器网络"
                          }
                        />
                        <p className="muted">
                          容器必须加入上面的真实网络；容器内 localhost
                          指向容器自己。
                        </p>
                      </>
                    )}
                    {connection.credentialHandles.map((handle) => (
                      <ConnectionRow
                        key={handle.handle}
                        label={handle.label}
                        value={secrets[handle.handle] ?? "••••••••"}
                        onCopy={
                          secrets[handle.handle]
                            ? () => void copy(secrets[handle.handle])
                            : undefined
                        }
                      >
                        <Button
                          variant="ghost"
                          size="sm"
                          aria-label={`${secrets[handle.handle] ? "隐藏" : "显示"} ${handle.label}`}
                          onClick={async () => {
                            if (secrets[handle.handle]) {
                              clearSecrets();
                              return;
                            }
                            if (!instanceId) return;
                            const current = scope.current;
                            try {
                              const value = await api.revealSecret(
                                instanceId,
                                handle.handle,
                              );
                              if (current === scope.current)
                                setSecrets((previous) => ({
                                  ...previous,
                                  [handle.handle]: value,
                                }));
                            } catch {
                              /* The global API boundary reports failures. */
                            }
                          }}
                        >
                          {secrets[handle.handle] ? (
                            <EyeOff data-icon="inline-start" />
                          ) : (
                            <Eye data-icon="inline-start" />
                          )}
                          {secrets[handle.handle] ? "隐藏" : "显示"}
                        </Button>
                      </ConnectionRow>
                    ))}
                  </div>
                  {connection.snippets.map((snippet, index) => (
                    <div key={index} className="flex min-w-0 flex-col gap-2">
                      <div className="flex items-center justify-between gap-3">
                        <h3 className="font-mono text-xs text-muted-foreground">
                          {snippet.label}
                        </h3>
                        <Button
                          variant="ghost"
                          size="icon"
                          aria-label="复制片段"
                          onClick={() => void copy(snippet.body)}
                        >
                          <Copy />
                        </Button>
                      </div>
                      <pre>{snippet.body}</pre>
                    </div>
                  ))}
                  {serviceId === "rocketmq" && (
                    <p className="muted">
                      使用 5.x Proxy endpoint 与匹配 SDK；旧版 Broker
                      直连需按高级指南验证网络。
                    </p>
                  )}
                </>
              ) : instanceId && (external || hasAppliedConfig) ? (
                <>
                  <Skeleton className="h-16" />
                  <p className="muted">
                    {t("连接参数尚未可用；等待已应用版本。")}
                  </p>
                </>
              ) : (
                <Empty>
                  <EmptyHeader>
                    <EmptyTitle>
                      {instanceId ? "尚未应用配置" : "尚未创建实例"}
                    </EmptyTitle>
                    <EmptyDescription>
                      启动后会提供真实连接参数。
                    </EmptyDescription>
                  </EmptyHeader>
                </Empty>
              )}
              <div className="flex flex-wrap gap-3">
                {!external && (
                  <StartAction
                    serviceIds={[serviceId]}
                    label={instanceId ? "重新启动并验证" : "启动"}
                  />
                )}
                {canBind && (
                  <Button variant="ghost" onClick={() => setBinding(true)}>
                    {t("连接已有数据库")}
                  </Button>
                )}
              </div>
            </div>
            <aside className="flex flex-col gap-5">
              <h2 className="section-title">运行信息</h2>
              <dl className="flex flex-col gap-5">
                <div>
                  <dt className="text-xs text-muted-foreground">运行方式</dt>
                  <dd>{external ? "外部 · 仅观察" : "本地托管"}</dd>
                </div>
                <div>
                  <dt className="text-xs text-muted-foreground">配置状态</dt>
                  <dd>
                    {external
                      ? "使用已验证的外部连接"
                      : hasAppliedConfig
                        ? "当前配置已应用"
                        : "尚未应用配置"}
                  </dd>
                </div>
                <div>
                  <dt className="text-xs text-muted-foreground">连接测试</dt>
                  <dd>{live?.phase === "Ready" ? "已通过" : "尚未验证"}</dd>
                </div>
              </dl>
              <p className="text-xs text-muted-foreground">
                停止服务会保留持久化数据。
              </p>
              {serviceId === "temporal" && (
                <p className="muted">业务 Worker 由应用提供。</p>
              )}
            </aside>
          </div>
        </TabsContent>
        <TabsContent value="config">
          <div className="max-w-2xl">
            {external ? (
              <p>{t("外部服务仅连接与观察；不提供配置重建或账号轮换。")}</p>
            ) : draft && draft.instanceId === instanceId ? (
              <ConfigEditor
                key={draft.draftId}
                draft={draft}
                disabled={instanceBusy || !!pendingApply}
                onApplied={(operation) =>
                  setPendingApply(operation.operationId)
                }
              />
            ) : instanceId ? (
              <Skeleton className="h-40" />
            ) : (
              <p className="muted">
                {t(
                  "实例创建后可编辑草稿。已初始化数据库账号不提供随机生成或服务器密码轮换。",
                )}
              </p>
            )}
          </div>
        </TabsContent>
        <TabsContent value="logs">
          {!external && hasAppliedConfig && tab === "logs" && instanceId ? (
            <LogViewer instanceId={instanceId} />
          ) : (
            <p className="muted">{t("创建实例后可查看后端脱敏日志。")}</p>
          )}
        </TabsContent>
        <TabsContent value="data">
          <div className="flex flex-col gap-6">
            <h2 className="section-title">{t("依赖复用")}</h2>
            <p>{spec?.dependencies.join("、") || "无前置服务"}</p>
            <Separator />
            <h2 className="section-title">{t("数据范围")}</h2>
            <ul className="flex flex-col gap-2 font-mono text-xs">
              {spec?.dataPaths.map((path) => (
                <li key={path}>{path}</li>
              ))}
            </ul>
            <p className="muted">
              {t(
                "停止会保留持久化数据。新建实例不会自动迁移旧数据；导入/导出请阅读备份指南。",
              )}
            </p>
            {external ? (
              <StatusBadge phase="ExternalObserve" />
            ) : (
              instanceId && (
                <div>
                  <Button
                    variant="destructive"
                    onClick={async () => {
                      const value = await run(() => api.planReset(instanceId));
                      if (value) setReset(value);
                    }}
                  >
                    {t("预览重置计划")}
                  </Button>
                </div>
              )
            )}
          </div>
        </TabsContent>
      </Tabs>
      {binding && (
        <BindingDialog
          serviceId={serviceId}
          onClose={() => setBinding(false)}
        />
      )}
      {!external && reset && (
        <ResetPlanDialog plan={reset} onClose={() => setReset(null)} />
      )}
      <Dialog
        open={!external && hasAppliedConfig && stopConfirm}
        onOpenChange={setStopConfirm}
      >
        <DialogContent>
          <DialogHeader>
            <DialogTitle>{t("停止服务并保留数据")}</DialogTitle>
            <DialogDescription>
              {t(
                "依赖于此服务的应用可能受影响。其他场景正在使用的共用依赖由后端保留，外部服务不会停止。",
              )}
            </DialogDescription>
          </DialogHeader>
          <p>
            受影响服务：
            {state.catalog?.services
              .filter((s) => s.dependencies.includes(serviceId))
              .map((s) => s.name)
              .join("、") || "无目录内直接依赖"}
          </p>
          <DialogFooter>
            <Button variant="outline" onClick={() => setStopConfirm(false)}>
              取消
            </Button>
            <Button
              disabled={busy || !state.snapshot?.target}
              onClick={async () => {
                const current = state.snapshot;
                if (
                  external ||
                  !hasAppliedConfig ||
                  !current?.target ||
                  !instanceId
                )
                  return;
                setBusy(true);
                try {
                  const result = await run(() =>
                    api.stopServices({
                      workspaceId: current.workspace.workspaceId,
                      targetId: current.target!.targetId,
                      instanceIds: [instanceId],
                      sceneId: null,
                    }),
                  );
                  if (result) setStopConfirm(false);
                } finally {
                  setBusy(false);
                }
              }}
            >
              {t("确认停止")}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </section>
  );
}
