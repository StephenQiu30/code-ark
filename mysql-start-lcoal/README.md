# MySQL 本地开发环境

启动 MySQL 8.0，使用独立数据目录和可选初始化 SQL 联调关系型数据与应用接口。

[项目首页](../README.md) · [快速启动](#快速启动) · [连接入口](#端口与访问入口) · [配置说明](#配置说明)

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

```bash
cd mysql-start-lcoal
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

- 镜像版本固定为 `mysql:8.0`
- 时区通过 `.env` 中的 `TZ` 设置，示例为 `Asia/Shanghai`
- 启动参数中已启用 `mysql_native_password`
- 初始化 SQL 可以放到 `./mysql-init`

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
