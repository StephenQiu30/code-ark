# 01 Code Ark 工作箱底座与蓝图管理 PRD

来源 design：[01 Code Ark 工作箱顶层架构与蓝图同步设计](../design/01-CodeArk工作箱顶层架构与蓝图同步设计.md)。本 PRD 只对应这一份 design，面向项目负责人、终端开发者与验收人员说明工作箱底座行为、范围边界与验收标准；架构分层、目录布局与具体 YAML/环境变量规则回来源 design。执行计划见 §8。

## 1. 摘要

让未安装 Git、未克隆仓库的 macOS / Windows / Linux 开发者下载安装包后直接运行 Code Ark 全部 17 套 Docker Compose 服务，同时保证根目录已有配置文件继续作为维护者唯一的构建期数据源（SSOT），绝不覆盖用户本机既有的 `.env` 与持久化数据。

## 2. 责任与确认

| 角色 | 责任 |
| --- | --- |
| 项目负责人 | 确认“纯独立开箱即用”、三端跨平台（Tauri `2.12.5`）、保持根仓库只读及非破坏性数据边界 |
| 开发与执行人员 | 严格按 [Plan 01](../plan/01-服务清单与蓝图同步实施计划.md) 实现构建期蓝图同步与 Rust 底座模块，提供可复现单元与验证证据 |
| 验收与日常使用者 | 按 §7.4 测试场景核对无 Git 物化、脏 `.env` 保序合并、覆盖层生效与根目录未受污染 |

核心规则来源于 [需求 01](../requirement/01-工作箱产品总览与核心诉求.md)（`REQ-01`、`REQ-02`、`C01–C06`）及 [设计 01](../design/01-CodeArk工作箱顶层架构与蓝图同步设计.md)。

## 3. 背景

- 当前仓库 17 套 Compose 服务分散在各自目录中，存在历史拼写目录名（`mysql-start-lcoal`、`rabbitmq-start-lcoal`）、多份必填 `.env`、硬编码宿主机端口，以及 Windows 下 `.sh` 换行敏感与 Linux 下 `host.docker.internal` 连通性问题。
- 若另行复制一份独立 Compose 给桌面端维护，会导致根目录 README/YAML 与桌面端长期漂移；因此必须在构建期自动提取并校验蓝图。

## 4. 目标与成功标准

1. **构建期单源同步零污染**：同步产出的 17 套蓝图 + 1 份 `mediacrawler` 指引与根仓库最新改动一致，且 **0 份** `.env`、`*-data/`、`data/`、`logs/`、`.DS_Store` 被打进应用资源包。
2. **用户端零 Git 开箱物化**：首次打开或启动服务时，自动释放到短路径托管区（默认 `~/.code-ark/stacks/<service_id>/`），所有 `.sh` 容器脚本严格写入 LF（`\n`）字节。
3. **无损配置与密钥合规**：已有 `.env` 的注释、顺序与含 `=` 值 100% 保留；缺失或占位凭据仅通过 `OsRng` 按各服务约束生成（如 `CANAL_MYSQL_PASSWORD <= 32` 字符）。
4. **非侵入式覆盖能力**：端口改绑、`seata`/`xxjob` 跨平台连接内置 MySQL、以及 `rocketmq` 宿主机直连全部通过 `compose.toolbox.override.yml` 完成，不改写原始蓝图模板。

## 5. 使用者与场景

### US-01 · 零 Git 新手直接解压运行工作箱

- **描述：** 作为刚接触编程且未安装 Git 的 Windows / macOS 新手，希望下载工作箱安装包后直接选一个服务启动，以零命令行前置跑通本地数据库与中间件。
- **来源与设计：** `REQ-01`、`C03`；[设计 01 §7.3](../design/01-CodeArk工作箱顶层架构与蓝图同步设计.md#73-运行时工作区与非破坏性物化规则设计建议)。
- **讨论：** 仍需本机存在可用 Docker 运行时（由 [PRD 02](02-跨平台环境预检与服务自动化编排PRD.md) 检测提示）；若工作区路径位于系统受保护目录以外的短路径，可避免 macOS TCC 频繁弹窗与 Windows 含空格路径挂载失败；不擅自联网下载云端配置文件作为前提（`Q02` 留 v2）。
- **确认条件：**
  - 正常路径：无 `.git` 目录与 Git CLI 环境下，`services.json` 与内置蓝图可完整物化到 `~/.code-ark/stacks/<id>/` 并以 `-p <canonical_project>` 调度 Compose；
  - 边界输入：Windows 用户目录下存在 `.sh` 脚本的服务（如 `temporal`）物化后行尾保持纯 `\n`；
  - 异常提示：默认托管路径不可写时给出明确文件系统错误及在“设置”更换路径入口；
  - 非破坏性：任何情况下不读写仓库根目录或安装目录之外的无关文件夹。
- **追踪：** `TS-01`、`TS-04`；Plan 01 任务 1–2、任务 4。

### US-02 · 老用户自定义 `.env` 与改绑端口升级兼容

- **描述：** 作为已有自定义 `.env` 注释、特殊密码或因端口冲突将 Kafka/MySQL 改绑至非默认端口的开发者，希望工具箱更新配置或升级版本时完全保留我的现有设置，以获得无痛升级体验。
- **来源与设计：** `C05`；[设计 01 §7.3–§7.5](../design/01-CodeArk工作箱顶层架构与蓝图同步设计.md#74-无损-env-合并与-osrng-密钥规则设计建议)。
- **讨论：** 仅当用户手动修改了托管目录中的 `docker-compose.yml` 时才需留存 `.bak.<timestamp>` 备份；用户自己的 `.env` 与数据卷/数据目录永远属于高安全边界；需校验 `CANAL_MYSQL_PASSWORD` 等协议长度限制。
- **确认条件：**
  - 正常路径：向已含注释、空行、Base64 `==` 值及缺失末尾换行的 `.env` 追加新字段时，原有行、注释与有效用户密钥原样保留；
  - 边界输入：占位符（空值、`change_me_*`、`replace_*`、`your_*`）在用户触发生成/启动时自动替换为合规随机值，非占位用户密码不被覆盖；
  - 覆盖层路径：修改 `kafka` 宿主机端口 `9092 → 19092` 时，生成 `compose.toolbox.override.yml` 使用 `ports: !override` 并同步改写 `KAFKA_ADVERTISED_LISTENERS`；启用 `seata`/`xxjob` MySQL 联动或 `rocketmq` 宿主机模式时生成对应网络与 `broker.toolbox.conf`；
  - 异常提示：用户手改过 `docker-compose.yml` 遇版本刷新时先备份 `.bak.<timestamp>` 再提示。
- **追踪：** `TS-02`、`TS-03`；Plan 01 任务 3–4。

### US-03 · 仓库维护者单一来源演进

- **描述：** 作为 Code Ark 仓库维护者，希望继续按根目录 `AGENTS.md` 与 `CONTRIBUTING.md` 维护原有 17 套服务目录，由构建脚本自动提取纯洁蓝图并校验完整性，以避免同步遗漏或误把本机密码/数据打包进发行版。
- **来源与设计：** `C04`、`C06`；[设计 01 §7.2](../design/01-CodeArk工作箱顶层架构与蓝图同步设计.md#72-架构分层与仓库目录设计设计建议已纳入基准方案)。
- **讨论：** 构建期同步脚本只读访问根目录，严禁回写根 `*-start-local` / `*-start-lcoal` 目录；`mysql-start-lcoal` 与 `rabbitmq-start-lcoal` 的历史拼写只在元数据 `sourceDir` 与 `canonicalProject` 中保留，对终端用户统一显示标准名称 `mysql` / `rabbitmq`。
- **确认条件：**
  - 正常路径：运行同步与校验脚本后完整提取 17 个 Compose 蓝图 + 1 份 `mediacrawler` 补丁指南 + 7 个场景预设元数据；
  - 异常阻止：若缺少 `tables_xxl_job.sql`、`broker.conf` 等关键文件，或资源目录内混入 `.env` / `*-data/` / `logs/` / CRLF `.sh`，校验脚本必须以非 0 退出码阻断打包；
  - 非破坏性：`git status` 检查根目录现有 18 个服务目录零修改。
- **追踪：** `TS-04`；Plan 01 任务 1。

## 6. 使用价值

- **对用户**：把 18 个碎片目录、`lcoal` 历史拼写、环境密钥规则与 Linux/RocketMQ 网络兼容细节收敛成开箱零配置体验。
- **对仓库维护**：继续坚持单一权威来源（SSOT），不额外增加手动同步负担，从源头杜绝本地密钥与数据目录打进 release 安装包的安全事故。

## 7. 范围、需求与验收

### 7.1 使用或服务流程

1. **构建期**：维护根目录 `*-start-local` → `sync-blueprints.mjs` 提取纯净蓝图与 `services.json` → `verify-blueprints.mjs` 完整性/脱敏/LF 校验 → Tauri `2.12.5` 打包 `resources/`。
2. **运行期**：用户启动工作箱 → Rust 读取内嵌 `services.json` → 物化目标服务至 `~/.code-ark/stacks/<id>/`（记录 `.blueprint-manifest.json`、强制 LF）→ 无损合并 `.env`（缺项由 `OsRng` 补齐）→ 按需写入 `compose.toolbox.override.yml` → 锁定 `-p <canonical_project>` 交由 Compose 执行。

### 7.2 需求清单

| 本 PRD ID | 来源设计 | 所需行为 | 观察标准与边界 |
| --- | --- | --- | --- |
| `PRD01-R01` | 设计 01 §7.1–7.2 | Tauri `2.12.5` 底座与 17+1+7 完整目录元数据 | `services.json` 包含 17 个 Compose 服务、1 个宿主机指引（`mediacrawler`）、7 个预设；正确映射 `mysql-start-lcoal`、`rabbitmq-start-lcoal`、`rsshub`、`searxng` 的 `canonicalProject` |
| `PRD01-R02` | 设计 01 §7.2 | 构建期 SSOT 同步与纯净资产校验 | 提取全部 `docker-compose.yml`、`.env.example`、`conf`/`init`/`scripts`/`patches`/SQL；零 `.env`、`*-data/`、`data/`、`logs/`、`.DS_Store`、宿主机 `start.sh`；只读源目录 |
| `PRD01-R03` | 设计 01 §7.3 | 非破坏性物化、哈希保护与 `.sh` 强制 LF | 默认物化到 `~/.code-ark/stacks/<id>/`；记录 `.blueprint-manifest.json`；用户改造过的 Compose 文件更新前先备份 `.bak`；全部 `.sh` 文件不含 `\r\n` |
| `PRD01-R04` | 设计 01 §7.4 | 无损 `.env` AST 合并与按服务规则 `OsRng` 生成密钥 | 保留注释、空行、键序与含 `=` 值；`NACOS_AUTH_TOKEN` >=32 字节 Base64、`CANAL_MYSQL_PASSWORD` <=32 字符、`SEARXNG_SECRET` 64 hex、`KIBANA_ENCRYPTION_KEY` >=32 字符 |
| `PRD01-R05` | 设计 01 §7.5 | 固定 `-p <canonical_project>` 与 `compose.toolbox.override.yml` 覆盖层 | 支持 `ports: !override` 并同步耦合环境变量（Kafka/Monitoring）；`seata`/`xxjob` 默认接入 `mysql-start-lcoal_default`；`rocketmq` 支持生成 `conf/broker.toolbox.conf` 宿主机直连 |
| `PRD01-R06` | 设计 01 §4 | 非破坏与根仓库隔离边界 | 永不覆盖已有 `.env` 与数据目录；根目录原有 `*-start-local` / `*-start-lcoal` 零改动；未安装 Git 的纯环境可完整执行底座能力 |

### 7.3 需求承接核对

| 已确认要求 | 唯一技术合同 | 验收与实施 |
| --- | --- | --- |
| `C02` 采用 Tauri 最新稳定版（`2.12.5`） | [设计 01 §7.1](../design/01-CodeArk工作箱顶层架构与蓝图同步设计.md#71-已确认规则c01c06) | `PRD01-R01`；Plan 01 任务 2 |
| `C03`/`C04`/`C06` 零 Git 开箱即用 + 17+1 全覆盖 + 根仓库只读 | [设计 01 §7.2–§7.3](../design/01-CodeArk工作箱顶层架构与蓝图同步设计.md#72-架构分层与仓库目录设计设计建议已纳入基准方案) | `PRD01-R01`、`PRD01-R02`、`PRD01-R03`、`PRD01-R06`；Plan 01 任务 1–2、任务 4 |
| `C05` 不覆盖 `.env`、秘钥随机生成、解决 Linux 与端口覆盖 | [设计 01 §7.4–§7.5](../design/01-CodeArk工作箱顶层架构与蓝图同步设计.md#74-无损-env-合并与-osrng-密钥规则设计建议) | `PRD01-R04`、`PRD01-R05`；Plan 01 任务 3–4 |

### 7.4 验收测试场景

#### TS-01 · 蓝图同步纯净性与强制 LF 校验

- **目标：** 核对 `PRD01-R01`、`PRD01-R02`、`PRD01-R03`、`US-01`、`US-03`。
- **前置条件：** 仓库根目录包含既有服务目录（可模拟或存在本机 `.env`、`mysql-data/`、`logs/` 等）；Node.js 可用。
- **角色：** 仓库维护者 / CI。
- **步骤与结果：**
  1. 执行同步与校验脚本，检查 17 个 Compose 蓝图和 `mediacrawler` 补丁均落入 `toolbox/src-tauri/resources/blueprints/`；
  2. 断言资产目录中不存在任何 `.env`、`*-data`、`data`、`logs`、`.DS_Store` 及宿主机 `start.sh`；
  3. 逐字节扫描 `resources/blueprints/` 下所有 `.sh` 文件（如 `temporal/scripts/*.sh`），断言不存在 `\r\n`；
  4. 检查根目录 `git status --short` 确认原有服务目录零改动。
- **例外：** 某服务缺少 `docker-compose.yml`、缺关键 SQL/conf 文件，或人工在资源中塞入 `.env` 时，`verify-blueprints.mjs` 必须非零退出报错。
- **证据：** 未执行。

#### TS-02 · 含注释与历史密钥的脏 `.env` 无损合并与占位符替换

- **目标：** 核对 `PRD01-R04`、`US-02`。
- **前置条件：** Rust 单元测试环境就绪。
- **角色：** 开发 / 验收人员。
- **步骤与结果：**
  1. 准备含中文注释、空行、CRLF 行尾、Base64 `==` 值、无结尾换行的现有 `.env`，其中保留用户已改密码且存在占位符字段（如 `change_me_token`）与缺失新必填字段；
  2. 调用 `.env` 无损合并逻辑生成更新后文档；
  3. 断言原注释、空行与用户密码 100% 未变，仅占位符与缺失项被按 `OsRng` 规则填补；
  4. 验证 `NacosBase64` 解码 >= 32 字节、`CanalPassword` <= 32 字符、`Hex64` 为 64 位十六进制、`EncryptionKey32` >= 32 字符。
- **例外：** `.env.example` 为空或为 CRLF 格式时仍正常合并写入且不崩溃。
- **证据：** 未执行。

#### TS-03 · `compose.toolbox.override.yml` 多模式覆盖层生成与备份保护

- **目标：** 核对 `PRD01-R03`、`PRD01-R05`、`US-02`。
- **前置条件：** 临时工作目录，已物化 `kafka`、`monitoring`、`seata`、`xxjob`、`rocketmq`。
- **角色：** 开发 / 验收人员。
- **步骤与结果：**
  1. 修改物化后的某份 `docker-compose.yml` 一行注释并再次触发物化，断言原用户版本已备份为 `.bak.<timestamp>` 且 `.env` 未变动；
  2. 配置 `kafka` 宿主机端口 `19092`、`monitoring` Grafana 端口 `23000`，断言生成的 `compose.toolbox.override.yml` 使用 `ports: !override` 并同步更新 `KAFKA_ADVERTISED_LISTENERS` 与 `GF_SERVER_ROOT_URL`；
  3. 为 `seata` / `xxjob` 开启内置 MySQL 联动，断言 override 将容器接入外部网络 `mysql-start-lcoal_default` 且设置 `MYSQL_HOST=mysql`、`MYSQL_PORT=3306`；
  4. 为 `rocketmq` 开启宿主机直连模式，断言写出 `conf/broker.toolbox.conf`（`brokerIP1 = 127.0.0.1`、`listenPort = 15911`、`haListenPort = 15912`）并挂载映射端口；当关闭全部覆盖开关时 `compose.toolbox.override.yml` 被清理。
- **例外：** Windows 风格大小写盘符路径物化与哈希比对不应误触发覆盖备份。
- **证据：** 未执行。

#### TS-04 · 零 Git 纯净环境物化与 `-p` 项目名锁定运行

- **目标：** 核对 `PRD01-R01`、`PRD01-R03`、`PRD01-R05`、`PRD01-R06`、`US-01`。
- **前置条件：** 移除环境 `GIT_DIR` 且在非仓库目录临时工作区运行（如 `CODE_ARK_WORKSPACE=$(mktemp -d)`），Docker Daemon 运行中。
- **角色：** 终端用户（无 Git 环境）。
- **步骤与结果：**
  1. 不依赖本地源码目录直接物化 `redis` 与 `mysql` 并启动；
  2. 检查启动命令携带 `-p redis-start-local` 与 `-p mysql-start-lcoal`，网络与容器标签正确归属于 `~/.code-ark/stacks/<id>`（或临时工作区）。
- **例外：** 目标目录只读无写权限时返回清晰可读错误提示而非静默失败。
- **证据：** 未执行。

### 7.5 假设与待定

- **依赖假定**：宿主机使用的 Docker Compose 版本为 Compose v2.20+（支持 `ports: !override` 语义）；若检测用户本机 Compose 版本过旧，[PRD 02](02-跨平台环境预检与服务自动化编排PRD.md) 预检模块需明确提示升级 Docker Desktop / Compose 插件。
- **`Q01` 代码签名与公证**：未购买商业签名证书阶段，macOS Gatekeeper / Windows SmartScreen 首次拦截提示随文档与前端指引解决，不影响本地核心功能。
- **`Q02` 在线热更新蓝图**：v1 明确不做，蓝图更新随工作箱版本发布迭代。

## 8. Plan 与阶段交付

- [01 服务清单与蓝图同步实施计划](../plan/01-服务清单与蓝图同步实施计划.md)（当前阶段：待实施，对应 `PRD01-R01`–`PRD01-R06` 及 `TS-01`–`TS-04`）。

[返回 PRD 导航](00-PRD导航.md)
