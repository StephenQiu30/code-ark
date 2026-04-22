# Seata 本地开发环境

> [返回项目首页](../README.md) | [Back to root](../README.md)

## English Summary

This directory provides a local Seata server for distributed transaction testing.
It depends on an external MySQL instance for metadata storage.
Create `.env` from `.env.example` and prepare the required database tables before startup.

## 服务简介

该目录提供一个本地 Seata Server，用于开发和验证分布式事务场景。

当前配置特点：

- Server 运行在 Docker 中
- 元数据存储模式为 `db`
- 依赖外部 MySQL

## 端口与访问入口

| 组件 | 端口 | 说明 |
| --- | --- | --- |
| Seata Console | `7091` | 管理控制台 |
| Seata Service | `8091` | 事务协调服务 |

- Console 账号：`seata / seata`

## 前置条件

- 已安装 Docker 和 Docker Compose
- 已准备可访问的 MySQL 实例
- 首次启动前准备 `.env`
- 目标 MySQL 中已导入 Seata 所需表结构

## 快速启动

```bash
cd seata-start-local
cp .env.example .env
docker compose up -d
```

## 配置说明

请在 `.env` 中设置：

- `MYSQL_HOST`
- `MYSQL_PORT`
- `MYSQL_DATABASE`
- `MYSQL_USERNAME`
- `MYSQL_PASSWORD`

当前目录还包含以下关键配置：

- `seata-config/application.yml`

其中已经指定：

- 控制台端口：`7091`
- 服务端口：`8091`
- 存储模式：`db`
- 需要的表：`global_table`、`branch_table`、`lock_table`、`distributed_lock`

## 数据持久化与清理

- 日志目录：`./seata/logs`
- 配置文件：`./seata-config/application.yml`

```bash
docker compose down
rm -rf ./seata/logs
```

## 常用命令

```bash
docker compose up -d
docker compose logs -f
docker compose stop
docker compose down
```

## 使用说明

- `MYSQL_HOST` 默认可使用 `host.docker.internal` 连接宿主机数据库
- 如果 Seata 无法启动，请优先检查 MySQL 连通性和元数据表是否已初始化
- 不要把真实数据库密码写进 README 或提交到仓库

## 返回导航

- 项目首页：[`../README.md`](../README.md)
