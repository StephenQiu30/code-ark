# Sentinel 本地开发环境

> [返回项目首页](../README.md) | [Back to root](../README.md)

## English Summary

This directory starts a local Sentinel dashboard for traffic control and circuit breaking tests.
It supports `.env` based port configuration and a simple one-command startup flow.
Create `.env` from `.env.example` before startup.

## 服务简介

该目录提供单机模式 Sentinel Dashboard，适合本地验证：

- 流量控制
- 熔断降级
- 系统保护
- 规则配置演练

## 端口与访问入口

| 组件 | 端口 | 说明 |
| --- | --- | --- |
| Dashboard | `8858` | Web 控制台 |
| API | `8719` | 客户端通信端口 |

- 控制台：`http://localhost:8858`
- 默认账号：`sentinel / sentinel`

## 前置条件

- 已安装 Docker 和 Docker Compose
- 首次启动前准备 `.env`

## 快速启动

```bash
cd sentinel-start-local
cp .env.example .env
docker compose up -d
```

也可以使用目录内脚本：

```bash
./start.sh
```

## 配置说明

请在 `.env` 中设置：

- `SENTINEL_DASHBOARD_PORT`
- `SENTINEL_API_PORT`
- `TZ`

## 数据持久化与清理

- 日志目录：`./sentinel/logs`
- 运行目录：`./sentinel/data`

```bash
docker compose down
docker compose down -v
rm -rf ./sentinel
```

## 常用命令

```bash
docker compose up -d
docker compose logs -f
docker compose restart
docker compose down
./start.sh
```

## 使用说明

- 当前配置为单机模式，Dashboard 规则默认不会持久化到外部存储
- Spring Cloud Alibaba 客户端通常连接 `localhost:8858`

## 返回导航

- 项目首页：[`../README.md`](../README.md)
