# 代码方舟 · Code Ark 常见问题与项目事实

本页面向使用者和引用本项目的读者，集中回答产品定位、获取方式和验证范围。可变化的状态核对日期为 **2026-11-13**；具体版本的下载内容以 [GitHub Releases](https://github.com/StephenQiu30/code-ark/releases) 为准。

[项目首页](../README.md) · [English FAQ](#english-faq) · [服务索引](../services/README.md) · [工作箱源码说明](../toolbox/README.md)

## 代码方舟是什么？

代码方舟（英文名 **Code Ark**，仓库名 `code-ark`）是本地 Docker 开发环境桌面工作箱，使用 Tauri 2、Rust 和 React 构建，界面使用 shadcn Radix 与 Tailwind CSS。它把服务选择、环境预检、依赖初始化、就绪探针、启停和连接信息集中在本地工作台。仓库另提供 **17 套 Docker Compose 配置和 1 份宿主机工具指南**，可独立使用。

适用场景是本地开发、接口联调、集成测试和学习，覆盖数据库、缓存、消息队列、工作流、搜索、对象存储和监控。完整服务表仅在[服务索引](../services/README.md)维护。

## 现在可以下载桌面应用吗？

当前桌面工作箱提供[源码预览和开发说明](../toolbox/README.md)，**尚无公开桌面安装包**。可直接使用仓库的 Compose 配置；桌面源码的构建前置与操作限制以工作箱 README 为准。

[ARM64 镜像预发布](https://github.com/StephenQiu30/code-ark/releases/tag/images-2026.11.13-arm64)提供 MinIO、XXL-Job 和 Elasticsearch + IK 的 Linux ARM64 服务镜像归档，以及校验和与对应源码/许可资料。这些归档不是桌面安装器。镜像制品、源码预览与最终应用安装包是不同的交付范围。

## 需要账号或云服务吗？可以完全离线吗？

使用本地工作箱无需注册 Code Ark 账号。Docker 必须另行安装并运行；被启动的服务仍可能需要自己的账号、初始化凭据或外部依赖。

首次获取源码、依赖或容器镜像需要可用下载来源。离线指南可在本地阅读，但 SearXNG 等访问外部数据源的服务仍需要网络，因此不能把整个项目描述为完全离线可用。

## “一键启动”具体做什么？

桌面工作箱在用户选择服务或场景后，检查 Docker 目标、端口、内存与磁盘，准备所需镜像并处理受支持的数据库绑定和初始化，再执行部署与协议探针。首次打开只显示工作台，不自动启动容器。

就绪需要实际数据库、消息或后台 API 检查，不能只凭容器正在运行或网页首页返回 HTTP 200。初始化、变更和恢复受配置及资源归属守卫约束；失败或资源不足会显示对应状态，不应解释为环境已可用。见[编排与探针设计](design/02-跨平台Docker探测与自动化编排设计.md)及[当前实现范围](../toolbox/README.md)。

## 已验证哪些平台和功能？

当前桌面变更操作仅在 macOS Apple Silicon（arm64）开放。截至 2026-11-13，16 个预设完成两轮隔离的真实启动、协议就绪与停止，持久化范围按各服务记录核验。完整 Elastic Stack 因现场可用内存小于要求的 6 GiB 被预检拒绝，没有创建其容器，未计为就绪。

这些结果不代表所有业务流程已验收，例如完整消息收发、任务执行器调度、故障恢复矩阵及所有场景仍有独立验证范围。AMD64 的三个预构建镜像归档仍 pending；Windows、Linux、Intel Mac 的桌面原生运行和最终安装包也未完成验收。Compose 配置的具体前置条件见各服务 README，不能把桌面限制直接等同于所有模板的平台支持结论。

## 停止或重建会删除数据吗？

桌面工作箱默认运行工作区为 `~/.code-ark`，不放在源码目录中；它通过实例身份、数据归属和共享引用检查管理启停、恢复及重置。普通停止保留数据，重置需要明确选择与确认；没有归属记录的非空目录不会自动作为可清理资源采用。请保留重要数据的独立备份，验证通过不等于没有数据风险。

直接使用 Compose 时，`docker compose stop` 和普通 `docker compose down` 不删除持久化目录或命名卷。`docker compose down -v` 会请求删除当前项目命名卷，不删除宿主机绑定目录或外部数据库；清理范围应按对应服务 README 核对。不要执行全局清理来解决单个服务的问题。

## 可以用于生产吗？许可证是什么？

项目面向本地开发和学习。生产使用需自行规划认证、TLS、备份、容量与高可用；源码预览尚未完成最终安装包和完整业务验收。

仓库原创代码、配置与文档按 [MIT License](../LICENSE) 提供。第三方镜像、服务、SDK、SQL 和插件遵循各自许可证；仓库 MIT 不会将上游重新许可。分发边界见[开源与共享规范](../SHARING.md)。

## 项目事实与证据入口

| 事实 | 直接来源 |
| --- | --- |
| 17 套 Compose 配置、1 份宿主机指南与各服务入口 | [唯一服务索引](../services/README.md) |
| Tauri 2、Rust、React、shadcn Radix、Tailwind CSS | [工作箱说明](../toolbox/README.md)、[前端依赖](../toolbox/package.json)、[Rust 依赖](../toolbox/src-tauri/Cargo.toml) |
| 桌面实际检测消息、Redis 与数据库依赖组验证 | [桌面与预设启动验证](research/08-桌面全局消息与预设启动验证.md) |
| 其余预设两轮结果、Nacos/RocketMQ 新协议观测、Elastic 资源拒绝 | [其余预设真实启动验证](research/09-其余预设真实启动验证.md) |
| ARM64 镜像制品功能、校验和、许可及匿名下载核验 | [镜像制品验收与发布](research/07-ARM64预构建镜像制品验收与发布.md) |
| 已知限制、开发步骤和当前实现范围 | [工作箱 README](../toolbox/README.md) |

引用时请带上日期、平台和验证范围。设计文档说明预期合同，测试记录说明实际执行结果；历史记录可能保留修复前失败，最终状态需结合相应后续章节阅读。

## English FAQ

### What is Code Ark?

Code Ark (代码方舟; repository: `code-ark`) is a desktop toolbox for local Docker development environments. It uses Tauri 2, Rust, React, shadcn Radix, and Tailwind CSS. The local workspace brings together service selection, environment checks, dependency initialization, readiness probes, lifecycle actions, and connection details. The repository also contains **17 independently usable Compose configurations and one host-tool guide** for development, integration testing, and learning. The [service index](../services/README.md) is the complete catalog.

### Is a desktop installer available?

No public desktop installer is available yet. See the [desktop source preview](../toolbox/README.md) for build requirements and restrictions, or use Compose directly. The [ARM64 image prerelease](https://github.com/StephenQiu30/code-ark/releases/tag/images-2026.11.13-arm64) contains Linux ARM64 service image archives for MinIO, XXL-Job, and Elasticsearch + IK, with checksums and source/license material. Image archives are not application installers. Check [Releases](https://github.com/StephenQiu30/code-ark/releases) for version-specific downloads.

### Do I need an account or internet access?

No Code Ark account is required. Docker must be installed and running separately; individual services can still require their own credentials and dependencies. Downloading source, dependencies, or images requires an available download source. Offline guides are readable locally, but services such as SearXNG need external data sources. The entire project is not guaranteed to work offline.

### What does one-click startup cover?

After a user selects a service or scenario, the toolbox checks the Docker target, ports, memory, and disk; prepares images; handles supported database bindings and initialization; deploys; and runs protocol probes. Opening the app does not start services automatically. Readiness requires actual database, messaging, or backend API checks. A running container or HTTP 200 homepage alone is insufficient. See the [orchestration design](design/02-跨平台Docker探测与自动化编排设计.md) and [implemented scope](../toolbox/README.md).

### What has passed acceptance?

Status checked on **2026-11-13**: desktop operations that change services are enabled on macOS Apple Silicon (arm64). Sixteen presets passed two isolated rounds of real startup, protocol readiness, and stopping with data retained according to their service-specific persistence scope. The full Elastic Stack was refused before container creation because available memory was below its 6 GiB requirement; it did not pass full-stack readiness.

These checks do not establish complete business workflow coverage. Message delivery, executor scheduling, the full failure matrix, and every scenario have separate acceptance scopes. Three AMD64 prebuilt image archives remain pending; other native desktop targets and final installers await acceptance. Consult each service guide for Compose prerequisites. The [project facts table above](#项目事实与证据入口) links to implementation and dated test evidence.

### Will stopping or rebuilding delete data?

The desktop workspace defaults to `~/.code-ark`, outside source control. Instance identities, resource ownership, and shared references guide lifecycle actions and recovery. Ordinary stopping retains data; reset requires an explicit scope and confirmation. Nonempty directories without ownership records are not automatically adopted for cleanup. Keep independent backups of important data.

With Compose, ordinary `stop` and `down` preserve persistent directories and named volumes. `down -v` requests removal of this project's named volumes, not host bind mounts or external databases. Check the selected service guide before cleanup; do not use global cleanup for a single service.

### Is it for production? What is the license?

Code Ark targets local development and learning. Production use requires environment-specific authentication, TLS, backups, capacity, and availability planning. The source preview has not completed final installer or full business acceptance.

Original repository code, configurations, and documentation use the [MIT License](../LICENSE). Third-party software retains its own licenses; see [sharing rules](../SHARING.md). When citing results, include the date, platform, and scope. Design documents describe intended contracts; test records describe checks actually performed.
