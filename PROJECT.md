# 项目结构规范

本文件定义 Code Ark 的目录职责和新增文件位置。当前仓库是 **17 套 Docker Compose 配置 + 1 份宿主机工具指南 + 工作箱设计文档**；`toolbox/` 已创建并开始实施，桌面工作箱尚未完成或发行。下方目标结构按计划逐步落实，已实现范围见 [工作箱 README](toolbox/README.md)。

English Summary: Keep Compose templates together under `services/`. Put service-owned configuration and initialization beside that service. The planned desktop application belongs under `toolbox/`; runtime secrets, data, caches, and release binaries stay outside source control.

## 当前根目录

```text
code-ark/
├── README.md / README.en.md       # 中英文使用入口与服务索引
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
├── services/                     # 17 套 Compose 配置、1 份宿主机指南与统一索引
└── toolbox/                      # 桌面工作箱源码（实施中）
```

完整服务表唯一维护在 [services/README.md](services/README.md)，根 README 只提供快速开始与索引入口。所有服务都在 `services/` 下，保留 `mysql-start-lcoal`、`rabbitmq-start-lcoal` 的历史名称。

目录迁移保留各服务的内部相对路径与 Docker 项目/容器/网络/卷名；已有克隆的本机配置、数据和绑定路径按 [迁移说明](services/README.md#从旧根目录布局迁移) 处理。端口、镜像、变量以各服务 Compose 与 README 为准。

## 文件应放在哪里

| 内容 | 位置与维护边界 |
| --- | --- |
| 快速开始、服务索引入口 | 根 README；中英文同步 |
| 完整服务表、默认端口与目录迁移 | `services/README.md`，集中维护，根 README 和本文不复制清单 |
| 提交流程与质量要求 | `CONTRIBUTING.md`；`AGENTS.md` 引用并补充执行规则 |
| 结构规范 | `PROJECT.md`，实际新增/移动路径必须同步 |
| 许可、共享、安全与社区 | 根对应专用文件，不散落在服务目录中 |
| 服务 Compose、配置、SQL、脚本、补丁 | `services/<service>-start-local/`，按已有命名；必要子目录如 `config/`、`scripts/`、`init/` 按需要创建 |
| 首页图片 | `assets/readme/`；不放含秘密或个人路径的截图 |
| 原始需求与确认记录 | `docs/requirement/` |
| 技术合同、状态机与接口 | `docs/design/` |
| 用户故事与验收 | `docs/prd/`，每份唯一来源 design |
| 实施步骤与验证任务 | `docs/plan/`，每份唯一来源 PRD |
| 有日期的调研与外部证据 | `docs/research/`；采纳的决策再写入 design |
| 文档模板 | `docs/templates/`；入口由各 `00-*导航.md` 维护 |

新服务最低布局如下；只创建确实需要的初始化和配置目录，不强制给所有服务加脚本。

```text
services/<service>-start-local/
├── docker-compose.yml
├── README.md
├── .env.example       # 使用环境变量时必需；不提交真实 .env
└── ...                # 当前服务专属配置、初始化或构建文件
```

## 工作箱源码布局（实施中）

以下路径来自当前 [底座计划](docs/plan/01-服务清单与蓝图同步实施计划.md)、[编排计划](docs/plan/02-跨平台Docker探测与编排引擎实施计划.md)、[前端计划](docs/plan/03-工作箱前端工作台与跨平台CI实施计划.md)。工程已开始实施；下图是目标结构，精确已实现范围以 [toolbox/README.md](toolbox/README.md) 和对应计划执行记录为准。

```text
toolbox/                                      # 已创建的桌面工程
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
│   │   ├── guides/                           # 生成的离线指南与 manifest
│   │   ├── examples/                         # 生成的白名单教学源码
│   │   └── probe-clients/                    # 生成的锁定协议 JAR
│   ├── examples/                             # 受控本地验证入口
│   └── tests/acceptance.rs                    # 显式隔离配置的真实 Redis 验证
├── guides/                                   # 11 份离线指南与来源清单
├── examples/                                 # 13 个教学模块与独立 SDK 锁
├── probe-clients/
│   ├── Dockerfile / pom.xml
│   └── modules/<protocol>/                   # 协议隔离的短时原生客户端
├── image-builds/
│   ├── minio/
│   ├── xxjob/
│   └── es-ik/                                # 制品配方、来源/工具/许可清单
└── vitest.config.ts                          # 前端夹具和测试位于 src/

.github/workflows/toolbox-ci.yml              # 源码检查流程
.github/workflows/toolbox-native.yml          # 原生自托管候选构建，尚未验收
```

| Rust 模块职责 | 当前模块 |
| --- | --- |
| 公共合同与入口 | `contracts`、`desktop`、`lib`、`main` |
| 清单、工作区、秘密和蓝图 | `catalog`、`workspace`、`env_store`、`blueprint`、`managed_data`；Windows ACL 在 `windows_permissions` |
| Docker、预检和制品 | `docker_cli`、`preflight`、`storage_probe`、`image_store` |
| 数据库绑定与初始化 | `bindings`、`database_claims`、`provisioner`、`initialization_store`、`sql_provisioner`、`sql_schema` |
| 探针与就绪 | `probes`、`probe_clients`、`live_status` |
| 操作、锁与跨工作区引用 | `operation_store`、`locks`、`usage_index` |
| 编排、配置应用、重置和诊断 | `orchestrator`、`runtime_state`、`recovery`、`config_apply`、`reset`；诊断命令在 `desktop` |

前端只通过类型化 API 调用后端，不获得通用 Shell 或秘密对象。公共 DTO 从后端生成，不手工维护两套接口。测试夹具使用合成配置，不拷贝用户运行目录。精确接口、工具链和验收由对应 design/PRD/plan 维护。

## 源码、生成物与运行数据边界

- `services/` 下各服务目录是现有 CLI 模板来源；同步脚本只读提取白名单文件，不反向修改原目录或打包真实 `.env`、数据、日志。
- `toolbox/src-tauri/resources/blueprints/` 是忽略的随包生成资源；通过同步/校验再生成，不能手改成另一份权威 Compose，不纳入 Git，必须可从同一来源复现。
- 现有 CLI 运行数据位置仍遵循各服务 README；工作箱默认工作区为 `~/.code-ark`，不得用源码目录作为默认运行目录。
- 公共控制目录分别为 macOS `~/Library/Application Support/CodeArk/control`、Windows `%LOCALAPPDATA%\CodeArk\control`、Linux `$XDG_DATA_HOME/codeark/control`（未设置时为 `~/.local/share/codeark/control`）。共享引用、锁和操作记录不是可提交源码。
- 构建输出、安装包、镜像归档、下载缓存和实际诊断包不提交 Git；未来发行制品放发行资产并记录校验和及许可。调研缓存 `.firecrawl/` 保持忽略。
- 本机 `.env`、数据与日志即使被忽略，也不能被同步、打包或共享脚本递归收集。共享白名单见 [SHARING.md](SHARING.md)。

## 结构变更规则

新增目录先确定归属，优先复用现有位置。跨服务脚本只有在多个真实调用方确有需要时才提出，不为单个服务增加根级脚本框架。目录移动必须说明路径、挂载和数据迁移影响，保留兼容或提供明确迁移方案。

结构变更同时更新本文件、相关导航/README；涉及工作箱设计时先同步 design，再同步其唯一 PRD 和 Plan。未来路径仍标为待实施，验证完成才改变状态。提交与验证流程见 [CONTRIBUTING.md](CONTRIBUTING.md)。

工作区每个栈的 `.codeark-instance.json` 保存实际部署与恢复意图，`.managed-data.json` 记录已创建数据目录身份；`.runtime/<generation>/draft.env` 暂存配置，`effective.env` 记录已应用秘密快照，均不属于可分享源码。公共控制根下 `usage/usage.json` 维护共享引用。没有归属记录的非空目录不得自动采用或清理。当前实现与未验收范围见 [首阶段执行记录](docs/research/06-工作箱首阶段实施与验证记录.md)。
