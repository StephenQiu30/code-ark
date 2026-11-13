# Monitoring 本地开发环境

使用 Prometheus 采集指标、Grafana 查看仪表盘，在开发环境中验证采集目标和告警规则。

[项目首页](../../README.md) · [服务索引](../README.md) · [快速启动](#快速启动) · [连接入口](#端口与访问入口) · [配置说明](#配置说明)

<details>
<summary>English summary</summary>

This directory provides a lightweight monitoring stack based on Prometheus and Grafana.
It is intended for local dashboards, metric collection, and alert rule validation.
The current setup does not require a `.env` file.
Reserve 1 GiB for the stack: 512 MiB each for Grafana and Prometheus, plus Docker host overhead.

</details>

## 服务简介

该目录用于启动本地监控环境，包含：

- Prometheus
- Grafana

适合开发阶段的指标采集、可视化和告警规则演练。

## 端口与访问入口

| 组件 | 端口 | 说明 |
| --- | --- | --- |
| Prometheus | `19090` | 指标采集与查询 |
| Grafana | `13000` | Web 可视化面板 |

- Prometheus：`http://localhost:19090`
- Grafana：`http://localhost:13000`
- Grafana 默认账号：`admin / admin`

## 前置条件

- 已安装 Docker 和 Docker Compose
- 为本组容器预留至少 1 GiB 可用内存，另保留 Docker 自身运行余量。Prometheus 和 Grafana 的内存上限各为 512 MiB。
- Grafana 官方最低推荐为 [512 MB 内存、1 个 CPU 核心](https://grafana.com/docs/grafana/v13.3/setup-grafana/installation/)。Grafana 13.3.1 在旧的 256 MiB 硬限下可能持续回收内存页，进程没有被 OOM 杀死也可能长期无法开始监听 HTTP。

## 快速启动

以下进入目录的命令从仓库根目录执行；已有 `.env` 时保留原文件。

```bash
cd services/monitoring-start-local
docker compose up -d
```

如果你希望使用现成脚本，也可以执行：

```bash
./start.sh
```

## 配置说明

当前目录的主要配置文件如下：

- `docker-compose.yml`
- `prometheus/prometheus.yml`
- `prometheus/rules/alerts.yml`
- `grafana/provisioning/datasources/prometheus.yml`
- `grafana/provisioning/dashboards/dashboards.yml`

说明：

- Prometheus 配置与规则目录以只读方式挂载到容器内
- Grafana 已通过 provision 预置 Prometheus 数据源，可继续扩展仪表板
- Grafana 默认账号为 `admin / admin`；复制 `.env.example` 为 `.env` 并设置 `GRAFANA_ADMIN_PASSWORD` 可覆盖
- `prometheus/prometheus.yml` 当前只采集 Prometheus 自身；部署 node-exporter、cAdvisor 等组件后再启用对应 job

## 数据持久化与清理

普通 `down` 保留命名卷；只有明确需要清空本项目数据时，备份后执行 `docker compose down -v`。

该目录使用 Docker named volumes：

- `prometheus-data`
- `grafana-data`

```bash
docker compose down
```

## 常用命令

```bash
docker compose up -d
docker compose logs -f
docker compose restart
docker compose stop
docker compose down
./stop.sh
```

## 使用说明

- 如果要接入新的 Prometheus 采集目标，请修改 `prometheus/prometheus.yml`
- 如果要新增告警规则，请修改 `prometheus/rules/alerts.yml`
- 如果需要预置 Grafana 数据源或面板，请在 `grafana/provisioning` 目录下扩展
