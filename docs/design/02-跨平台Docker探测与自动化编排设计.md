# 02 跨平台 Docker 探测与自动化编排设计

状态：**审查修订版，技术合同待实现与实机验证**（2026-11-12）。来源：[需求 02](../requirement/02-服务依赖与新手使用场景清单.md)、[审查报告](../research/04-一键启动工具箱设计完整性审查.md) `ARK-01/03/04/07/08/09/10/11/12`。实例、覆盖层和有效配置遵循设计 01；本文件是运行语义的权威来源。

## 1. 摘要

编排器在固定的本地 Docker 目标上，完成发现、预检、依赖复用、初始化、部署、业务探针与可恢复操作。所有动作受资源所有权和权限范围约束；实际可连接才成为 Ready。外部观察不授予写权限，选择复用也不授予容器生命周期和整库删除权限。

## 2. 参与者

- `DockerRunner`：固定二进制、context、daemon、工作目录和参数，返回结构化错误，禁止 shell 命令拼接。
- 目录/配置层：提供实例、最终插值配置、绑定和探针定义。
- 编排器：跨进程锁、持久步骤记录、依赖排序、重试/取消、恢复和危险操作复核。
- 用户：通过界面选择已有依赖及一次性授权，处理硬件/网络问题，明确确认重置范围。

## 3. 背景

源码服务混合数据库、消息系统、Web 控制台和一次性初始化任务；相同 `running` 状态不能证明它们具有相同可用性。旧计划只有内存锁和正常重启测试，无法覆盖半途建表、应用退出、外部资源、配置应用及数据重置。

## 4. 目标

- 无需终端发现本地 Docker；在有外部环境、特殊路径及端口占用时给出可执行出口。
- 17 套服务、7 场景的启停和幂等初始化具有明确步骤、超时、状态及复用策略。
- 专属数据库和数据目录的归属可验证；停止/删除外部实例和通用 prune 不属于普通流程。
- 非目标：管理远端 daemon、绕过权限自动提权、自动迁移不兼容数据、默认接管外部生命周期。

## 5. 使用场景

- 新手机器未安装 Docker：引导安装/启动和复检，不要求输入启动命令。
- 已有仓库 PG/MySQL：先展示复用入口，认证和授权完成后只初始化当前服务需要的专属资源。
- 场景启动在 SQL 导入或镜像下载中断：保留已有成功服务，显示失败步骤，复检后安全续跑。
- 多个场景共用数据库：停止场景不默认停止仍被其他场景使用的基础服务；重置先列清归属和影响。

## 6. 价值

避免以“命令成功退出”替代产品结果；将可恢复运行状态与资源边界落实到后端，使新手不需要理解 Docker 标签、SQL 重放或进程退出码。

## 7. 方案

### 7.1 Docker 发现、目标与能力门禁

macOS 探测 PATH、`~/.docker/bin`、`/usr/local/bin`、`/opt/homebrew/bin`、Docker.app、OrbStack/Rancher 常用路径；Windows 探测 ProgramFiles Docker 路径，全部子进程使用 `CREATE_NO_WINDOW`；Linux 探测标准路径，区分 socket 权限不足并提示按发行版授权后重新登录，不后台执行提权。

`DockerTarget { target_id, cli_path, context_name, endpoint_kind, daemon_id, os_type, architecture }` 在环境选择后绑定。只支持已核验的本地 Linux containers 目标：Unix socket、Windows named pipe、本地 Desktop/OrbStack 虚拟化端点；SSH/TCP 远端及 Windows containers 拒绝执行。对本地 endpoint 也读取并记录实际 daemon 身份，每个变更步骤前复核，变化则中止。运行时不能修改用户全局 context；清除未选中的 `DOCKER_HOST/DOCKER_CONTEXT` 等覆盖，所有命令显式指向所选目标。[Context 官方说明](https://docs.docker.com/engine/manage-resources/contexts/)。

门禁包括 CLI 存在、daemon 可达、Compose `>=2.24.4` 和 `!override` 能力、Linux containers、镜像digest与目标CPU清单匹配、工作区权限、端口、内存和存储。候选隔离验收允许实机结果pending，公开发行才要求全部验收通过，遵循设计01的candidate/release分级。版本比较支持新主版本；能力探测使用合成无秘密配置，不调度服务。

### 7.2 端口、内存、磁盘与网络检查

- 默认仅 IPv4 回环发布；同时区分 TCP/UDP，按完整映射及耦合组检查。探测端口不能消除检查后竞争，实际 bind 冲突归类为 `PortConflict` 并保留重选入口，不静默终止占用进程。
- 仅在 inspect 证明为同一 target/instance/component 的发布端口时免重复占用报错。外部容器、其他实例、非 Docker 进程不能被误当“本服务”。
- Docker `MemTotal` 与当前运行场景共同估算；Elastic 至少6 GiB，使用整数单位换算并报告需求、可用值及运行时设置入口。
- 分别检查工作区磁盘和 Docker 存储（镜像/构建缓存/命名卷）空间。支持目标需提供 `StorageCapacityProbe`：原生 Linux 对 daemon 数据目录所在文件系统查询；Desktop/OrbStack 使用其可读取容量信息，或经用户同意运行锁定 digest 的轻量诊断任务在 daemon 侧查询可用空间。读取失败返回 `UnknownCapacity` 并给出运行时存储设置/复检入口，不能拿宿主机工作区空间冒充 daemon 空间。
- catalog 每项声明 `minWorkspaceFreeBytes/minDaemonFreeBytes/estimatedPullBytes`；实施基于锁定制品实测填值。未知空间时允许观察，默认不执行大镜像拉取/下载/加载；普通启动需显式接受本次未知容量风险并记录，发行测试必须验证可测目标路径。
- 现有外部网络检查 target、网络 ID、可达别名；Linux bind 目录按镜像 UID/GID 创建并做写权限验证，不对任意宿主目录递归 chmod/chown。

### 7.3 资源识别与数据库绑定

`ContainerOwnership = Managed | ExternalObserve | NotPresent | OwnershipConflict`。Managed 要求设计 01 的实例标签、registry、daemon 身份、资源 ID/挂载路径全部相符；相同路径或相同名称均不足以获得所有权。发生丢失、漂移或歧义时只观察和修复登记。

`DatabaseBinding { binding_id, target_id, engine, lifecycle_owner, container_ref, network_name, network_alias, container_port, host_endpoint, credential_ref, allowed_databases, grant }`：

v1的ExternalConnected明确限定为**同一已固定本地Linux-containers daemon中的MySQL/PG容器**，具备可验证实例身份、可加入的网络和可认证账号。本机原生数据库、远端数据库/daemon、无法验证身份或不可达的网络拓扑不在自动编排范围：返回`UnsupportedDependencyTopology`，说明原因并提供观察/拓扑指引或显式创建独立托管实例，不悄悄新建替代库。`host_endpoint: Option<Endpoint>`；有发布端口时从宿主认证，无发布端口则使用该daemon内已锁定一次性SQL客户端在绑定网络认证/查询指纹，凭据经受限输入传递，不要求用户开放端口。两条验证路径都不能把不同服务器当同一绑定。

- `lifecycle_owner = Managed(instance_id) | ExternalConnected`；`ExternalConnected` 是显式连接授权，不改变外部容器所有权。
- 引用其他工作区Managed提供者时仍记录其真实所有者，但消费者不获得提供者的启停/重置权限；只有拥有者工作区在查询公共UsageIndex并完成影响确认后可发起生命周期操作。
- 发现已有可用基础数据库时不自动启动第二套；引导选择已登记绑定或在界面填凭据并验证。没有已有实例时才按场景创建一个托管基础库；同工作区后续场景复用它。
- 外部默认 `grant=ReadOnlyConnect`，不读取未选择的外部 `.env`。用户可在界面显式授权 `InitializeNamedResources`，授权展示并锁定服务器、准确库名、账号、待执行建库/授权/Schema 步骤；不能凭“连接成功”推断可自动写任意库。
- 启动上层服务前认证到实际端点，并确认内网别名、账号权限与数据库范围；不能把 root 初始化密码写成另一个随机值。宿主机端点用于认证/连接测试，内网端点用于服务连接；二者都必须指向所登记服务器。
- 外部服务的启停/移除、外部账号轮换、整库重置始终禁用。撤销绑定先提示依赖，保留外部资源；所有外部写操作仅针对已授权专属资源，不改其他业务记录。

原 `temporal-start-local/` CLI 仍按根规范复用 `pgsql-start-dev` 和 `pgsql-start-local_default`，本轮不更改其配置。桌面检测到该 PG 时优先登记此真实网络与账号；全新环境仅创建一个托管 PG，再把上层覆盖层绑定到它。不得另起 Temporal 私有 PG。

### 7.4 凭据、数据库归属及可恢复初始化

`DatabaseClaim { claim_id, binding_id, daemon_id, server_fingerprint, database_names, owner_instance_id, created_by_codeark, ddl_version, ddl_checksum, status }`；各专属数据库内有保留的归属表 `codeark_resource_owner`（字段含 claim/instance），registry 另记；对同一服务器/库名跨工作区加锁。标记存在、服务器指纹与登记记录相符才可视为自有，不能因名称相似或存在框架表就认领。服务器指纹采用可核验的引擎实例标识及目标资源身份；权限不足以核验时可只读连接，但不授予重置权限。

托管 MySQL 的 Seata/XXL 默认分别使用 `ca_<workspace标识>_seata`、`ca_<workspace标识>_xxjob`，每服务专属账号；保持合法标识符、长度及系统库拒绝列表。Canal 使用专属复制账号及所需权限。管理凭据来自已认证绑定，服务凭据只由一次账号创建产生并写入 `SecretRef`；失配进入修复，不悄悄 `ALTER USER` 外部账号。

Temporal 沿用 PG 现有账号，数据库名固定 `temporal`、`temporal_visibility`，Server 和 Schema 工具同一 `TEMPORAL_VERSION`，History 分片数不变；对同一服务器的这对库只允许一个所有者。已有非本实例 Schema 不能直接认领/迁移；需单独展示版本、备份和专属授权后复用，或只观察。多工作区可连接同一已登记 Temporal 实例，但不各自迁移同一对库。

| 服务 | 初始化合同 | 中断/已有状态处理 |
| --- | --- | --- |
| Nacos | 使用实例专属卷；不预创建源码固定名称卷，不宣称其是 external | 归属不明的同名卷拒绝挂载或删除 |
| Temporal | 认证 PG → 检查/取得双库 claim → Schema job → Server 集群可用 → Namespace job → UI | 成功任务退出0；保留已有 Namespace 设置，新增默认3天；Schema 不降级，失败不伪造完成标记 |
| Canal | 校验 ROW binlog、服务端账号/权限、指定账号密码认证，写入同一来源凭据 | `CREATE USER IF NOT EXISTS` 后还要验证认证；已有账号不匹配则阻断，不只覆盖本地密码 |
| XXL-Job | 把真实 SQL 解析为版本化建表及种子步骤，移除固定库选择；执行于已 claim 专属库，逐步记录 | 检查全部必需表/列与种子状态，不能仅看 `xxl_job_info`；仅补缺失兼容项，已存在用户/任务不重写；不兼容进入迁移门禁 |
| Seata | 内置对应1.8.0系列四表 DDL及校验和；专属账号与实际 JDBC 配置 | 逐表/列核验，缺失兼容项补建，已有业务行不改；版本不符拒绝自动迁移 |
| Elastic | 内存/磁盘/架构门禁 → 校验/加载producer的ES/IK原生归档 → ES → 只负责 `kibana_system` 配置的 job → Kibana/Logstash | 桌面移除源码build项，不让用户端隐式构建；覆盖job去除自动trial，Basic按真实状态处理，HTTP错误不吞掉、不打印秘密 |

初始化 journal 与 SQL 结果核验共同决定状态，不能把 MySQL DDL 当作可整体回滚事务。创建数据库前持久化claim预留及计划，创建后立即写归属标记；若在创建与标记间崩溃、无法证实成功归属，则进入RecoveryRequired，不自动认领未知空库或删除它。进程重开重新探测已完成事实；对已有未知库不尝试“补表修复”。SQL标识符严格校验/引用、值参数化，禁止把用户输入直接拼入数据库/账号语句。

v1不执行已初始化数据库账号轮换；更新连接凭据必须先证明新值已在真实服务器有效，再应用依赖候选，失败保留旧连接值。首次新账号创建仍在本期。后续轮换需单独定义旧/新SecretRef持久恢复和依赖重建失败处理，本期不暴露轮换命令，禁止只改.env假称密码已更换。

### 7.5 就绪判定与全量探针

`ProbeSpec { probe_id, component_id, role, kind, endpoint_ref, credential_ref, timeout_secs, attempts, success_rule }`；job 必须成功退出0，daemon 需要进程存活和表中业务探针。没有 Docker healthcheck 的镜像由 Rust 网络/协议客户端或锁定验证任务探测，不假定镜像内有 curl。

| 服务 ID | 运行就绪条件 | 发行功能验证（隔离数据） |
| --- | --- | --- |
| pgsql | 展示的账号/库成功认证，SQL `SELECT 1` | 写入记录、停止/保留数据重建后仍存在 |
| mysql | 实际下游账号可认证、权限/库/ROW binlog符合使用者需求 | 专属库读写、半初始化恢复及重建保留 |
| redis | PING成功 | 测试键读写及AOF保留 |
| minio | S3认证可列出授权范围、服务ready | 测试桶/对象上传下载、保留数据 |
| kafka | broker元数据可用、UI后端能读集群 | 宿主机和同网容器生产/消费，改绑端口后重复 |
| canal | MySQL复制前提/账号通过、Canal客户端可订阅并取空批次，无复制致命错误 | 专用测试表变更由Canal客户端读取；不把Kafka同时运行当转发成功 |
| rabbitmq | AMQP实际账号/vhost可连接、管理API认证 | 发布/消费、持久消息保留 |
| rocketmq | NameServer/Broker路由、Proxy SDK连接和Console后台查询通过 | Proxy宿主机/容器收发及Console Topic查询 |
| temporal | Schema/Namespace job成功；CLI cluster health、namespace describe、UI Namespace API通过 | 测试Workflow由测试Worker执行，重复启动设置/数据保留 |
| xxjob | 实际专属库认证和结构/种子完整，管理端登录/API读取成功 | 登录后读取测试任务，示例执行器完成一次调度 |
| nacos | 认证API、配置读取及SDK连接可用，Console后台连接通过 | 专用配置写读、重建保留，端口组改绑后SDK通过 |
| sentinel | Dashboard接口可用；无人接入时标明等待客户端 | 示例客户端注册并产生可观察请求/限流结果 |
| seata | 四表结构/账号可用，协调端客户端握手及Console认证通过 | 示例业务库的undo_log及事务回滚；不写用户业务库 |
| elastic | job退出0，ES认证健康达到yellow/green，IK接口、Kibana后台、Logstash pipeline API可用 | 测试日志进入索引，可查询且保留数据 |
| monitoring | Prometheus ready及预置targets正常、Grafana认证/数据源健康和查询通过 | 实际查询有数据，仪表盘/账号保留 |
| rsshub | healthz和内置确定性RSS校验通过 | 可解析XML；真实站点路由独立验证并区分上游错误 |
| searxng | JSON搜索接口可调用、至少一个配置引擎实际有可用响应 | 请求成功、JSON结构/引擎失败可诊断；网络不可用为Degraded |

就绪默认只做授权范围内非破坏性读取/连接；发行写读探针在明确隔离的测试实例和测试标识下运行，并清理其自建资源。对于框架客户端缺失，显示“基础服务就绪，等待应用接入”，不声称用户业务已跑通；SDK/协议探针工具、版本与镜像由 catalog 声明并锁定，安装包不依赖用户开发工具。

### 7.6 状态机、步骤与错误

`ServicePhase = Uninitialized | Stopped | Starting | RunningUnverified | Ready | Degraded | Failed | AwaitingAction | ExternalObserve`。只有全部必需job和探针通过才为 Ready；外部只观察不冒充托管就绪。探针失效后撤销Ready，快照注明采样时间和最后成功时间。

`OperationRecord { operation_id, target_id, instance_ids, request_digest, desired_generation, applied_generation, last_ready_generation, phase, steps, created_resources, error_code }`；`StepProgressEvent { operation_id, seq, service_id, step_id, phase, message_code }`，事件可由持久快照补齐。秘密不出现在参数日志或journal。

启动按DAG**逐节点**执行，不是全局先认证/建库再启动基础库：

1. 加锁、复核目标/所有权，解析已有绑定与去重DAG，完成可离线预检及候选计划。
2. 新托管基础库：物化初始化凭据→拉取镜像→启动MySQL/PG→端口/服务探针→管理账号真实认证→登记绑定；管理连接未通过不得开始任何下游DDL。已登记外部或已运行托管基础库不重建，直接认证/复核。
3. 上层节点：登记跨工作区消费者→认证其依赖→claim/账号/可恢复预置→生成包含实际绑定的最终配置→拉取/下载/加载→该节点部署/job→业务探针→记录applied/last_ready。
4. 对下一个节点重复，全部必需节点通过才结束场景。Temporal的Schema/Server/Namespace/UI顺序在其节点内保持；资源不足/某节点失败保留已完成节点并定位失败步。

动态依赖根据绑定解析，不固定“只要启动Temporal就再启动PG”。

默认上限：单次协议连接5秒、单探针30秒/重试3次、初始化/就绪阶段10分钟、停止60秒、拉取/下载/加载30分钟；等待每2秒更新状态，拉取/下载/加载连续120秒无进展提示网络检查，仍有总截止时间。服务可在catalog给出有依据的更具体期限，不能无限等待。错误码至少包含 `DockerMissing/DaemonUnavailable/ComposeTooOld/UnsupportedTarget/UnsupportedDependencyTopology/TargetChanged/OwnershipConflict/PortConflict/InsufficientMemory/InsufficientStorage/UnknownCapacity/CredentialMismatch/InitializationPartial/SchemaIncompatible/ProbeFailed/TimedOut/Cancelled/RecoveryRequired`。

### 7.7 锁、取消、恢复及配置应用

应用每用户单实例，工作区另用OS文件锁；公共控制目录按daemon ID及数据库claim加跨进程锁，操作多个资源时按稳定ID排序避免死锁。不能只靠 `tokio::Mutex`。锁所有者死亡后根据实际资源和journal恢复；非协作外部工具仍可能改变资源，每一步变更前再核验ID/归属。

`UsageIndex`位于设计01规定的公共控制根，不随workspace切换。每个消费者在上层初始化/启动前原子登记provider/target/claim/consumer workspace+instance/场景，安全停止/解绑并确认无活跃操作后才注销；心跳过期仅为Unknown，不能自动当作已停止。提供者stop/down/reset在持同一公共锁时查询全部workspace使用者并核对daemon实际容器；索引损坏、无法访问消费者工作区、登记缺失但发现关联容器时保守拒绝，提供索引修复/重新验证入口。不同OS用户或未登记业务连接无法完整枚举时不能声称“全部外部依赖已发现”；基础库停止前明确这一限制，整实例重置还必须枚举并核验其全部非系统库/账号/连接范围，存在未知业务资源则拒绝。

索引修复只加载用户重新选择的工作区registry、实际实例标签/网络及操作journal，在公共锁内重建可证实引用；没有证据的条目保持Unknown，不以“修复”清空。已无运行实例/活跃操作且资源关系可验证的停用消费者，可经用户确认解除陈旧登记；无法核验时只提供导出诊断/恢复原工作区指引，继续禁止提供者重置。

取消下载停止网络流并处理受控临时文件；取消pull/load可终止CLI，但daemon可能继续内部任务，须重新inspect展示实际状态。SQL不强制中断未知事务，在步骤边界停止，不确定结果标RecoveryRequired。不自动回滚迁移或删除已有依赖；失败保留成功步骤，提供复检重试与停止本次新建且未被其他场景使用的服务。

配置流程：`Draft → Validated → Applying → Ready | Failed | RecoveryRequired`。预览差异，候选配置校验后使用 `compose up -d` 应用必要重建，**不能用restart代替配置重建**；就绪后提交`.env`更改和last_ready。保存草稿不改变运行服务。失败且无数据格式变更可按记录恢复上一配置并重新探针；有Schema/数据变更时禁止自动降级。崩溃后用actual inspect判定applied_generation，而非强行回写旧指针。

### 7.8 停止、拆除、重置和外部迁移

- `stop_service/stop_preset` 计算反向依赖及场景使用记录。停止场景默认只停止该场景独占并由它启动的托管实例；共用依赖和启动前就运行的服务保留，影响其他场景时明确列出并确认。外部数据库永不自动停止。
- `down_service` 先检查依赖与所有权，仅移除本实例容器/网络，保留数据；不使用全局prune或删除孤立资源选项。
- `plan_reset` 返回不可变 `ResetPlan { plan_id, target_id, instance_id, exact_resource_ids, database_claims, impact, fingerprint, expires_at }`，有效5分钟。用户输入服务名并确认准确资源后，后端携plan_id复核目标、ID、归属和依赖，再执行；前端确认不是唯一保护。
- 本地清理只针对catalog声明且位于实例边界内的真实目录、明确登记的自有卷。禁止顺着symlink越界，禁止按名称前缀批量删。执行清单化的容器/卷删除，不把宽泛 `down -v` 当所有权证明。
- 数据库重置要求 `created_by_codeark=true`、registry和库内claim双重吻合、服务器指纹一致、独占且非系统/外部库；否则禁用整库删除。Temporal只处理其已claim双库；外部连接库即使有初始化授权也默认禁止整库重置。基础数据库重置要列出**全部**已登记使用者，不能承诺整实例重置还保留该实例其他库；存在外部/未知业务库时拒绝整实例重置，改用单服务清理或导出指引。
- 原“停止并删除外部容器后接管”入口取消。提供只读观察、显式绑定、创建新托管实例三条路径；数据迁移使用逻辑导出/导入指南，并明确原数据不会自动出现在新实例。外部容器自动移除和自动恢复重建均不属于v1。

### 7.9 后端库、协议工具与兼容门禁

依据[技术选型核验](../research/05-完整技术选型与服务制品核验.md)，候选后端为Tokio1.54.1、serde1.0.229/serde_json1.0.154、SQLx0.9.0、Reqwest0.13.5、sysinfo0.40.3、dirs7.0.0、uuid1.29.2、thiserror2.0.21、tracing0.1.44、secrecy0.10.3/zeroize1.9.1。编译候选Rust1.99.1；各版本存在性/声明MSRV已查，联合编译与依赖树未验，不提前写锁或创建源码。

SQLx关闭默认Any/SQLite/编译期查询宏，只启用MySQL/PG、Tokio及选定rustls；本地允许范围内的无TLS MySQL需要显式mysql-rsa以适配现有认证。Reqwest采用0.13的json/stream/rustls特性，SQLx使用同路线AWS-LC后端，验证依赖树无provider歧义及构建原生依赖；不用旧版本特性名拼凑。编译不连接业务库，运行凭据显式注入；健康探针不经过继承的公共HTTP代理，发行资源下载另使用受控网络配置，TLS不因方便被禁用。

跨进程锁采用Rust标准File专用锁文件（1.89已稳定）；try_lock配合操作期限/异步等待，不在Tokio工作线程无限阻塞，不复制持锁句柄重入。JSON/journal/UsageIndex协议保持，不引入新的元数据服务或SQLite；跨文件恢复依赖journal与实际资源核验，不声称JSON本身具备事务性。

镜像保证器先解析ImageArtifactRef：registry走不可变manifest；原生重打包归档按设计01校验/加载并核对实际image ID及CPU，所有步骤加入operation journal、下载/加载超时、容量与错误分类；401表示本次认证失败，不能自动认定镜像不存在、删除服务或改为未登记镜像站。

复杂协议工具为原生Java17短命镜像，按协议独立模块/classpath，统一受控JSON输入/ProbeResult输出，秘密经受限stdin/短期只读文件传递。候选SDK：Rocket gRPC5.2.2、Canal1.1.8、Seata1.8.0、XXL3.5.0、Nacos3.3.0、Sentinel1.8.10、Rabbit AMQP5.37.1、MinIO9.0.4、PG JDBC42.7.14/MySQL JDBC26.10.0；JRE/SDK/传递依赖/双架构按锁验证，不把镜像1.8.0.2当Seata Maven版本。用户只需Docker，工具镜像包含所需运行时，执行结束清理自有短命任务，不增加常驻中间件。

Temporal1.32.2与既定PG18.6复用关系保持。官方文档的PG12+ Visibility条件及列举的测试版本不能证明PG18已认证；Schema工具、双库、账号、cluster health、Namespace和保留数据重建必须实测，不为绕过失败另起PG16或降级现有数据。Kafka新UI、Rocket Dashboard8082及MinIO/XXL/ES-IK新制品均需更新对应探针/默认端口映射后验证，架构清单不代表功能已通过。

## 8. 实施与验收

唯一承接：[PRD 02](../prd/02-跨平台环境预检与服务自动化编排PRD.md) → [Plan 02](../plan/02-跨平台Docker探测与编排引擎实施计划.md)。先完成Redis闭环，再完成已认证MySQL依赖组和PG→Temporal，再扩展全量探针；最终范围仍为17套服务。

必须覆盖每一步故障注入、已存在账号、部分DDL/种子、错误凭据、跨进程并发、远端/切换context、中文路径、数据库误指、外部库、共用场景、保留数据重建、恢复及重置竞态。源码/Mock通过不替代设计03的安装包发行验收。所有运行检查当前均未执行。

[返回设计导航](00-设计导航.md)
