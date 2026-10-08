# Code Ark · 代码方舟

<p align="center">
  <img src="./assets/readme/hero.png" width="100%" alt="Code Ark：按需选择本地基础服务，通过 Docker Compose 启动。方舟承载模块化容器的概念插画。">
</p>

<p align="center">
  <a href="https://github.com/StephenQiu30/code-ark/stargazers"><img src="https://img.shields.io/github/stars/StephenQiu30/code-ark?style=flat-square&amp;color=407898" alt="GitHub Stars"></a>
  <a href="https://github.com/StephenQiu30/code-ark/commits/main"><img src="https://img.shields.io/github/last-commit/StephenQiu30/code-ark?style=flat-square&amp;color=407898" alt="最近提交"></a>
  <a href="https://docs.docker.com/compose/"><img src="https://img.shields.io/badge/Docker-Compose-407898?style=flat-square&amp;logo=docker&amp;logoColor=white" alt="Docker Compose"></a>
  <a href="./LICENSE"><img src="https://img.shields.io/badge/License-MIT-d78958?style=flat-square" alt="MIT License"></a>
</p>

**把搭建中间件的时间，留给写代码。**

Code Ark（代码方舟）收集了 **16 套 Docker Compose 配置**，覆盖数据库、缓存、消息队列、工作流、搜索、对象存储和监控。适合 Java、Go、Python、Node.js 开发者进行本地开发、接口联调和集成测试：选择需要的服务目录，按对应说明启动。

[English](./README.en.md) · [快速开始](#快速开始) · [服务目录](#服务目录) · [日常使用](#日常使用) · [参与贡献](#参与贡献)

## 快速开始

先运行一个不需要 `.env` 的 Redis 示例。需要 **Docker Engine / Docker Desktop、Docker Compose v2 及以上版本，以及 Git**。

```bash
git clone https://github.com/StephenQiu30/code-ark.git
cd code-ark/redis-start-local
docker compose up -d
docker compose exec redis redis-cli ping
```

看到 `PONG`，就可以让本机应用连接 `localhost:6379`。用完运行 `docker compose stop`；Redis 的 AOF 数据保存在当前目录的 `data/` 中。

> 其他服务可能需要 `.env`、初始化 SQL 或已有数据库。每个目录的 README 都说明了自己的启动条件；无需同时启动整套环境。

## 选一个适合你的入口

- **后端接口开发** → [PostgreSQL](./pgsql-start-local/README.md)、[MySQL](./mysql-start-lcoal/README.md)、[Redis](./redis-start-local/README.md)、[MinIO](./minio-start-local/README.md)。
- **消息与异步任务** → [Kafka](./kafka-start-local/README.md)、[RabbitMQ](./rabbitmq-start-lcoal/README.md)、[Temporal](./temporal-start-local/README.md)。
- **服务发现与分布式应用** → [Nacos](./nacos-start-local/README.md)、[Sentinel](./sentinel-start-local/README.md)、[Seata](./seata-start-local/README.md)。
- **搜索、日志与指标** → [Elastic Stack](./elastic-start-local/README.md)、[Prometheus / Grafana](./monitoring-start-local/README.md)。
- **RSS 与搜索 API** → [RSSHub](./rsshub-start-local/README.md)、[SearXNG](./searxng-start-local/README.md)。

## 为什么放在同一个仓库

**按目录选择服务。** 每个服务有自己的 Compose 和说明文件，方便按应用需求组合；需要复用数据库的服务会明确列出依赖。

**配置与数据位置可见。** 需要环境变量的目录提供 `.env.example`，文档说明连接端口、初始化、数据目录或命名卷，便于团队复用。

**沿用熟悉的 Docker 命令。** 多数服务使用 `docker compose up -d` 启动；Elastic Stack 提供构建和内存预检脚本，复杂服务的步骤留在各自文档中。

## 服务目录

下面列出默认宿主机端口；点击服务名查看凭据、资源需求和启动步骤。端口有冲突时，按对应服务配置调整。

### 数据与存储

| 服务 | 默认端口 | 用途 |
| --- | --- | --- |
| [PostgreSQL 18](./pgsql-start-local/README.md) | `5432` | SQL、关系型数据、初始化脚本 |
| [MySQL 8.0](./mysql-start-lcoal/README.md) | `3306` | 关系型数据与应用联调 |
| [Redis](./redis-start-local/README.md) | `6379` | 缓存、锁与消息实验；AOF 持久化 |
| [MinIO](./minio-start-local/README.md) | `9000` / `9001` | S3 兼容接口与管理控制台 |

### 消息、工作流与调度

| 服务 | 默认端口 | 用途 |
| --- | --- | --- |
| [Kafka + Kafka UI](./kafka-start-local/README.md) | `9092` / `19000` | 事件流、主题与消费者组观察 |
| [RabbitMQ](./rabbitmq-start-lcoal/README.md) | `5672` / `15672` | AMQP 消息与管理控制台 |
| [RocketMQ](./rocketmq-start-local/README.md) | `15876` / `15909` / `15911` / `15912` / `18180` | NameServer、Broker 与 Console |
| [Temporal + UI](./temporal-start-local/README.md) | `7233` / `18080` | 持久化工作流；复用本仓库 PostgreSQL |
| [XXL-Job](./xxjob-start-local/README.md) | `18081` | 调度中心；需 MySQL 与初始化 SQL |

### 治理、搜索与观测

| 服务 | 默认端口 | 用途 |
| --- | --- | --- |
| [Nacos](./nacos-start-local/README.md) | `8840` / `8848` / `9848–9850` | 注册与配置中心；先创建外部数据卷 |
| [Sentinel](./sentinel-start-local/README.md) | `8858` / `8719` | 流控、熔断与规则实验 |
| [Seata](./seata-start-local/README.md) | `7091` / `8091` | 分布式事务；需 MySQL 元数据表 |
| [Elastic Stack](./elastic-start-local/README.md) | `9200` / `5601` / `5044` | 搜索、IK 分词、日志与 Kibana |
| [Prometheus + Grafana](./monitoring-start-local/README.md) | `19090` / `13000` | 指标采集、告警与仪表盘 |

Logstash 还发布 `5000`（TCP / UDP）和 `9600`，详见服务文档。

### 内容与搜索 API

| 服务 | 默认端口 | 用途 |
| --- | --- | --- |
| [RSSHub](./rsshub-start-local/README.md) | `1200` | RSS 路由；Chromium bundled 镜像 |
| [SearXNG](./searxng-start-local/README.md) | `8888` | 聚合搜索与 JSON API |

另有 [MediaCrawler 固定版本与补丁说明](./mediacrawler-start-local/README.md)，用于宿主机运行的采集工具，不提供 Compose 服务。历史目录名 `mysql-start-lcoal`、`rabbitmq-start-lcoal` 保留以兼容已有使用方式。

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
- Temporal 复用 `pgsql-start-local`，先启动 PostgreSQL。Seata、XXL-Job 需要可访问的 MySQL 和对应表结构。
- Nacos 使用外部命名卷，首次运行 `docker volume create nacos-start-local-data`；详情见其 README。
- Elastic Stack 建议通过 `./start.sh` 启动。脚本要求 Docker 虚拟机至少有 6 GB 内存，Windows 下使用 WSL / 兼容 Shell。

</details>

这些配置面向本地开发与学习。生产部署需结合实际环境补充认证、TLS、备份、高可用与容量规划；部分目录使用 `latest`，团队复现前可自行固定兼容版本。

## 参与贡献

缺一个你常用的服务？欢迎 [提交需求](https://github.com/StephenQiu30/code-ark/issues/new) 或按 [贡献指南](./CONTRIBUTING.md) 添加模板。修复启动问题、校准文档和分享使用反馈同样有帮助；自动化协作约定见 [AGENTS.md](./AGENTS.md)。

**如果 Code Ark 帮你省下了搭建环境的时间，欢迎 [给项目一个 Star](https://github.com/StephenQiu30/code-ark)。** 收藏这份工具箱，下次搭建新项目时可以直接回来找需要的服务。

## License

仓库配置与文档采用 [MIT License](./LICENSE)。第三方软件、镜像及 MediaCrawler 的使用需分别遵循各自上游许可。
