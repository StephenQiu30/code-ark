// Interface copy only. Backend catalog names, error codes and bundled guides keep their source language.
export const english: Record<string, string> = {
  "当前平台尚未完成运行验收": "Native runtime acceptance is pending",
  "当前源码预览仅开放 macOS arm64 运行操作。其他平台请先使用服务目录中的 Compose 指南。": "This source preview enables runtime operations on macOS arm64 only. Use the service Compose guides on other platforms.",
  "此服务的自动初始化尚未接入": "Automatic dependency initialization is not connected yet",
  "当前版本尚不能一键启动这条数据库依赖链。请先使用对应服务的 Compose 指南。": "This database dependency workflow is not available yet. Use the corresponding service Compose guide.",
  "操作未完成": "Operation did not complete",

  连接并授权已有数据库: "Connect and authorize an existing database",
  "只支持后端发现并验证的容器数据库。不会接管生命周期或静默创建替代实例。":
    "Only discovered, verified container databases are supported. Existing containers stay externally managed.",
  已有数据库: "Existing database",
  选择已发现的容器: "Select a discovered container",
  "未发现可复用容器。原生进程或远程数据库拓扑暂不支持，请检查目标与网络。":
    "No reusable containers found. Native and remote databases are not supported; check the Docker target and network.",
  用户名: "Username",
  连接密码: "Connection password",
  "允许的专属数据库（逗号分隔）":
    "Allowed dedicated databases (comma-separated)",
  授权数据库: "Authorized databases",
  账号范围: "Account scope",
  操作: "Operations",
  服务器指纹: "Server fingerprint",
  "外部数据库仅连接/观察，不提供停止、删除或整库重置。":
    "External databases allow connection and observation only, with no stop, delete or full database reset.",
  验证连接并预览授权: "Verify connection and preview authorization",
  确认以上专属初始化授权: "Authorize the initialization shown above",
  保存草稿并验证: "Save and validate draft",
  预览并应用: "Preview and apply",
  应用将重建受影响的托管组件:
    "Applying rebuilds the affected managed components",
  "当前连接仍是已应用版本。应用会重新验证端口、凭据、数据兼容和依赖；完成后以新的后端快照为准。":
    "Current connections keep using the applied version. Applying rechecks ports, credentials, data compatibility and dependencies. The new backend snapshot determines the result.",
  确认应用: "Confirm apply",
  预览诊断内容: "Preview diagnostics",
  "只导出以下白名单摘要。不会自动发送给任何人。日志默认排除；未知秘密仍可能需要手动排除。":
    "Export only the summaries below. Nothing is sent automatically. Logs are excluded by default; manually exclude any remaining sensitive content.",
  请复核高风险内容: "Review potentially sensitive content",
  选择要包含的摘要: "Select summaries to include",
  "包含已预览的限定日志（最多 100 行）":
    "Include previewed logs (up to 100 lines)",
  确认导出: "Confirm export",
  日志连接需要恢复: "Reconnect the log stream",
  "保留已显示的脱敏内容，请重新连接。不会显示原始 IPC 错误。":
    "The redacted content already shown is retained. Reconnect to continue. Raw IPC errors are not displayed.",
  重新连接日志: "Reconnect logs",
  复制脱敏日志: "Copy redacted logs",
  "只保留受限最近日志，不保证跨订阅完整重放。未知秘密仍需您复核。":
    "Only bounded recent logs are retained. Replay across streams is not guaranteed. Review for unknown secrets before sharing.",
  "只请求删除下列后端登记资源。资源或归属变化时需重新计划，服务名不是唯一校验。":
    "Delete only the backend-registered resources listed below. Changed resources or ownership require a new plan; typing a service ID is only one check.",
  "输入服务 ID 确认": "Type the service ID to confirm",
  执行精确重置: "Execute scoped reset",
  当前视图需要处理: "This view needs attention",
  "请查看运行任务或对应指南；尚未通过连接验证。":
    "Check operations or the relevant guide. The connection has not been verified.",
  宿主机连接: "Host connections",
  打开控制台: "Open console",
  容器连接与网络: "Container connections and networks",
  "容器必须加入上面的真实网络；容器内 localhost 指向容器自己。":
    "Containers must join the actual networks shown above. localhost inside a container refers to that container.",
  复制片段: "Copy snippet",
  已复制: "Copied",
  "使用 5.x Proxy endpoint 与匹配 SDK；旧版 Broker 直连需按高级指南验证网络，不自动开启回环广播。":
    "Use the 5.x Proxy endpoint with a matching SDK. Legacy Broker connections require network verification; loopback broadcasting is not enabled automatically.",
  "基础设施连接已验证。业务客户端、Worker 或执行器仍需按教程接入。":
    "Infrastructure connectivity is verified. Connect your application clients, Workers or executors using the guides.",
  "连接参数尚未可用；等待已应用版本。":
    "Connection parameters are not available yet. Waiting for an applied version.",
  "尚未创建实例，启动并完成应用后才会生成真实连接参数。":
    "No instance exists yet. Start and apply it to get actual connection parameters.",
  连接已有数据库: "Connect an existing database",
  "外部服务仅连接与观察；不提供配置重建或账号轮换。":
    "External services allow connection and observation only. Configuration rebuilds and account rotation are unavailable.",
  "实例创建后可编辑草稿。已初始化数据库账号不提供随机生成或服务器密码轮换。":
    "Create an instance before editing drafts. Initialized database accounts have no password generation or server password rotation.",
  "创建实例后可查看后端脱敏日志。":
    "Create an instance to view backend-redacted logs.",
  依赖复用: "Reused dependencies",
  数据范围: "Data scope",
  "停止会保留持久化数据。新建实例不会自动迁移旧数据；导入/导出请阅读备份指南。":
    "Stopping preserves persistent data. Creating a new instance does not migrate old data; read the backup guide for import and export.",
  "外部服务 · 仅观察": "External service · observe only",
  "没有停止、删除、接管或整库重置入口。":
    "Stop, delete, takeover and full database reset are unavailable.",
  "停止服务（保留数据）": "Stop service (keep data)",
  预览重置计划: "Preview reset plan",
  停止服务并保留数据: "Stop the service and retain data",
  "依赖于此服务的应用可能受影响。其他场景正在使用的共用依赖由后端保留，外部服务不会停止。":
    "Dependent applications may be affected. The backend retains shared dependencies used by other scenes. External services are never stopped.",
  确认停止: "Confirm stop",
  本地开发工作箱: "Local development toolbox",
  "源码实施中 · 尚未发行": "Under development · unreleased",
  "预检只针对本次选择。未知容量需要您单独确认，其他错误必须先修复。":
    "This preflight applies only to this selection. Unknown capacity requires explicit consent. Fix other errors before starting.",
  仅本次继续: "Continue this time only",
  "服务是否可用由真实连接验证决定；外部服务仅观察。":
    "Readiness requires a real connection check. External services are observed only.",
  "基础设施就绪 · 业务客户端待接入":
    "Infrastructure ready · application client pending",
  没有匹配的服务: "No matching services",
  "换一个服务名或分类。": "Try another service name or category.",
  "基础设施连接测试不需要 Git / Node / Rust。教学源码的语言工具链另行安装；待验证示例不代表已通过。":
    "Infrastructure checks need no Git, Node or Rust. Install language tools only for source examples. Pending examples have not passed acceptance.",
  "Docker 安装与复检": "Install and recheck Docker",
  系统运行库与首次打开: "System prerequisites and first launch",
  返回全部教程: "Back to all guides",
  客户端版本: "Client version",
  所需工具: "Required tools",
  验证状态: "Validation status",
  执行: "Run",
  "预期结果（不是本次执行证据）": "Expected result (not evidence of execution)",
  清理教学数据: "Clean up tutorial data",
  阅读指南: "Read guide",
  "MediaCrawler · 宿主机工具": "MediaCrawler · host tool",
  "不计入 17 个服务，不通过 Docker 服务启动。":
    "Separate from the 17 services. Not started through Docker service actions.",
  "需要 Git / uv。请按包内锁定 commit 与补丁指南操作，实际抓取能力及站点条件单独验证。":
    "Requires Git and uv. Follow the bundled pinned-commit and patch instructions. Scraping and site requirements need separate verification.",
  "消息系统（只启动所选队列）":
    "Message system (only the selected queue starts)",
  选择范围: "Select scope",
  同时启动: "Start both",
  "重开应用会恢复后端任务。取消不会删除已存在数据，保留资源见任务详情。":
    "Reopening restores backend operations. Cancellation preserves existing data. See operation details for retained resources.",
  本次登记的资源: "Resources registered for this operation",
  "启动服务后的步骤、耗时与恢复操作会显示在这里。":
    "Steps, elapsed time and recovery actions appear here after a service starts.",
  "当前 Docker 目标": "Current Docker target",
  "体检不会启动服务或更改运行时配置。":
    "Preflight does not start services or change runtime settings.",
  "系统 / CPU": "System / CPU",
  检查服务: "Service to check",
  执行体检: "Run preflight",
  体检结果: "Preflight result",
  "默认中文，语言选择只保留在当前窗口。":
    "Chinese is the default. Language selection stays in this window only.",
  工作区: "Workspace",
  "切换不移动数据，不停止旧工作区实例。":
    "Switching workspaces does not move data or stop existing instances.",
  打开另一工作区: "Open another workspace",
  数据与发行状态: "Data and release status",
  "源码实施中；最终安装包与全部原生目标验证尚未完成。":
    "Under development. Final installers and all native target checks are still pending.",
  "备份与恢复使用离线迁移指南。已迁移的数据没有自动降级按钮。":
    "Use the offline migration guide for backup and restore. Migrated data has no automatic downgrade action.",
  "当前工作区的实例仍然运行。新位置不会自动迁移旧数据，请先按需停止与备份。":
    "Existing workspace instances keep running. Data does not move automatically; stop and back up as needed first.",
  选择工作区: "Choose workspace",
  从一小步开始: "Start small",
  "选择服务，工作箱负责准备；连接验证结果会如实显示。":
    "Choose a service and let the toolbox prepare it. Connection results come from actual backend checks.",
  "推荐入门 · 轻量": "Recommended · lightweight",
  "启动缓存服务，完成内置连接测试，再复制真实地址。":
    "Start a cache, complete the built-in connection check, then copy the actual endpoint.",
  "检测 Docker 环境": "Check Docker",
  准备镜像与配置: "Prepare images and configuration",
  启动并验证连接: "Start and verify connectivity",
  复制客户端配置: "Copy client configuration",
  查看连接: "View connections",
};
Object.assign(english, {
  应用已验证的配置: "Apply verified configuration",
  "停止场景（保留数据）": "Stop scene (keep data)",
  "停止场景会保留数据；其他场景共用的依赖由后端保留，具体范围见运行任务。":
    "Stopping a scene preserves data. The backend retains dependencies shared by other scenes; see operations for the exact scope.",
  "正在检查…": "Checking…",
  启动前检查未通过: "Preflight checks failed",
  无法确认可用磁盘容量: "Available disk capacity is unknown",
  后端脱敏日志: "Backend-redacted logs",
  "已丢弃 / 截断": "Dropped / truncated",
  行: "lines",
  "等待最近日志…": "Waiting for recent logs…",
  "30 秒未收到新批次；订阅可能已过期，可重新连接读取最近日志。":
    "No new batch for 30 seconds. The stream may have expired; reconnect for recent logs.",
  "10 秒未收到新批次；可能暂无日志或正在等待消费。":
    "No new batch for 10 seconds. There may be no new logs, or the stream may be waiting for consumption.",
  "计划已过期，请重新获取": "This plan expired. Request a new plan.",
  此操作可能永久删除列出的数据:
    "This action may permanently delete the listed data.",
  "尚未安装 Docker": "Docker is not installed",
  "Docker 尚未启动": "Docker is not running",
  "Docker 权限不足": "Docker access is denied",
  "请切换到 Linux containers": "Switch to Linux containers",
  "Compose 版本过旧": "Compose is too old",
  "当前 Docker 目标不受支持": "This Docker target is unsupported",
  "请安装适合系统与 CPU 的 Docker Desktop 或 Engine，然后重新检测。":
    "Install Docker Desktop or Engine for your system and CPU, then check again.",
  "启动已安装的 Docker 运行时，等待引擎就绪后重新检测。":
    "Start your installed Docker runtime, wait for the engine, then check again.",
  "Linux 请按官方说明配置 docker 组并重新登录。工作箱不会后台提权。":
    "On Linux, follow the official docker group setup and log in again. The toolbox does not elevate privileges.",
  "Windows 请检查 WSL2、虚拟化与 Docker Linux 容器模式。":
    "On Windows, check WSL2, virtualization and Docker Linux container mode.",
  "请更新 Docker Compose v2，再重新检测。":
    "Update Docker Compose v2, then check again.",
  "请选择本地 Linux containers 目标，不会自动切换全局 context。":
    "Select a local Linux containers target. The global context will not be changed automatically.",
  "应用 API 由您提供；教学范围为 SQL 查询与 S3 对象读写。":
    "Provide your own application API. Tutorials cover SQL queries and S3 object reads and writes.",
  "应用 API、Agent 与向量扩展由应用提供。":
    "Your application provides the API, Agent and vector extensions.",
  "真实微服务与客户端集成由应用提供。":
    "Your application provides real microservices and client integration.",
  "业务 undo_log、业务代码与 XXL 执行器由应用提供。":
    "Your application provides undo_log, business code and XXL executors.",
  "基础设施不包含业务 Worker；教程另提供测试 Worker。":
    "Business Workers are separate. The tutorial provides a test Worker example.",
  "分别验证订阅与消息收发；Canal → 队列桥接为选做示例，不会自动部署。":
    "Subscription and messaging are checked separately. The optional Canal-to-queue bridge is not deployed automatically.",
  "真实业务采集器及采集目标由应用提供。":
    "Your application provides production collectors and collection targets.",
});

Object.assign(english, {
  导出教学源码: "Export tutorial sources",
  "在原生目录选择器中选择导出位置。只写入公开白名单源码，不会运行示例或复制凭据。":
    "Choose an export folder in the native dialog. Only public allowlisted sources are written; examples are not run and credentials are not copied.",
});

Object.assign(english, {
  "安装 Docker": "Install Docker",
  本草稿的真实环境预检: "Actual preflight for this draft",
  可用磁盘容量未知: "Available disk capacity is unknown",
  请先保存草稿并完成真实环境预检:
    "Save and validate the draft to run actual preflight checks.",
  "草稿所属工作区、目标或已应用版本已变化，请重新验证":
    "The workspace, target or applied generation changed. Validate this draft again.",
  "我理解容量未知，仅同意本次应用":
    "I understand capacity is unknown and consent to this apply only.",
  "此确认仅适用于当前草稿、目标与基础版本；重新验证或身份变化会清除。":
    "Consent applies only to this draft, target and base generation. Validation or identity changes clear it.",
});
