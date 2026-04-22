# Redis 本地开发环境

> [返回项目首页](../README.md) | [Back to root](../README.md)

## English Summary

This directory starts a local Redis instance with append-only persistence enabled.
It is suitable for local caching, message tests, and lightweight development scenarios.
The current setup does not require a `.env` file.

## 服务简介

该目录提供一个简洁的 Redis 本地环境。

当前特性：

- 使用官方 `redis:latest` 镜像
- 开启 AOF 持久化
- 未配置访问密码

## 端口与访问入口

| 组件 | 端口 | 说明 |
| --- | --- | --- |
| Redis | `6379` | 宿主机连接端口 |

## 前置条件

- 已安装 Docker 和 Docker Compose

## 快速启动

```bash
cd redis-start-local
docker compose up -d
```

## 配置说明

当前配置直接写在 `docker-compose.yml` 中：

- Redis 启动命令：`redis-server --appendonly yes`
- 未设置 `requirepass`
- 数据目录挂载到 `./data`

如果你希望增加密码认证或自定义参数，请直接修改 `docker-compose.yml`。

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
docker compose logs -f
docker compose stop
docker compose down
```

## 使用说明

- 宿主机连接地址：`localhost:6379`
- 当前配置更适合本地开发，不适合直接暴露到不受信任网络

## 返回导航

- 项目首页：[`../README.md`](../README.md)
