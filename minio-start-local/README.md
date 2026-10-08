# MinIO 本地开发环境

为应用准备本地 S3 兼容存储，通过 Web 控制台管理文件并联调上传、下载流程。

[项目首页](../README.md) · [快速启动](#快速启动) · [连接入口](#端口与访问入口) · [配置说明](#配置说明)

<details>
<summary>English summary</summary>

This directory provides a local MinIO server with the web console enabled.
Use it for S3-compatible object storage testing during development.
Create `.env` from `.env.example` before startup.

</details>

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
```

普通 `down` 保留绑定目录。需要完全重置时，先备份，再手动删除 `./data` 中的对应数据。

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
