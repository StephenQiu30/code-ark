# MinIO 本地开发环境

> [返回项目首页](../README.md) | [Back to root](../README.md)

## English Summary

This directory provides a local MinIO server with the web console enabled.
Use it for S3-compatible object storage testing during development.
Create `.env` from `.env.example` before startup.

## 服务简介

该目录提供单节点 MinIO，本地开发时可以把它当作 S3 兼容对象存储使用。

## 端口与访问入口

| 组件 | 端口 | 说明 |
| --- | --- | --- |
| MinIO API | `9000` | S3 兼容接口 |
| MinIO Console | `9001` | Web 控制台 |

- 控制台：`http://localhost:9001`
- S3 Endpoint：`http://localhost:9000`

## 前置条件

- 已安装 Docker 和 Docker Compose
- 首次启动前准备 `.env`

## 快速启动

```bash
cd minio-start-local
cp .env.example .env
docker compose up -d
```

## 配置说明

请在 `.env` 中设置：

- `MINIO_ROOT_USER`
- `MINIO_ROOT_PASSWORD`

建议不要在公开仓库或 README 中写入真实账号密码。

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

- S3 SDK 可直接连接 `http://localhost:9000`
- 浏览器访问控制台时使用 `.env` 中的账号密码登录

## 返回导航

- 项目首页：[`../README.md`](../README.md)
