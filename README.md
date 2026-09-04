# Code Ark（代码方舟）：Docker Compose 本地开发环境一键启动

[![GitHub stars](https://img.shields.io/github/stars/StephenQiu30/code-ark?style=flat-square&logo=github)](https://github.com/StephenQiu30/code-ark)
[![GitHub last commit](https://img.shields.io/github/last-commit/StephenQiu30/code-ark?style=flat-square)](https://github.com/StephenQiu30/code-ark/commits/main)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg?style=flat-square)](./LICENSE)
[![Docker Compose](https://img.shields.io/badge/Docker-Compose-2496ED?style=flat-square&logo=docker&logoColor=white)](https://docs.docker.com/compose/)

**Code Ark（代码方舟，原 DevForge）** 是一个开源的 Docker Compose
本地开发环境合集。无需手动安装中间件，即可按需启动 Elasticsearch、Kibana、
Logstash、Kafka、MySQL、PostgreSQL、Redis、RabbitMQ、MinIO、Nacos、RocketMQ、
Seata、Sentinel、Prometheus、Grafana 和 XXL-Job 等常用服务。

适用于 Java、Go、Python、Node.js 等项目的本地开发、接口联调、集成测试和团队环境初始化。

[English README](./README.en.md) · [服务列表](#支持的-docker-服务) · [快速开始](#快速开始) · [贡献指南](./CONTRIBUTING.md)

## 为什么选择 Code Ark

- **开箱即用**：每个服务都提供可直接运行的 `docker-compose.yml`。
- **按需启动**：中间件彼此独立，不必一次运行整套基础设施。
- **配置清晰**：通过 `.env.example` 管理版本、端口、凭据和资源参数。
- **数据可控**：每个目录都说明数据持久化、停止和清理方式。
- **跨平台**：适用于 Linux、macOS Docker Desktop 和 Windows WSL 2。
- **适合协作**：统一的目录和文档结构可帮助团队快速搭建开发环境。

> 本仓库面向本地开发与测试，不建议直接作为生产环境配置使用。

## 支持的 Docker 服务

Code Ark 覆盖数据库、消息队列、搜索与日志、对象存储、服务治理、分布式事务、任务调度、监控和文档处理等常见开发基础设施。

| 分类 | 服务 | 目录 | 默认端口 | 用途 |
| --- | --- | --- | --- | --- |
| 搜索与日志 | Elasticsearch + Kibana + Logstash | [`elastic-start-local`](./elastic-start-local/README.md) | `9200`, `5601`, `5044` | 全文搜索、日志采集与可视化 |
| 消息队列 | Kafka + Kafka UI | [`kafka-start-local`](./kafka-start-local/README.md) | `9092`, `19000` | 消息流与事件驱动开发 |
| 文档处理 | LibreOffice | [`libreoffice-start-local`](./libreoffice-start-local/README.md) | 无 | Headless 文档格式转换 |
| 对象存储 | MinIO | [`minio-start-local`](./minio-start-local/README.md) | `9000`, `9001` | S3 兼容对象存储 |
| 可观测性 | Prometheus + Grafana | [`monitoring-start-local`](./monitoring-start-local/README.md) | `19090`, `13000` | 指标采集、告警与仪表盘 |
| 数据库 | MySQL 8 | [`mysql-start-lcoal`](./mysql-start-lcoal/README.md) | `3306` | 关系型数据库 |
| 服务治理 | Nacos | [`nacos-start-local`](./nacos-start-local/README.md) | `8840`, `8848`, `9848-9850` | 注册中心与配置中心 |
| AI / OCR | PaddleOCR | [`ocr-start-local`](./ocr-start-local/README.md) | `18868` | 本地图片文字识别 API |
| 数据库 | PostgreSQL | [`pgsql-start-local`](./pgsql-start-local/README.md) | `5432` | 关系型数据库 |
| 消息队列 | RabbitMQ | [`rabbitmq-start-lcoal`](./rabbitmq-start-lcoal/README.md) | `5672`, `15672` | AMQP 消息队列与管理台 |
| 缓存 | Redis | [`redis-start-local`](./redis-start-local/README.md) | `6379` | 缓存、分布式锁与消息 |
| 消息队列 | RocketMQ | [`rocketmq-start-local`](./rocketmq-start-local/README.md) | `15876`, `15911`, `18180` | NameServer、Broker 与 Console |
| 分布式事务 | Seata | [`seata-start-local`](./seata-start-local/README.md) | `7091`, `8091` | 分布式事务协调 |
| 服务治理 | Sentinel | [`sentinel-start-local`](./sentinel-start-local/README.md) | `8858`, `8719` | 流量控制、熔断和限流 |
| 任务调度 | XXL-Job | [`xxjob-start-local`](./xxjob-start-local/README.md) | `18081` | 分布式任务调度中心 |

> 为保持兼容性，`mysql-start-lcoal` 和 `rabbitmq-start-lcoal` 暂时保留了历史目录拼写。

## 快速开始

### 环境要求

- Docker Engine 或 Docker Desktop
- Docker Compose v2（使用 `docker compose` 命令）
- Git

检查环境：

```bash
docker --version
docker compose version
```

### 克隆 Code Ark

```bash
git clone https://github.com/StephenQiu30/code-ark.git
cd code-ark
```

### 启动一个本地服务

以 PostgreSQL 为例：

```bash
cd pgsql-start-local
cp .env.example .env
docker compose up -d
docker compose ps
```

启动带有 Elasticsearch、Kibana 和 Logstash 的 Elastic Stack：

```bash
cd elastic-start-local
cp .env.example .env
./start.sh
```

每个服务目录都包含独立 README，请在启动前查看相应的端口、密码、内存和数据卷说明。

## 常用 Docker Compose 命令

在目标服务目录中执行：

```bash
# 后台启动
docker compose up -d

# 查看容器状态和日志
docker compose ps
docker compose logs -f

# 停止或移除容器
docker compose stop
docker compose down

# 同时删除命名数据卷（会丢失本地数据）
docker compose down -v
```

## 配置与安全

1. 将服务目录中的 `.env.example` 复制为 `.env`。
2. 按需修改镜像版本、端口、用户名、密码和内存限制。
3. 不要把包含真实密码、Token 或 API Key 的 `.env` 提交到版本控制。
4. 如果修改了默认端口，请同步修改应用程序的连接地址。

## 项目结构

```text
code-ark/
├── README.md                   # 中文项目首页与服务导航
├── README.en.md                # English documentation
├── CONTRIBUTING.md             # 贡献指南
├── elastic-start-local/        # Elasticsearch、Kibana、Logstash
├── kafka-start-local/          # Kafka 与 Kafka UI
├── minio-start-local/          # MinIO 对象存储
├── monitoring-start-local/     # Prometheus 与 Grafana
├── mysql-start-lcoal/          # MySQL
├── nacos-start-local/          # Nacos
├── pgsql-start-local/          # PostgreSQL
├── rabbitmq-start-lcoal/       # RabbitMQ
├── redis-start-local/          # Redis
├── rocketmq-start-local/       # RocketMQ
├── seata-start-local/          # Seata
├── sentinel-start-local/       # Sentinel
└── xxjob-start-local/          # XXL-Job
```

## 常见问题

### 可以只启动一个中间件吗？

可以。每个目录都是独立的 Docker Compose 项目，只会启动当前目录定义的服务。

### 如何快速搭建本地 Elasticsearch 和 Kibana？

进入 `elastic-start-local`，复制 `.env.example` 后执行 `./start.sh`。
脚本会检查 Docker 内存、构建 IK 分词插件并等待服务健康。

### 数据在容器停止后会丢失吗？

普通的 `docker compose stop` 或 `docker compose down` 不会删除命名卷。
执行 `docker compose down -v` 会删除对应数据卷，请谨慎使用。

### 这些配置可以直接用于生产环境吗？

不建议。本仓库优先考虑本地开发的易用性。生产环境还需要完善的密钥管理、备份、高可用、TLS、访问控制、监控和容量规划。

## 参与贡献

欢迎提交新的 Docker Compose 开发环境、文档改进和问题修复。开始前请阅读 [CONTRIBUTING.md](./CONTRIBUTING.md)。

如果 Code Ark 对你有帮助，欢迎为仓库点一个 Star，方便更多开发者找到它。

## License

Code Ark 使用 [MIT License](./LICENSE) 开源。
