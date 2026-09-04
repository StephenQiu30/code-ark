# Elastic Stack 本地开发环境

> [返回项目首页](../README.md) | [Back to root](../README.md)

## English Summary

This directory provides a local Elastic Stack for development with
Elasticsearch, Kibana, and Logstash.
Use it for local search, log ingestion, and dashboard validation.
Create your own `.env` from `.env.example` before the first run.

## 服务简介

该目录用于启动本地 Elastic Stack，包含：

- Elasticsearch
- Kibana
- Logstash

适合搜索联调、日志采集实验和本地可视化验证，不建议直接作为生产环境配置使用。

## 端口与访问入口

| 组件 | 端口 | 说明 |
| --- | --- | --- |
| Elasticsearch | `9200` | HTTP API |
| Kibana | `5601` | Web 控制台 |
| Logstash | `5044` | Beats / Logstash 输入 |
| Logstash | `5000` | TCP / UDP 输入 |
| Logstash | `9600` | Logstash 监控接口 |

## 前置条件

- 已安装 Docker 和 Docker Compose
- 首次启动前准备 `.env`

## 快速启动

```bash
cd elastic-start-local
cp .env.example .env
./start.sh
```

首次启动时，脚本会先完成 Elasticsearch 自定义镜像构建，再启动整套服务，避免
BuildKit 与正在运行的 Elasticsearch 争抢内存。Docker 虚拟机默认至少需要 6 GB
内存；如果还同时运行其他开发容器，建议在 Docker Desktop 中分配 8–10 GB。

需要重新构建 IK 插件镜像时使用：

```bash
./start.sh --build
```

显式重建会先平滑停止本目录下的 Elastic Stack，构建完成后再启动，数据卷不会被删除。

## 配置说明

当前目录通过 `.env` 驱动主要配置。建议从 `.env.example` 开始修改：

- 镜像版本：`ES_LOCAL_VERSION`、`LOGSTASH_LOCAL_VERSION`、`IK_VERSION`
- 端口：`ES_LOCAL_PORT`、`KIBANA_LOCAL_PORT`、`LOGSTASH_LOCAL_PORT`
- 凭据：`ES_LOCAL_PASSWORD`、`KIBANA_LOCAL_PASSWORD`
- Kibana 加密密钥：`KIBANA_ENCRYPTION_KEY`（至少 32 个字符，并在同一环境中保持稳定）
- JVM / Node.js 堆：`ES_LOCAL_JAVA_OPTS`、`KIBANA_LOCAL_NODE_OPTIONS`、`LOGSTASH_LOCAL_JAVA_OPTS`
- 容器资源边界：`ES_LOCAL_MEM_*`、`KIBANA_LOCAL_MEM_*`、`LOGSTASH_LOCAL_MEM_*`
- Docker 最低内存预检：`ELASTIC_STACK_MIN_DOCKER_MEMORY_GB`

补充说明：

- `start.sh` 会读取 `.env`，并在特定条件下尝试处理 Elastic 许可证逻辑。
- 建议使用 `./start.sh` 启动；它会执行内存预检，并确保构建与运行阶段分离。
- Elasticsearch、Kibana 和 Logstash 设置了内存边界及 `unless-stopped` 重启策略。
- 请不要把真实密码或 API key 提交到仓库。

## 数据持久化与清理

该目录使用 Docker named volumes 持久化数据：

- `dev-elasticsearch`
- `dev-kibana`
- `dev-logstash`

常见清理方式：

```bash
docker compose down
docker compose down -v
```

## 常用命令

```bash
./start.sh
./start.sh --build
docker compose logs -f
docker compose stop
docker compose down
docker compose down -v
./stop.sh
```

## 使用说明

- 应用连接 Elasticsearch 时，宿主机地址使用 `http://localhost:9200`
- Kibana 入口为 `http://localhost:5601`
- 如果修改了密码，请确保 Kibana 与 Elasticsearch 使用的密码保持一致

## 返回导航

- 项目首页：[`../README.md`](../README.md)
