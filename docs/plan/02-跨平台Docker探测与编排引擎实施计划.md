# 02 跨平台 Docker 探测与编排引擎实施计划

唯一来源：[02 跨平台环境预检与服务自动化编排 PRD](../prd/02-跨平台环境预检与服务自动化编排PRD.md)。本 Plan 只属于该 PRD；Docker 搜索路径、`ContainerOwnership`、6 服务幂等初始化流程与 3 类重置作用域沿来源 PRD 指向的 [设计 02](../design/02-跨平台Docker探测与自动化编排设计.md) 读取。当前状态：**未执行**。

## 1. 目标、架构与约束

- **目标**：实现工作箱核心后端编排链路：跨平台无弹窗 Docker CLI 引擎（`docker_cli.rs`）、宿主机资源/端口/外部容器碰撞预检（`preflight.rs`）、6 套复杂服务原生幂等初始化与作用域重置（`provisioner.rs` + 内置 `seata_1_8_0_mysql.sql`），以及串行并发锁编排调度与接管逻辑（`orchestrator.rs`）。
- **全局约束**：
  - Windows 上每一次 `tokio::process::Command` 调用统一附加 `CREATE_NO_WINDOW`（`0x08000000`）；
  - 编排期不在宿主机执行任何根目录下旧版 Unix `start.sh`，全跨平台逻辑用原生 Rust + `reqwest` + `docker exec` 实现；
  - 严格锁定 `-p <canonical_project>`，严禁对任何场景调用 `docker system prune`；
  - **Review Focus（重点守护用例）**：
    - **本机已存在外部碰撞容器**：准确识别非 `~/.code-ark/stacks/<id>` 运行的 `nacos-standalone` / `mysql-start-dev` 为 `ExternalObserve`，禁止充当下游自动初始化宿主，替换时绝不删除外部原数据目录或命名卷；
    - **`xxjob` 自定义库名初始化**：传入仓库真实 `tables_xxl_job.sql` 到非默认库名时必须剥离硬编码 `CREATE DATABASE / USE xxl_job;` 后向目标库导入；
    - **并发连点与中途失败止损**：两次并发触发场景/单服务时受全局操作锁保护不交错执行；中间步骤失败停在精确报错步；
    - **停止与重置隔离边界**：停基础库且存在运行中上层服务时先拒停并回报列表；重置 `temporal`/`xxjob`/`seata` 不触及 `test_db` 等用户他库。

## 2. 文件清单与接口契约

| 文件路径 | 职责与关键接口 / 约束 |
| --- | --- |
| `toolbox/src-tauri/src/docker_cli.rs` | `DockerRunner`、`discover_docker_binary(Option<&Path>)`、`candidate_search_paths()`、`info()`、`list_containers()`、`inspect_container(&str)`、`ensure_volume(&str)`、`compose_run(...)`、`exec_stdin(...)`、`stream_compose_lines(...)` |
| `toolbox/src-tauri/src/preflight.rs` | `ContainerOwnership { Managed, ExternalObserve { container_name, working_dir, mounts }, NotPresent }`、`classify_container_ownership(...)`、`check_tcp_port_available(&str, u16) -> bool`、`run_service_preflight(...)`、`run_host_preflight(...)` |
| `toolbox/src-tauri/resources/sql/seata_1_8_0_mysql.sql` | 内置 Seata 1.8.0 四张元数据表幂等 DDL（`global_table`、`branch_table`、`lock_table`、`distributed_lock`，使用 `CREATE TABLE IF NOT EXISTS`） |
| `toolbox/src-tauri/src/provisioner.rs` | `rewrite_xxjob_sql_for_database(&str, &str) -> String`、`pre_start_provision(...)`、`post_start_provision(..., force_rebuild: bool)`、`reset_service_data(...) -> Result<ResetSummary, ProvisionError>` |
| `toolbox/src-tauri/src/orchestrator.rs` | `OrchestratorState`（内含 `tokio::sync::Mutex<()>` 并发互斥锁）、`StepProgressEvent`、`start_services(...)`、`stop_services(..., cascade_dependents: bool)`、`replace_external_container(...)`、`inspect_mediacrawler_host_status() -> MediaCrawlerHostStatus` |
| `toolbox/src-tauri/src/lib.rs` | 导出 `docker_cli`、`preflight`、`provisioner`、`orchestrator` |

## 3. 实施任务与测试验证

### 任务 1：跨平台 Docker CLI 封装与错误分类（对应 `PRD02-R01`、`TS-05`）

- [ ] **Step 1（失败单测先行）**：在 `docker_cli.rs` 编写 4 个单元测试：
  - `test_candidate_search_paths_includes_platform_defaults`（覆盖 macOS Homebrew/Docker Desktop/OrbStack/RD、Windows ProgramFiles、Linux `/usr/bin`）；
  - `test_parse_docker_info_json`（解析含 `MemTotal` 的 JSON 输出及字段缺省报错）；
  - `test_classify_docker_error_permission_denied`（将 `/var/run/docker.sock` permission denied 映射为含 `usermod -aG docker` 指引的 `DockerError::SocketPermissionDenied`）；
  - `test_build_compose_command_pins_project_and_files`（验证 `-p <project> -f docker-compose.yml -f compose.toolbox.override.yml ...` 顺序）。
- [ ] **Step 2（运行确认失败）**：执行 `cargo test --manifest-path toolbox/src-tauri/Cargo.toml docker_cli::tests` 确认红灯失败。
- [ ] **Step 3（实现 `docker_cli.rs`）**：实现全平台扫描、统一的 Windows `creation_flags(0x08000000)` 进程构造器、line-delimited JSON `docker ps -a` / `inspect` 解析、以及异步行流式日志/构建输出器。
- [ ] **Step 4（运行通过校验）**：执行 `cargo test --manifest-path toolbox/src-tauri/Cargo.toml docker_cli::tests` 全部通过。

### 任务 2：预检器、端口探测与外部容器分类（对应 `PRD02-R02`、`PRD02-R03`、`TS-05`、`TS-06`）

- [ ] **Step 1（失败单测先行）**：在 `preflight.rs` 编写 Review Focus 测试：
  - `test_classify_managed_vs_external_container`（匹配 `~/.code-ark/stacks/nacos` 返回 `Managed`；不同工作目录或无标签返回 `ExternalObserve` 并带源目录及挂载路径）；
  - `test_classify_case_insensitive_windows_path_match`（Windows 盘符与斜杠归一化比对不误判）；
  - `test_tcp_port_probe_detects_bound_loopback_listener`（绑定随机回环端口测试 occupied/free 状态反转）；
  - `test_elastic_memory_gate_flags_under_6gb`（构造 4 GiB `DockerInfo` 验证 Elastic 启动门禁报错）。
- [ ] **Step 2（运行确认失败）**：执行 `cargo test --manifest-path toolbox/src-tauri/Cargo.toml preflight::tests` 验证失败。
- [ ] **Step 3（实现 `preflight.rs`）**：实现路径归一化所有权判定、`TcpListener::bind` 端口探测、`sysinfo::Disks` 空间查询与服务/宿主机聚合预检器。
- [ ] **Step 4（运行通过校验）**：执行 `cargo test --manifest-path toolbox/src-tauri/Cargo.toml preflight::tests` 全部通过。

### 任务 3：6 服务幂等初始化与作用域精准重置（对应 `PRD02-R05`、`PRD02-R06`、`TS-07`、`TS-08`）

- [ ] **Step 1（失败单测先行）**：在 `provisioner.rs` 编写 3 个单元测试：
  - `test_rewrite_xxjob_sql_supports_custom_database_name`（输入仓库真实 `xxjob-start-local/tables_xxl_job.sql`，指定 `custom_job_db` 后校验库名替换且不破坏表结构与初始 INSERT）；
  - `test_seata_embedded_ddl_contains_all_four_required_tables`（校验内置 SQL 含有 `global_table`、`branch_table`、`lock_table`、`distributed_lock` 的 `CREATE TABLE IF NOT EXISTS`）；
  - `test_reset_plan_isolates_shared_db_services`（确认重置 `temporal` 只动 `temporal` + `temporal_visibility`、重置 `xxjob` 只动其配置库 + `./logs/xxl-job`、重置 `mysql` 生成包含 `canal`/`seata`/`xxjob` 的级联计划）。
- [ ] **Step 2（运行确认失败）**：执行 `cargo test --manifest-path toolbox/src-tauri/Cargo.toml provisioner::tests` 验证失败。
- [ ] **Step 3（实现 `provisioner.rs` 与内置 Seata DDL）**：实现 `nacos` 外部卷、`temporal` PG 账密同步与认证先验、`canal` MySQL 复制授权、`xxjob` / `seata` 查表后幂等建库导入、`elastic` 内存/磁盘门禁 + `COMPOSE_PARALLEL_LIMIT=1` 构建 + `reqwest` License 请求、以及分作用域重置逻辑。
- [ ] **Step 4（运行通过校验）**：执行 `cargo test --manifest-path toolbox/src-tauri/Cargo.toml provisioner::tests` 全部通过。

### 任务 4：并发编排锁、接管流与 MediaCrawler 宿主机探测（对应 `PRD02-R03`、`PRD02-R04`、`PRD02-R06`、`TS-06`、`TS-07`、`TS-09`）

- [ ] **Step 1（失败单测先行）**：在 `orchestrator.rs` 编写 3 个 Review Focus 测试：
  - `test_concurrent_start_calls_serialize_and_reject_duplicate_active_run`（双并发启动命中锁保护不交叉执行）；
  - `test_start_aborts_cleanly_when_dependency_preflight_fails`（外部 `mysql-start-dev` 碰撞时，`canal` 在预检步中断于 `ExternalConflictRequiresAction` 且不残余半截密钥）；
  - `test_stop_mysql_warns_when_dependents_running`（当 `canal`/`xxjob` 在运行且 `cascade_dependents == false` 时拒绝停库并返回受影响服务列表）。
- [ ] **Step 2（运行确认失败）**：执行 `cargo test --manifest-path toolbox/src-tauri/Cargo.toml orchestrator::tests` 验证失败。
- [ ] **Step 3（实现 `orchestrator.rs`）**：实现发射 `StepProgressEvent` 的拓扑启停管线、`replace_external_container`（仅 `docker stop && docker rm`）、级联停止控制与 `inspect_mediacrawler_host_status`（检测 `git`、`uv` 与工作目录）。
- [ ] **Step 4（运行通过校验）**：执行 `cargo test --manifest-path toolbox/src-tauri/Cargo.toml orchestrator::tests` 全部通过。

## 4. 停止与接续边界

- **本 Plan 完成判据**：4 个 Rust 核心模块编译通过且全部单元测试全绿；可接入 [Plan 03](03-工作箱前端工作台与跨平台CI实施计划.md) 的 IPC 命令、真实 Docker 冒烟及三端构建验证。
- **未执行记录**：全部 4 个任务当前均 **未执行**。

[返回 Plan 导航](00-Plan导航.md)
