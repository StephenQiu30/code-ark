<div align="center">

# 📈 分布式监控面板

[![Prometheus](https://img.shields.io/badge/Prometheus-latest-red.svg)](https://prometheus.io/)
[![Grafana](https://img.shields.io/badge/Grafana-latest-orange.svg)](https://grafana.com/)

**基于 Prometheus + Grafana 的轻量级监控系统**

</div>

---

## 📖 目录

- [简介](#简介)
- [服务架构](#服务架构)
- [快速开始](#快速开始)
- [使用指南](#使用指南)
- [告警规则](#告警规则)
- [扩展监控](#扩展监控)

---

## 简介

本监控方案提供了一套轻量级、易部署的监控系统，适用于开发环境和中小型项目。

### 核心特性

- ✅ **开箱即用**：一条命令启动完整监控栈
- ✅ **自动配置**：Grafana 数据源自动配置
- ✅ **告警支持**：内置常用告警规则
- ✅ **易于扩展**：轻松添加新的监控目标

---

## 服务架构

```
┌─────────────┐      ┌─────────────┐
│  Prometheus │◄─────┤   Grafana   │
│ (指标采集)   │      │  (可视化)    │
└─────────────┘      └─────────────┘
       │
       └─────► 监控目标
```

### 端口说明

| 服务 | 端口 | 说明 |
|------|:----:|------|
| Prometheus | **19090** | 时序数据库 |
| Grafana | **13000** | 可视化面板 |

> 使用非常用端口，避免与系统服务冲突

---

## 快速开始

### 前置要求

- Docker Desktop 已安装并运行

### 启动服务

```bash
cd monitoring-start-local
./start.sh
```

### 访问界面

| 服务 | 地址 | 账号 |
|------|------|:----:|
| Grafana | http://localhost:13000 | admin / admin |
| Prometheus | http://localhost:19090 | - |

---

## 使用指南

### Grafana 快速上手

**1. 登录**

访问 http://localhost:13000，使用 `admin/admin` 登录

**2. 数据源**

Prometheus 数据源已自动配置，无需手动添加

**3. 导入仪表板**

点击 **Dashboards → Import**，推荐以下仪表板：

| 仪表板 | ID | 说明 |
|--------|-----|------|
| Node Exporter Full | 1860 | 系统监控 |
| Docker Monitoring | 179 | 容器监控 |
| Prometheus Stats | 2 | Prometheus 状态 |

### Prometheus 查询示例

```promql
# CPU 使用率
100 - (avg(rate(node_cpu_seconds_total{mode="idle"}[5m])) * 100)

# 内存使用率
(1 - node_memory_MemAvailable_bytes / node_memory_MemTotal_bytes) * 100

# 磁盘使用率
(1 - node_filesystem_avail_bytes / node_filesystem_size_bytes) * 100

# 容器状态
up{job="docker"}
```

---

## 告警规则

内置告警规则位于 `prometheus/rules/alerts.yml`：

| 告警 | 级别 | 条件 |
|------|:----:|------|
| HighCPUUsage | warning | CPU > 80% 持续 5 分钟 |
| HighMemoryUsage | warning | 内存 > 80% 持续 5 分钟 |
| HighDiskUsage | warning | 磁盘 > 85% 持续 5 分钟 |

---

## 扩展监控

### 添加 Node Exporter（主机监控）

**1. 安装 Node Exporter**

```bash
# macOS
brew install node_exporter
brew services start node_exporter

# Linux
sudo apt install prometheus-node-exporter
```

**2. 配置 Prometheus**

编辑 `prometheus/prometheus.yml`：

```yaml
scrape_configs:
  - job_name: 'node-exporter'
    static_configs:
      - targets: ['host.docker.internal:9100']
```

**3. 重启服务**

```bash
docker compose restart prometheus
```

### 添加应用监控

在你的 Spring Boot 应用中添加 Actuator 和 Prometheus 依赖：

```xml
<dependency>
    <groupId>org.springframework.boot</groupId>
    <artifactId>spring-boot-starter-actuator</artifactId>
</dependency>
<dependency>
    <groupId>io.micrometer</groupId>
    <artifactId>micrometer-registry-prometheus</artifactId>
</dependency>
```

配置文件：
```yaml
management:
  endpoints:
    web:
      exposure:
        include: prometheus
  metrics:
    tags:
      application: ${spring.application.name}
```

---

## 配置文件说明

```
monitoring-start-local/
├── docker-compose.yml           # 服务编排
├── prometheus/
│   ├── prometheus.yml           # 主配置
│   └── rules/
│       └── alerts.yml           # 告警规则
├── grafana/
│   └── provisioning/
│       ├── datasources/         # 数据源
│       └── dashboards/          # 仪表板
├── start.sh                     # 启动脚本
└── stop.sh                      # 停止脚本
```

---

## 管理命令

```bash
# 启动
./start.sh

# 停止
./stop.sh

# 查看日志
docker compose logs -f

# 重启
docker compose restart

# 完全删除
docker compose down -v
```

---

## 参考资源

- [Prometheus 官方文档](https://prometheus.io/docs/)
- [Grafana 官方文档](https://grafana.com/docs/)
- [Grafana 仪表板市场](https://grafana.com/grafana/dashboards/)

---

<div align="center">

**Made with ❤️ using Prometheus & Grafana**

</div>
