<div align="center">

# 🔨 DevForge

**开发工坊 - 打造工具的地方**

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Docker](https://img.shields.io/badge/Docker-Ready-blue.svg)](https://www.docker.com/)
[![Platform](https://img.shields.io/badge/Platform-Windows%20%7C%20macOS%20%7C%20Linux-lightgrey.svg)](https://github.com/yourusername/devforge)
[![GitHub stars](https://img.shields.io/github/stars/yourusername/devforge?style=social)](https://github.com/yourusername/devforge/stargazers)

**还在为环境配置头疼吗？DevForge 来拯救你！**

</div>

---

## 📖 目录

- [✨ 项目简介](#-项目简介)
- [🎯 核心特性](#-核心特性)
- [📦 包含的服务](#-包含的服务)
- [🚀 快速开始](#-快速开始)
- [📖 服务详解](#-服务详解)
- [❓ 常见问题](#-常见问题)
- [🤝 贡献指南](#-贡献指南)

---

## ✨ 项目简介

**DevForge** 是一个精心设计的 Docker Compose 配置集合，帮助开发者快速搭建本地开发环境。

就像工坊为工匠提供趁手的工具，**DevForge** 为开发者提供开箱即用的开发环境。

作为一个开发者，你是否经历过：

- ❌ 为了安装 Elasticsearch 浪费一下午
- ❌ MySQL 版本冲突导致项目无法启动
- ❌ 换电脑后要重新配置所有开发环境
- ❌ Windows 上跑不起来 Kafka，macOS 上权限问题频发

**现在，让 DevForge 帮你摆脱这一切！**

---

## 🎯 核心特性

| 特性 | 说明 |
|------|------|
| 🔧 **即开即用** | 无需掌握 Docker，一条命令启动服务 |
| 🌍 **跨平台** | Windows、macOS、Linux，一套配置处处运行 |
| 📦 **完全隔离** | 服务间互不干扰，可同时运行多个版本 |
| 💾 **数据持久化** | 重启不丢失，随时可清理 |
| 🔒 **安全可靠** | 密码与配置分离，支持自定义 |
| 🚪 **非常用端口** | 避免与系统服务冲突 |

---

## 📦 包含的服务

| 服务 | 目录 | 说明 | 管理界面 |
|------|------|------|:------:|
| 📊 **Elasticsearch + Kibana** | `elastic-start-local/` | 搜索引擎与可视化 | [端口说明](#-elasticsearch--kibana) |
| 🗄️ **MySQL** | `mysql-start-local/` | 关系数据库 | - |
| 🔴 **Redis** | `redis-start-local/` | 内存数据库 | - |
| 🐰 **RabbitMQ** | `rabbitmq-start-local/` | 消息队列 | [端口说明](#-rabbitmq) |
| 🚀 **RocketMQ** | `rocketmq-start-local/` | 分布式消息队列 | [端口说明](#-rocketmq) |
| 🔥 **Kafka** | `kafka-start-local/` | 分布式流处理 | [端口说明](#-kafka) |
| 🔧 **Nacos** | `nacos-start-local/` | 服务发现与配置中心 | [端口说明](#-nacos) |
| 🛡️ **Sentinel** | `sentinel-start-local/` | 流量防卫与熔断降级 | [端口说明](#-sentinel) |
| 📦 **MinIO** | `minio-start-local/` | 对象存储 | [端口说明](#-minio) |
| 📈 **Prometheus + Grafana** | `monitoring-start-local/` | 监控系统 | [端口说明](#-prometheus--grafana) |
| 🔗 **Seata** | `seata-start-local/` | 分布式事务 | - |
| 📋 **XXL-Job** | `xxjob-start-local/` | 分布式任务调度 | [端口说明](#-xxl-job) |

---

## 🚀 快速开始

### 前置要求

- **Docker Desktop** (Windows/macOS/Linux)
- **Docker Compose** (Docker Desktop 已自带)

### 验证安装

```bash
docker --version
docker compose version
```

### 三步启动

**1️⃣ 克隆项目**
```bash
git clone https://github.com/yourusername/devforge.git
cd devforge
```

**2️⃣ 进入服务目录**
```bash
cd mysql-start-local  # 以 MySQL 为例
```

**3️⃣ 启动服务**
```bash
./start.sh  # 或 docker compose up -d
```

🎉 **完成！** 你的开发环境已经运行起来了！

### 常用命令

```bash
# 查看日志
docker compose logs -f

# 停止服务
docker compose down

# 停止并删除数据（谨慎使用）
docker compose down -v

# 重启服务
docker compose restart
```

---

## 📖 服务详解

### 📊 Elasticsearch + Kibana

**功能**: 全文搜索引擎和可视化分析平台

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

**默认账号**: 见 `.env` 中的配置

---

### 🗄️ MySQL

**功能**: 关系型数据库

**端口**: `13306` (避免冲突)

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

### 🔴 Redis

**功能**: 内存数据库

**端口**: `16379` (避免冲突)

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
- 流量控制（QPS、并发线程数）
- 熔断降级（慢调用、异常比例）
- 系统保护（CPU、RT、线程数）
- 授权规则（黑白名单）

---

### 📦 MinIO

**功能**: 对象存储（S3 兼容）

**端口**: `19000` (API) / `19001` (控制台)

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

## ❓ 常见问题

### 端口被占用怎么办？

```bash
# 查看端口占用
lsof -i :端口号  # macOS/Linux
netstat -ano | findstr :端口号  # Windows

# 或修改 docker-compose.yml 中的端口映射
ports:
  - "新端口:容器端口"
```

### 数据会丢失吗？

所有服务已配置数据持久化，重启不会丢失数据。如需清理：

```bash
docker compose down -v
rm -rf ./data
```

### 如何查看服务日志？

```bash
docker compose logs -f      # 所有服务
docker compose logs -f mysql  # 指定服务
```

---

## 🤝 贡献指南

欢迎贡献更多服务的 Docker Compose 配置！

1. Fork 本项目
2. 创建特性分支 (`git checkout -b feature/AmazingService`)
3. 提交更改 (`git commit -m 'Add AmazingService'`)
4. 推送到分支 (`git push origin feature/AmazingService`)
5. 提交 Pull Request

**贡献规范**:
- 每个服务包含 `docker-compose.yml` 和 `README.md`
- 使用非常用端口，避免冲突
- 配置数据持久化
- 添加启动/停止脚本

---

## 📄 许可证

本项目采用 [MIT License](https://opensource.org/licenses/MIT) 开源协议。

---

## ⭐ Star History

如果这个项目对你有帮助，请给一个 ⭐ Star 支持一下！

[![Star History Chart](https://api.star-history.com/svg?repos=yourusername/devforge&type=Date)](https://star-history.com/#yourusername/devforge&Date)

---

<div align="center">

**🔨 DevForge - 打造工具的地方**

**Made with ❤️ by Developers, for Developers**

**让开发更简单，让创造更自由** ✨

</div>
