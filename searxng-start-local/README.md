# SearXNG 本地开发环境

为应用准备聚合搜索与 JSON API，默认只监听 `127.0.0.1:8888`。

[项目首页](../README.md) · [快速启动](#快速启动) · [配置说明](#配置说明)

<details>
<summary>English summary</summary>

This independent Compose project runs a pinned SearXNG image with HTML and JSON search enabled. Copy `.env.example` to `.env`, set a random `SEARXNG_SECRET`, and start the service on `127.0.0.1:8888`.

</details>

## 服务简介

适合本地搜索页面体验与应用搜索 API 联调。镜像固定为 `searxng/searxng:2026.9.25-487f51922`，使用本目录的 `settings.yml`；搜索结果取决于各上游引擎的可达性。

## 端口与访问入口

| 入口 | 默认地址 |
| --- | --- |
| 搜索页面 | `http://127.0.0.1:8888` |
| JSON API | `http://127.0.0.1:8888/search?q=example&format=json` |

## 前置条件

安装 Docker 和 Docker Compose；本机 `8888` 端口可用。首次启动需创建 `.env` 并填写随机 `SEARXNG_SECRET`。

## 快速启动

从仓库根目录执行：

```bash
cd searxng-start-local
cp .env.example .env
# 编辑 .env，设置随机 SEARXNG_SECRET
docker compose config --quiet
docker compose up -d
docker compose ps
curl -fsS 'http://127.0.0.1:8888/search?q=example&format=json'
```

PowerShell 使用 `Copy-Item .env.example .env`，API 检查使用 `Invoke-RestMethod 'http://127.0.0.1:8888/search?q=example&format=json'`。已有 `.env` 时不要覆盖；不要提交密钥。

## 配置说明

- `.env`：必填 `SEARXNG_SECRET`，可调整 `SEARXNG_PORT`（默认 `8888`）。
- `settings.yml`：只读挂载，启用 HTML / JSON 返回格式；关闭公共实例模式和 limiter，供本地使用。
- 时区固定 `TZ=UTC`；容器内存上限 512 MB。
- 更改镜像或搜索设置后，重新验证应用使用的 JSON 查询。

## 数据持久化与清理

当前配置没有数据库或数据卷。设置保存在宿主机的 `settings.yml` 与 `.env`，`docker compose down` 不删除这两个文件。

## 常用命令

```bash
docker compose logs -f
docker compose stop
docker compose up -d
docker compose down
```

`unless-stopped` 会在 Docker 恢复后启动未被手动停止的容器。默认本地入口适合开发联调；远程共享前需另行配置访问控制。
