import { useEffect, useState, useRef } from "react";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogDescription,
} from "./ui/dialog";
import { Tabs, TabsList, TabsTrigger, TabsContent } from "./ui/tabs";
import { Button } from "./ui/button";
import { Badge } from "./ui/badge";
import { Skeleton } from "./ui/skeleton";
import { Alert, AlertTitle, AlertDescription } from "./ui/alert";
import { ConfigEditor } from "./ConfigApplyDialog";
import { ResetPlanDialog } from "./ResetPlanDialog";
import { BindingDialog } from "./BindingDialog";
import { LogViewer } from "./LogViewer";
import { StartAction } from "./StartAction";
import { useWorkspace } from "../lib/workspace";
import { useT, phaseText } from "../lib/i18n";
import { errorCode } from "../api/tauri";
import type {
  ConnectionView,
  ConfigDraftView,
  ResetPlan,
} from "../types/toolbox";
export function ServiceDetailDrawer({
  serviceId,
  onClose,
}: {
  serviceId: string;
  onClose: () => void;
}) {
  const { state, api, run } = useWorkspace();
  const t = useT();
  const service = state.catalog?.services.find((s) => s.id === serviceId);
  const snapshot = state.snapshot?.services.find(
    (s) => s.serviceId === serviceId,
  );
  const instanceId = snapshot?.instanceId;
  const external =
    snapshot?.phase === "ExternalObserve" ||
    snapshot?.ownership.toLowerCase() === "external";
  const viewScope = useRef(0);
  const [tab, setTab] = useState("connection");
  const [connection, setConnection] = useState<ConnectionView | null>(null);
  const [draft, setDraft] = useState<ConfigDraftView | null>(null);
  const [secrets, setSecrets] = useState<Record<string, string>>({});
  const [error, setError] = useState<string | null>(null);
  const [reset, setReset] = useState<ResetPlan | null>(null);
  const [binding, setBinding] = useState(false);
  const [stopConfirm, setStopConfirm] = useState(false);
  const [busy, setBusy] = useState(false);
  const [copied, setCopied] = useState(false);
  useEffect(() => {
    let alive = true;
    setConnection(null);
    setDraft(null);
    setSecrets({});
    viewScope.current++;
    if (instanceId) {
      void api
        .connectionView(instanceId)
        .then((value) => {
          if (alive) setConnection(value);
        })
        .catch((e) => {
          if (alive) setError(errorCode(e));
        });
      void api
        .configDraft(instanceId)
        .then((value) => {
          if (alive) setDraft(value);
        })
        .catch(() => {});
    }
    return () => {
      alive = false;
      viewScope.current++;
    };
  }, [api, instanceId, snapshot?.appliedGeneration]);
  const close = () => {
    viewScope.current++;
    setSecrets({});
    setConnection(null);
    setDraft(null);
    onClose();
  };
  return (
    <Dialog
      open
      onOpenChange={(open) => {
        if (!open) close();
      }}
    >
      <DialogContent className="service-drawer" showCloseButton={false}>
        <DialogHeader>
          <div className="flex items-center justify-between gap-3">
            <DialogTitle>{service?.name ?? serviceId}</DialogTitle>
            <Button variant="ghost" aria-label="关闭详情" onClick={close}>
              {t("关闭")}
            </Button>
          </div>
          <DialogDescription>
            {service?.category} ·{" "}
            <Badge variant="secondary">
              {t(phaseText[snapshot?.phase ?? "Uninitialized"])}
            </Badge>
          </DialogDescription>
        </DialogHeader>
        {error && (
          <Alert>
            <AlertTitle>{t("当前视图需要处理")}</AlertTitle>
            <AlertDescription>{error}</AlertDescription>
          </Alert>
        )}
        {snapshot?.messageCode && (
          <Alert>
            <AlertTitle>{snapshot.messageCode}</AlertTitle>
            <AlertDescription>
              {t("请查看运行任务或对应指南；尚未通过连接验证。")}
            </AlertDescription>
          </Alert>
        )}
        <Tabs
          value={tab}
          onValueChange={(value) => {
            viewScope.current++;
            setSecrets({});
            setTab(value);
          }}
        >
          <TabsList className="detail-tabs">
            <TabsTrigger value="connection">{t("连接与入口")}</TabsTrigger>
            <TabsTrigger value="config">{t("配置")}</TabsTrigger>
            <TabsTrigger value="logs">{t("日志")}</TabsTrigger>
            <TabsTrigger value="data">{t("依赖与数据")}</TabsTrigger>
          </TabsList>
          <TabsContent value="connection">
            <div className="flex flex-col gap-4">
              {connection ? (
                <>
                  <p>
                    实际已应用版本：<code>{connection.appliedGeneration}</code>
                  </p>
                  <h3>{t("宿主机连接")}</h3>
                  {connection.hostEndpoints.map((endpoint) => (
                    <div key={endpoint.label} className="endpoint">
                      <span>{endpoint.label}</span>
                      <code>
                        {endpoint.host}:{endpoint.port}
                      </code>
                      {["http", "https"].includes(endpoint.protocol) && (
                        <Button
                          size="sm"
                          variant="outline"
                          onClick={() =>
                            void run(() =>
                              api.openServiceConsole(
                                instanceId!,
                                endpoint.label,
                              ),
                            )
                          }
                        >
                          打开控制台
                          {snapshot?.phase !== "Ready"
                            ? "（后台尚未连接）"
                            : ""}
                        </Button>
                      )}
                    </div>
                  ))}
                  <h3>{t("容器连接与网络")}</h3>
                  <p>
                    {connection.requiredNetworks.join("、") || "未提供容器网络"}
                  </p>
                  {connection.containerEndpoints.map((endpoint) => (
                    <p key={endpoint.label}>
                      {endpoint.label}：
                      <code>
                        {endpoint.host}:{endpoint.port}
                      </code>
                    </p>
                  ))}
                  <p className="muted">
                    {t(
                      "容器必须加入上面的真实网络；容器内 localhost 指向容器自己。",
                    )}
                  </p>
                  {connection.credentialHandles.map((handle) => (
                    <div key={handle.handle} className="endpoint">
                      <span>{handle.label}</span>
                      <code>{secrets[handle.handle] ?? "••••••••"}</code>
                      <Button
                        variant="outline"
                        size="sm"
                        onClick={async () => {
                          if (!instanceId) return;
                          const scope = viewScope.current;
                          const value = await api
                            .revealSecret(instanceId, handle.handle)
                            .catch((e) => {
                              setError(errorCode(e));
                              return null;
                            });
                          if (value !== null && scope === viewScope.current)
                            setSecrets((previous) => ({
                              ...previous,
                              [handle.handle]: value,
                            }));
                        }}
                      >
                        显示 {handle.label}
                      </Button>
                    </div>
                  ))}
                  {connection.snippets.map((snippet, index) => (
                    <div key={index}>
                      <h3>{snippet.label}</h3>
                      <pre>{snippet.body}</pre>
                      <Button
                        size="sm"
                        variant="outline"
                        onClick={async () => {
                          try {
                            await navigator.clipboard.writeText(snippet.body);
                            setCopied(true);
                          } catch {
                            setError("ClipboardUnavailable");
                          }
                        }}
                      >
                        {t("复制片段")}
                      </Button>
                    </div>
                  ))}
                  {copied && <p role="status">{t("已复制")}</p>}
                  {serviceId === "rocketmq" && (
                    <p className="muted">
                      {t(
                        "使用 5.x Proxy endpoint 与匹配 SDK；旧版 Broker 直连需按高级指南验证网络，不自动开启回环广播。",
                      )}
                    </p>
                  )}
                  {snapshot?.phase === "Ready" && (
                    <p>
                      {t(
                        "基础设施连接已验证。业务客户端、Worker 或执行器仍需按教程接入。",
                      )}
                    </p>
                  )}
                </>
              ) : instanceId ? (
                <>
                  <Skeleton className="h-12" />
                  <p>{t("连接参数尚未可用；等待已应用版本。")}</p>
                </>
              ) : (
                <p>
                  {t("尚未创建实例，启动并完成应用后才会生成真实连接参数。")}
                </p>
              )}
              {!external && <StartAction serviceIds={[serviceId]} />}
              <Button variant="outline" onClick={() => setBinding(true)}>
                {t("连接已有数据库")}
              </Button>
            </div>
          </TabsContent>
          <TabsContent value="config">
            {external ? (
              <p>{t("外部服务仅连接与观察；不提供配置重建或账号轮换。")}</p>
            ) : draft ? (
              <ConfigEditor
                key={draft.draftId}
                draft={draft}
                onApplied={() =>
                  void run(() => api.connectionView(instanceId!)).then(
                    (value) => {
                      if (value) setConnection(value);
                    },
                  )
                }
              />
            ) : (
              <p>
                {t(
                  "实例创建后可编辑草稿。已初始化数据库账号不提供随机生成或服务器密码轮换。",
                )}
              </p>
            )}
          </TabsContent>
          <TabsContent value="logs">
            {tab === "logs" && instanceId ? (
              <LogViewer instanceId={instanceId} />
            ) : (
              <p>{t("创建实例后可查看后端脱敏日志。")}</p>
            )}
          </TabsContent>
          <TabsContent value="data">
            <div className="flex flex-col gap-4">
              <h3>{t("依赖复用")}</h3>
              <p>{service?.dependencies.join("、") || "无前置服务"}</p>
              <h3>{t("数据范围")}</h3>
              <ul>
                {service?.dataPaths.map((path) => (
                  <li key={path}>{path}</li>
                ))}
              </ul>
              <p>
                {t(
                  "停止会保留持久化数据。新建实例不会自动迁移旧数据；导入/导出请阅读备份指南。",
                )}
              </p>
              {external ? (
                <Alert>
                  <AlertTitle>{t("外部服务 · 仅观察")}</AlertTitle>
                  <AlertDescription>
                    {t("没有停止、删除、接管或整库重置入口。")}
                  </AlertDescription>
                </Alert>
              ) : (
                instanceId && (
                  <>
                    <Button
                      variant="outline"
                      onClick={() => setStopConfirm(true)}
                    >
                      {t("停止服务（保留数据）")}
                    </Button>
                    <Button
                      variant="destructive"
                      onClick={async () => {
                        const plan = await run(() => api.planReset(instanceId));
                        if (plan) setReset(plan);
                      }}
                    >
                      {t("预览重置计划")}
                    </Button>
                  </>
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
        )}{" "}
        {reset && (
          <ResetPlanDialog plan={reset} onClose={() => setReset(null)} />
        )}
        <Dialog open={stopConfirm} onOpenChange={setStopConfirm}>
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
            <Button
              disabled={busy}
              onClick={async () => {
                const current = state.snapshot;
                if (!current?.target || !instanceId) return;
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
          </DialogContent>
        </Dialog>
      </DialogContent>
    </Dialog>
  );
}
