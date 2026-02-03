# CodeArk

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Docker](https://img.shields.io/badge/Docker-Ready-blue.svg)](https://www.docker.com/)
[![Platform](https://img.shields.io/badge/Platform-Windows%20%7C%20macOS%20%7C%20Linux-lightgrey.svg)](https://github.com/StephenQiu30/code-ark)
[![GitHub stars](https://img.shields.io/github/stars/StephenQiu30/code-ark?style=social)](https://github.com/StephenQiu30/code-ark/stargazers)

---

💻 **还在为环境配置头疼吗？**

**CodeArk（代码方舟）** 来拯救你了！

作为一个开发者，你是否经历过：
- ❌ 为了安装 Elasticsearch 浪费一下午
- ❌ MySQL 版本冲突导致项目无法启动
- ❌ 换电脑后要重新配置所有开发环境
- ❌ Windows 上跑不起来 Kafka，macOS 上权限问题频发

**现在，让 CodeArk 帮你摆脱这一切！**

---

## 📑 目录

- [🎯 一条命令，即可启航](#-一条命令即可启航)
- [✨ 核心特性](#-核心特性)
- [📦 包含的服务](#-包含的服务)
- [🚀 快速开始](#-快速开始)
- [📖 各服务详细说明](#-各服务详细说明)
- [❓ 常见问题](#-常见问题)
- [🔧 高级用法](#-高级用法)
- [💡 最佳实践](#-最佳实践)
- [🤝 贡献指南](#-贡献指南)

---

## 🎯 一条命令，即可启航

```bash
cd mysql-redis-start-local
docker-compose up -d
# 30秒后，MySQL + Redis 已经运行起来了 🎉
```

## ✨ 核心特性

✅ **9+ 种常用中间件**：MySQL、Redis、Elasticsearch、Kafka、RabbitMQ、RocketMQ、Nacos、MinIO...
✅ **跨平台支持**：Windows、macOS、Linux，一套配置处处运行
✅ **开箱即用**：无需掌握 Docker，复制配置文件即可
✅ **完全隔离**：互不干扰，可同时运行多个版本
✅ **数据持久化**：重启不丢失，随时可清理
✅ **安全可靠**：密码与配置分离，支持自定义

🌊 **拯救开发者脱离环境配置的苦海**，一行命令启航你的开发之旅！

---

## 📦 包含的服务

| 服务 | 目录 | 说明 | 管理界面 |
|------|------|------|---------|
| 📊 **Elasticsearch + Kibana** | `elastic-start-local/` | 搜索引擎和可视化 | http://localhost:5601 |
| 🗄️ **MySQL + Redis** | `mysql-redis-start-local/` | 关系数据库和缓存 | - |
| 🐰 **RabbitMQ** | `rabbitmq-start-lcoal/` | 消息队列 | http://localhost:15672 |
| 🚀 **RocketMQ** | `rocketmq-start-local/` | 分布式消息队列 | http://localhost:9001 |
| 🔥 **Kafka** | `kafka-start-local/` | 分布式流处理平台 | http://localhost:9000 |
| 🔧 **Nacos** | `nacos-start-local/` | 服务发现和配置中心 | http://localhost:8081/nacos |
| 📦 **MinIO** | `minio-start-local/` | 对象存储（S3 兼容） | http://localhost:9001 |
| 🔴 **Redis** | `redis-start-local/` | 独立内存数据库 | - |
| 🗄️ **MySQL** | `mysql-start-lcoal/` | 独立关系数据库 | - |

---

## 🚀 快速开始

### 前置要求

- **Docker Desktop** (Windows/macOS/Linux)
- **Docker Compose** (Docker Desktop 已自带)

### 安装 Docker Desktop

**Windows:** https://docs.docker.com/desktop/install/windows-install/
**macOS:** https://docs.docker.com/desktop/install/mac-install/

验证安装：
```bash
docker --version
docker-compose --version
```

### 三步启动

**1️⃣ 克隆项目**
```bash
git clone https://github.com/StephenQiu30/code-ark.git
cd code-ark
```

**2️⃣ 配置环境变量**
```bash
# 方式一：逐个服务配置
cd mysql-redis-start-local
cp .env.example .env
# 编辑 .env 文件，修改密码等敏感信息

# 方式二：批量处理（macOS/Linux）
find . -name ".env.example" -exec sh -c 'cp "$1" "${1%.example}"' _ {} \;
```

**3️⃣ 启动服务**
```bash
cd mysql-redis-start-local
docker-compose up -d
```

🎉 **完成！** 你的开发环境已经运行起来了！

### 常用命令

```bash
# 查看日志
docker-compose logs -f

# 停止服务
docker-compose down

# 停止服务并删除数据卷（谨慎使用）
docker-compose down -v

# 重启服务
docker-compose restart
```

---

## 📖 各服务详细说明

### 📊 Elasticsearch + Kibana

**功能**: 全文搜索引擎和可视化分析平台

**默认端口**:
- Elasticsearch: `http://localhost:9200`
- Kibana: `http://localhost:5601`

**默认账号**:
- 用户名: `elastic`
- 密码: 见 `.env` 中的 `ES_LOCAL_PASSWORD`

**启动**:
```bash
cd elastic-start-local
docker-compose up -d
```

**使用说明**:
1. 等待约 1-2 分钟，Elasticsearch 健康检查通过
2. 访问 Kibana: `http://localhost:5601`
3. 使用 `elastic` 账号登录

---

### 🗄️ MySQL + Redis

**功能**: 关系型数据库和内存数据库

**默认端口**:
- MySQL: `localhost:3306`
- Redis: `localhost:6379`

**默认配置**:
```bash
# MySQL
用户名: root (见 .env)
密码: 见 .env 中的 MYSQL_ROOT_PASSWORD
数据库: test_db

额外用户: user (见 .env)
密码: 见 .env 中的 MYSQL_PASSWORD

# Redis
无密码（如需密码请修改 docker-compose.yml）
```

**启动**:
```bash
cd mysql-redis-start-local
docker-compose up -d
```

**连接示例**:
```bash
# MySQL 连接
mysql -h 127.0.0.1 -u root -p

# Redis 连接
redis-cli
```

---

### 🐰 RabbitMQ

**功能**: 消息队列服务

**默认端口**:
- AMQP: `localhost:5672`
- 管理界面: `http://localhost:15672`

**默认账号**:
- 用户名: 见 `.env` 中的 `RABBITMQ_DEFAULT_USER`
- 密码: 见 `.env` 中的 `RABBITMQ_DEFAULT_PASS`

**启动**:
```bash
cd rabbitmq-start-lcoal
docker-compose up -d
```

**使用说明**:
访问管理界面 `http://localhost:15672`，使用 `.env` 中配置的账号登录。

---

### 🚀 RocketMQ

**功能**: 分布式消息队列（阿里开源）

**默认端口**:
- NameServer: `9876`
- Broker: `10909`, `10911`, `10912`
- Dashboard: `http://localhost:9001`

**启动**:
```bash
cd rocketmq-start-local
docker-compose up -d
```

**使用说明**:
访问 Dashboard `http://localhost:9001` 查看消息队列状态。

---

### 🔥 Kafka

**功能**: 分布式流处理平台

**默认端口**:
- Kafka: `localhost:9092`
- Kafka UI: `http://localhost:9000`

**启动**:
```bash
cd kafka-start-local
docker-compose up -d
```

**使用说明**:
访问 Kafka UI `http://localhost:9000` 管理主题和消费者组。

---

### 🔧 Nacos

**功能**: 服务发现和配置中心（Spring Cloud Alibaba）

**默认端口**:
- **控制台**: `http://localhost:8081/nacos` （主要访问入口）
- 主端口: `8848`
- gRPC: `9848`, `9849`
- Jraft: `9850`

**默认账号**:
- 用户名: `nacos`
- 密码: `nacos`

**启动**:
```bash
cd nacos-start-local
docker-compose up -d
```

**使用说明**:
访问控制台 `http://localhost:8081/nacos`，默认账号密码均为 `nacos`。

**认证密钥**: 需在 `.env` 中配置 `NACOS_AUTH_TOKEN`（用于服务间认证）

---

### 📦 MinIO

**功能**: 对象存储服务（兼容 S3 API）

**默认端口**:
- API: `localhost:9000`
- 控制台: `http://localhost:9001`

**默认账号**:
- 用户名: 见 `.env` 中的 `MINIO_ROOT_USER`
- 密码: 见 `.env` 中的 `MINIO_ROOT_PASSWORD`

**启动**:
```bash
cd minio-start-local
docker-compose up -d
```

**使用说明**:
访问控制台 `http://localhost:9001`，创建 Bucket 进行文件存储。

---

### 🔴 Redis（独立）

**功能**: 独立的 Redis 服务

**默认端口**: `localhost:6379`

**启动**:
```bash
cd redis-start-local
docker-compose up -d
```

**使用说明**:
```bash
redis-cli
```

---

## ❓ 常见问题

### 1. 端口被占用

**问题**: 启动时提示端口已被使用

```bash
Error: port is already allocated
```

**解决方案**:

**Windows (PowerShell)**:
```powershell
# 查看端口占用
netstat -ano | findstr :3306

# 结束进程（替换 PID）
taskkill /PID <进程ID> /F
```

**macOS/Linux**:
```bash
# 查看端口占用
lsof -i :3306

# 结束进程
kill -9 <PID>
```

或者修改 `.env` 文件中的端口号：
```bash
# 例如将 MySQL 端口改为 3307
MYSQL_PORT=3307
```

### 2. 数据持久化

**问题**: 容器删除后数据丢失

**说明**: 本项目所有服务都已配置数据卷挂载，数据会保存在各服务目录的 `data/` 或 `*-data/` 目录中。

**删除数据**:
```bash
# 停止服务并删除数据卷
docker-compose down -v

# 手动删除数据目录
rm -rf ./data
```

### 3. 内存不足

**问题**: Elasticsearch 等服务启动失败，提示内存不足

**解决方案**: 修改 Docker Desktop 内存分配

- **macOS**: Docker Desktop > Settings > Resources > Memory (建议 4GB+)
- **Windows**: Docker Desktop > Settings > Resources > WSL Integration > Memory

或者调整 `.env` 中的 JVM 参数：
```bash
ES_LOCAL_JAVA_OPTS="-Xms512m -Xmx1g"
```

### 4. 容器无法启动

**调试步骤**:
```bash
# 查看容器日志
docker-compose logs

# 查看特定服务日志
docker-compose logs mysql

# 查看容器状态
docker-compose ps

# 重新构建并启动
docker-compose up -d --force-recreate
```

### 5. macOS 用户特殊说明

某些服务（如 Elasticsearch）在 macOS 上可能需要额外配置：

```bash
# 确保 Docker Desktop 有足够的文件描述符限制
ulimit -n 65536
```

### 6. Windows 用户特殊说明

如果遇到文件路径问题，确保：
- 使用 WSL 2 后端
- 项目路径不在 `C:\Windows\System32` 等系统目录
- 文件路径不包含中文和特殊字符

---

## 🔧 高级用法

### 自定义配置

每个服务的 `docker-compose.yml` 都可以根据需求修改：

```yaml
# 修改端口映射
ports:
  - "3307:3306"  # 宿主机端口:容器端口

# 修改环境变量
environment:
  - MYSQL_DATABASE=my_custom_db

# 挂载自定义配置文件
volumes:
  - ./my.cnf:/etc/mysql/conf.d/custom.cnf
```

### 同时运行多个版本

```bash
# 在不同目录启动不同版本
cd mysql-redis-start-local
docker-compose up -d

cd ../mysql-start-lcoal
docker-compose up -d
```

### 开机自启动

在 `docker-compose.yml` 中已配置 `restart: always`，Docker Desktop 启动时会自动启动容器。

### 网络连接

不同服务之间可以通过容器名互相访问：

```yaml
# 在 app 服务中连接 MySQL
environment:
  - DB_HOST=mysql  # 使用容器名而非 localhost
  - DB_PORT=3306
```

---

## 💡 最佳实践

### 1. 密码安全

- **生产环境**：务必修改 `.env` 中的默认密码
- **密码强度**：使用强密码，包含大小写字母、数字和特殊字符
- **密钥生成**：对于 `NACOS_AUTH_TOKEN` 等密钥，使用安全随机字符串

生成安全密钥：
```bash
# macOS/Linux
openssl rand -base64 32

# 或使用在线工具
# https://generate-random.org/encryption-key-generator
```

### 2. 资源限制

为防止服务占用过多资源，可以在 `docker-compose.yml` 中添加限制：

```yaml
services:
  mysql:
    deploy:
      resources:
        limits:
          cpus: '1'
          memory: 1G
        reservations:
          cpus: '0.5'
          memory: 512M
```

### 3. 备份数据

定期备份持久化数据：

```bash
# MySQL 备份示例
docker-compose exec mysql mysqldump -u root -p test_db > backup.sql

# Redis 备份示例
cp redis-data/dump.rdb redis-data/dump.rdb.backup
```

### 4. 日志管理

防止日志文件无限增长：

```yaml
services:
  mysql:
    logging:
      driver: "json-file"
      options:
        max-size: "10m"
        max-file: "3"
```

---

## 📂 项目结构

```
CodeArk/
├── .gitignore                    # Git 忽略规则
├── README.md                     # 本文件
├── elastic-start-local/          # Elasticsearch + Kibana
│   ├── docker-compose.yml
│   ├── .env                      # 敏感配置（不提交）
│   ├── .env.example              # 配置模板
│   └── config/
│       └── telemetry.yml
├── mysql-redis-start-local/      # MySQL + Redis
│   ├── docker-compose.yml
│   ├── .env
│   ├── .env.example
│   ├── mysql-data/               # 数据持久化
│   └── redis-data/
├── minio-start-local/            # MinIO
├── nacos-start-local/            # Nacos
├── rabbitmq-start-lcoal/         # RabbitMQ
├── redis-start-local/            # Redis
├── rocketmq-start-local/         # RocketMQ
└── kafka-start-local/            # Kafka
```

---

## 🤝 贡献指南

欢迎贡献更多服务的 Docker Compose 配置！

1. Fork 本项目
2. 创建新的服务目录 (`git checkout -b feature/AmazingService`)
3. 编写 `docker-compose.yml` 和 `.env.example`
4. 更新 README 添加服务说明
5. 提交 Pull Request

**贡献规范**:
- 每个服务应包含 `docker-compose.yml` 和 `.env.example`
- `.env.example` 中敏感信息使用占位符（如 `your_password_here`）
- README 中添加服务说明，包括端口、账号、使用方法
- 遵循现有的目录命名规范

---

## 📄 许可证

本项目采用 [MIT License](https://opensource.org/licenses/MIT) 开源协议。

---

## 📚 参考资源

- [Docker 官方文档](https://docs.docker.com/)
- [Docker Compose 文档](https://docs.docker.com/compose/)
- [Docker Hub 镜像仓库](https://hub.docker.com/)

---

## 🚢 为什么叫 CodeArk？

**CodeArk (代码方舟)** 的寓意：
- 🚢 **方舟**：承载所有开发所需的工具和环境
- 🌊 **拯救**：将开发者从环境配置的苦海中解救出来
- ⚡ **即用**：靠岸即用，一行命令启航
- 🌍 **普世**：跨平台，处处运行

---

## ⭐ Star History

如果这个项目对你有帮助，请给一个 ⭐ Star 支持一下！

[![Star History Chart](https://api.star-history.com/svg?repos=StephenQiu30/code-ark&type=Date)](https://star-history.com/#StephenQiu30/code-ark&Date)

---

## 📮 联系方式

- **GitHub Issues**: [提交问题](https://github.com/StephenQiu30/code-ark/issues)
- **Pull Requests**: 欢迎提交 PR

---

<div align="center">

**Made with ❤️ by [StephenQiu30](https://github.com/StephenQiu30)**

**拯救开发者脱离环境配置的苦海** 🌊

</div>
