# 代码方舟 · Code Ark：本地 Docker 开发环境工作箱

<p align="center">
  <img src="./assets/readme/hero.png" width="100%" alt="Code Ark：按需选择本地基础服务，通过 Docker Compose 启动。方舟承载模块化容器的概念插画。">
</p>

<p align="center">
  <a href="https://github.com/StephenQiu30/code-ark/stargazers"><img src="https://img.shields.io/github/stars/StephenQiu30/code-ark?style=flat-square&amp;color=407898" alt="GitHub Stars"></a>
  <a href="https://github.com/StephenQiu30/code-ark/commits/main"><img src="https://img.shields.io/github/last-commit/StephenQiu30/code-ark?style=flat-square&amp;color=407898" alt="最近提交"></a>
  <a href="https://docs.docker.com/compose/"><img src="https://img.shields.io/badge/Docker-Compose-407898?style=flat-square&amp;logo=docker&amp;logoColor=white" alt="Docker Compose"></a>
  <a href="./LICENSE"><img src="https://img.shields.io/badge/License-MIT-d78958?style=flat-square" alt="MIT License"></a>
</p>

**把搭建环境的时间，留给写代码。**

代码方舟（Code Ark）是一款面向本地开发的 **Docker 环境桌面工作箱**，使用 Tauri 2、Rust 和 React 构建。选择服务或场景后，由工作箱检查 Docker、端口和资源，处理依赖初始化，再用真实协议探针确认就绪。界面采用 shadcn Radix 与 Tailwind CSS，提供浅色、深色和跟随系统主题；使用工作箱无需注册 Code Ark 账号。

仓库同时提供 **17 套可独立使用的 Docker Compose 配置和 1 份宿主机工具指南**，覆盖数据库、缓存、消息队列、工作流、搜索、对象存储和监控，适合 Java、Go、Python、Node.js 开发者做本地开发、接口联调和学习。

[English](./README.en.md) · [获取与状态](#获取与状态) · [快速开始](#快速开始) · [服务目录](#服务目录) · [常见问题](./docs/faq.md) · [参与贡献](#参与贡献)

## 获取与状态

最新：[v0.1.0-alpha.1 发布说明](./docs/releases/v0.1.0-alpha.1.md)。

| 使用方式 | 入口 | 当前范围 |
| --- | --- | --- |
| Docker Compose | 下方[快速开始](#快速开始)与[服务索引](./services/README.md) | 可直接按各服务文档使用，无需构建桌面应用 |
| 桌面工作箱源码预览 | [toolbox/README.md](./toolbox/README.md) | 当前变更操作开放于 macOS Apple Silicon（arm64）；其他原生平台尚待验收 |
| 发行与下载 | [GitHub Releases](https://github.com/StephenQiu30/code-ark/releases) | [v0.1.0-alpha.1 源码预览](https://github.com/StephenQiu30/code-ark/releases/tag/v0.1.0-alpha.1)与 ARM64 服务镜像；尚无公开桌面安装包 |

截至 2026-11-13，**16 个预设在 macOS arm64 上通过两轮隔离的真实启动、协议探针和保留数据的停止验证**。完整 Elastic Stack 因现场可用内存不足 6 GiB 被预检拒绝，未计入通过；AMD64 镜像归档、其他原生系统与最终安装包仍待验证。范围与证据见[常见问题及项目事实](./docs/faq.md)、[桌面与依赖预设验证](./docs/research/08-桌面全局消息与预设启动验证.md)和[其余预设验证](./docs/research/09-其余预设真实启动验证.md)。

## 工作箱能帮你做什么

- **按需启动本地环境**：从服务或场景选择入口，先检查 Docker 目标、端口、内存与磁盘，再编排已选服务及其依赖。
- **处理依赖和初始化**：为受支持的数据库依赖链校验账号、专属数据库和初始化结果，减少手工拼接配置；已有配置不会因再次启动而直接覆盖。
- **确认后台真的可用**：使用数据库、消息与 HTTP/API 探针确认就绪，界面提供状态、连接信息、任务与日志入口。
- **保留数据与操作归属**：工作区数据在源码目录之外，默认位置为 `~/.code-ark`；通过实例身份、资源归属和操作记录保护启停与恢复，重置需要明确范围与确认。
- **边用边学**：内置离线指南和教学示例，服务模板仍可单独使用 Compose 管理。首次打开工作箱不会自动启动服务。

源码预览的具体能力和未验收项见[工作箱说明](./toolbox/README.md)；完整技术资料见[文档中心](./docs/00-文档导航.md)。

## 快速开始

先运行一个不需要 `.env` 的 Redis 示例。需要 **Docker Engine / Docker Desktop、Docker Compose v2 及以上版本，以及 Git**。

```bash
git clone https://github.com/StephenQiu30/code-ark.git
cd code-ark/services/redis-start-local
docker compose up -d
docker compose exec redis redis-cli ping
```

看到 `PONG`，就可以让本机应用连接 `localhost:6379`。用完运行 `docker compose stop`；Redis 的 AOF 数据保存在当前目录的 `data/` 中。

> 其他服务可能需要 `.env`、初始化 SQL 或已有数据库。每个目录的 README 都说明了自己的启动条件；无需同时启动整套环境。

## 服务目录

完整服务清单、默认端口与启动前置条件统一维护在 [services/README.md](./services/README.md)。数据库、缓存、消息、工作流、搜索、存储和监控均按目录选择，不需要启动整套环境。

```text
services/    # 17 套 Compose 配置与 1 份宿主机工具指南
```

已有克隆升级目录布局前，请先阅读 [迁移说明](./services/README.md#从旧根目录布局迁移)，保留本机配置和数据。

## 日常使用

在选定的服务目录中运行：

```bash
docker compose ps -a           # 容器状态，包括一次性初始化任务
docker compose logs -f        # 查看日志
docker compose stop           # 停止容器
docker compose up -d          # 再次启动
docker compose down           # 移除当前项目容器和网络
```

需要 `.env` 的服务，**首次**复制 `.env.example` 并填写自己的配置；已有 `.env` 时保留它。PowerShell 可使用 `Copy-Item .env.example .env`。密码、Token、API Key 不应提交到 Git。

<details>
<summary><strong>数据会保留吗？容器里的应用连接哪个地址？</strong></summary>

- `stop` 和普通 `down` 不删除持久化目录或命名卷。`down -v` 会删除本项目命名卷，不会删除宿主机绑定目录，也不会清空外部数据库。
- 宿主机应用使用发布端口；容器里的 `localhost` 指向容器自己，应按服务 README 使用服务名、容器端口或宿主机入口。
- Temporal 复用 `pgsql-start-local`，先启动 PostgreSQL。Canal 复用 MySQL，需启用 ROW binlog 并配置复制账号；Seata、XXL-Job 需要可访问的 MySQL 和对应表结构。
- Nacos 使用固定名称命名卷 `nacos-start-local-data`，由 Compose 创建，并未声明为外部卷；`down -v` 会请求删除该卷，清理前核对归属与数据。详情见其 README。
- Elastic Stack 建议通过 `./start.sh` 启动。脚本要求 Docker 虚拟机至少有 6 GB 内存，Windows 下使用 WSL / 兼容 Shell。

</details>

这些配置面向本地开发与学习。生产部署需结合实际环境补充认证、TLS、备份、高可用与容量规划；镜像、端口与依赖版本以各服务 Compose 和文档为准。

## 参与贡献

缺一个你常用的服务？欢迎 [提交需求](https://github.com/StephenQiu30/code-ark/issues/new) 或按 [贡献指南](./CONTRIBUTING.md) 添加模板。修复启动问题、校准文档和分享使用反馈同样有帮助；自动化协作约定见 [AGENTS.md](./AGENTS.md)。

新提交与 PR 标题统一使用 `type(scope): 描述`，scope 必填，例如 `docs(project): 完善项目结构规范`。目录职责见 [PROJECT.md](./PROJECT.md)，公开复用与第三方许可边界见 [SHARING.md](./SHARING.md)；另提供 [安全报告政策](./SECURITY.md) 和 [社区行为准则](./CODE_OF_CONDUCT.md)。

**如果 Code Ark 帮你省下了搭建环境的时间，欢迎 [给项目一个 Star](https://github.com/StephenQiu30/code-ark)。** 收藏这份工具箱，下次搭建新项目时可以直接回来找需要的服务。

## License

仓库原创代码、配置与文档采用 [MIT License](./LICENSE)。第三方软件、镜像及 MediaCrawler 的使用需分别遵循各自上游许可。
