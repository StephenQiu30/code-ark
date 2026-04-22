# DevForge

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](./LICENSE)
[![Docker](https://img.shields.io/badge/Docker-Compose-blue.svg)](https://www.docker.com/)
[![Status](https://img.shields.io/badge/Use-Local%20Development-success.svg)](#)

DevForge 是一个面向本地开发与联调场景的 Docker Compose 配置集合。仓库把常见基础设施拆成独立目录，尽量做到开箱即用、端口清晰、配置可控，方便个人开发、团队入职和多项目并行调试。

## English Summary

DevForge is a collection of Docker Compose stacks for local development.
Each service lives in its own directory with focused documentation and reproducible startup steps.
These stacks are intended for development and integration testing, not production use.

## 特性

- 服务拆分清晰：每个基础设施一个目录，按需启动，不必一次性拉起全部依赖。
- 文档统一：根 README 负责导航，各服务目录 README 负责具体启动与配置说明。
- 配置可控：优先通过 `.env.example` 管理公开配置入口，避免在文档中传播真实凭据。
- 数据可预期：持久化目录或 Docker volume 都会在 README 中明确说明。
- 适合协作：新同学可以按统一步骤快速复现本地环境。

## 适用场景

- 本地开发依赖服务快速启动
- 后端联调和接口测试
- 中间件、数据库、监控组件的实验环境
- 团队统一开发环境模板

## 服务矩阵

> 说明：仓库中保留了少量历史目录命名，例如 `mysql-start-lcoal`、`rabbitmq-start-lcoal`。本轮只统一文档，不改目录名。

| 服务 | 目录 | 主要端口 | 说明 |
| --- | --- | --- | --- |
| Elasticsearch + Kibana + Logstash | [`elastic-start-local`](./elastic-start-local/README.md) | `9200`, `5601`, `5044` | 本地搜索与日志处理栈 |
| Kafka + Kafka UI | [`kafka-start-local`](./kafka-start-local/README.md) | `9092`, `19000` | 本地消息流平台 |
| MinIO | [`minio-start-local`](./minio-start-local/README.md) | `9000`, `9001` | S3 兼容对象存储 |
| Prometheus + Grafana | [`monitoring-start-local`](./monitoring-start-local/README.md) | `19090`, `13000` | 本地监控与可视化 |
| MySQL | [`mysql-start-lcoal`](./mysql-start-lcoal/README.md) | `3306` | 本地 MySQL 8 开发库 |
| Nacos | [`nacos-start-local`](./nacos-start-local/README.md) | `8840`, `8848`, `9848-9850` | 服务发现与配置中心 |
| PostgreSQL | [`pgsql-start-local`](./pgsql-start-local/README.md) | `5432` | 本地 PostgreSQL 16 |
| RabbitMQ | [`rabbitmq-start-lcoal`](./rabbitmq-start-lcoal/README.md) | `5672`, `15672` | AMQP 队列与管理台 |
| Redis | [`redis-start-local`](./redis-start-local/README.md) | `6379` | 开启 AOF 持久化的 Redis |
| RocketMQ | [`rocketmq-start-local`](./rocketmq-start-local/README.md) | `15876`, `15911`, `18180` | NameServer + Broker + Console |
| Seata | [`seata-start-local`](./seata-start-local/README.md) | `7091`, `8091` | 本地事务协调器 |
| Sentinel | [`sentinel-start-local`](./sentinel-start-local/README.md) | `8858`, `8719` | 流量防护控制台 |
| XXL-Job | [`xxjob-start-local`](./xxjob-start-local/README.md) | `18081` | 调度中心，依赖外部 MySQL |

## 快速开始

### 1. 准备 Docker 环境

```bash
docker --version
docker compose version
```

### 2. 克隆仓库

```bash
git clone <your-repo-url>
cd <repo-dir>
```

### 3. 选择一个服务并启动

以 PostgreSQL 为例：

```bash
cd pgsql-start-local
cp .env.example .env
docker compose up -d
```

如果某个目录提供了 `start.sh`，你也可以使用便捷脚本启动；但文档默认都优先给出标准 `docker compose` 命令。

## 配置方式

- 如果目录内存在 `.env.example`，推荐先执行 `cp .env.example .env`，再根据本地环境调整变量。
- 如果目录不需要 `.env`，README 会直接给出可运行命令。
- 不要把真实密码、token 或 API key 写进 README。
- 不要把个人环境中的真实 `.env` 提交到版本控制。

## 目录结构

```text
.
├── README.md
├── CONTRIBUTING.md
├── LICENSE
├── elastic-start-local/
├── kafka-start-local/
├── minio-start-local/
├── monitoring-start-local/
├── mysql-start-lcoal/
├── nacos-start-local/
├── pgsql-start-local/
├── rabbitmq-start-lcoal/
├── redis-start-local/
├── rocketmq-start-local/
├── seata-start-local/
├── sentinel-start-local/
└── xxjob-start-local/
```

## 服务导航

- [Elastic Stack](./elastic-start-local/README.md)
- [Kafka](./kafka-start-local/README.md)
- [MinIO](./minio-start-local/README.md)
- [Monitoring](./monitoring-start-local/README.md)
- [MySQL](./mysql-start-lcoal/README.md)
- [Nacos](./nacos-start-local/README.md)
- [PostgreSQL](./pgsql-start-local/README.md)
- [RabbitMQ](./rabbitmq-start-lcoal/README.md)
- [Redis](./redis-start-local/README.md)
- [RocketMQ](./rocketmq-start-local/README.md)
- [Seata](./seata-start-local/README.md)
- [Sentinel](./sentinel-start-local/README.md)
- [XXL-Job](./xxjob-start-local/README.md)

## 常见使用方式

在任意服务目录中，都可以使用以下命令：

```bash
docker compose up -d
docker compose logs -f
docker compose stop
docker compose down
docker compose down -v
```

说明：

- `down -v` 会删除对应 Docker volume。
- 使用本地目录持久化的服务，可能还需要手动删除数据目录后再重建。
- 具体数据目录和清理方式，请查看对应服务 README。

## 开源协作

欢迎提交新的服务目录、文档优化和问题修复。

- 提交规范、目录约定和 README 要求请查看 [CONTRIBUTING.md](./CONTRIBUTING.md)
- 提交前请确认端口、脚本名和配置说明与实际文件一致
- 如果新增依赖环境变量，请同时提供 `.env.example`

## License

本项目使用 [MIT License](./LICENSE)。
