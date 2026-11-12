# 项目结构规范

本文件定义 Code Ark 的目录职责和新增文件位置。当前仓库是 **17 套 Docker Compose 配置 + 1 份宿主机工具指南 + 工作箱设计文档**；`toolbox/` 尚未创建、桌面工作箱尚未实现或发行。下方规划结构不作为已实现能力声明。

English Summary: Keep current Compose templates at the repository root. Put service-owned configuration and initialization beside that service. The planned desktop application belongs under `toolbox/`; runtime secrets, data, caches, and release binaries stay outside source control.

## 当前根目录

```text
code-ark/
├── README.md / README.en.md       # 中英文使用入口与服务表
├── AGENTS.md                     # 人工与自动化协作规则
├── PROJECT.md                    # 本文件：结构与职责
├── CONTRIBUTING.md               # 贡献、提交与验证流程
├── SHARING.md                    # 公开复用、共享与第三方边界
├── LICENSE                       # 现有 MIT 许可证
├── SECURITY.md                   # 安全问题报告政策
├── CODE_OF_CONDUCT.md            # 社区行为准则
├── .gitignore                    # 秘密、运行数据及缓存排除
├── .gitattributes                # 文本与 LF 规则
├── .editorconfig                 # 编辑器默认格式
├── .gitmessage                   # 可选本地提交模板
├── .github/
│   ├── PULL_REQUEST_TEMPLATE.md
│   └── ISSUE_TEMPLATE/           # 问题、功能建议与安全政策入口
├── assets/readme/                # 项目首页静态素材
├── docs/                         # 需求、设计、PRD、计划与证据
├── <service>-start-local/        # 下表 17 个 Compose 服务目录
└── mediacrawler-start-local/     # 宿主机指南与补丁，不是 Compose 服务
```

| 当前 Compose 目录 | 主要职责 |
| --- | --- |
| `pgsql-start-local/` | PostgreSQL 与数据库初始化 |
| `mysql-start-lcoal/` | MySQL，保留历史目录名 |
| `redis-start-local/` | Redis |
| `minio-start-local/` | MinIO 对象存储 |
| `kafka-start-local/` | Kafka 与 Kafka UI |
| `canal-start-local/` | MySQL binlog 订阅 |
| `rabbitmq-start-lcoal/` | RabbitMQ，保留历史目录名 |
| `rocketmq-start-local/` | RocketMQ 与控制台 |
| `temporal-start-local/` | Temporal 与 UI，复用现有 PostgreSQL |
| `xxjob-start-local/` | XXL-Job，依赖 MySQL 与初始化 SQL |
| `nacos-start-local/` | 服务注册与配置 |
| `sentinel-start-local/` | 流量治理 |
| `seata-start-local/` | 分布式事务，依赖 MySQL |
| `elastic-start-local/` | Elasticsearch、IK、Logstash、Kibana |
| `monitoring-start-local/` | Prometheus、Grafana 与监控配置 |
| `rsshub-start-local/` | RSSHub |
| `searxng-start-local/` | SearXNG |

不为了目录整齐迁移现有服务到新的总目录；保留现有命令、挂载相对路径和历史名称的兼容性。端口、镜像、变量具体值以各服务 Compose 与 README 为准，本文不复制第二份配置表。

## 文件应放在哪里

| 内容 | 位置与维护边界 |
| --- | --- |
| 快速开始、服务选择 | 根 README；中英文同步 |
| 提交流程与质量要求 | `CONTRIBUTING.md`；`AGENTS.md` 引用并补充执行规则 |
| 结构规范 | `PROJECT.md`，实际新增/移动路径必须同步 |
| 许可、共享、安全与社区 | 根对应专用文件，不散落在服务目录中 |
| 服务 Compose、配置、SQL、脚本、补丁 | 当前服务目录，按已有命名；必要子目录如 `config/`、`scripts/`、`init/` 按需要创建 |
| 首页图片 | `assets/readme/`；不放含秘密或个人路径的截图 |
| 原始需求与确认记录 | `docs/requirement/` |
| 技术合同、状态机与接口 | `docs/design/` |
| 用户故事与验收 | `docs/prd/`，每份唯一来源 design |
| 实施步骤与验证任务 | `docs/plan/`，每份唯一来源 PRD |
| 有日期的调研与外部证据 | `docs/research/`；采纳的决策再写入 design |
| 文档模板 | `docs/templates/`；入口由各 `00-*导航.md` 维护 |

新服务最低布局如下；只创建确实需要的初始化和配置目录，不强制给所有服务加脚本。

```text
<service>-start-local/
├── docker-compose.yml
├── README.md
├── .env.example       # 使用环境变量时必需；不提交真实 .env
└── ...                # 当前服务专属配置、初始化或构建文件
```

## 未来工作箱布局（待实施）

以下路径来自当前 [底座计划](docs/plan/01-服务清单与蓝图同步实施计划.md)、[编排计划](docs/plan/02-跨平台Docker探测与编排引擎实施计划.md)、[前端计划](docs/plan/03-工作箱前端工作台与跨平台CI实施计划.md)。本次只制定规范，不创建这些目录、依赖或 CI。

```text
toolbox/                                      # 未来独立桌面工程
├── README.md
├── package.json / pnpm-lock.yaml
├── rust-toolchain.toml / .node-version
├── index.html / vite.config.ts / tsconfig*.json
├── catalog/
│   ├── services.json                         # 服务、依赖、场景与资源描述
│   └── images.lock.json                      # 按目标架构锁定镜像/归档来源
├── scripts/                                  # 蓝图同步、校验及其测试
├── src/
│   ├── main.tsx / App.tsx
│   ├── components/                           # 组件与相邻 __tests__
│   ├── views/                                # 页面与相邻 __tests__
│   ├── api/                                  # 类型化 Tauri 调用
│   └── types/generated/                      # 后端共享 DTO 生成结果
├── src-tauri/
│   ├── Cargo.toml / Cargo.lock / tauri.conf.json
│   ├── capabilities/                         # 最小权限配置
│   ├── src/                                  # 按下表拆分 Rust 模块
│   ├── resources/
│   │   ├── blueprints/                       # 只读来源同步的生成资源
│   │   ├── sql/                              # 有来源、许可及校验和的初始化
│   │   └── guides/                           # 离线指南、manifest 与最小示例
│   └── tests/                                # acceptance 与 fixtures/runtime
├── probe-clients/
│   ├── Dockerfile / pom.xml
│   └── modules/<protocol>/                   # 协议隔离的短时原生客户端
├── image-builds/
│   ├── minio/
│   ├── xxjob/
│   └── es-ik/                                # 制品配方、来源/工具/许可清单
└── tests/fixtures/                           # 蓝图、环境、更新等合成夹具

.github/workflows/toolbox-ci.yml              # 未来 CI，实施时再创建
```

| Rust 模块职责 | 计划模块 |
| --- | --- |
| 公共合同与入口 | `contracts`、`commands`、`lib`、`main` |
| 清单、工作区、秘密和蓝图 | `catalog`、`workspace`、`env_store`、`blueprint` |
| Docker、预检和制品 | `docker_cli`、`preflight`、`storage_probe`、`image_store` |
| 数据库绑定与初始化 | `bindings`、`database_claims`、`provisioner`、`initialization_store` |
| 探针与就绪 | `probes`、`probe_clients` |
| 操作、锁与跨工作区引用 | `operation_store`、`locks`、`usage_index` |
| 编排、配置应用、重置和诊断 | `orchestrator`、`config_apply`、`reset`、`diagnostics` |

前端只通过类型化 API 调用后端，不获得通用 Shell 或秘密对象。公共 DTO 从后端生成，不手工维护两套接口。测试夹具使用合成配置，不拷贝用户运行目录。精确接口、工具链和验收由对应 design/PRD/plan 维护。

## 源码、生成物与运行数据边界

- 根服务目录是现有 CLI 模板来源；未来同步脚本只读提取白名单文件，不反向修改原目录或打包真实 `.env`、数据、日志。
- `toolbox/src-tauri/resources/blueprints/` 是计划生成的随包资源；实施时通过同步/校验再生成，不能手改成另一份权威 Compose。是否纳入 Git 随构建方案明确，但必须可从同一来源复现。
- 现有 CLI 运行数据位置仍遵循各服务 README；未来工作箱默认工作区为 `~/.code-ark`，不得用源码目录作为默认运行目录。
- 未来公共控制目录分别为 macOS `~/Library/Application Support/CodeArk/control`、Windows `%LOCALAPPDATA%\CodeArk\control`、Linux `$XDG_DATA_HOME/codeark/control`（未设置时为 `~/.local/share/codeark/control`）。共享引用、锁和操作记录不是可提交源码。
- 构建输出、安装包、镜像归档、下载缓存和实际诊断包不提交 Git；未来发行制品放发行资产并记录校验和及许可。调研缓存 `.firecrawl/` 保持忽略。
- 本机 `.env`、数据与日志即使被忽略，也不能被同步、打包或共享脚本递归收集。共享白名单见 [SHARING.md](SHARING.md)。

## 结构变更规则

新增目录先确定归属，优先复用现有位置。跨服务脚本只有在多个真实调用方确有需要时才提出，不为单个服务增加根级脚本框架。目录移动必须说明路径、挂载和数据迁移影响，保留兼容或提供明确迁移方案。

结构变更同时更新本文件、相关导航/README；涉及工作箱设计时先同步 design，再同步其唯一 PRD 和 Plan。未来路径仍标为待实施，验证完成才改变状态。提交与验证流程见 [CONTRIBUTING.md](CONTRIBUTING.md)。
