# RSSHub 本地开发环境

启动带 Chromium 的 RSSHub，用本地 RSS 路由接入内容源；默认只监听 `127.0.0.1:1200`。

[项目首页](../README.md) · [快速启动](#快速启动) · [配置说明](#配置说明)

<details>
<summary>English summary</summary>

This independent Compose project runs a pinned Chromium bundled RSSHub image on `127.0.0.1:1200`. Environment configuration is optional, and the cache lives in memory.

</details>

## 服务简介

适合本地验证 RSS 路由、内容采集和订阅集成。镜像为 `diygod/rsshub:chromium-bundled-2026-09-24`；路由可用性取决于上游内容源。

## 端口与访问入口

| 入口 | 默认地址 |
| --- | --- |
| RSSHub | `http://127.0.0.1:1200` |
| 健康检查 | `http://127.0.0.1:1200/healthz` |

## 前置条件

安装 Docker 和 Docker Compose；本机 `1200` 端口可用。默认配置不需要 `.env`。

## 快速启动

从仓库根目录执行：

```bash
cd rsshub-start-local
docker compose config --quiet
docker compose up -d
docker compose ps
curl -fsS http://127.0.0.1:1200/healthz
```

PowerShell 可用 `Invoke-WebRequest http://127.0.0.1:1200/healthz`。然后访问需要的 RSS 路由，例如 `http://127.0.0.1:1200/36kr/newsflashes`；健康检查成功只说明服务启动，具体路由仍需单独验证。

## 配置说明

需要调整端口时，首次复制 `.env.example` 为 `.env` 并修改 `RSSHUB_PORT`；已有 `.env` 时保留它。Compose 当前使用内存缓存，缓存时间 300 秒、请求超时 20 秒，内存上限 1 GB。

## 数据持久化与清理

本配置没有数据卷，缓存位于容器内存，容器重启会重置缓存。`docker compose down` 移除本项目容器和网络。

## 常用命令

```bash
docker compose logs -f
docker compose stop
docker compose up -d
docker compose down
```

`unless-stopped` 会在 Docker 恢复后启动未被手动停止的容器。更改镜像版本前，验证健康接口及实际使用的 RSS 路由。
