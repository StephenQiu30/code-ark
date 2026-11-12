# Code Ark 现有 17 套服务痛点与兼容基线盘点

核查日期：2026-11-12（Asia/Shanghai）。状态：**仓库源码现状盘点（S）**。基于对仓库根目录 18 个服务目录（17 个 Compose 服务 + 1 个宿主机工具指南）的 `docker-compose.yml`、`.env.example`、`README.md` 与脚本逐一核验。

## 1. 全量服务盘点表（S）

| 目录名 | 建议标准 ID | 默认端口 | 是否需 `.env` | 隐藏依赖 / 前置步骤 | 特殊风险与跨平台卡点 |
| --- | --- | --- | --- | --- | --- |
| `pgsql-start-local` | `pgsql` | `5432` | 是 | 初始化脚本 `pgsql-init/01-init.sql`（`uuid-ossp`） | 容器名 `pgsql-start-dev`，网络 `pgsql-start-local_default` |
| `mysql-start-lcoal` | `mysql` | `3306` | 是 | 默认开启 `ROW` binlog 与 `mysql_native_password` | 目录历史拼写 `lcoal`；端口硬编码无 env 变量 |
| `redis-start-local` | `redis` | `6379` | 否 | 无 | 端口硬编码无 env 变量 |
| `minio-start-local` | `minio` | `9000` / `9001` | 是 | 无 | 镜像 tag 为 `latest`；端口硬编码 |
| `kafka-start-local` | `kafka` | `9092` / `19000` | 否 | Kafka UI 依赖 Kafka 健康 | `KAFKA_ADVERTISED_LISTENERS` 硬编码 `localhost:9092` |
| `canal-start-local` | `canal` | `11111` | 是 | 强依赖 `mysql` + 外部网络 `mysql-start-lcoal_default` + 手动建复制账号 SQL | MySQL 复制账号密码协议限制 **<= 32 字符** |
| `rabbitmq-start-lcoal` | `rabbitmq` | `5672` / `15672` | 是 | 无 | 目录历史拼写 `lcoal`；端口硬编码 |
| `rocketmq-start-local` | `rocketmq` | `15876` / `15909` / `15911` / `15912` / `18180` | 否 | 依赖 `conf/broker.conf` | `broker.conf` 中 `brokerIP1 = broker`，宿主机直连需改为宿主机 IP 并同步映射端口；自带 `.sh` |
| `temporal-start-local` | `temporal` | `7233` / `18080` | 是 | 强依赖 `pgsql` + 外部网络 `pgsql-start-local_default` + 同账号密码 | 挂载 `scripts/*.sh`（需 LF 换行）；如 PG 已由旧密码初始化容易认证失败 |
| `xxjob-start-local` | `xxjob` | `18081` | 是 | 依赖 MySQL + 手动导入 `tables_xxl_job.sql` | 默认连 `host.docker.internal:3306`（Linux 下连 `127.0.0.1` 绑定会失败）；SQL 顶部硬编码 `USE xxl_job;` |
| `nacos-start-local` | `nacos` | `8840` / `8848` / `9848–9850` | 是 | README 要求先建卷 `nacos-start-local-data` + 有效 Base64 密钥 | 本机现场已观察到外部同名容器 `nacos-standalone` 正在运行（O） |
| `sentinel-start-local` | `sentinel` | `8858` / `8719` | 是 | 无 | 端口已通过 `.env` 参数化；自带 `start.sh` |
| `seata-start-local` | `seata` | `7091` / `8091` | 是 | 依赖 MySQL + `seata-config/application.yml` + 手动建 4 张元数据表 | 仓库**未自带 Seata SQL 文件**；`application.yml` 中 JDBC URL 硬编码 `:3306` 且默认走 `host.docker.internal` |
| `elastic-start-local` | `elastic` | `9200` / `5601` / `5044` / `5000` / `9600` | 是 | 依赖 `start.sh` 做 6 GB 内存/磁盘检查、单线程构建 IK 镜像、License 续期 | Windows 原生无法直接跑 `start.sh`；`5000`/`9600` 端口硬编码 |
| `monitoring-start-local` | `monitoring` | `19090` / `13000` | 可选 | 依赖 `prometheus/` 与 `grafana/` 预配置 | `GF_SERVER_ROOT_URL=http://localhost:13000` 与端口硬编码 |
| `rsshub-start-local` | `rsshub` | `1200` | 可选 | 无 | 顶层指定 `name: rsshub`；支持 `RSSHUB_PORT` |
| `searxng-start-local` | `searxng` | `8888` | 是 | 必填 `SEARXNG_SECRET`（`.env.example` 为空）+ 挂载 `settings.yml` | 顶层指定 `name: searxng`；不填密钥直接 `docker compose up` 会失败 |
| `mediacrawler-start-local` | `mediacrawler`（宿主机指南） | — | — | 不提供 Docker Compose，需 Git + `uv` 拉取固定 commit 并应用两份 `.patch` | 应放在工作箱“教程与宿主机工具”页签，避免误作为容器卡片 |

## 2. 盘点出的核心设计约束（I）

1. **不能假定直接修改根目录 17 个 Compose 文件**：现有 CLI 用户仍在按根目录 `README.md` 使用原有配置；工作箱应在构建期复制蓝图并在运行时通过固定 `-p <project>` 与 `compose.toolbox.override.yml` 解决端口冲突、Linux MySQL 连通性与 RocketMQ 宿主机模式。
2. **必须解决 Linux 跨容器连 MySQL 的通病**：`seata` 与 `xxjob` 默认连接 `host.docker.internal`，在 Linux Docker Engine 下遇到绑定 `127.0.0.1:3306` 的 MySQL 会超时失败，通过工作箱 Override 层接入 `mysql-start-lcoal_default` 网络改为 `mysql:3306` 可三端一致打通。
3. **重置操作不能只删本地目录**：`temporal`、`xxjob`、`seata` 的真实业务状态都在共享 PostgreSQL/MySQL 中，设计上必须区分“独立目录清理”与“共享库中仅清空专属数据库”。

[返回调研导航](00-调研导航.md)
