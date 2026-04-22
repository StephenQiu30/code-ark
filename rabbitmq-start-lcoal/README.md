# RabbitMQ 本地开发环境

> [返回项目首页](../README.md) | [Back to root](../README.md)

## English Summary

This directory starts RabbitMQ with the management console enabled.
It uses `.env` based credentials and persists broker data locally.
Create `.env` from `.env.example` before startup.

## 服务简介

该目录提供带管理界面的 RabbitMQ，本地开发时可用于：

- AMQP 消息联调
- 队列、交换机、绑定关系验证
- 消费者和生产者测试

## 端口与访问入口

| 组件 | 端口 | 说明 |
| --- | --- | --- |
| AMQP | `5672` | 应用连接端口 |
| Management UI | `15672` | Web 管理界面 |

- 管理界面：`http://localhost:15672`

## 前置条件

- 已安装 Docker 和 Docker Compose
- 首次启动前准备 `.env`

## 快速启动

```bash
cd rabbitmq-start-lcoal
cp .env.example .env
docker compose up -d
```

## 配置说明

请在 `.env` 中设置：

- `RABBITMQ_DEFAULT_USER`
- `RABBITMQ_DEFAULT_PASS`

镜像版本固定为 `rabbitmq:4.2.3-management-alpine`。

## 数据持久化与清理

- 数据目录：`./rabbitmq-data`

```bash
docker compose down
docker compose down -v
rm -rf ./rabbitmq-data
```

## 常用命令

```bash
docker compose up -d
docker compose logs -f
docker compose stop
docker compose down
```

## 使用说明

- 宿主机应用连接 RabbitMQ 时使用 `localhost:5672`
- 浏览器登录管理界面时使用 `.env` 中的账号密码
- 当前目录名保留历史拼写 `rabbitmq-start-lcoal`

## 返回导航

- 项目首页：[`../README.md`](../README.md)
