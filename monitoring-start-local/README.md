# Monitoring 本地开发环境

> [返回项目首页](../README.md) | [Back to root](../README.md)

## English Summary

This directory provides a lightweight monitoring stack based on Prometheus and Grafana.
It is intended for local dashboards, metric collection, and alert rule validation.
The current setup does not require a `.env` file.

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

## 快速启动

```bash
cd monitoring-start-local
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

- Prometheus 规则目录已经挂载到容器内
- Grafana provision 目录已经预留，可继续扩展数据源和仪表板
- 如果你不希望使用默认 `admin / admin`，请在 `docker-compose.yml` 中调整 Grafana 环境变量

## 数据持久化与清理

该目录使用 Docker named volumes：

- `prometheus-data`
- `grafana-data`

```bash
docker compose down
docker compose down -v
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

## 返回导航

- 项目首页：[`../README.md`](../README.md)
