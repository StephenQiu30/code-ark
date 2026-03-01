# 分布式监控面板

基于 Prometheus + Grafana 的分布式监控系统。

## 服务组件

| 服务 | 端口 | 说明 |
|------|------|------|
| Prometheus | 19090 | 指标采集与存储 |
| Grafana | 13000 | 可视化监控面板 |

## 快速开始

### 启动监控服务

```bash
./start.sh
```

### 停止监控服务

```bash
./stop.sh
```

### 完全删除（包括数据）

```bash
docker compose down -v
```

## 访问地址

| 服务 | 地址 | 账号 |
|------|------|------|
| Grafana | http://localhost:13000 | admin / admin |
| Prometheus | http://localhost:19090 | - |

## Grafana 使用指南

### 1. 登录 Grafana

访问 http://localhost:13000，使用 `admin/admin` 登录。

### 2. 添加数据源

Prometheus 数据源已自动配置，可直接使用。

### 3. 导入仪表板

点击 **Dashboards → Import**，可导入以下仪表板：

- **Node Exporter Full**: ID `1860`
- **Docker and system monitoring**: ID `179`
- **Prometheus Stats**: ID `2`

## Prometheus 查询示例

### 基础查询

```promql
# 查询所有指标
up

# 查询目标状态
up{job="prometheus"}
```

## 配置文件说明

```
monitoring-start-local/
├── docker-compose.yml           # 服务编排配置
├── prometheus/
│   ├── prometheus.yml           # Prometheus 主配置
│   └── rules/
│       └── alerts.yml           # 告警规则
└── grafana/
    └── provisioning/
        ├── datasources/         # 数据源配置
        └── dashboards/          # 仪表板配置
```

## 添加监控目标

编辑 `prometheus/prometheus.yml`，在 `scrape_configs` 中添加新目标，然后重启：

```bash
docker compose restart prometheus
```

### 示例：添加 Node Exporter

```yaml
scrape_configs:
  - job_name: 'node-exporter'
    static_configs:
      - targets: ['host.docker.internal:9100']
```
