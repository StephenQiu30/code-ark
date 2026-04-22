# RocketMQ 本地开发环境

> [返回项目首页](../README.md) | [Back to root](../README.md)

## English Summary

This directory provides a local RocketMQ stack with NameServer, Broker, and Console.
It is suitable for local messaging tests and integration debugging.
The current setup does not require a `.env` file.

## 服务简介

该目录提供一个本地 RocketMQ 单机环境，包含：

- NameServer
- Broker
- Console

适合本地消息发送、消费验证和 Topic 管理。

## 端口与访问入口

| 组件 | 端口 | 说明 |
| --- | --- | --- |
| NameServer | `15876` | 映射到容器 `9876` |
| Broker | `15909` | Broker 内部通信 |
| Broker | `15911` | 消息收发主端口 |
| Broker | `15912` | HA 端口 |
| Console | `18180` | Web 控制台 |

- Console：`http://localhost:18180`

## 前置条件

- 已安装 Docker 和 Docker Compose

## 快速启动

```bash
cd rocketmq-start-local
docker compose up -d
```

如果你希望使用目录内脚本，也可以执行：

```bash
./start.sh
```

## 配置说明

当前目录的关键配置文件：

- `docker-compose.yml`
- `conf/broker.conf`

当前编排中固定了以下行为：

- Broker 通过 `namesrv:9876` 连接 NameServer
- 数据挂载到 `./data/broker`
- Console 默认连接同一网络中的 NameServer

## 数据持久化与清理

- 日志目录：`./data/broker/logs`
- 存储目录：`./data/broker/store`

```bash
docker compose down
docker compose down -v
rm -rf ./data/broker
```

## 常用命令

```bash
docker compose up -d
docker compose ps
docker compose logs -f
docker compose stop
docker compose down
./stop.sh
```

## 使用说明

- 宿主机应用连接 NameServer 时使用 `localhost:15876`
- Docker 网络内的容器应使用 `namesrv:9876`
- 如果你要调整 Broker 参数，请修改 `conf/broker.conf`

## 返回导航

- 项目首页：[`../README.md`](../README.md)
