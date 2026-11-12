# 02 跨平台环境预检与服务自动化编排 PRD

来源 design：[02 跨平台 Docker 探测与自动化编排设计](../design/02-跨平台Docker探测与自动化编排设计.md)。本 PRD 只对应这一份 design，面向负责人、后端实现与验收人员说明跨平台环境检测、外部容器边界、自动化编排与作用域隔离重置的可观察行为与验收标准；具体命令参数与结构体字段回来源 design。执行计划见 §8。

## 1. 摘要

消除 macOS GUI 丢失 Terminal `PATH`、Windows 弹黑窗、Linux Docker Socket 权限拒绝，以及 Elastic/Temporal/Canal/XXL-Job/Seata/Nacos 多步手动前置导致的启动失败；在存在外部手动容器（如 `nacos-standalone`）和共享数据库时提供明确、无损的冲突控制与精确重置边界。

## 2. 责任与确认

| 角色 | 责任 |
| --- | --- |
| 项目负责人 | 确认依赖编排自动化范围、外部容器只读观察与显式替换边界、拒绝静默删数据原则 |
| 开发与执行人员 | 严格按 [Plan 02](../plan/02-跨平台Docker探测与编排引擎实施计划.md) 实现预检、幂等初始化、并发锁与作用域重置，编写对应失败先行的单元与联调测试 |
| 验收与日常使用者 | 按 §7.4 验证 Docker 探测、端口/内存拦截、`ExternalObserve`、复合服务链重入与作用域重置 |

核心规则来源于 [需求 02](../requirement/02-服务依赖与新手使用场景清单.md)（`REQ-03`、`REQ-04`、`C01`、`C04`、`C05`）及 [设计 02](../design/02-跨平台Docker探测与自动化编排设计.md)。

## 3. 背景

- `elastic-start-local`、`monitoring-start-local` 等旧脚本强依赖 Unix Bash 与 `jq`/`curl`，Windows 原生无法运行；
- `temporal`、`canal`、`xxjob`、`seata`、`nacos` 存在顺序依赖、卷预创建或手动导入 SQL，其中 `seata` 根目录下原本没有 DDL 文件；
- 当前开发环境常并存用户此前手工启动的同名容器（实测现场已存在外部运行中的 `nacos-standalone`），如果直接 `docker compose up` 会报晦涩冲突或误删用户手工容器与数据。

## 4. 目标与成功标准

1. **三端可靠探测与明确报错**：无需手工加 `PATH` 即可定位 Docker CLI；Windows 下子进程全量无黑窗；`/var/run/docker.sock` 权限不足或 Docker 未启动时返回中文可操作指引。
2. **启动前门禁拦截**：目标宿主机端口被占用、工作区剩余磁盘不足或 Elastic 启动时 Docker VM 内存 `< 6 GB`，在容器调度前拦截并指明冲突来源或调整方式。
3. **外部容器和平共处**：同名容器工作目录不匹配 `~/.code-ark/stacks/<id>` 时严格分类为 `ExternalObserve`（只读禁止写操作），仅在用户显式确认后执行 `docker stop && docker rm` 并保留原挂载卷/目录。
4. **全链路并发锁与幂等初始化**：DAG 自动排定基础库先行就绪；重复点击一键启动不出现竞争损坏，重复启动已初始化的栈保留既有业务数据。
5. **精准停止与隔离重置**：停止基础库前预警关联上层服务；重置 `temporal`/`xxjob`/`seata` 只清空其专属库与本地日志，绝不触碰同一 MySQL/PG 内的其他业务库。

## 5. 使用者与场景

### US-04 · 跨平台首次环境体检与无障碍定位 Docker

- **描述：** 作为在 macOS Finder 双击打开应用或使用 Windows 11 的开发者，希望工作箱自动找到我的 Docker 环境并提前告诉我端口与内存是否达标，以避免一点击启动就莫名失败。
- **来源与设计：** `C01`、`REQ-04`；[设计 02 §7.1–§7.2](../design/02-跨平台Docker探测与自动化编排设计.md#71-跨平台-docker-cli-探测与无窗执行设计建议)。
- **讨论：** 若用户使用 OrbStack / Rancher Desktop / Docker Desktop 自定义路径，既需自动探测常见路径也需保留设置覆盖入口；Windows 必须全量注入 `CREATE_NO_WINDOW`。
- **确认条件：**
  - 正常路径：三端默认搜索路径均能发现可执行 Docker 并读取 `DockerInfo.MemTotal`、工作区磁盘空间与实时容器列表；
  - 边界输入：Docker VM 内存 `4 GB` 触发 Elastic 启动时，在拉起容器前即被 6 GB 门禁拦截；目标端口被本地非本服务进程绑定时提前标为 `端口冲突`；
  - 异常提示：Daemon 未运行、CLI 缺失或 Linux socket 权限拒绝分别映射明确诊断码与修复命令建议。
- **追踪：** `TS-05`；Plan 02 任务 1–2。

### US-05 · 本机已有手动启动容器时的识别、观察与安全接管

- **描述：** 作为之前已从其它路径手动起过 `nacos-standalone` 或 `mysql-start-dev` 的开发者，希望工作箱认出这是一个“外部容器”并提供只读观察或主动安全替换，不要直接报错崩溃，也绝对不要默默删掉我的旧数据。
- **来源与设计：** `REQ-04`、`C05`；[设计 02 §7.2](../design/02-跨平台Docker探测与自动化编排设计.md#72-预检与外部容器分类规则设计建议)。
- **讨论：** 处于 `ExternalObserve` 的底层数据库绝不能被工作箱自动当作 `canal`/`temporal`/`xxjob`/`seata` 的初始化宿主（防止工作箱向陌生外部实例建表授权）。
- **确认条件：**
  - 正常路径：与工作箱托管目录不一致的同名容器正确识别为 `ExternalObserve`，只允许查看状态/端口/日志，禁用启停、编辑配置与重置按钮；
  - 边界输入：Windows 下盘符大小写或路径斜杠差异不导致误分类；
  - 异常与接管：尝试用工作箱自动编排依赖该外部库的上层服务时立即阻断并提示原因；用户二次确认“替换接管”时只执行 `docker stop <name> && docker rm <name>` 后从托管路径拉起，原外部仓库绑定目录与命名卷完整保留。
- **追踪：** `TS-06`、`TS-07`；Plan 02 任务 2、任务 4。

### US-06 · 复杂服务依赖链与 6 套特色初始化的一键幂等启动

- **描述：** 作为需要同时联调 MySQL、Canal、XXL-Job、Seata、Temporal 或 Elastic + IK 分词的开发者，希望点一次启动就能按正确依赖顺序自动建库建表、建账号、构建分词镜像或续期 License，并且下次重启不丢数据。
- **来源与设计：** `REQ-03`、`REQ-04`；[设计 02 §7.3](../design/02-跨平台Docker探测与自动化编排设计.md#73-拓扑编排锁与幂等初始化引擎设计建议)。
- **讨论：** 全部用原生 Rust + `reqwest` + `docker exec` 实现，`start.sh` 逻辑在 Windows 同样无缝运行；用户连点两次触发按钮时通过互斥锁拒绝重入而非并行拉起互相打架。
- **确认条件：**
  - 正常路径：`temporal` 先待 `pgsql` healthy 后同步账密建表并建 `default` Namespace；`canal` 待 `mysql` healthy 后幂等授权；`xxjob` 按实际 `${MYSQL_DATABASE}` 重写 SQL 建表；`seata` 缺失表时自动导入内置 4 张元数据表 DDL；`nacos` 提前 `ensure_volume`；`elastic` 串行构建 IK 并通过 HTTP 续期 Basic License；
  - 边界输入：对已运行/已建库栈再次点击启动，跳过重复破坏性写入并保留既有测试记录；快速连点并发操作被锁序列化或明确返回“任务进行中”；
  - 异常止损：中间步骤（如依赖预检或旧 `pgsql-data` 密码认证不匹配）失败时立即中止于具体失败步骤，不向未启动成功的栈残余半截密钥或忽略错误。
- **追踪：** `TS-07`、`TS-08`；Plan 02 任务 3–4。

### US-07 · 安全级联停止与作用域精准重置

- **描述：** 作为需要清理实验环境重新开始某个中间件测试的用户，希望“重置 XX 服务”只清它自己的日志和专属数据库，停止 MySQL 前提醒有哪些服务正连着它，绝不误删别的项目数据库或触发全盘 `docker prune`。
- **来源与设计：** `C05`；[设计 02 §7.4](../design/02-跨平台Docker探测与自动化编排设计.md#74-分级停止与作用域精准重置设计建议)。
- **讨论：** 这是防止工作箱成为“数据销毁器”的关键边界，重置分为 `local_only`（12 套）、`shared_db_and_local`（`temporal`/`xxjob`/`seata`）与 `shared_db_root`（`mysql`/`pgsql`）。
- **确认条件：**
  - 正常路径：重置 `temporal` 仅在 PG 执行 `DROP DATABASE IF EXISTS temporal` 与 `temporal_visibility`；重置 `xxjob`/`seata` 仅清理各自配置库 `${MYSQL_DATABASE}`；独立服务仅执行当前 `-p <project> down -v` 与本地数据日志目录清理；
  - 级联预警：停止或重置 `mysql`/`pgsql` 时若检测到 `canal`/`xxjob`/`seata`/`temporal` 运行或已关联，先返回受影响服务清单，获确认后一并安全停止或重设初始化标记；
  - 非破坏性：`test_db` 等用户自建库与无关容器/卷 100% 不受影响，严禁调用 `docker system prune`。
- **追踪：** `TS-08`、`TS-09`；Plan 02 任务 3–4。

## 6. 使用价值

- 将过去容易卡住 90% 新手与 Windows 用户的 `start.sh`、手敲授权 SQL、`tables_xxl_job.sql` 硬编码库名、`nacos` 外部卷和 PG 账号失配全部收归 Rust 确定性编排。
- 让“已经手动起过容器的开发者”与“使用共享数据库实验多套框架的用户”零风险共存。

## 7. 范围、需求与验收

### 7.1 使用或服务流程

1. **环境预检**：定位 Docker CLI → 查询 `docker info` / `docker ps -a` / `docker inspect` / 磁盘空间 → 探测宿主机目标端口与外部容器所有权（`Managed` / `ExternalObserve` / `NotPresent`）。
2. **编排管线**：获取 `OrchestratorState` 操作互斥锁 → 计算 DAG 拓扑启动序列 → 发射 `StepProgressEvent` 依次推进每项服务的 `Preflight → Materialize & .env → Pre-provision → Compose Up & Wait Healthy → Post-provision`。
3. **停止与重置**：计算反向关联服务 → 对共享根库弹出影响列表 → 按 `resetScope` 执行局部目录/专属数据库删除。

### 7.2 需求清单

| 本 PRD ID | 来源设计 | 所需行为 | 观察标准与边界 |
| --- | --- | --- | --- |
| `PRD02-R01` | 设计 02 §7.1 | 跨平台 Docker CLI 自动探测与非弹窗执行 | 覆盖 macOS/Windows/Linux 典型安装路径与手动覆盖；Windows 全量启用 `CREATE_NO_WINDOW`；明确区分 CLI 缺失、Daemon 未开与 Socket 权限拒绝 |
| `PRD02-R02` | 设计 02 §7.2 | 资源门禁与宿主机 TCP 端口冲突探测 | 检测磁盘余额、TCP loopback 端口占用，并强制 Elastic 栈 >= `6 GB` Docker VM 内存门槛（`ELASTIC_STACK_MIN_DOCKER_MEMORY_GB`） |
| `PRD02-R03` | 设计 02 §7.2 | 外部容器 `ExternalObserve` 隔离与确认替换接管 | 归一化路径比对识别外部实例；观察态禁用启停/编辑/重置且禁止充当自动依赖库；替换仅移除旧容器外壳（`docker stop && docker rm`）并保留绑定数据/命名卷 |
| `PRD02-R04` | 设计 02 §7.3 | 并发锁控、DAG 拓扑排序与分步失败止损 | 严禁并发重入启停；`mysql`/`pgsql` 先达 `healthy` 后再启上层服务；任一步骤失败立即定位失败步骤并中断后续链路 |
| `PRD02-R05` | 设计 02 §7.3 | 6 套特殊服务纯原生幂等初始化 | 涵盖 `nacos` 卷、`temporal` PG 账密同步校验、`canal` 复制账号、`xxjob` SQL 自定义库名重写、`seata` 内置 DDL 导入、`elastic` 串行构建 + `reqwest` License 续期 |
| `PRD02-R06` | 设计 02 §7.4 | 分级停止提醒与 3 类作用域隔离重置 | 停根库先提示运行中的上层依赖；`shared_db_and_local` 仅删专属库；`shared_db_root` 级联警示后清理；全程禁 `docker system prune` |

### 7.3 需求承接核对

| 已确认要求 | 唯一技术合同 | 验收与实施 |
| --- | --- | --- |
| `C01` macOS + Windows + Linux 一致行为 | [设计 02 §7.1](../design/02-跨平台Docker探测与自动化编排设计.md#71-跨平台-docker-cli-探测与无窗执行设计建议) | `PRD02-R01`；Plan 02 任务 1 |
| `REQ-03`/`REQ-04` 7 场景与 6 类痛点自动化 | [设计 02 §7.2–§7.3](../design/02-跨平台Docker探测与自动化编排设计.md#73-拓扑编排锁与幂等初始化引擎设计建议) | `PRD02-R02`、`PRD02-R04`、`PRD02-R05`；Plan 02 任务 2–4 |
| `C05` 外部容器保护与共享库作用域不误伤 | [设计 02 §7.2、§7.4](../design/02-跨平台Docker探测与自动化编排设计.md#74-分级停止与作用域精准重置设计建议) | `PRD02-R03`、`PRD02-R06`；Plan 02 任务 2–4 |

### 7.4 验收测试场景

#### TS-05 · Docker 错误分类、TCP 端口占用与 Elastic 6 GB 内存门禁

- **目标：** 核对 `PRD02-R01`、`PRD02-R02`、`US-04`。
- **前置条件：** Rust 测试环境；本地回环监听测试端口；模拟 Docker 信息样本。
- **角色：** 开发 / 验收人员。
- **步骤与结果：**
  1. 解析含 `socket permission denied`、Daemon 未启动等错误字符串，断言映射到对应可读错误分支而非原始 exit code；
  2. 在 `127.0.0.1` 绑定临时端口，断言 `check_tcp_port_available` 返回 `false`，释放后返回 `true`；
  3. 传入 `MemTotal = 4 GiB` 对 Elastic 执行预检，断言阻断启动并输出调大 Docker 内存至 >= 6 GB 提示；
  4. 核对三端候选扫描路径清单及 Windows `CREATE_NO_WINDOW` 标志覆盖。
- **例外：** 用户显式配置自定义 Docker CLI 路径时优先于默认扫描路径生效。
- **证据：** 未执行。

#### TS-06 · 外部容器 `ExternalObserve` 隔离与替换接管验证

- **目标：** 核对 `PRD02-R03`、`US-05`。
- **前置条件：** 存在非 `~/.code-ark/stacks/nacos` 目录运行的外部容器 `nacos-standalone`（与本机现场观测情况一致）或外部 `mysql-start-dev`。
- **角色：** 开发者 / 验收人员。
- **步骤与结果：**
  1. 刷新工作箱容器状态，断言 `nacos` 被标记为 `ExternalObserve`（附来源目录与挂载目录）；
  2. 尝试通过工作箱启动依赖外部 `mysql-start-dev` 的 `canal`，断言在预检步中止于 `ExternalConflictRequiresAction`，未写入半截 `.env` 密钥；
  3. 仅在工作箱内确认“替换接管”后，断言执行 `docker stop` + `docker rm` 移除旧容器外壳并由托管目录启动新容器，原外部目录与命名卷内容保留。
- **例外：** Windows 路径存在反斜杠与盘符大小写差异时不得把 `Managed` 容器误判为 `ExternalObserve`。
- **证据：** 未执行。

#### TS-07 · 复合依赖链并行连点拦截、拓扑启动与幂等重启

- **目标：** 核对 `PRD02-R04`、`PRD02-R05`、`US-06`。
- **前置条件：** Docker Daemon 运行中，使用隔离临时工作区 `CODE_ARK_WORKSPACE=$(mktemp -d)`。
- **角色：** 验收人员 / CI。
- **步骤与结果：**
  1. 对同一运行编排器并发发起两次 `start_services`（如两次 `["mysql", "canal", "xxjob", "seata"]`），断言互斥锁拒绝重入（`AlreadyRunning`）或严格串行，状态流不交错；
  2. 观察首次启动步骤事件：`mysql` 先到达 `healthy`，随后执行 `canal` 复制账号授权、`xxjob` SQL 导入与 `seata` 内置 4 表导入并启动就绪；
  3. 检验 `pgsql` -> `temporal` 链路自动同步 `PGSQL_USER`/`PGSQL_PASSWORD`、初始化 Schema 与 `default` Namespace；
  4. 在表内写入测试数据后停止再重启全部服务，断言原有数据保留且不报错。
- **例外：** 若 `pgsql-data` 已由旧密码初始化导致 `temporal` 校验不匹配，应直接报告凭证不匹配引导改回密码或确认重置，不覆盖原有目录。
- **证据：** 未执行。

#### TS-08 · `tables_xxl_job.sql` 自定义库名重写与共享数据库作用域隔离重置

- **目标：** 核对 `PRD02-R05`、`PRD02-R06`、`US-06`、`US-07`。
- **前置条件：** 隔离工作区中 `mysql` 与 `pgsql` 均运行，且分别建有与框架无关的测试库 `test_db`。
- **角色：** 开发 / 验收人员。
- **步骤与结果：**
  1. 将仓库真实 [tables_xxl_job.sql](../../xxjob-start-local/tables_xxl_job.sql) 传入重写函数并指定 `custom_job_db`，断言生成 SQL 将顶部 `CREATE DATABASE` / `USE` 替换为 `custom_job_db` 且保留其余 `CREATE TABLE`/`INSERT` 语句；
  2. 执行 `temporal` 重置，断言仅清理 `temporal` 与 `temporal_visibility` 库及本地日志，`pgsql` 内的 `test_db` 完好无损；
  3. 执行 `xxjob` 重置，断言仅清理 `${MYSQL_DATABASE}` 与本地日志，同实例下的 `seata` 与 `test_db` 完好无损。
- **例外：** 正在执行重置时，所有相关服务需按预检查先就绪数据库或在安全停止状态下处理，不静默忽略 SQL 执行错误。
- **证据：** 未执行。

#### TS-09 · 基础数据库级联停止警示与根库重置安全保护

- **目标：** 核对 `PRD02-R06`、`US-07`。
- **前置条件：** `mysql` 与上层服务 `canal`、`xxjob` 同时处于运行态。
- **角色：** 开发者 / 验收人员。
- **步骤与结果：**
  1. 发起 `stop_services(["mysql"], cascade_dependents = false)`，断言函数不关闭 `mysql` 而是返回包含 `canal`、`xxjob` 的 `DependentServicesActive` 清单；
  2. 确认带 `cascade_dependents = true` 后先安全停掉上层依赖再停 `mysql`；
  3. 对 `mysql` 发起数据重置，确认返回级联影响清单，经二次确认后停止依赖、清本地绑定/卷并复位上层服务状态标记。
- **例外：** 若当前没有上层服务运行/初始化，则直接完成正常停止。
- **证据：** 未执行。

### 7.5 假设与待定

- 本阶段不自动迁移用户过去手工维护在其它 Git 克隆目录下 `mysql-data` / `pgsql-data` 里的二进制数据库文件；外部容器场景通过只读观察或用户自主导出 SQL 迁移后再用工作箱接管。

## 8. Plan 与阶段交付

- [02 跨平台 Docker 探测与编排引擎实施计划](../plan/02-跨平台Docker探测与编排引擎实施计划.md)（当前阶段：待实施，对应 `PRD02-R01`–`PRD02-R06` 及 `TS-05`–`TS-09`）。

[返回 PRD 导航](00-PRD导航.md)
