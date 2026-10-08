# Nacos 本地开发环境

启动单机 Nacos 3.1.1，在本地验证服务注册、配置管理和客户端接入。首次运行需准备外部数据卷。

[项目首页](../README.md) · [快速启动](#快速启动) · [连接入口](#端口与访问入口) · [配置说明](#配置说明)

<details>
<summary>English summary</summary>

This directory provides a standalone Nacos server for local service discovery and config testing.
It uses `.env` for token-related configuration, a Docker named volume for database data, and a local directory for logs.
Create `.env` from `.env.example` before startup.

</details>

## 服务简介

该目录提供单机模式 Nacos，适合本地开发时验证：

- 服务注册与发现
- 配置管理
- Nacos 客户端接入

## 端口与访问入口

| 组件 | 端口 | 说明 |
| --- | --- | --- |
| Nacos Console | `8840` | Web 控制台映射到容器 `8080` |
| Nacos Main Port | `8848` | 主服务端口 |
| Nacos gRPC | `9848` | 客户端通信 |
| Nacos gRPC | `9849` | 客户端通信 |
| Nacos gRPC | `9850` | 客户端通信 |

- 控制台地址：`http://localhost:8840/`
- 默认控制台账号：`nacos / nacos`

## 前置条件

- 已安装 Docker 和 Docker Compose
- 首次启动前准备 `.env`，使用有效的 Base64 编码认证密钥
- 数据卷 `nacos-start-local-data` 是外部卷，首次运行需先创建

## 快速启动

```bash
cd nacos-start-local
cp .env.example .env
# 编辑 .env，设置 NACOS_AUTH_TOKEN
docker volume create nacos-start-local-data
docker compose up -d
```

## 配置说明

请在 `.env` 中设置：

- `NACOS_AUTH_TOKEN`

当前 `docker-compose.yml` 已固定以下行为：

- 单机模式启动：`MODE=standalone`
- 认证开启：`NACOS_AUTH_ENABLE=true`
- 控制台用户名与密码：`nacos / nacos`

## 数据持久化与清理

- 数据卷：`nacos-start-local-data`（Docker 管理，避免 Derby 数据库放在 Windows bind mount 上）
- 日志目录：`./nacos/logs`

`docker compose down` 不会删除数据卷。需要重置 Nacos 时，请先确认并备份该卷中的数据；删除数据卷会清空 Nacos 配置及注册数据。

```bash
docker compose down
```

## 常用命令

```bash
docker compose up -d
docker compose logs -f
docker compose stop
docker compose down
```

## 使用说明

- 宿主机应用通常连接 `localhost:8848`
- 控制台登录与客户端 token 是两类配置，请分别处理
- 不要把实际 token 提交到仓库
