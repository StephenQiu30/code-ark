# 01 Code Ark 工作箱顶层架构与蓝图同步设计

状态：已对齐基线，待进入实施。依据：[需求 01](../requirement/01-工作箱产品总览与核心诉求.md)（`REQ-01`、`REQ-02`、`C01–C06`）、[调研 02](../research/02-Tauri最新稳定版本与跨平台技术选型调研.md)、[调研 03](../research/03-CodeArk现有17套服务痛点与兼容基线盘点.md)。

## 1. 摘要

确定基于 **Tauri v2 最新稳定系列（`tauri = "2.12.5"`、`tauri-build = "2.7.4"`、`@tauri-apps/cli = "2.12.4"`、`@tauri-apps/api = "2.12.4"`）** 构建跨平台桌面工作箱（源码位于仓库 `toolbox/`）。核心目标是让根目录现有 17 套服务配置保持原样作为构建期单一数据源（SSOT），由工作箱打包内嵌蓝图并在用户本机 `~/.code-ark/stacks/<service_id>/` 非破坏性物化运行，彻底消除终端用户对 Git 和手工编辑 `.env` 的依赖。

## 2. 参与者

- **终端用户（编程新手 / 日常开发者）**：下载独立安装包，零命令行/零 Git 启动服务，管理本地持久化数据与端口。
- **仓库维护者**：继续按 [CONTRIBUTING.md](../../CONTRIBUTING.md) 与 [AGENTS.md](../../AGENTS.md) 维护根目录各 `*-start-local` 配置，无需维护两份重复 YAML。

## 3. 背景

- 根目录存在 17 套 Compose 服务与 1 份宿主机工具指南，包含历史目录拼写（`mysql-start-lcoal`、`rabbitmq-start-lcoal`）、12 份以上依赖 `.env` 的服务及多处硬编码端口/网络名（详见 [调研 03](../research/03-CodeArk现有17套服务痛点与兼容基线盘点.md)）。
- 现场核查显示：Tauri `3.0.0-alpha.6` 尚属 alpha 预览版，而 `2.12.5` 为 2026-10-21 发布的最新稳定版，满足三端打包与插件兼容要求（见 [调研 02](../research/02-Tauri最新稳定版本与跨平台技术选型调研.md)）。

## 4. 目标

- **可检查交付结果**：
  1. 维护一套 `toolbox/catalog/services.json` 与 `sync-blueprints` 构建同步流水线，完整提取 17 套 Compose 蓝图及 `mediacrawler` 补丁指引，且同步产物 100% 排除本地 `.env`、`*-data/`、`logs/`、`.DS_Store`；
  2. 支持零 Git 开箱物化到 `~/.code-ark/stacks/<service_id>/`，屏蔽历史拼写目录名同时保持 `-p <canonical_project>` 网络/卷名兼容；
  3. 通过 `compose.toolbox.override.yml` 解决端口冲突改绑、Linux 下 `seata`/`xxjob` 接入 `mysql-start-lcoal_default` 与 RocketMQ 宿主机直连，且不改动根目录原 YAML。
- **Non-Goals**：不内置 Docker Engine / 虚拟机；不做在线热更新蓝图通道（列为 v2 评估 `Q02`）。
- **Anti-Goals**：不修改根目录已有服务文件破坏 CLI 用户体验；不覆盖用户已存在的 `.env` 或持久化数据。

## 5. 使用场景

- 新手从 Releases 下载 `.dmg` / `.exe` / `.AppImage` 首次打开工作箱，未安装 Git、未 clone 仓库，选择任意服务即可直接配置并运行。
- 用户已有自定义 `.env` 或因本机冲突改过端口（如 MySQL `3306 → 13306`、Kafka `9092 → 19092`），升级工作箱版本时保留全部用户设置与数据。

## 6. 价值

- 维护成本单一化（Compose 修改只改原目录，构建自动同步）；
- 新手零 Git 门槛、零历史拼写心智负担；
- 解决原仓库 12 个硬编码端口与 Linux `host.docker.internal` 跨容器访问障碍。

## 7. 方案

### 7.1 已确认规则（`C01–C06`）

- 采用 Tauri `2.12.5`（Rust 核心 + React 19 / TypeScript / Tailwind CSS 前端），覆盖 macOS（arm64/x86_64）、Windows 10/11 与 Linux；
- 不依赖本地 Git 仓库，全量蓝图随应用打包（`tauri.conf.json` `bundle.resources`）；
- 根目录现有 `*-start-local` / `*-start-lcoal` 保持完全只读不被运行期或构建脚本改写。

### 7.2 架构分层与仓库目录设计（设计建议，已纳入基准方案）

```text
toolbox/
├── package.json                  # @tauri-apps/cli 2.12.4, @tauri-apps/api 2.12.4
├── catalog/services.json         # 17 Compose + 1 Host Guide + 7 Presets 结构化元数据
├── scripts/
│   ├── sync-blueprints.mjs       # 构建前从 ../<sourceDir> 提取纯净蓝图
│   └── verify-blueprints.mjs     # 校验 17+1 完整性、LF 行尾与零敏感/数据残留
├── src/                          # React + TS 前端工作台（见设计 03）
└── src-tauri/
    ├── Cargo.toml                # tauri 2.12.5, tauri-build 2.7.4, plugins 2.x
    ├── tauri.conf.json           # beforeDevCommand/beforeBuildCommand 触发 sync
    ├── resources/
    │   ├── blueprints/<id>/      # sync-blueprints 产物（.gitignore 排除或构建前生成）
    │   └── sql/seata_1_8_0_mysql.sql
    └── src/                      # catalog / docker_cli / preflight / env_store / blueprint / provisioner / orchestrator / diagnostics / commands
```

### 7.3 运行时工作区与非破坏性物化规则（设计建议）

- **默认托管路径**：`~/.code-ark/stacks/<service_id>/`（Windows 为 `%USERPROFILE%\.code-ark\stacks\<service_id>\`）。**选择理由**：短路径、跨平台无空格（避免 Bind Mount 路径转义问题），并避开 macOS Desktop/Documents TCC 弹窗；允许用户在“设置”中切换存储路径。
- **物化与哈希保护（`.blueprint-manifest.json`）**：
  - 每个服务栈首次需要时从内嵌资源释放静态资产，并记录各蓝图文件 SHA-256；
  - App 升级触发刷新时：仅当文件当前哈希等于旧 manifest 哈希（未被用户手动改动）才更新；若检测到用户手工改过 `docker-compose.yml` 等文件，先保存 `.bak.<timestamp>` 备份再提示/处理，**永不覆盖 `.env` 与数据/日志目录**。
- **强制 LF 写入**：对所有 `.sh` 文件（如 `temporal` 的 `scripts/setup-postgres.sh`、`scripts/create-namespace.sh`）按 `\n` 字节写入磁盘，防止 Windows 下 CRLF 导致容器内 `/bin/sh` 执行失败。

### 7.4 无损 `.env` 合并与 `OsRng` 密钥规则（设计建议）

- 使用行级 AST 解析现有 `.env` 与蓝图 `.env.example`：保留原有注释、空行、键顺序与含 `=` 的 Base64/URL 值；
- 仅当 `.env` 缺失某必填变量，或原有值为占位符（空值、`change_me_*`、`replace_*`、`your_*`）且用户点击生成/一键启动时，才使用 `rand::rngs::OsRng` 生成密钥：
  - `NACOS_AUTH_TOKEN`：随机生成原始 >= 32 字节并 Base64 编码；
  - `CANAL_MYSQL_PASSWORD`：安全随机字符，长度严格 **<= 32 字符**（满足 Canal 协议限制）；
  - `SEARXNG_SECRET`：64 位十六进制字符串；
  - `KIBANA_ENCRYPTION_KEY`：>= 32 字符随机密钥；
  - 数据库 / MinIO / RabbitMQ / ES 密码：生成强随机密码并持久化到本地 `.env`。

### 7.5 固定 Project Name 与 `compose.toolbox.override.yml` 覆盖层（设计建议）

- **Project Name 锁定**：所有 `docker compose` 调用显式携带 `-p <canonical_project>`（例如 `mysql` → `mysql-start-lcoal`，`pgsql` → `pgsql-start-local`，`rsshub` → `rsshub`），保证网络名（`mysql-start-lcoal_default`、`pgsql-start-local_default`）及命名卷前缀与原仓库完全一致。
- **动态 Override 层**（当启用端口覆盖、MySQL 联动或 RocketMQ 宿主机模式时生成）：
  - **端口改绑**：利用 Compose v2.20+ `ports: !override [...]` 替换宿主机端口，并同步重写耦合环境（如 `kafka` 更新 `KAFKA_ADVERTISED_LISTENERS`、`monitoring` 更新 `GF_SERVER_ROOT_URL`）；
  - **`seata` 与 `xxjob` 跨平台 MySQL 联动**（默认开启）：在 `compose.toolbox.override.yml` 中将容器接入外部网络 `mysql-start-lcoal_default` 并注入 `MYSQL_HOST=mysql`、`MYSQL_PORT=3306`（既打通 Linux 下不依赖 `host.docker.internal`，又兼容 MySQL 宿主机端口被改绑与 Seata `application.yml` 硬编码 `:3306`）；
  - **RocketMQ 宿主机直连模式**（默认开启）：生成 `conf/broker.toolbox.conf`（`brokerIP1 = 127.0.0.1`、`listenPort = 15911`、`haListenPort = 15912`）并在 override 中映射 `15876:9876`、`15909:15909`、`15911:15911`、`15912:15912`。

## 8. 实施与验收

- **下游来源映射**：由 [PRD 01](../prd/01-CodeArk工作箱底座与蓝图管理PRD.md) 承接范围与验收标准，由 [Plan 01](../plan/01-服务清单与蓝图同步实施计划.md) 定义文件与自动化验证步骤。
- **核心验证出口**：`verify-blueprints.mjs` 完整性与零敏感文件检查、Rust `catalog` / `env_store` / `blueprint` 单元测试（含脏 `.env` 保留、LF 字节验证、Override YAML 断言）。

[返回设计导航](00-设计导航.md)
