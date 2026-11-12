# 01 Code Ark 工作箱顶层架构与蓝图同步设计

状态：**审查修订版，技术合同待实现与实机验证**（2026-11-12）。来源：[需求 01](../requirement/01-工作箱产品总览与核心诉求.md) `C01–C08`、[审查报告](../research/04-一键启动工具箱设计完整性审查.md) `ARK-01/02/05/06/09/11/13`。本轮只修订文档，没有工作箱产品代码或发行制品。

## 1. 摘要

工作箱使用 Tauri 2、Rust、React、TypeScript 和 Tailwind，随安装包内嵌现有 17 套服务蓝图与 1 份宿主机工具指南。源码配置保持唯一来源；用户不需要 Git、源码仓库、Node 或 Rust。运行时将逻辑服务、独立实例、已有外部服务分开登记，通过确定性的覆盖层消除手改 YAML、密钥和网络前置。

本次采用“**独立托管实例 + 显式绑定已有依赖**”：文件放在独立工作区，Docker 资源也必须独立命名；发现已有数据库时优先引导复用，不能因路径不同就自动另起一套。同名旧实例只观察，不自动替换。保留原项目名作为运行名会使隔离失效，因此只将其保留为源码兼容元数据；全量复制并维护另一份 YAML 会漂移，因此采用构建期提取加运行时覆盖。

## 2. 参与者

- 新手：安装应用、完成 Docker 引导、选场景启动并复制实际可用的连接参数。
- 日常开发者：复用已有依赖，修改端口或工作区，保留配置及数据。
- 维护者：维护原服务模板、结构化目录、镜像锁及验证证据，不手工维护第二份 Compose。

## 3. 背景

[调研 03](../research/03-CodeArk现有17套服务痛点与兼容基线盘点.md) 记录源码现状；审查进一步确认 Nacos 卷仅有固定 `name`，不是外部卷；XXL-Job JDBC 参数在 Compose 插值时构造；旧 RocketMQ 默认网络入口只适合容器侧。原始模板不能直接视为适配后的桌面蓝图。

框架沿用已核验存在的锁定版本：`tauri 2.12.5`、`tauri-build 2.7.4`、npm CLI/API `2.12.4`。这是实施起点，不宣称持续为“最新”或已经兼容验证；Rust 和前端锁文件必须随实施提交。

## 4. 目标

- 17 套服务、7 个场景、MediaCrawler 指引随包交付，排除本机秘密与数据。
- 资源身份、插值输入、覆盖层和连接片段来自同一份有效配置，端口改绑不遗漏后台地址。
- 升级、工作区切换和配置应用可解释、可恢复，不将旧数据留在磁盘等同于迁移成功。
- 非目标：内置 Docker/虚拟机、通用容器面板、远端 Docker、在线蓝图热更新、自动数据库大版本升级。
- 禁止：运行时回写源码目录；共享旧项目/固定卷后自动删除；静默改已有密码或覆盖手改蓝图。

## 5. 使用场景

1. 无 Git 的用户从安装包启动 Redis，后续选择后端场景，自动准备数据库与对象存储。
2. 已有 `pgsql-start-dev` 的用户将其登记为依赖；Temporal 复用其实际网络和账号，不新建第二个 PG。
3. 不同工作区使用同一 Docker，本机既有 CLI 实例继续运行；改绑端口后各自保留数据。
4. 用户升级应用或改配置，先看到变更范围；不兼容数据升级停止在明确门禁。

## 6. 价值

把首次配置与依赖知识交给目录和编排器；让用户看到真实地址、数据归属和操作结果。发布承诺由实际安装包与客户端矩阵验证，不用固定完成时间承诺替代网络、镜像和硬件差异。

## 7. 方案

以下为本轮依据 `C08` 采纳的实施合同，具体实现选择由维护者按此执行，不回填成历史用户原话。

### 7.1 单一来源、发行资源与模块边界

```text
toolbox/
├── catalog/services.json             # 17 服务、1 指南、7 场景及字段/探针元数据
├── catalog/images.lock.json          # 逐架构 digest、来源与验证状态
├── scripts/sync-blueprints.mjs       # 只读提取白名单资产
├── scripts/verify-blueprints.mjs     # 完整性、引用、敏感文件、LF 与锁定校验
├── src/                             # 界面与 IPC 类型
└── src-tauri/
    ├── resources/blueprints/<id>/    # 构建产物，不手工修改
    ├── resources/sql/               # Seata DDL 及校验和/上游许可
    └── src/                         # contracts/catalog/workspace/env_store/blueprint
```

`ServiceSpec` 至少包含 `id/sourceDir/sourceProject/components/ports/dependencies/envSchema/storage/imageRefs/probes/snippets`；`sourceProject` 仅用于识别原 CLI 资源，不得直接作为托管运行项目名。组件记录 `role=daemon|job`、源 Compose service key 和网络别名。字段记录公开默认值、敏感性、校验器、依赖来源和 `secretPolicy`；探针与生命周期见设计 02，前端协议及安装矩阵见设计 03。

同步严格按**受版本控制的运行资产白名单**复制：Compose、公开样例、只读配置、Dockerfile、SQL、容器内脚本、补丁和内置指南；拒绝软链接越界及 `.env`、数据、日志、备份、密钥、下载缓存。所有 bind/config/build 引用必须在包内可解析；空初始化目录由元数据明确创建，不从用户运行目录复制。`.sh` 转 LF，实施时补 `.gitattributes` 的 `*.sh text eol=lf`，不改已有服务内容。

源码模板中的 `latest` 在发行覆盖层必须替换为 `images.lock.json` 中已解析并核验注册源/架构清单的digest，不现场猜版本。记录 ES/IK 构建基础镜像与插件下载版本、校验和、上游许可。生成蓝图清单包含源提交、资产 SHA-256、目录 schema 版本和镜像锁摘要；应用版本更新不自动更换运行中的镜像。

为避免“先通过实机才能构建首个验收包”的循环，验证分两级：`candidate`要求全部digest/下载校验和可解析、目标架构在镜像清单内、资源和源码检查通过，允许实机结果为pending；仅用于独立隔离验收，不宣称已支持。`release`另要求对应目标的17服务、场景和最终候选制品全部通过。实机/制品结果写在包外验收清单，以源码/锁摘要和候选制品SHA-256关联，不能为改成passed而重写包内锁导致制品哈希再次变化。同一SHA候选包通过后可原样推广发行，失败必须修订、重新构建并重验受影响范围。

### 7.2 工作区、实例及资源所有权

默认工作区为 `~/.code-ark/`，Windows 为用户配置目录下 `.code-ark`；**路径允许中文、空格和非默认盘符**。目录结构：

```text
<workspace>/
├── workspace.json                  # workspace_id、schema_version，无明文密码
├── registry.json                   # 实例、Docker 目标及资源归属，无明文密码
├── operations/<operation_id>.json   # 脱敏操作记录及恢复状态
└── stacks/<service_id>/
    ├── docker-compose.yml          # 原始提取蓝图
    ├── .env                        # 用户值；权限受限，不自动改已有有效值
    ├── .blueprint-manifest.json
    ├── .runtime/<generation_id>/   # 受限权限的候选插值文件、覆盖层与部署记录
    └── <declared-data-paths>/
```

- `WorkspaceIdentity { workspace_id: UUID, root: PathBuf }` 持久化且不得因重开应用变化；新工作区生成新 ID。
- `InstanceIdentity { workspace_id, service_id, instance_id: UUID, project_name }`；项目名 `ca-<workspace UUID前12位>-<service_id>`。创建前检查截断标识碰撞，碰撞时延长标识并持久化实际名称，后续不重新计算。
- 所有固定 `container_name`、显式网络名、显式卷 `name` 均生成实例前缀名称；普通 Compose 网络/卷也使用独立项目名。Nacos 不再引用源码固定卷 `nacos-start-local-data`，使用实例专属卷。
- 容器/网络/卷标记 `io.codeark.workspace-id`、`io.codeark.instance-id`、`io.codeark.service-id`、`io.codeark.blueprint-sha`。资源 ID、规范化 bind 路径、实际卷名与 Docker daemon 身份登记到 registry；工作目录标签只能辅助判断，不能单独授予所有权。
- 依赖网络取 `DatabaseBinding.network_name`，上层加入**该已登记网络**，使用其可解析别名。一个工作区复用一个已选择的基础数据库实例；不同工作区不默认创建重复中间件，优先展示已登记依赖供复用。外部库权限由设计 02 的 `ExternalConnected` 管理。
- 外部实例保持原有名称、卷和生命周期，不能因与 `sourceProject` 一致就成为 Managed。真实外部卷保留 `external: true`，禁止加入删除列表。

跨工作区使用关系不只存各自registry：控制根固定为macOS `~/Library/Application Support/CodeArk/control/`、Windows `%LOCALAPPDATA%\CodeArk\control\`、Linux `$XDG_DATA_HOME/codeark/control/`（未设置则`~/.local/share/codeark/control/`），维护`usage-index.json`及锁；工作区切换不改变控制根。索引记录target/daemon、提供者实例、消费者workspace/实例、binding/claim、场景引用、活动操作和登记状态，不存秘密。测试控制根只能由显式隔离验收配置指定。注册/注销和保守失败规则由设计02定义；移动工作区或删除索引不能视为消费者已停止。

### 7.3 非破坏性物化、升级与工作区切换

首次物化只创建不存在的资产；记录已分发文件哈希。刷新时当前哈希等于旧哈希才允许生成升级候选；用户手改文件时保持原文件，提供差异和显式合并/保留入口，`.bak` 不是自动覆盖的许可。数据/日志/`.env` 不属于蓝图覆盖集合。

更新先完成 schema/镜像兼容检查、备份指引、候选 `config --quiet`；需要迁移的数据库禁止自动降级或以换镜像恢复。活动部署和最近就绪部署分别记录，失败后显示实际部分成功状态，不伪装已回滚。原实例继续使用旧蓝图，直到用户应用更新。

切换工作区只切换视图和注册表，不移动或清空数据，也不停止旧容器；切换前展示仍在运行的实例。v1 不提供运行中数据库二进制目录自动迁移；提供受控导出/导入指南。符号链接、嵌套工作区、Docker 数据目录或无写权限路径不能用于清理目标；写文件使用同目录临时文件、同步与原子替换，Unix 文件 `0600`、敏感目录 `0700`，Windows 限制到当前用户 ACL。

### 7.4 `.env`、密钥与有效配置生成

`EnvDocument` 行级解析保留注释、键序、引号、CRLF 及无末尾换行；支持值含 `=`、`$`、`#`、引号。生成 Docker 插值文件时按 Compose dotenv 语义编码，拒绝无法无损表达的输入并保留原文件。前端不持久化秘密；registry 和操作日志只保存 `SecretRef`。

`SecretPolicy = GenerateOnFirstManagedCreate | InheritBinding | PreserveExisting | OptionalEmpty`。新托管实例按逐字段策略生成，不靠 `change_me_*` 等通用前缀识别全部默认值：

| 字段/服务 | 初次托管策略 | 已初始化实例 |
| --- | --- | --- |
| Nacos Token/身份值 | `OsRng` 原始不少于32字节、Base64；替换公开样例 Token，覆盖模板硬编码身份环境 | 保留；轮换是显式配置操作 |
| Canal 复制密码 | 安全字母数字，24 字符且不超过32 | 从已登记账号取值；失配报错 |
| SearXNG Secret | 32随机字节转64位小写hex | 保留 |
| Kibana Encryption Key | 32随机字节转64位hex，包括替换公开 `change-this-...` 样例 | 保留，避免无法解密已有内容 |
| MySQL/PG/MinIO/RabbitMQ/ES/Grafana 初始化密码、XXL Token | 按服务校验生成32位安全字母数字；无需生成的用户名采用公开默认 | 验证已有有效值，不因补配置重新生成 |
| Seata/XXL 数据库密码、Temporal PG 凭据 | `InheritBinding`，取已认证的依赖来源或其专属账号 | 不生成无关联密码 |
| 可选 License API Key | 默认空；Basic License 用已认证管理连接处理 | 用户明确配置才使用 |

`.env` 缺字段可提出补全候选；已有文件的非空有效值即使恰好等于公开默认，也不自动替换。新托管实例可生成秘密；服务Token的显式更改走配置预览/应用。**v1不提供已初始化数据库账号的一键密码轮换**，包括自有基础库/专属账号；已初始化密码字段禁用随机生成。用户可更新已在数据库生效的连接凭据，但须先认证再应用，不改服务器账号。数据库已存在而本地密钥缺失时进入修复流程，不以重置解决。将来若增加账号轮换，须另立完整计划，不在本期IPC预留一个无后端实现的可用按钮。

### 7.5 Compose 输入与覆盖层合同

最低 Compose **`>= 2.24.4`**，接受兼容的更新主版本；另做实际 `!override` 能力探测。[官方依据](https://docs.docker.com/reference/compose-file/merge/#replace-value)。每次命令使用固定绝对 Docker 路径、设计 02 绑定的目标、`--project-directory <stack>`、独立 `-p <instance_project>`、显式 `--env-file <candidate.runtime.env>`，并依次指定源 Compose 和 `compose.toolbox.override.yml`。清除继承的 `COMPOSE_*`、同名配置环境变量与未经选择的 Docker 目标覆盖；保留经用户选择的必要运行时连接配置，不执行 shell 拼接。

候选有效配置由一次解析得到：用户输入 → 绑定值 → 可选默认值 → 校验 → 编码。源文件先插值再合并，因此覆盖层需要**完整重建 XXL-Job `PARAMS`**，含实际 JDBC URL、用户名、密码和 accessToken；不能只给容器追加 `MYSQL_HOST`。Seata 生成只读 `application.toolbox.yml` 替代挂载，JDBC 使用绑定主机、容器端口、库名与账号，解决源码硬编码 `:3306`。生成 YAML 中字面 `$` 正确转义，禁止二次插值损坏秘密。[插值时机依据](https://docs.docker.com/reference/compose-file/interpolation/)。

所有宿主机入口默认 `127.0.0.1`；用 `ports: !override` 替换每个组件完整发布端口集合，保留 TCP/UDP 区别，不能追加新端口而留下原端口。网络、卷、容器名、只读挂载及镜像 digest 均在确定性覆盖层处理。调用 `config --quiet` 后，内部读取最终模型做精确断言；不得向日志或诊断导出展开的秘密。

| 联动对象 | 必须同步的有效配置 |
| --- | --- |
| Kafka | 外部 advertised listener 使用实际宿主机端口；内部 listener 仍使用同网别名和29092；Kafka UI 取内部地址 |
| Monitoring/SearXNG/Elastic | Grafana root URL、SearXNG base URL、Kibana public ES URL 和全部展示链接随发布端口生成 |
| Nacos | Console 独立端口；主端口与 SDK 所需偏移端口按元数据成组选择、预检及映射，不能只改8848 |
| Seata/XXL/Canal/Temporal | 最终连接值来自 `DatabaseBinding`，引用真实网络、别名与容器端口，宿主机改端口不影响容器间连接 |
| RocketMQ | 采用 §7.6 的 Proxy 入口；Console 和 Broker 继续用内部网络路由 |

### 7.6 RocketMQ 的可达路由

托管默认采用 **RocketMQ 5 Proxy 模式**，不把 Broker 广播地址改成 `127.0.0.1`。在现有 Broker 组件中通过 `mqbroker ... --enable-proxy` 启动本地 Proxy，不额外启动一套 Broker/NameServer；实例只读配置保持 Broker 内网别名、内部 `10911/10912`，Console 通过内部 NameServer 查询 Broker。

新增托管发布入口默认 `127.0.0.1:18082 → broker:8081`，可改绑；先检查全仓端口与本机占用。新手示例使用官方 `rocketmq-client-java` 的 gRPC endpoint `127.0.0.1:<proxy_port>`，同网容器使用真实 Broker 网络别名`:8081`。发布的传统 Broker 端口仅用于明确标注的高级兼容操作；**宿主机旧版直接访问 Broker 路由不在默认承诺内**，不能继续提供声称双环境都可用的旧开关。官方启动及客户端入口见 [RocketMQ Quick Start](https://rocketmq.apache.org/docs/quickStart/02quickstart/)。

实施必须以锁定镜像验证 Proxy 参数、资源上限、Topic/消费组初始化及 SDK 兼容；三端实际宿主机收发、容器收发与 Console 查询全部通过才可发布此服务。不通过应阻断相应发行矩阵，不能退回不可达回环广播地址。

### 7.7 共享类型与文件契约

`contracts.rs` 定义稳定、可序列化的跨模块类型，前后端类型从同一 schema 生成。至少包括 `WorkspaceIdentity/InstanceIdentity/ResourceRef/SecretRef/DatabaseBinding/EffectiveConfig/ProbeSpec/OperationRecord/ServiceSnapshot`，字段 schema_version 明确；Plan 01 定义构建入口，Plan 02 扩充运行协议，Plan 03 消费相同类型。

`EffectiveConfig` 含 `generation_id/target_id/instance_id/ports/endpoints/secret_refs/config_digest`；秘密仅在后端执行时解引用。运行快照必须指出 desired、applied、last_ready 三种 generation，不能把保存草稿当应用完成。

## 8. 实施与验收

唯一承接：[PRD 01](../prd/01-CodeArk工作箱底座与蓝图管理PRD.md) → [Plan 01](../plan/01-服务清单与蓝图同步实施计划.md)。须验证白名单提取、无 Git 安装包资源、两工作区/旧 CLI 隔离、字面特殊密码、最终 JDBC 及所有端口耦合、Nacos 固定卷改名、RocketMQ Proxy、用户文件冲突、升级及中文/空格路径。

文档合同已覆盖审查项不等于实现已通过。每项测试、镜像架构及制品验证均保持未执行，真实证据由对应 Plan 记录，任何缺项阻断“全量一键可用”的发行声明。

[返回设计导航](00-设计导航.md)
