# MySQL 本地开发环境

启动 MySQL 8.4，使用独立数据目录和可选初始化 SQL 联调关系型数据与应用接口。

[项目首页](../../README.md) · [服务索引](../README.md) · [快速启动](#快速启动) · [连接入口](#端口与访问入口) · [配置说明](#配置说明)

<details>
<summary>English summary</summary>

This directory starts a local MySQL 8 instance for development.
It supports `.env` based configuration and optional initialization scripts.
Create `.env` from `.env.example` before startup.

</details>

## 服务简介

该目录提供一个本地 MySQL 8 开发环境，适合：

- 本地接口开发
- 数据库结构联调
- 初始化脚本验证

## 端口与访问入口

| 组件 | 端口 | 说明 |
| --- | --- | --- |
| MySQL | `3306` | 数据库连接端口 |

## 前置条件

- 已安装 Docker 和 Docker Compose
- 首次启动前准备 `.env`

## 快速启动

以下进入目录的命令从仓库根目录执行；已有 `.env` 时保留原文件。

```bash
cd services/mysql-start-lcoal
cp .env.example .env
docker compose up -d
```

## 配置说明

请在 `.env` 中设置：

- `MYSQL_ROOT_PASSWORD`
- `MYSQL_DATABASE`
- `MYSQL_USER`
- `MYSQL_PASSWORD`

其他说明：

- 镜像版本固定为 `mysql:8.4.13`
- 从 8.0 数据目录升级到 8.4 时首次启动会自动升级元数据；降级不受支持，升级前请备份 `./mysql-data`
- 时区通过 `.env` 中的 `TZ` 设置，示例为 `Asia/Shanghai`
- 启动参数中已启用 `mysql_native_password`
- 已启用 `server-id=1`、ROW 格式 binlog 和 FULL 行镜像，供 Canal 等本地订阅服务使用；需重建 MySQL 容器以应用配置，现有数据目录会保留
- 初始化 SQL 可以放到 `./mysql-init`，该目录以只读方式挂载
- 容器带 `restart: unless-stopped`；首次初始化失败后自动重启会因数据目录已存在而跳过初始化脚本，首次启动请查看日志，失败时删除 `./mysql-data` 后重建

## 数据持久化与清理

- 数据目录：`./mysql-data`
- 初始化目录：`./mysql-init`

```bash
docker compose down
```

普通 `down` 保留绑定目录。需要完全重置时，先备份，再手动删除 `./mysql-data` 中的对应数据。

## 常用命令

```bash
docker compose up -d
docker compose logs -f
docker compose stop
docker compose down
```

## 使用说明

- 宿主机连接地址：`localhost:3306`
- 如果需要重新执行初始化脚本，通常需要先停止容器并清空 `./mysql-data`
- 当前目录名保留历史拼写 `mysql-start-lcoal`
