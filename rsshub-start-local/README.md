# RSSHub 本地服务

本目录是独立的 Docker Compose 项目。使用固定的 Chromium bundled 镜像，只监听本机 `127.0.0.1:1200`；缓存位于容器内存。需要调整端口时，可复制 `.env.example` 为 `.env` 后修改 `RSSHUB_PORT`。

在本目录运行：

```bash
docker compose config --quiet
docker compose up -d
docker compose ps
curl -fsS http://127.0.0.1:1200/healthz
curl -fsS http://127.0.0.1:1200/36kr/newsflashes
docker compose stop
```

Docker Desktop 启动后，`unless-stopped` 会恢复未被手动停止的容器；本机 Docker Desktop 当前未设登录自启。更改镜像版本前先验证实际 RSS 路由。
