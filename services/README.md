# 服务索引 · Service Catalog

所有服务集中放在本目录：**17 套 Docker Compose 配置 + 1 份宿主机工具指南**。各服务的 Compose、README、公开样例和初始化文件都随服务放置；完整服务表只在本文件维护。

English Summary: This directory contains 17 local Docker Compose projects and one host-tool guide. Follow each service README for prerequisites, credentials, ports, and data ownership. Run commands from the selected service directory; do not start the whole collection at once. Existing Docker project, network, and volume names remain unchanged by the move.

[项目首页](../README.md) · [English home](../README.en.md) · [结构规范](../PROJECT.md) · [贡献指南](../CONTRIBUTING.md)

## 服务目录

下面列出默认宿主机端口；点击服务名查看凭据、资源需求和启动步骤。端口有冲突时，按对应服务配置调整。

### 数据与存储

| 服务 | 默认端口 | 用途 |
| --- | --- | --- |
| [PostgreSQL 18](./pgsql-start-local/README.md) | `5432` | SQL、关系型数据、初始化脚本 |
| [MySQL 8.4](./mysql-start-lcoal/README.md) | `3306` | 关系型数据与应用联调 |
| [Redis](./redis-start-local/README.md) | `6379` | 缓存、锁与消息实验；AOF 持久化 |
| [MinIO](./minio-start-local/README.md) | `9000` / `9001` | S3 兼容接口与管理控制台 |

### 消息、工作流与调度

| 服务 | 默认端口 | 用途 |
| --- | --- | --- |
| [Kafka + Kafka UI](./kafka-start-local/README.md) | `9092` / `19000` | 事件流、主题与消费者组观察 |
| [Canal Server](./canal-start-local/README.md) | `11111` | 复用 MySQL，订阅 ROW binlog 变更 |
| [RabbitMQ](./rabbitmq-start-lcoal/README.md) | `5672` / `15672` | AMQP 消息与管理控制台 |
| [RocketMQ](./rocketmq-start-local/README.md) | `15876` / `15909` / `15911` / `15912` / `18180` | NameServer、Broker 与 Console |
| [Temporal + UI](./temporal-start-local/README.md) | `7233` / `18080` | 持久化工作流；复用本仓库 PostgreSQL |
| [XXL-Job](./xxjob-start-local/README.md) | `18081` | 调度中心；需 MySQL 与初始化 SQL |

### 治理、搜索与观测

| 服务 | 默认端口 | 用途 |
| --- | --- | --- |
| [Nacos](./nacos-start-local/README.md) | `8840` / `8848` / `9848–9850` | 注册与配置中心；Compose 自动创建固定名称持久卷 |
| [Sentinel](./sentinel-start-local/README.md) | `8858` / `8719` | 流控、熔断与规则实验 |
| [Seata](./seata-start-local/README.md) | `7091` / `8091` | 分布式事务；需 MySQL 元数据表 |
| [Elastic Stack](./elastic-start-local/README.md) | `9200` / `5601` / `5044` | 搜索、IK 分词、日志与 Kibana |
| [Prometheus + Grafana](./monitoring-start-local/README.md) | `19090` / `13000` | 指标采集、告警与仪表盘 |

Logstash 还发布 `5000`（TCP / UDP）和 `9600`，详见服务文档。

### 内容与搜索 API

| 服务 | 默认端口 | 用途 |
| --- | --- | --- |
| [RSSHub](./rsshub-start-local/README.md) | `1200` | RSS 路由；Chromium bundled 镜像 |
| [SearXNG](./searxng-start-local/README.md) | `8888` | 聚合搜索与 JSON API |

另有 [MediaCrawler 固定版本与补丁说明](./mediacrawler-start-local/README.md)，用于宿主机运行的采集工具，不提供 Compose 服务。历史目录名 `mysql-start-lcoal`、`rabbitmq-start-lcoal` 保留以兼容已有使用方式。

## 目录与使用约定

从仓库根目录进入需要的服务，例如 `cd services/redis-start-local`，然后按服务 README 操作。新服务也放到 `services/<service>-start-local/`，保留 `mysql-start-lcoal`、`rabbitmq-start-lcoal` 的历史名称。MediaCrawler 仅提供宿主机指南，不是第 18 套 Compose 服务。

Temporal 先启动同级 PostgreSQL；Canal 复用同级 MySQL；Seata、XXL-Job 需要 MySQL 与元数据表。目录分组不改变 Docker 项目名、显式容器/网络/卷名、服务间网络和容器内地址。不要将 `services` 当作 Compose 项目名，也不要在本目录统一启动全部服务。

## 从旧根目录布局迁移

旧版本将服务直接放在仓库根目录，新版本统一为 `services/<原目录名>/`。服务内部的相对挂载、配置和初始化逻辑保持不变。

已有克隆在更新前，应先在原服务目录停止相关容器并备份数据；更新后核对原目录中被 Git 忽略的 `.env`、数据及日志，将它们逐项迁移到同名新目录，已有目标内容时先比对，禁止覆盖合并或删除旧数据。Git 只迁移受跟踪源码，不能保证自动搬迁本机忽略文件。确认内容和权限完整后，从新服务目录运行 `docker compose up -d --force-recreate`，让容器使用新的绝对绑定路径；仅 `start` 或 `restart` 不会更新绑定路径。

保留原 Compose 项目名（自定义 `-p` 或 `COMPOSE_PROJECT_NAME` 的用户继续使用原值）、网络和命名卷；不执行 `down -v`、全局 prune 或数据库升级。旧目录确认不再被容器引用、数据迁移验证完成后，才考虑清理空目录。重建前后检查客户端连接、初始化退出码和数据保留，未做这些验证不能宣称运行迁移通过。

English migration note: Before updating an existing clone, stop affected containers and back up data. Git moves tracked files only; move ignored local configuration and data separately without overwriting existing targets. Keep the same Compose project name, then recreate containers from the new directory so bind mounts use the new paths. Do not remove volumes or prune data.
