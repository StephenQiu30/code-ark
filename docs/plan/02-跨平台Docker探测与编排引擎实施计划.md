# 02 跨平台Docker探测与编排引擎实施计划

> 2026-11-12 开发授权已取得，当前开始实施；下述“本轮只修订文档/不创建”是制定计划时的历史限制，已被本次开发指令覆盖。未通过的运行/发行验证仍不能标为完成。
> 原计划说明：获得明确开发指令后使用 `superpowers:executing-plans` 逐任务执行。**本轮只修订文档，以下源码、测试及容器操作全部不得执行。**

唯一来源：[PRD 02](../prd/02-跨平台环境预检与服务自动化编排PRD.md)，技术协议沿该PRD唯一design读取。状态：**全部任务未执行**（2026-11-12）。

**目标**：让固定本地目标上的场景/单服务启动、初始化、探针、配置应用、恢复和重置具有完整权限与生命周期边界。

**架构**：复用Plan01的catalog、有效配置和共享schema；DockerRunner与数据库连接层提供受控能力，orchestrator持久化步骤，前端不持有任意执行权限。现有数据库优先绑定，不自动创建重复基础依赖。

**技术栈**：Rust/Tokio、类型化Docker CLI、原生协议/HTTP/SQL客户端及锁定的一次性探针工具；全部依赖锁定，Windows无窗，Docker Linux containers、Compose>=2.24.4。

采用来源design §7.9候选：Rust1.99.1、Tokio1.54.1、SQLx0.9.0、Reqwest0.13.5、sysinfo0.40.3等；HTTP/SQL使用一致rustls AWS-LC路线，编译不连接业务库，显式验证本地MySQL RSA认证/CA/依赖树。锁采用标准File API，不再默认加fs2/fs4；复杂协议按SDK分模块的Java17工具，不用一个胖JAR混合Netty/gRPC。注册元数据并不代表这些依赖已联合编译通过。

## 1. 全局约束

- services下源码YAML/配置/.env.example及既有数据不改；所有动作只针对已核验target与登记实例；远端及Windows containers不执行。
- `ExternalObserve`无写权限；`ExternalConnected`只获准确列名初始化授权，无外部生命周期/整库删除权限。
- 外部绑定v1仅支持同一本地daemon中的MySQL/PG容器；未发布宿主端口用锁定一次性SQL客户端认证。本机原生/远端库不自动编排，提示拓扑限制；不悄悄新建替代库。
- Temporal复用已选PG和现有账号，固定双库、同版本Server/Schema工具及既有History分片，不另启私有PG。
- 不用只看核心表判定初始化，不假设DDL整体可回滚；秘密来源一致，partial/schema不兼容状态明确。
- v1不提供已初始化数据库账号密码轮换，既有连接值先认证后应用；新账号首次创建仍自动完成，不预留无后端实现的轮换IPC。
- Ready需要job退出0和全量必要协议/API探针；running、首页200、Mock不替代。
- 单实例应用、OS工作区锁、跨进程daemon/claim锁与步骤前核验并存；取消后必须对账实际资源。
- 删除使用不可变ResetPlan及后端二次核验；不按名称前缀或用户配置库名直接DROP，不执行全局prune。
- 真实测试必须同时隔离target/实例/端口/卷/库，显式选定测试配置；仅临时目录不构成隔离。

## 2. Review Focus

1. endpoint相同但daemon已换、环境变量覆盖或context远端：任务1/6复核身份并阻断。
2. 已有Canal账号密码不符、XXL仅建一半/种子已存在、Temporal另一所有者：任务3/4不认领/覆盖，安全续跑兼容步骤。
3. 外部/系统/共用库及symlink、过期/资源变动计划：任务7后端拒删，名称确认不能绕过。
4. 无healthcheck组件、job失败、上游不通和错误密码：任务5不假报Ready。
5. 崩溃、多进程、取消后daemon仍在工作、配置已应用但探针失败：任务6用journal+inspect恢复真实generation。

## 3. 文件与接口

以下均为将来创建/修改的工程文件，本轮不创建。

| 文件 | 接口与输出 |
| --- | --- |
| `toolbox/src-tauri/src/contracts.rs` | 采用来源design的DockerTarget/DatabaseBinding/DatabaseClaim/ProbeSpec/OperationRecord/ResetPlan/ServicePhase；复用Plan01 schema，不复制前后端类型 |
| `toolbox/src-tauri/src/docker_cli.rs` | `discover_target(Option<&Path>, Option<&str>) -> Result<DockerTarget, ToolboxError>`；`DockerRunner::connect(DockerTarget) -> Result<Self, ToolboxError>`；`verify_identity() -> Result<(), ToolboxError>`；`compose_run(&Path, &InstanceIdentity, &OverrideArtifact, &[OsString]) -> Result<CommandOutcome, ToolboxError>`，网络/执行方法为async |
| `toolbox/src-tauri/src/preflight.rs`、`storage_probe.rs` | `run_preflight(&DockerRunner, &EffectiveConfig) -> Result<PreflightReport, ToolboxError>`；`probe_storage(&DockerRunner, &Path) -> Result<StorageCapacity, ToolboxError>`；容量unknown显式枚举 |
| `toolbox/src-tauri/src/image_store.rs` | `ensure_service_image(&DockerRunner, &ImageArtifactRef, &OperationContext) -> Result<VerifiedImage, ToolboxError>`；registry digest或受校验image-save归档下载/load，ID/架构复核，不执行任意外部构建命令 |
| `toolbox/src-tauri/src/bindings.rs` | `verify_binding(&DockerRunner, &DatabaseBinding) -> Result<VerifiedBinding, ToolboxError>`；`authorize_binding(&VerifiedBinding, &InitializationGrant) -> Result<DatabaseBinding, ToolboxError>`；无默认写授权 |
| `toolbox/src-tauri/src/database_claims.rs` | `plan_database_claim(&VerifiedBinding, &InstanceIdentity, &[String]) -> Result<ClaimPlan, ToolboxError>`；`verify_database_claim(&VerifiedBinding, &DatabaseClaim) -> Result<VerifiedClaim, ToolboxError>` |
| `toolbox/src-tauri/src/provisioner.rs`、`initialization_store.rs` | `plan_initialization(&ServiceSpec, &VerifiedBinding, &EffectiveConfig) -> Result<InitializationPlan, ToolboxError>`；`execute_initialization(&InitializationPlan, &OperationContext) -> Result<InitializationOutcome, ToolboxError>`；逐DDL/种子记录 |
| `toolbox/src-tauri/resources/sql/seata_1_8_0_mysql.sql`、`sql/manifest.json` | 对应四表DDL、上游许可/校验和；XXL源SQL转换为版本步骤，不手工复制另一套业务结构 |
| `toolbox/src-tauri/src/probes.rs`、`probe_clients.rs` | `run_probe(&ProbeSpec, &ProbeContext) -> Result<ProbeResult, ToolboxError>`；`evaluate_service_readiness(&ServiceSpec, &[ProbeResult], &[JobExit]) -> ServicePhase` |
| `toolbox/probe-clients/` | `Dockerfile`、聚合`pom.xml`、`modules/<protocol>/pom.xml`及各自`src/main/java/io/codeark/ProbeMain.java`；按协议独立classpath、锁定SDK/JRE17双架构，不要求宿主JDK，不是新常驻中间件 |
| `toolbox/src-tauri/src/operation_store.rs`、`locks.rs` | `OperationStore::load/persist`、`acquire_resource_locks(&[ResourceLockKey]) -> Result<OperationGuard, ToolboxError>`；稳定排序、受限权限与原子写 |
| `toolbox/src-tauri/src/usage_index.rs` | `register_usage(&UsageRegistration)`、`unregister_verified_usage(&UsageRegistration)`、`query_dependents(&ResourceRef) -> Result<UsageReport, ToolboxError>`、`repair_usage_index(&RepairUsageRequest) -> Result<UsageReport, ToolboxError>`；公共control_root下跨工作区原子索引，变更返回`Result<(), ToolboxError>`；repair只重建可核验引用，不能清除Unknown |
| `toolbox/src-tauri/src/orchestrator.rs`、`config_apply.rs` | `start_services(&StartRequest) -> Result<OperationId, ToolboxError>`、`stop_services(&StopRequest)`、`retry_operation(OperationId)`、`cancel_operation(OperationId)`、`recover_operations()`；`validate_draft(&ConfigDraft) -> Result<ValidatedDraft, ToolboxError>`、`apply_draft(&ValidatedDraft)` |
| `toolbox/src-tauri/src/reset.rs` | `plan_reset(&ResetRequest) -> Result<ResetPlan, ToolboxError>`；`execute_reset(&ResetConfirmation) -> Result<ResetOutcome, ToolboxError>`；async、同一所有权/锁路径 |
| `toolbox/src-tauri/tests/acceptance.rs`、`tests/fixtures/runtime/` | 独立真实测试与故障fixture；测试配置缺失/目标不符直接拒绝，不回退当前默认daemon |

`CommandOutcome { exit_code, redacted_stdout, redacted_stderr }`；`VerifiedBinding/VerifiedClaim`为后端验证后才可构造的能力类型，不接受前端直接反序列化。`InitializationGrant { target_id, server_fingerprint, allowed_databases, allowed_accounts, operations, confirmed_at }`；`OperationContext { operation_id, target, guard, journal }`。`ProbeContext`含已验证实例/端点、临时secret访问与超时，不序列化秘密。请求结构均以workspace/target/instance/generation ID引用受控资源，不接受任意shell或文件路径。

`ResetConfirmation { plan_id, fingerprint, typed_service_id }`；`ResetPlan.expires_at`为生成后5分钟。`ConfigDraft`引用base_generation，过时草稿需要重新比较，不直接覆盖新配置。`UsageRegistration { target_id, provider_instance_id, claim_id, consumer_workspace_id, consumer_instance_id, preset_refs, operation_id, status }`；`UsageReport { consumers, unknown_entries, inspected_at }`；只有安全停止/解绑核验后注销，心跳过期不得自动移除。

`VerifiedImage { source_id, target_id, platform, image_id, runtime_reference }`为后端保证器产生；load后的内容tag必须pull_policy=never，部署前后再核对实际Image。归档producer由Plan01任务6形成，不依赖桌面安装包；本任务只消费已解析来源和实际artifact，不能把尚未生成的下载地址当可用。

## 4. 实施任务

### 任务1：固定目标的DockerRunner与能力检查

**文件**：`contracts.rs`、`docker_cli.rs`、runtime fixtures。覆盖R01，TS-05/16；消费Plan01身份和目录类型。

- [ ] 1. 写 `docker_cli::tests`：候选路径与Windows creation_flags；Compose2.20/2.24.3拒绝、2.24.4及兼容新主版本能力通过；远端/Windows containers拒绝；宿主覆盖变量清除；target变化返回TargetChanged，所有命令固定参数且无密码日志。
- [ ] 2. 运行 `cargo test --locked --manifest-path toolbox/src-tauri/Cargo.toml docker_cli::tests` 确认未实现行为失败。
- [ ] 3. 实现绝对路径发现、context/daemon绑定、类型化参数构造、无窗进程和错误分类；不修改全局context，保留必要的已选本地连接配置。Docker认证首期为Desktop/Engine，OrbStack/Rancher被发现仍须单独实测；不默认引入Bollard重写Compose。
- [ ] 4. 重跑模块测试，在每目标原生环境验证GUI打开时发现和无窗；未知target不得走默认daemon。
- [ ] 5. 提交 `feat(toolbox): pin local docker targets and capabilities`。

### 任务2：资源预检与存储探针

**文件**：`preflight.rs`、`storage_probe.rs`、catalog容量字段/锁定诊断工具配置。覆盖R02，TS-05。

- [ ] 1. 写模块测试：TCP/UDP/端口组/4–6GiB/双盘/unknown/路径/权限；加入归档坏SHA、截断下载、错误image ID/CPU、local内容tag被替换、下载401和取消；不能用源码AMD64镜像模拟通过ARM，未知来源URL拒绝。
- [ ] 2. 运行 `cargo test --locked --manifest-path toolbox/src-tauri/Cargo.toml preflight::tests` 及完整同前缀的 `storage_probe::tests`、`image_store::tests`，确认容量/归档错误分支红灯。
- [ ] 3. 实现容量/架构预检及image_store：registry不可变digest或按允许HTTPS来源下载image-save归档、核SHA并在固定target load、inspect ID/CPU；缓存按内容复用、受操作期限约束、不进入reset清单。下载401给准确错误，不自动删服务/换镜像站。容量未知展示复检/本次接受入口，诊断任务需用户同意且锁定镜像。
- [ ] 4. 重跑preflight/storage_probe/image_store测试，实测每运行时容量及归档下载/load/实际Image；填真实空间/传输体积，MinIO/XXL/ES-IK producer尚无可获取归档则保持pending并阻断对应candidate/release，而非从维护者缓存伪造冷启动通过。
- [ ] 5. 提交 `feat(toolbox): preflight ports capacity and architecture`。

### 任务3：外部绑定、权限和数据库claim

**文件**：`bindings.rs`、`database_claims.rs`、`contracts.rs`及SQL/PG认证适配。覆盖R03/R09，TS-06/19。

- [ ] 1. 写`ownership_requires_registry_and_labels`、`external_grant_is_scoped`、`unpublished_container_authenticates_on_network`、`unsupported_topology_is_explicit`、`claim_requires_server_identity`测试：同名/同路径不授予Managed，准确授权仍无生命周期权限；PG优先复用，无发布端口可网络认证；原生/远端库拒绝但不新建替代；专属账号/库、双claim和Temporal唯一所有者正确。
- [ ] 2. 运行 `cargo test --locked --manifest-path toolbox/src-tauri/Cargo.toml bindings::tests` 及完整同前缀的 `database_claims::tests`，确认红灯。
- [ ] 3. 实现认证、可选宿主端点/一次性网络SQL客户端和服务器身份核验、权限scope、归属表和registry；只对已授权资源创建/核验，不读取未选择外部.env，不悄悄改账号；无归属标记的崩溃窗口返回RecoveryRequired。
- [ ] 4. 重跑测试，在隔离库验证ReadOnlyConnect、精确初始化授权及外部整库禁删；记录未授权服务器无变更。
- [ ] 5. 提交 `feat(toolbox): bind existing databases with scoped ownership`。

### 任务4：可恢复初始化与凭据联动

**文件**：`provisioner.rs`、`initialization_store.rs`、SQL资源及manifest，目录初始化元数据。覆盖R05/R09，TS-07/19。

- [ ] 1. 写失败测试：Canal旧账号认证不符必须失败；XXL自定义库、每个表/种子中断、已有用户/任务保持；Seata四表/列版本；Temporal认证/claim/双job/版本与Namespace；Nacos独立卷；Elastic只配置kibana_system、Basic实际状态和HTTP错误。
- [ ] 2. 运行 `cargo test --locked --manifest-path toolbox/src-tauri/Cargo.toml provisioner::tests` 及完整同前缀的 `initialization_store::tests`，确认失败原因。
- [ ] 3. 实现初始化计划、逐步journal及实际完成核验，SQLx0.9仅启用需要的MySQL/PG/Tokio/TLS/mysql-rsa；不用编译期业务库宏。业务账号同源，未知或不兼容库进入门禁；不实现通用SQL重放/整库重建修复。
- [ ] 4. 重跑单测及隔离每步DDL/种子故障；强制验证Temporal1.32.2/同版本Schema对PG18.6的双库、权限、集群和Namespace，不能把官方PG12+ Visibility条件当已测试18的证明；不改History/retention或另起PG16绕过。
- [ ] 5. 提交 `feat(toolbox): resume scoped service initialization safely`。

### 任务5：17服务就绪和协议客户端

**文件**：`probes.rs`、`probe_clients.rs`、catalog probes、必要一次性客户端工程及镜像锁。覆盖R07，TS-17。

- [ ] 1. 为来源design §7.5的17行各写正反例，断言job非0、错误凭据、后台断连、缺probe和上游不可达不为Ready；Sentinel等缺业务客户端显示等待接入；ProbeResult记录时间和错误码。
- [ ] 2. 运行 `cargo test --locked --manifest-path toolbox/src-tauri/Cargo.toml probes::tests`，确认每类错误现状失败。
- [ ] 3. 实现SQL/HTTP/Redis等原生探针，Rabbit AMQP/S3/Rocket/Canal/Seata等专用客户端用分协议独立classpath的一次性JRE17任务，版本取来源design §7.9；JSON输入/ProbeResult输出有上限，秘密不进命令。验证Kafbat1.5.0、Dashboard2.1.0/8082的实际后台API；不假定宿主有JDK或镜像有curl。
- [ ] 4. 重跑测试与隔离17行真实探针；就绪阶段只做授权读取/连接，写读/测试Topic等在明确测试scope的功能验收执行并清理自建资源；缺任意一行不得完成此任务的运行验证。
- [ ] 5. 提交 `feat(toolbox): verify readiness for every packaged service`。

### 任务6：持久编排、配置应用、取消与恢复

**文件**：`locks.rs`、`operation_store.rs`、`usage_index.rs`、`orchestrator.rs`、`config_apply.rs`、contracts。覆盖R04/R08，TS-09/16/18；消费前五任务能力和Plan01 OverrideArtifact。

- [ ] 1. 写`fresh_database_precedes_dependent_initialization`、`cross_workspace_usage_is_visible`、`stale_usage_fails_closed`、`recover_actual_generation`、`cancel_reconciles_daemon`测试：全新基础库拉取/启动/认证后才做上层DDL；跨工作区登记在上层启动前发生；陈旧/丢失索引不能当无消费者；多进程锁序、事件续订、deadline、过时草稿和迁移后禁降级均覆盖。
- [ ] 2. 运行 `cargo test --locked --manifest-path toolbox/src-tauri/Cargo.toml orchestrator::tests`、`config_apply::tests`、`locks::tests`、`operation_store::tests`各模块过滤测试（每次使用同一完整cargo前缀），确认红灯。
- [ ] 3. 实现逐节点DAG，基础库启动/认证后才做上层，镜像准备统一调用image_store并记录实际Image；公共UsageIndex和标准File专用锁不随workspace切换，try_lock/期限异步等待，不复制持锁句柄或阻塞Tokio。实现journal/实际generation/重试取消/必要重建后提交；既有账号先认证新连接值，禁轮换；保留design的各阶段期限和无进展提示。
- [ ] 4. 重跑测试，隔离故障注入分别杀进程/断网络/换daemon/中断DDL；只有无数据格式变更才允许经验证恢复旧配置；不确定状态显示RecoveryRequired。
- [ ] 5. 提交 `feat(toolbox): persist operations and recover configuration changes`。

### 任务7：共享场景停止及清单化重置

**文件**：`reset.rs`、orchestrator停止分支、contracts。覆盖R06/R09，TS-08/09/19。

- [ ] 1. 写测试：跨两个workspace的两场景/先前运行/外部依赖保留，公共索引损坏或陈旧必须拒绝并修复；反向停止；过期5分钟、错误指纹、目标或卷ID改变、symlink、伪造claim、外部/系统/共用库全部拒删；基础库含未知业务库不能整实例重置。
- [ ] 2. 运行 `cargo test --locked --manifest-path toolbox/src-tauri/Cargo.toml reset::tests` 及orchestrator停止测试，确认失败。
- [ ] 3. 实现ResetPlan生成、准确影响清单和执行复核；执行精确ID操作，不以`down -v`/库名/前缀替代所有权，外部连接无生命周期权限；普通down保留数据。
- [ ] 4. 重跑测试，在隔离含test_db及其他工作区的数据集执行允许/拒绝两组，按行/对象哈希比较未授权数据；拒绝不能被前端直接调用绕过。
- [ ] 5. 提交 `feat(toolbox): scope lifecycle and reset plans to owned resources`。

### 任务8：完整真实服务与故障矩阵

**文件**：`tests/acceptance.rs`、`tests/fixtures/runtime/`、后续执行证据目录。覆盖TS-04/07/08/09/15/16/17/18/19。

**接口**：`AcceptanceConfig { target_id, expected_daemon_id, workspace_root, run_id, port_range, allow_create, allow_destructive_owned_only }` 从显式 `CODE_ARK_ACCEPTANCE_CONFIG` 文件读取。未设置、daemon不符、端口/实例不隔离时测试非0拒绝；不隐式使用默认工作区。测试销毁只允许本run_id创建且归属复核通过的资源。

- [ ] 1. 先写隔离守护测试：缺配置、target不符、旧CLI卷/网络冲突或reset清单混外部ID均应在任何变更前拒绝。
- [ ] 2. 运行 `cargo test --locked --manifest-path toolbox/src-tauri/Cargo.toml --test acceptance isolation_guard`，只验证拒绝路径，不创建容器。
- [ ] 3. 实现分组运行入口：Redis→MySQL/Canal/Seata/XXL→PG/Temporal→剩余17服务、Rocket三方路由、共享场景及故障/恢复；写入专用测试记录，普通停止/保留数据重建后二次验证；准确记录每个job退出码与客户端结果。
- [ ] 4. 在明确准备好的独立目标设置验收配置，再运行 `cargo test --locked --manifest-path toolbox/src-tauri/Cargo.toml --test acceptance -- --ignored`。分组执行以控制资源，记录每OS/CPU/镜像/探针实际结果；清理后对比外部资源及原业务数据完全未变。原生环境不足则对应行未执行并阻断支持声明。
- [ ] 5. 提交测试及脱敏证据 `test(toolbox): verify isolated service lifecycle and recovery`，不得提交真实.env、凭据、业务数据或原始展开配置。

## 5. 完成判据与接续

模块完成需单元/配置检查全绿；运行完成需任务8的17行真实探针、每步中断、资源隔离和保留数据重建通过，不能把“有测试文件”记为完成。Plan01的TS-04/15在这里产生源码环境证据，最终安装包仍需Plan03单独验收。

**当前：8个任务全部未执行。** 开发工具链、Docker状态和原生执行器在将来执行前重新检查，不依赖旧调研声称本机当前可编译/运行。

[返回Plan导航](00-Plan导航.md)
