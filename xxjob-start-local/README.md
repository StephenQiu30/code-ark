# XXL-Job 本地开发环境

> [返回项目首页](../README.md) | [Back to root](../README.md)

## English Summary

This directory starts an XXL-Job admin server for local scheduling tests.
It depends on an external MySQL instance and includes the initialization SQL file.
Create `.env` from `.env.example` and import the SQL before startup.

## 服务简介

该目录提供本地 XXL-Job Admin 调度中心，适合：

- 任务调度平台联调
- 执行器注册测试
- 调度管理台验证

## 端口与访问入口

| 组件 | 端口 | 说明 |
| --- | --- | --- |
| XXL-Job Admin | `18081` | Web 管理界面 |

- 控制台：`http://localhost:18081/xxl-job-admin`

## 前置条件

- 已安装 Docker 和 Docker Compose
- 已准备可访问的 MySQL 实例
- 首次启动前准备 `.env`
- 已把 `tables_xxl_job.sql` 导入目标数据库

## 快速启动

```bash
cd xxjob-start-local
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
- `XXL_JOB_ACCESS_TOKEN`

补充说明：

- 当前默认数据库连接目标适合使用宿主机 MySQL
- 初始化 SQL 文件位于 `tables_xxl_job.sql`
- 管理端账号密码通常沿用上游镜像默认值 `admin / 123456`；如果你的数据库已初始化为其他值，请以实际情况为准

## 数据持久化与清理

- 日志目录：`./logs/xxl-job`

```bash
docker compose down
rm -rf ./logs/xxl-job
```

## 常用命令

```bash
docker compose up -d
docker compose logs -f
docker compose stop
docker compose down
```

## 使用说明

- `host.docker.internal` 适合容器访问宿主机 MySQL
- 如果管理台无法启动，请优先检查数据库连通性和初始化 SQL 是否已执行

## 返回导航

- 项目首页：[`../README.md`](../README.md)
