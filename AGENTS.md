# Code Ark 工作规范

## 适用范围与协作

- 本文件适用于整个仓库，人工与自动化协作共同遵守。目录职责以 [PROJECT.md](PROJECT.md) 为准，贡献流程见 [CONTRIBUTING.md](CONTRIBUTING.md)，公开复用见 [SHARING.md](SHARING.md)。
- 当前交付是 17 套 Compose 配置及 1 份宿主机工具指南；`docs/` 中的桌面工作箱尚未实现。没有开发授权时，不创建 `toolbox/` 骨架、不安装产品依赖、不启停环境。
- 开始任务先检查分支、远端和工作区，保留其他任务的修改。仅暂存本次相关文件，不强推或擅自重写历史。
- 新提交与 PR 标题统一使用 `type(scope): 描述`，scope 必填；类型、范围、破坏性变更和验证要求遵循贡献指南。历史提交不为符合新规范而重写。
- 未被要求时不提交、推送、部署或发送外部消息；用户已明确授权的动作按授权执行。创建代理工作分支时使用 `codex/` 前缀。

## 仓库与目录

- 本仓库是本地 Docker Compose 中间件配置集合。修改前阅读根目录 `README.md`、`CONTRIBUTING.md` 和目标服务的 README、Compose、`.env.example`。
- 新服务使用 `<service>-start-local/`，提供 `docker-compose.yml`、`README.md`，使用环境变量时提供 `.env.example`。保留 `mysql-start-lcoal`、`rabbitmq-start-lcoal` 的历史名称。
- 优先复用已有数据库及网络，满足需求即可；不要为新服务重复启动已有中间件或引入不必要的脚本、可配置项。必要的初始化任务放在当前服务目录内。
- 使用 Compose v2 命令 `docker compose`，在目标目录执行。新增配置不写废弃的顶层 `version`；项目与容器名称避免冲突。
- 运行数据、缓存、安装包和本机凭据不属于源码结构。新增或移动目录时同步 `PROJECT.md`；未来工作箱结构变化先同步对应 design、PRD、plan，不能把计划路径写成已实现。

## 配置与数据

- 新增镜像使用验证过的明确版本。按现有 Compose 风格设置资源限制、健康检查和日志轮转；常驻服务使用 `unless-stopped`，初始化任务成功后退出。
- 检查仓库端口与本机占用，默认本地入口绑定 `127.0.0.1`，文档说明容器内部连接地址和宿主机地址。复用外部服务时明确记录网络、主机名和启动前置条件。
- `.env` 已被忽略，不提交或打印真实凭据，不覆盖用户已有配置。`.env.example` 只放公开默认值和凭据占位值，必要变量使用 Compose 必填校验。
- 复用数据库时创建当前服务专属数据库/表，不更改既有业务数据。初始化必须可重复执行，不吞掉真实错误。
- 配置和脚本只读挂载；容器内 Shell 脚本使用 LF 换行，并通过 `.gitattributes` 保持 Windows 检出兼容。
- 普通停止使用 `stop` 或 `down`。不要执行全局 prune 或清空既有数据；`down -v` 只删除当前项目的命名卷，不删除绑定目录或外部数据库。

## 文档与验证

- 文档使用 UTF-8，中文优先。服务 README 包含 English Summary、简介、组件/端口、前置条件、启动、配置、持久化/清理、常用命令和返回根 README 链接。
- 新服务同步中英文根 README 的服务表、数量及中文目录树；描述以实际配置为准。
- 保留用户原有修改，验证只操作任务相关服务。在目标目录运行 `docker compose config --quiet`，避免输出展开后的密码。
- 用户要求启动时实际执行 `docker compose up -d`，检查 `ps -a`、健康状态、初始化退出码和客户端/API；网页首页 200 不代表后台连接正常。
- 初始化流程验证重复启动和保留数据的重建，检查 `git diff --check`、文档一致性和 `.env` 未跟踪，交付时报告实际入口和验证结果。
- 纯文档变更检查 UTF-8、相对链接、目录和来源关系，不为验证文档启动 Docker。提交前检查暂存差异与秘密泄露；交付区分静态检查、真实服务验证及未执行项。

## Temporal

- `temporal-start-local/` 复用 `pgsql-start-local/` 中的 `pgsql-start-dev`，通过外部网络 `pgsql-start-local_default` 连接。先启动现有 PostgreSQL，再启动 Temporal。
- Temporal 创建 `temporal` 和 `temporal_visibility` 两个数据库，沿用 PostgreSQL 现有账号；数据持久化由 `pgsql-start-local/pgsql-data` 管理。
- Server 和 Schema 初始化工具共用 `TEMPORAL_VERSION`。启动顺序：Schema 初始化 → Server 就绪 → Namespace 初始化 → UI。初始化保留已有 Schema 和 Namespace。
- 默认 gRPC `127.0.0.1:7233`，UI `http://127.0.0.1:18080`，Namespace `default`，新 Namespace 保留时间 3 天。业务 Workflow/Activity Worker 由应用提供。
- 验证 CLI 集群健康、Namespace describe、UI Namespace API 及重复启动后的数据保留。停止 Temporal 不停止、删除现有 PostgreSQL；不随意修改已初始化的 History 分片数或降级已迁移的 Schema。
