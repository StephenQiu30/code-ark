# Canal 本地增量订阅

Canal Server 复用仓库中的 MySQL 8.4，通过 MySQL binlog 提供增量订阅 TCP 服务。启动前需启用 MySQL ROW binlog 并创建具有复制权限的专用账号。

[项目首页](../../README.md) · [服务索引](../README.md) · [快速启动](#快速启动) · [连接入口](#端口与访问入口) · [配置说明](#配置说明)

<details>
<summary>English Summary</summary>

This directory runs Canal Server v1.1.8 and reuses the repository's MySQL service. It reads MySQL row-based binlogs and exposes Canal's TCP subscription endpoint. Configure MySQL binlogging and a dedicated replication user before startup.

</details>

## 服务简介

本服务仅运行 Canal Server 单机实例，默认 destination 为 `example`。数据源使用 `mysql-start-lcoal` 中的 MySQL，容器通过外部网络 `mysql-start-lcoal_default` 访问 `mysql:3306`。不另起 MySQL、Kafka 或 Canal Admin。

## 组件与端口

| 组件 | 端口 | 说明 |
| --- | --- | --- |
| Canal Server | `11111` | Canal TCP 订阅协议；宿主机只绑定 `127.0.0.1` |

宿主机 Canal 客户端连接 `127.0.0.1:11111`；同一 Docker 网络的容器连接 `canal-server:11111`。

## 前置条件

- Docker Engine / Docker Desktop 和 Docker Compose v2
- 先启动仓库 MySQL：在 `mysql-start-lcoal` 中执行 `docker compose up -d`
- MySQL 使用 ROW binlog，并已创建 Canal 专用复制账号

MySQL 服务首次启动或重建容器时会应用 `services/mysql-start-lcoal/docker-compose.yml` 中的 binlog 参数。已存在的数据目录会保留。创建专用账号（将占位密码替换成 `.env` 中实际使用的值）：

```sql
CREATE USER IF NOT EXISTS 'canal'@'%' IDENTIFIED BY 'replace_with_your_password';
GRANT SELECT, REPLICATION SLAVE, REPLICATION CLIENT ON *.* TO 'canal'@'%';
```

从仓库根目录进入 MySQL，在容器中交互执行 SQL：

```bash
cd services/mysql-start-lcoal
docker compose exec -it mysql mysql -uroot -p
```

## 快速启动

以下进入目录的命令从仓库根目录执行；已有 `.env` 时保留原文件。

```bash
cd services/canal-start-local
cp .env.example .env
```

编辑 `.env`，填写 MySQL 复制账号密码；然后启动：

```bash
docker compose up -d
docker compose ps -a
docker compose logs -f canal-server
```

日志出现 Canal instance `example` 启动成功后，客户端可连接 `127.0.0.1:11111` 并订阅 `example`。

## 配置说明

| 变量 | 默认值 | 用途 |
| --- | --- | --- |
| `CANAL_MYSQL_ADDRESS` | `mysql:3306` | 外部 MySQL 在共享 Docker 网络内的地址 |
| `CANAL_MYSQL_USER` | `canal` | MySQL 复制账号 |
| `CANAL_MYSQL_PASSWORD` | 占位值 | MySQL 复制账号密码 |
| `CANAL_FILTER_REGEX` | `.*\\..*` | 订阅库表正则，默认匹配全部库表 |

默认地址要求 Canal 和 MySQL 使用仓库的外部网络 `mysql-start-lcoal_default`。不要在 Canal 容器配置中使用 `localhost` 指向 MySQL。修改订阅范围时按 Canal 正则规则填写库名和表名。MySQL 复制账号密码请使用不超过 32 个字符的值，以满足 Canal 的复制注册协议限制。

## 数据持久化与清理

Canal 不存储 MySQL 业务数据；业务数据及 binlog 由 `services/mysql-start-lcoal/mysql-data` 持久化。当前单机快速启动配置使用容器内默认的位点/元数据管理，删除并重建 Canal 容器不会删除 MySQL 数据，但 Canal 消费位点可能重新初始化。需要稳定保存订阅位点时，应配置持久化元数据或由下游客户端自行保存消费位点。

```bash
docker compose stop
docker compose down
```

以上命令只停止或移除 Canal 项目容器与网络，不会停止、删除或清理 PostgreSQL/MySQL 数据。

## 常用命令

```bash
docker compose config --quiet
docker compose up -d
docker compose ps -a
docker compose logs -f canal-server
docker compose stop
docker compose down
```
