# Kafka 本地开发环境

> [返回项目首页](../README.md) | [Back to root](../README.md)

## English Summary

This directory starts a local Kafka broker together with Kafka UI.
It is suitable for local messaging tests and stream integration work.
No `.env` file is required for the current setup.

## 服务简介

该目录提供一个单节点 Kafka 本地环境，包含：

- Kafka Broker
- Kafka UI

适合本地消息收发测试、消费者联调和主题观察。

## 端口与访问入口

| 组件 | 端口 | 说明 |
| --- | --- | --- |
| Kafka | `9092` | 宿主机访问地址 |
| Kafka UI | `19000` | Web 管理界面 |
| Kafka 内部监听 | `29092` | Docker 网络内访问地址 |

Kafka UI 地址：`http://localhost:19000`

## 前置条件

- 已安装 Docker 和 Docker Compose

## 快速启动

```bash
cd kafka-start-local
docker compose up -d
```

## 配置说明

当前配置直接写在 `docker-compose.yml` 中，未使用 `.env`。

关键约定：

- 宿主机应用连接地址：`localhost:9092`
- 同一 Docker 网络中的其他容器连接地址：`kafka:29092`
- 数据目录挂载到当前目录下的 `./data`

## 数据持久化与清理

- 数据目录：`./data`

```bash
docker compose down
docker compose down -v
rm -rf ./data
```

## 常用命令

```bash
docker compose up -d
docker compose ps
docker compose logs -f
docker compose stop
docker compose down
```

## 使用说明

- 如果宿主机应用无法消费消息，请优先检查是否连接到了 `localhost:9092`
- Kafka UI 适合查看主题、分区、消息和消费者组状态

## 返回导航

- 项目首页：[`../README.md`](../README.md)
