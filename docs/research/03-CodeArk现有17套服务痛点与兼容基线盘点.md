# Code Ark 现有 17 套服务痛点与兼容基线盘点

核查日期：2026-11-12（Asia/Shanghai）。状态：**仓库源码现状盘点（S）**。基于对当时仓库根目录的 18 个服务目录（17 个 Compose 服务 + 1 个宿主机工具指南）的 `docker-compose.yml`、`.env.example`、`README.md` 与脚本逐一核验。

审查后校准：源码事实继续保留在本页；桌面运行合同以当前design为准。Nacos固定命名卷不等于external卷，源码兼容项目名也不再作为托管运行名。

目录说明（2026-11-12 增量）：上述核查保留历史基线；18 个目录现统一迁入 `services/`，本文链接已更新到新位置，Compose 内容不因迁移改变。完整服务索引见 [services/README.md](../../services/README.md)。

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
| `nacos-start-local` | `nacos` | `8840` / `8848` / `9848–9850` | 是 | Compose自动创建固定名称卷 `nacos-start-local-data`（无 `external: true`）+ 有效Base64密钥 | 固定卷跨项目引用风险；同名容器现场观察仅是当时状态，不代表当前运行事实 |
| `sentinel-start-local` | `sentinel` | `8858` / `8719` | 是 | 无 | 端口已通过 `.env` 参数化；自带 `start.sh` |
| `seata-start-local` | `seata` | `7091` / `8091` | 是 | 依赖 MySQL + `seata-config/application.yml` + 手动建 4 张元数据表 | 仓库**未自带 Seata SQL 文件**；`application.yml` 中 JDBC URL 硬编码 `:3306` 且默认走 `host.docker.internal` |
| `elastic-start-local` | `elastic` | `9200` / `5601` / `5044` / `5000` / `9600` | 是 | 依赖 `start.sh` 做 6 GB 内存/磁盘检查、单线程构建 IK 镜像、License 续期 | Windows 原生无法直接跑 `start.sh`；`5000`/`9600` 端口硬编码 |
| `monitoring-start-local` | `monitoring` | `19090` / `13000` | 可选 | 依赖 `prometheus/` 与 `grafana/` 预配置 | `GF_SERVER_ROOT_URL=http://localhost:13000` 与端口硬编码 |
| `rsshub-start-local` | `rsshub` | `1200` | 可选 | 无 | 顶层指定 `name: rsshub`；支持 `RSSHUB_PORT` |
| `searxng-start-local` | `searxng` | `8888` | 是 | 必填 `SEARXNG_SECRET`（`.env.example` 为空）+ 挂载 `settings.yml` | 顶层指定 `name: searxng`；不填密钥直接 `docker compose up` 会失败 |
| `mediacrawler-start-local` | `mediacrawler`（宿主机指南） | — | — | 不提供 Docker Compose，需 Git + `uv` 拉取固定 commit 并应用两份 `.patch` | 应放在工作箱“教程与宿主机工具”页签，避免误作为容器卡片 |

## 2. 盘点出的核心设计约束（I）

1. **不能假定直接修改源码17个Compose文件**：工作箱构建期提取模板，运行时生成独立实例项目/容器/网络/卷名和完整覆盖层。源码 `sourceProject` 只用于识别旧CLI实例，不能作为托管运行名；端口/JDBC/路由等覆盖以当前design为准。
2. **必须解决 Linux 跨容器连 MySQL 的通病**：`seata` 与 `xxjob` 默认连接 `host.docker.internal`，在 Linux Docker Engine 下遇到绑定 `127.0.0.1:3306` 的 MySQL 会超时失败，通过工作箱 Override 层接入 `mysql-start-lcoal_default` 网络改为 `mysql:3306` 可三端一致打通。
3. **重置操作不能只删本地目录**：`temporal`、`xxjob`、`seata` 的真实业务状态都在共享 PostgreSQL/MySQL 中，设计上必须区分“独立目录清理”与“共享库中仅清空专属数据库”。

[返回调研导航](00-调研导航.md)
