# PostgreSQL 本地开发环境

> [返回项目首页](../README.md) | [Back to root](../README.md)

## English Summary

This directory starts a local PostgreSQL 16 instance for development and testing.
It uses `.env` based configuration and supports optional initialization scripts.
Create `.env` from `.env.example` before startup.

## 服务简介

该目录提供一个本地 PostgreSQL 开发环境，适合：

- 本地接口开发
- SQL 调试
- 集成测试

## 端口与访问入口

| 组件 | 端口 | 说明 |
| --- | --- | --- |
| PostgreSQL | `5432` | 数据库连接端口 |

## 前置条件

- 已安装 Docker 和 Docker Compose
- 首次启动前准备 `.env`

## 快速启动

```bash
cd pgsql-start-local
cp .env.example .env
docker compose up -d
```

## 配置说明

请在 `.env` 中设置：

- `PGSQL_DATABASE`
- `PGSQL_USER`
- `PGSQL_PASSWORD`

其他说明：

- 镜像版本固定为 `postgres:16`
- 初始化脚本目录：`./pgsql-init`
- 健康检查会使用 `.env` 中的用户名和数据库名

## 数据持久化与清理

- 数据目录：`./pgsql-data`
- 初始化目录：`./pgsql-init`

```bash
docker compose down
docker compose down -v
rm -rf ./pgsql-data
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

- 宿主机连接地址：`localhost:5432`
- 如果你需要重新执行初始化 SQL，通常需要先清空 `./pgsql-data`

## 返回导航

- 项目首页：[`../README.md`](../README.md)
