# Nacos 本地开发环境

> [返回项目首页](../README.md) | [Back to root](../README.md)

## English Summary

This directory provides a standalone Nacos server for local service discovery and config testing.
It uses `.env` for token-related configuration and stores data and logs in local directories.
Create `.env` from `.env.example` before startup.

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

- 控制台地址：`http://localhost:8840/nacos`
- 默认控制台账号：`nacos / nacos`

## 前置条件

- 已安装 Docker 和 Docker Compose
- 首次启动前准备 `.env`

## 快速启动

```bash
cd nacos-start-local
cp .env.example .env
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

- 数据目录：`./nacos/data`
- 日志目录：`./nacos/logs`

```bash
docker compose down
docker compose down -v
rm -rf ./nacos/data ./nacos/logs
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

## 返回导航

- 项目首页：[`../README.md`](../README.md)
