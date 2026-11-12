# 02 跨平台 Docker 探测与自动化编排设计

状态：已对齐基线，待进入实施。依据：[需求 02](../requirement/02-服务依赖与新手使用场景清单.md)（`REQ-03`、`REQ-04`、`C01`、`C04`、`C05`）、[调研 03](../research/03-CodeArk现有17套服务痛点与兼容基线盘点.md)。

## 1. 摘要

定义工作箱 Rust 后端如何跨 macOS、Windows、Linux 稳定发现并调度 Docker，如何在启动前拦截端口/内存/外部容器冲突，以及如何通过拓扑锁与幂等初始化引擎解决 Temporal、Canal、XXL-Job、Seata、Nacos 与 Elastic Stack 的复杂启动前置。

## 2. 参与者

- **Rust 编排核心（`docker_cli` / `preflight` / `provisioner` / `orchestrator`）**：负责探测、加锁、按依赖启停、初始化与作用域隔离重置。
- **终端用户**：通过界面一键触发场景或单服务动作，遇到冲突或硬件不足时得到明确中文指引与选项，而非原始 Docker 错误堆栈。

## 3. 背景

- macOS GUI 应用默认不继承用户终端 `PATH`；Windows 子进程默认会弹出黑色 CMD 窗口；Linux 常遇 `/var/run/docker.sock` 权限不足；
- 仓库已有服务中，存在硬编码 `container_name`（如 `nacos-standalone`、`mysql-start-dev`、`pgsql-start-dev`），容易与用户此前手动起过的容器碰撞；
- `temporal`、`xxjob`、`seata` 的实际状态存储在共享 PostgreSQL / MySQL 内而非本地挂载目录。

## 4. 目标

- **可检查结果**：
  1. 三端无黑窗、无需手工配置 `PATH` 即可发现 Docker CLI 与读取 Daemon 内存/状态；
  2. 存在外部同名容器时准确分类为 `ExternalObserve`（只读观察）并支持显式“替换接管”，绝不静默删数据或直接报错崩溃；
  3. 一键启动任意依赖链服务（如 `temporal`、`canal`、`xxjob`、`seata`、`elastic`）均保持跨平台幂等，可重复启动且保留既有业务数据；
  4. 按存储归属精确执行 `stop` / `down` / `重置数据` 与级联影响提醒。
- **Non-Goals**：不跨文件系统自动迁移旧手工仓库里的 `mysql-data` / `pgsql-data` 二进制数据文件（避免数据库版本/权限不一致引发损坏）。

## 5. 使用场景

- 场景 1：新手点击启动 Elastic Stack，工作箱先检查 Docker 内存（当前如仅 4 GB 则阻断并指导去 Docker Desktop Settings 调至 >= 6 GB）；满足后自动在无并发冲突下构建 IK 镜像并续期 Basic License。
- 场景 2：开发者电脑里已有外部手动启动的 `nacos-standalone`（与本机现场观测情况一致），工作箱识别后标记“外部运行中”，提供“只读观察”或“停止并移除旧容器外壳后由工作箱接管（保留 `nacos-start-local-data` 卷）”。
- 场景 3：开发者需要重置 XXL-Job 到干净状态，工作箱仅清空 `./logs/xxl-job` 与 `${MYSQL_DATABASE}` 库后重导 `tables_xxl_job.sql`，不影响同一 MySQL 内的其他业务库。

## 6. 价值

- 把原本需要阅读十余份 README、手敲 SQL 与判断网络依赖的多步操作收敛为确定性状态机；
- 彻底摆脱 `elastic-start-local/start.sh` 等 Unix Shell 制约，让 Windows 原生开发者拥有同等一键体验。

## 7. 方案

### 7.1 跨平台 Docker CLI 探测与无窗执行（设计建议）

- **二进制路径探测**：除读取 `PATH` 及用户设置覆盖路径外，按 OS 追加探测：
  - macOS：`~/.docker/bin`、`/usr/local/bin`、`/opt/homebrew/bin`、`/Applications/Docker.app/Contents/Resources/bin`、`~/.orbstack/bin`、`~/.rd/bin`；
  - Windows：`%ProgramFiles%\Docker\Docker\resources\bin\docker.exe`，所有 `tokio::process::Command` 统一设置 `CREATE_NO_WINDOW` (`0x08000000`)；
  - Linux：`/usr/bin/docker`、`/usr/local/bin/docker`，若命中 `/var/run/docker.sock` permission denied，返回可操作指引（加入 `docker` 用户组并重新登录）。

### 7.2 预检与外部容器分类规则（设计建议）

- **宿主机资源与端口探测**：
  - 通过 `docker info --format '{{json .}}'` 读取 `MemTotal`；通过 `sysinfo` 读取工作区磁盘剩余空间；
  - 启动前用 `TcpListener::bind` 对目标宿主机端口做可用性探测（仅当目标容器尚未运行且非本服务自用端口时拦截）。
- **外部容器分类（`ContainerOwnership`）**：调用 `docker ps -a` 与 `docker inspect`：
  - 若 `com.docker.compose.project.working_dir` 归一化路径等于 `~/.code-ark/stacks/<id>` → **`Managed`**；
  - 若存在同 `container_name` 但工作目录不同或无标签 → **`ExternalObserve`**：
    - **仅观察语义**：只读展示状态、来源目录、端口与实时日志；禁用启停、配置编辑和重置；禁止作为下游依赖（`canal`/`temporal`/`seata`/`xxjob`）的自动初始化目标（下游如需连外部库，在表单切为“手动填写外部库”）；
    - **替换接管语义**：弹窗展示原容器工作目录及挂载路径后，由用户明确确认执行 `docker stop <name> && docker rm <name>`（**严禁删除原绑定数据目录与命名卷**），随后从 `~/.code-ark/stacks/<id>` 启动工作箱托管实例。

### 7.3 拓扑编排锁与幂等初始化引擎（设计建议）

- **串行互斥锁**：`OrchestratorState` 持有全局/依赖组异步操作锁，快速连点场景或单服务启动时拒绝并发重入，并按步骤发射 `StepProgressEvent`（`Preflight` → `Materialize & .env` → `Pre-provision` → `Compose Up & Wait Healthy` → `Post-provision`）。
- **依赖启动顺序**：基于 DAG 拓扑排序，优先启动 `mysql` / `pgsql` 并轮询容器 `Health.Status == healthy` 后再推进上层服务；若中间步骤失败立即中止并定位到具体失败步骤，不留下未决的半残流程。
- **各服务幂等初始化策略**：
  - `nacos`：执行 `docker volume inspect/create nacos-start-local-data`；
  - `temporal`：从 `pgsql/.env` 同步 `PGSQL_USER`/`PGSQL_PASSWORD`，连接验证 `pgsql-start-dev` 账号连通性（若历史 `pgsql-data` 密码不匹配则明确报错提示修改配置），再启动 Temporal Compose（内部容器自动完成幂等 Schema 与 Namespace 初始化）；
  - `canal`：确保 MySQL 健康后通过 `docker exec` 幂等执行 `CREATE USER IF NOT EXISTS 'canal'@'%' ...` 及 `GRANT SELECT, REPLICATION SLAVE, REPLICATION CLIENT`，写回 `canal/.env`；
  - `xxjob`：检查配置库 `${MYSQL_DATABASE}`（默认 `xxl_job`）是否存在 `xxl_job_info` 等核心表；若不存在，剥离 `tables_xxl_job.sql` 顶部硬编码 `CREATE DATABASE / USE xxl_job` 并针对 `${MYSQL_DATABASE}` 执行建库与导入；
  - `seata`：检查 `${MYSQL_DATABASE}`（默认 `seata`）是否存在四张元数据表，缺失时执行内置 `seata_1_8_0_mysql.sql` 建库建表；
  - `elastic`：校验 Docker 内存 >= `ELASTIC_STACK_MIN_DOCKER_MEMORY_GB`（6 GB）与磁盘空间；当 `elastic-start-local-elasticsearch:${ES_LOCAL_VERSION}-${IK_VERSION}` 缺失或要求重建时先停栈并以 `COMPOSE_PARALLEL_LIMIT=1` 构建镜像；全部容器就绪后如需要，以 Rust `reqwest` 调用 `/_license/start_basic?acknowledge=true` 完成 Basic License 授权。

### 7.4 分级停止与作用域精准重置（设计建议）

- **停止（`stop`）**：若目标为 `mysql`/`pgsql` 且检测到有上层服务（`canal`/`seata`/`xxjob`/`temporal`）正在运行，先弹出影响列表，提供“一并安全停止”或取消；
- **重置数据（`reset_service_data`）**（需二次输入服务名确认，严禁全局 `docker system prune`）：
  - 独立存储服务（12 个）：仅清空 `~/.code-ark/stacks/<id>` 下本地数据/日志目录及该项目命名卷（`docker compose -p <project> down -v`）；
  - 共享数据库上层服务（`temporal`、`xxjob`、`seata`）：除清理自身日志外，仅在共享 PG/MySQL 中执行 `DROP DATABASE IF EXISTS <专属库>` 并标记待重新初始化（`temporal` 清理 `temporal` + `temporal_visibility`；`xxjob` 与 `seata` 清理各自配置的 `${MYSQL_DATABASE}`），绝不碰 `test_db` 等其他库；
  - 基础数据库自身（`mysql`、`pgsql`）：显示受影响上层服务**级联警告**，确认后先停止关联上层服务，再清理数据库目录并重设上层初始化状态。

## 8. 实施与验收

- **下游来源映射**：由 [PRD 02](../prd/02-跨平台环境预检与服务自动化编排PRD.md) 承接验收边界，由 [Plan 02](../plan/02-跨平台Docker探测与编排引擎实施计划.md) 定义 Rust 测试与集成验证命令。
- **核心验证出口**：Docker 错误分类测试、TCP 端口占用与 6 GB 内存门禁测试、`ExternalObserve` 边界测试、`tables_xxl_job.sql` 自定义库名替换测试、并发启停锁与级联停止警示测试。

[返回设计导航](00-设计导航.md)
