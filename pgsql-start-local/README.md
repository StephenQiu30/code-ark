# PostgreSQL 本地开发环境

启动 PostgreSQL 18，联调 SQL、应用接口与初始化脚本；也可作为本仓库 Temporal 的共享数据库。

[项目首页](../README.md) · [快速启动](#快速启动) · [连接入口](#端口与访问入口) · [配置说明](#配置说明)

<details>
<summary>English summary</summary>

This directory starts a local PostgreSQL 18 instance for development and testing.
It uses `.env` based configuration and supports optional initialization scripts.
Create `.env` from `.env.example` before startup.

</details>

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

- 镜像版本固定为 `postgres:18`
- 初始化脚本目录：`./pgsql-init`
- 健康检查会使用 `.env` 中的用户名和数据库名

## 数据持久化与清理

- 数据目录：`./pgsql-data`
- 初始化目录：`./pgsql-init`

```bash
docker compose down
```

普通 `down` 保留绑定目录。需要完全重置时，先备份，再手动删除 `./pgsql-data` 中的对应数据。

## 常用命令

```bash
docker compose up -d
docker compose ps
docker compose logs -f
docker compose stop
docker compose down
```

## 使用说明

- 数据目录已初始化后，修改 `.env` 中的用户名、密码或数据库名不会自动修改已有数据库；连接时使用实际已存在的账号，不要通过清空共享数据来修复凭据问题。
- 宿主机连接地址：`localhost:5432`
- 入口初始化 SQL 仅在空数据目录首次启动时执行；已有数据库需按实际变更执行 SQL，重置整个数据目录会影响其全部数据库。
