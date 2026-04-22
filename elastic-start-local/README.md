# Elastic Stack 本地开发环境

> [返回项目首页](../README.md) | [Back to root](../README.md)

## English Summary

This directory provides a local Elastic Stack for development with Elasticsearch, Kibana, and Logstash.
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
docker compose up -d
```

如果你希望沿用目录内脚本，也可以执行：

```bash
./start.sh
```

## 配置说明

当前目录通过 `.env` 驱动主要配置。建议从 `.env.example` 开始修改：

- 镜像版本：`ES_LOCAL_VERSION`、`LOGSTASH_LOCAL_VERSION`、`IK_VERSION`
- 端口：`ES_LOCAL_PORT`、`KIBANA_LOCAL_PORT`、`LOGSTASH_LOCAL_PORT`
- 凭据：`ES_LOCAL_PASSWORD`、`KIBANA_LOCAL_PASSWORD`
- 资源配置：`ES_LOCAL_JAVA_OPTS`、`LOGSTASH_LOCAL_JAVA_OPTS`

补充说明：

- `start.sh` 会读取 `.env`，并在特定条件下尝试处理 Elastic 许可证逻辑。
- 如果你只需要快速验证容器编排，优先使用 `docker compose up -d`。
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
docker compose up -d
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
