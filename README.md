<div align="center">

# 🔨 DevForge

**DevForge · DevOps Local Stack**

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Docker](https://img.shields.io/badge/Docker-Ready-blue.svg)](https://www.docker.com/)
[![Platform](https://img.shields.io/badge/Platform-Windows%20%7C%20macOS%20%7C%20Linux-lightgrey.svg)](https://www.docker.com/)

**一个面向开发者的本地 Docker 开发环境集合，专注于“快、稳、可维护”。**

</div>

如果你正在经常遇到“本地环境搭不起来、换机器要重装一遍、数据库版本冲突、消息中间件启动复杂”等问题，这个仓库就是为你准备的：  
把常用基础设施组件拆成独立目录，开箱即用运行在本地，按需组合，保持环境一致性。

---

## 🚀 你会得到什么

- 统一风格：所有服务都使用 `docker-compose.yml` + `.env` 的约定，便于记忆和复用。
- 高可用启动体验：大多数服务支持一键启动脚本与标准命令并行。
- 开发友好端口：避免与常见本地服务冲突，减少“端口占用地狱”。
- 数据可控：默认挂载持久化目录，重启恢复快，清理也清晰可控。
- 跨团队协作：新成员可按同一套规则快速恢复同样的本地栈。

---

## 📌 目录

- [📦 服务一览](#-服务一览)
- [⚙️ 三步上手](#️-三步上手)
- [📖 服务详解](#-服务详解)
- [🧪 常用场景推荐](#-常用场景推荐)
- [❓ 常见问题](#-常见问题)
- [🤝 参与共建](#-参与共建)
- [📄 许可证](#-许可证)

---

## 📦 服务一览

<table>
  <thead>
    <tr>
      <th>服务</th><th>目录</th><th>说明</th><th>管理界面</th>
    </tr>
  </thead>
  <tbody>
    <tr><td>📊 Elasticsearch + Kibana</td><td><code>elastic-start-local/</code></td><td>搜索引擎与可视化</td><td><a href="#-elasticsearch--kibana">端口说明</a></td></tr>
    <tr><td>🗄️ MySQL</td><td><code>mysql-start-local/</code></td><td>关系数据库</td><td>-</td></tr>
    <tr><td>🐘 PostgreSQL</td><td><code>pgsql-start-local/</code></td><td>关系数据库</td><td>-</td></tr>
    <tr><td>🔴 Redis</td><td><code>redis-start-local/</code></td><td>内存数据库</td><td>-</td></tr>
    <tr><td>🐰 RabbitMQ</td><td><code>rabbitmq-start-local/</code></td><td>消息队列</td><td><a href="#-rabbitmq">端口说明</a></td></tr>
    <tr><td>🚀 RocketMQ</td><td><code>rocketmq-start-local/</code></td><td>分布式消息队列</td><td><a href="#-rocketmq">端口说明</a></td></tr>
    <tr><td>🔥 Kafka</td><td><code>kafka-start-local/</code></td><td>分布式流处理</td><td><a href="#-kafka">端口说明</a></td></tr>
    <tr><td>🔧 Nacos</td><td><code>nacos-start-local/</code></td><td>服务发现与配置中心</td><td><a href="#-nacos">端口说明</a></td></tr>
    <tr><td>🛡️ Sentinel</td><td><code>sentinel-start-local/</code></td><td>流控与熔断</td><td><a href="#-sentinel">端口说明</a></td></tr>
    <tr><td>📦 MinIO</td><td><code>minio-start-local/</code></td><td>对象存储</td><td><a href="#-minio">端口说明</a></td></tr>
    <tr><td>📈 Prometheus + Grafana</td><td><code>monitoring-start-local/</code></td><td>监控系统</td><td><a href="#-prometheus--grafana">端口说明</a></td></tr>
    <tr><td>🔗 Seata</td><td><code>seata-start-local/</code></td><td>分布式事务</td><td>-</td></tr>
    <tr><td>📋 XXL-Job</td><td><code>xxjob-start-local/</code></td><td>分布式任务调度</td><td><a href="#-xxl-job">端口说明</a></td></tr>
  </tbody>
</table>

---

## ⚙️ 三步上手

### 1) 环境准备

- 安装 [Docker Desktop](https://www.docker.com/)（Win / macOS / Linux）
- 打开终端并确认：

```bash
docker --version
docker compose version
```

### 2) 拉取并进入仓库

```bash
git clone https://github.com/yourusername/devforge.git
cd devforge
```

### 3) 选择需要的服务启动

以 PostgreSQL 为例：

```bash
cd pgsql-start-local
docker compose up -d
```

> 你也可以直接运行每个目录内的 `start.sh`（存在于部分服务），或使用 `docker compose up -d`。

---

### 全局常用命令（任意服务目录可执行）

```bash
docker compose logs -f      # 查看日志
docker compose restart       # 重启
docker compose down          # 停止
docker compose down -v       # 停止并清理持久化数据
```

---

## 📖 服务详解

### 📊 Elasticsearch + Kibana

**功能**: 全文搜索引擎与可视化分析平台  
**端口**:

| 服务 | 端口 |
|------|:----:|
| Elasticsearch | 9200 |
| Kibana | 5601 |

**启动**:

```bash
cd elastic-start-local
./start.sh
```

**默认账号**: 见 `.env` 中配置

---

### 🗄️ MySQL

**功能**: 关系数据库  
**端口**: `13306`（避免与系统默认冲突）  

**启动**:

```bash
cd mysql-start-local
./start.sh
```

**连接配置**:

```yaml
host: localhost
port: 13306
username: root
password: 见 .env
```

---

### 🐘 PostgreSQL

**功能**: 关系数据库  
**端口**: `5432`  

**启动**:

```bash
cd pgsql-start-local
docker compose up -d
```

**连接配置**:

```yaml
host: localhost
port: 5432
username: user
password: 见 .env
database: test_db
```

---

### 🔴 Redis

**功能**: 内存数据库  
**端口**: `16379`（避免与系统默认冲突）  

**启动**:

```bash
cd redis-start-local
./start.sh
```

---

### 🐰 RabbitMQ

**功能**: 消息队列  
**端口**:

| 服务 | 端口 |
|------|:----:|
| AMQP | 15672 |
| 管理界面 | 25672 |

**管理界面**: http://localhost:25672

---

### 🚀 RocketMQ

**功能**: 分布式消息队列（阿里开源）  
**端口**:

| 服务 | 端口 |
|------|:----:|
| NameServer | 15876 |
| Broker | 15909, 15911, 15912 |
| Console | 18180 |

**启动**:

```bash
cd rocketmq-start-local
./start.sh
```

**控制台**: http://localhost:18180

---

### 🔥 Kafka

**功能**: 分布式流处理平台  
**端口**:

| 服务 | 端口 |
|------|:----:|
| Kafka | 19092 |
| Kafka UI | 19000 |

**UI**: http://localhost:19000

---

### 🔧 Nacos

**功能**: 服务发现与配置中心  
**端口**:

| 服务 | 端口 |
|------|:----:|
| 控制台 | 8840 |
| 主端口 | 8848 |

**控制台**: http://localhost:8840/nacos  
**默认账号**: `nacos` / `nacos`

---

### 🛡️ Sentinel

**功能**: 流量防卫组件（阿里开源）  
**端口**:

| 服务 | 端口 |
|------|:----:|
| Dashboard | 8858 |
| API | 8719 |

**启动**:

```bash
cd sentinel-start-local
./start.sh
```

**控制台**: http://localhost:8858  
**默认账号**: `sentinel` / `sentinel`

**核心功能**:
- 流量控制（QPS、并发线程）
- 熔断降级（慢调用、异常比例）
- 系统保护（CPU、RT、线程）
- 授权规则（黑白名单）

---

### 📦 MinIO

**功能**: 对象存储（S3 兼容）  
**端口**: `19000`（API）/ `19001`（控制台）  

**控制台**: http://localhost:19001

---

### 📈 Prometheus + Grafana

**功能**: 监控系统  
**端口**:

| 服务 | 端口 |
|------|:----:|
| Prometheus | 19090 |
| Grafana | 13000 |

**启动**:

```bash
cd monitoring-start-local
./start.sh
```

**Grafana**: http://localhost:13000 (`admin` / `admin`)

---

### 🔗 Seata

**功能**: 分布式事务协调器  

**启动**:

```bash
cd seata-start-local
./start.sh
```

---

### 📋 XXL-Job

**功能**: 分布式任务调度平台  
**端口**: `18888`  
**控制台**: http://localhost:18888/xxl-job-admin  
**默认账号**: `admin` / `123456`

---

## 🧪 常用场景推荐

- **后端开发新手环境**：MySQL + Redis + Nacos + Sentinel
- **任务调度项目**：XXL-Job + Redis + MySQL
- **中间件联调**：Kafka + Redis + Nacos + Sentinel
- **对象存储联调**：MinIO + Nacos
- **全链路监控**：Prometheus + Grafana + 目标服务日志

每个服务可独立启动，也可按项目实际组合，你不必一次性起完全部服务。

---

## ❓ 常见问题

### 端口被占用怎么办？

```bash
# 查看端口占用
lsof -i :端口号  # macOS/Linux
netstat -ano | findstr :端口号  # Windows

# 或直接修改 docker-compose.yml 中的映射端口
ports:
  - "新端口:容器端口"
```

### 数据会不会丢失？

服务默认挂载持久化目录，重启不会丢数据；  
`docker compose down -v` 会清理对应卷内数据，请在确认后执行。

### 看不到某个服务控制台？

确认端口无误并查看日志：

```bash
docker compose logs -f
```

---

## 🤝 参与共建

欢迎你提交新的服务配置或优化文档。  

1. Fork 本项目  
2. 创建分支：`git checkout -b feature/xxx`  
3. 提交更改：`git commit -m "Add xxx service"`  
4. 推送并发起 PR：`git push origin feature/xxx`

建议 PR 约定：

- 服务目录至少包含 `docker-compose.yml`
- 推荐提供 `.env` 与 `.env.example`
- 提供基本启动和连接说明（`README`）
- 尽量避免与现有端口冲突，优先复用项目内端口策略

---

## 📄 许可证

本项目采用 [MIT License](https://opensource.org/licenses/MIT) 开源协议。

---

<div align="center">

**如果 DevForge 对你有帮助，欢迎 Star / Fork / 提建议！**

**🔨 DevForge - 让本地环境搭建从“耗时”变“秒起”**

</div>

