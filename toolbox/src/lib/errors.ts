import { errorCode } from "../api/tauri";

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
  UnknownCapacity: [
    "需要确认磁盘容量",
    "重新预检并检查磁盘空间，确认后仅本次继续。",
  ],
  UnknownMemory: [
    "暂时无法确认可用内存",
    "请等待 Docker 状态稳定后重新检测，再启动服务。",
  ],
  InsufficientMemory: [
    "Docker 可用内存不足",
    "请在环境体检核对所需内存，释放资源或调整 Docker 内存后重试。",
  ],
  ProbeClientLaunchFailed: [
    "协议检测工具未能启动",
    "请重新检测 Docker，并在运行任务中查看检测状态后重试。",
  ],
  CleanupPending: [
    "临时任务尚未完成清理",
    "请稍后重试，工作箱会先核验并清理本次临时任务，再继续操作。",
  ],
  ArtifactDownloadFailed: [
    "镜像下载未完成",
    "请检查网络后重试，已通过校验的下载会保留。",
  ],
  ProbeClientOomKilled: [
    "协议检测任务内存不足",
    "Docker 已终止本次检测，请检查可用资源后重试。",
  ],
  ProbeClientTimedOut: [
    "协议检测超时",
    "请等待 Docker 状态稳定后重试，当前尚未确认服务就绪。",
  ],
  ProbeClientExitFailed: [
    "协议检测进程未完成",
    "请在运行任务中查看检测状态后重试。",
  ],
  NacosBootstrapUnavailable: [
    "Nacos 初始化暂未完成",
    "请稍后重试，已有数据和密码会保留。",
  ],
  CredentialRepairRequired: [
    "需要核对服务凭据",
    "请确认保存的连接配置；工作箱不会自动重置已有账号密码。",
  ],
  PreflightFailed: [
    "启动条件尚未满足",
    "请检查端口、内存和环境体检中的问题后重试。",
  ],
  StaleDraft: [
    "配置草稿已经过期",
    "请重新打开配置，核对当前运行版本后再次应用。",
  ],
  StaleOperation: [
    "原操作对应的配置已经变化",
    "请重新预检并发起操作，原确认不会沿用到新配置。",
  ],
};
fixes.DockerMissing = fixes.DockerNotInstalled;
fixes.ProbeClientCleanupPending = fixes.CleanupPending;
fixes.ComposeCapabilityMissing = [
  "Compose 能力检查未通过",
  "请更新 Docker Compose v2，确认支持所需的配置和等待能力后重新检测。",
];
fixes.UnsupportedArchitecture = [
  "当前 CPU 架构尚未验证",
  "请选择已验证的镜像与原生 CPU 组合；不能用模拟运行代替原生支持。",
];
fixes.DockerOutputInvalid = [
  "Docker 返回的状态无法读取",
  "请检查 Docker 运行时与当前 context，恢复后重新检测；工作箱不会据此推断已就绪。",
];

Object.assign(fixes, {
  DependencyBindingRequired: [
    "需要确认数据库使用范围",
    "确认已有数据库连接及当前服务的专属库授权后即可继续启动。",
  ],
  ExistingDatabaseBindingRequired: [
    "发现已有数据库",
    "请先确认已有数据库连接，避免重复创建环境。",
  ],
  CanalBinlogRequired: [
    "MySQL 尚未启用 ROW binlog",
    "请按 Canal 指南启用 binlog 后重试。",
  ],
  CanalReplicationGrantRequired: [
    "Canal 连接账号缺少复制权限",
    "请使用已具备复制权限的专用账号重新验证。",
  ],
  EnvironmentNotDetected: [
    "尚未连接 Docker",
    "请先检测并确认 Docker 引擎，再启动预设环境。",
  ],
  BackendUnavailable: [
    "连接工作箱后端失败",
    "请确认桌面后端已启动，然后重新检测。",
  ],
  CatalogUnavailable: ["无法读取服务目录", "请重新检测，恢复服务目录后继续。"],
  PortConflict: [
    "端口已被占用",
    "请调整服务端口或停止占用该端口的应用后重试。",
  ],
  LogSubscriptionUnavailable: ["无法订阅日志状态", "请重新连接日志。"],
  ClipboardUnavailable: ["无法访问剪贴板", "请检查剪贴板权限后重试。"],
  FrontendUnexpectedError: [
    "界面发生异常",
    "请重试当前操作；问题持续时重新打开工作箱。",
  ],
  FrontendRenderFailed: ["页面加载失败", "请重新加载页面。"],
  StaleResetPlan: [
    "重置计划已失效",
    "计划已过期或所属实例、目标已变化，请重新获取计划。",
  ],
  DraftValidationFailed: ["配置验证未通过", "请检查配置字段后重新验证。"],
  PreflightRequired: ["配置尚未完成预检", "请重新保存草稿并验证。"],
  LogStreamExpired: ["日志订阅已过期", "保留已显示的脱敏日志，请重新连接。"],
  LogReadFailed: ["日志读取失败", "保留已显示的脱敏日志，请重新连接。"],
  LogStateUnavailable: ["无法确认日志订阅状态", "请重新连接日志。"],
});

// Never display an IPC rejection message, stack, field value, path, or credentials.
export function publicError(error: unknown) {
  const code = errorCode(error);
  const [title, description] = fixes[code] ?? [
    "操作未完成",
    "请稍后重试，或在运行任务中检查操作状态。",
  ];
  return { code, title, description };
}
