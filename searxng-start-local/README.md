# SearXNG 本地服务

本目录是独立的 Docker Compose 项目。固定版本镜像使用本目录的 `settings.yml`，只监听本机 `127.0.0.1:8888`。`TZ=UTC` 与 HotKey 对无时区发布时间的解析约定一致。

首次运行需复制 `.env.example` 为未跟踪的 `.env`，并设置随机 `SEARXNG_SECRET`；当前机器已完成配置。不要提交或打印 `.env`。

在本目录运行：

```bash
docker compose config --quiet
docker compose up -d
docker compose ps
curl -fsS 'http://127.0.0.1:8888/search?q=example&format=json'
docker compose stop
```

Docker Desktop 启动后，`unless-stopped` 会恢复未被手动停止的容器；本机 Docker Desktop 当前未设登录自启。更改镜像版本前先验证 JSON 搜索及 HotKey 预设。
