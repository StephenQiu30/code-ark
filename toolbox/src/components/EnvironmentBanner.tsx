import { Alert, AlertTitle, AlertDescription } from "./ui/alert";
import { Button } from "./ui/button";
import { useWorkspace } from "../lib/workspace";
import { useT } from "../lib/i18n";
const fixes: Record<string, [string, string]> = {
  DockerNotInstalled: [
    "尚未安装 Docker",
    "请安装适合系统与 CPU 的 Docker Desktop 或 Engine，然后重新检测。",
  ],
  DockerUnavailable: [
    "Docker 尚未启动",
    "启动已安装的 Docker 运行时，等待引擎就绪后重新检测。",
  ],
  DaemonUnavailable: [
    "Docker 尚未启动",
    "启动已安装的 Docker 运行时，等待引擎就绪后重新检测。",
  ],
  PermissionDenied: [
    "Docker 权限不足",
    "Linux 请按官方说明配置 docker 组并重新登录。工作箱不会后台提权。",
  ],
  UnsupportedContainerMode: [
    "请切换到 Linux containers",
    "Windows 请检查 WSL2、虚拟化与 Docker Linux 容器模式。",
  ],
  ComposeTooOld: ["Compose 版本过旧", "请更新 Docker Compose v2，再重新检测。"],
  UnsupportedTarget: [
    "当前 Docker 目标不受支持",
    "请选择本地 Linux containers 目标，不会自动切换全局 context。",
  ],
  NativeRuntimeAcceptanceRequired: [
    "当前平台尚未完成运行验收",
    "当前源码预览仅开放 macOS arm64 运行操作。其他平台请先使用服务目录中的 Compose 指南。",
  ],
  DependencyInitializationRequired: [
    "此服务的自动初始化尚未接入",
    "当前版本尚不能一键启动这条数据库依赖链。请先使用对应服务的 Compose 指南。",
  ],
  UnknownCapacity: ["需要确认磁盘容量", "重新预检并检查磁盘空间，确认后仅本次继续。"],
  PreflightFailed: ["启动条件尚未满足", "请检查端口、内存和环境体检中的问题后重试。"],
  StaleDraft: ["配置草稿已经过期", "请重新打开配置，核对当前运行版本后再次应用。"],
  StaleOperation: ["原操作对应的配置已经变化", "请重新预检并发起操作，原确认不会沿用到新配置。"],
};
fixes.DockerMissing = fixes.DockerNotInstalled;
fixes.ComposeCapabilityMissing = ["Compose 能力检查未通过", "请更新 Docker Compose v2，确认支持所需的配置和等待能力后重新检测。"];
fixes.UnsupportedArchitecture = ["当前 CPU 架构尚未验证", "请选择已验证的镜像与原生 CPU 组合；不能用模拟运行代替原生支持。"];
fixes.DockerOutputInvalid = ["Docker 返回的状态无法读取", "请检查 Docker 运行时与当前 context，恢复后重新检测；工作箱不会据此推断已就绪。"];
export function EnvironmentBanner() {
  const { state, api, refresh, run } = useWorkspace();
  const t = useT();
  const error = state.snapshot?.environmentError;
  const issue = error ? (fixes[error] ?? ["环境需要处理", error]) : null;
  const actionIssue = state.error
    ? (fixes[state.error] ?? ["操作未完成", state.error])
    : null;
  const disconnected = state.error === "BackendUnavailable";
  return (
    <>
      {state.error && (
        <Alert variant="destructive">
          <AlertTitle>{t(disconnected ? "连接工作箱后端失败" : actionIssue![0])}</AlertTitle>
          <AlertDescription>
            <p>{t(disconnected ? state.error : actionIssue![1])}</p>
            {disconnected && <p>
              上次失联：
              {new Date(state.disconnectedAt ?? Date.now()).toLocaleString()}
              。显示的状态来自最近一次快照。
            </p>}
            <Button variant="outline" onClick={() => void refresh()}>
              {t("重新检测")}
            </Button>
          </AlertDescription>
        </Alert>
      )}
      {issue && (
        <Alert>
          <AlertTitle>{t(issue[0])}</AlertTitle>
          <AlertDescription>
            <p>{t(issue[1])}</p>
            <div className="flex flex-wrap gap-2">
              <Button
                variant="outline"
                onClick={() =>
                  void run(() => api.openOfficial("docker-install"))
                }
              >
                {t("Docker 官方安装指引")}
              </Button>
              <Button onClick={() => void run(() => api.detectEnvironment())}>
                {t("重新检测")}
              </Button>
            </div>
          </AlertDescription>
        </Alert>
      )}
    </>
  );
}
