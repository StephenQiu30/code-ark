# Temporal 本地开发环境

复用现有 PostgreSQL，启动 Temporal Server 与 UI，在本地接入持久化工作流并查看执行状态。

[项目首页](../README.md) · [快速启动](#快速启动) · [连接入口](#组件与端口) · [配置说明](#配置说明)

<details>
<summary>English summary</summary>

This Compose project runs Temporal Server 1.31.0 and UI 2.49.1 using the existing
PostgreSQL service in `pgsql-start-local`. Start PostgreSQL first, copy
`.env.example` to `.env`, set its existing database credentials, then run
`docker compose up -d`. The gRPC endpoint is `127.0.0.1:7233`, the UI is
`http://127.0.0.1:18080`, and the namespace is `default`.

</details>

## 服务简介

Temporal 提供持久化 Workflow 执行、重试和执行状态查询。本目录复用现有 PostgreSQL，只增加 Server、UI 和必要的 Schema / Namespace 初始化任务。应用自行启动 Workflow / Activity Worker，并配置相同 Namespace 和 Task Queue。

初始化方式参考 [Temporal 官方 PostgreSQL Compose 示例](https://github.com/temporalio/samples-server/blob/main/compose/docker-compose-postgres.yml)，使用 `temporalio/server` 和同版本 `temporalio/admin-tools`。Visibility 同样存储在 PostgreSQL。

## 组件与端口

| 组件 | 地址 | 说明 |
| --- | --- | --- |
| Temporal Server | `127.0.0.1:7233` | SDK / CLI gRPC 接口 |
| Temporal UI | `http://127.0.0.1:18080` | Workflow、Task Queue、Namespace 查询 |
| 现有 PostgreSQL | 网络内 `pgsql-start-dev:5432` | 复用，不新增数据库容器或端口 |
| Schema / Namespace 初始化 | 无 | 成功后显示 `Exited (0)`，属于正常状态 |

## 前置条件

- 已安装 Docker 和支持 `docker compose` 的 Compose v2 及以上版本。
- `pgsql-start-local/` 已启动，网络 `pgsql-start-local_default` 已存在。
- 填写实际可用的 PostgreSQL 账号与密码；已有数据目录中的账号可能与后来修改的 `.env` 不同。账号需拥有 Temporal 两个数据库的 Schema 权限。
- 首次使用时，账号需有创建数据库权限，或由管理员预先创建 `temporal`、`temporal_visibility` 并将其所有者设置为该账号。
- `7233`、`18080` 未被占用；约 1.5 GB 内存预算，另计现有 PostgreSQL 和其他服务。

## 快速启动

在仓库根目录执行（PostgreSQL 已运行时跳过其启动命令）：

```bash
cd pgsql-start-local
# 首次使用 PostgreSQL 时，按其 README 准备 .env
docker compose up -d

cd ../temporal-start-local
cp .env.example .env
# 编辑 .env，将 PGSQL_USER / PGSQL_PASSWORD 设置为现有 PostgreSQL 的实际可用凭据
docker compose config --quiet
docker compose up -d
docker compose ps -a
```

PowerShell 使用 `Copy-Item .env.example .env` 代替 `cp`；已有 `.env` 时不要覆盖。启动流程会等待数据库端口，创建/迁移 `temporal`、`temporal_visibility`，等待集群健康并创建 Namespace，然后启动 UI。

打开 [Temporal UI](http://127.0.0.1:18080)。宿主机应用连接 `127.0.0.1:7233`，Namespace 默认 `default`。应用容器加入已有 `pgsql-start-local_default` 网络后使用 `temporal-start-dev:7233`。

## 配置说明

| 变量 | 默认值 | 说明 |
| --- | --- | --- |
| `TEMPORAL_VERSION` | `1.31.0` | Server 与 admin-tools 共用版本 |
| `TEMPORAL_UI_VERSION` | `2.49.1` | UI 版本 |
| `PGSQL_DOCKER_NETWORK` | `pgsql-start-local_default` | 现有 PostgreSQL 网络 |
| `TEMPORAL_POSTGRES_HOST` | `pgsql-start-dev` | 现有 PostgreSQL 容器名 |
| `PGSQL_USER` / `PGSQL_PASSWORD` | 示例 / 占位值 | 必须填写现有 PostgreSQL 凭据 |
| `TEMPORAL_BIND_ADDRESS` | `127.0.0.1` | 宿主机监听地址 |
| `TEMPORAL_PORT` / `TEMPORAL_UI_PORT` | `7233` / `18080` | 宿主机端口 |
| `TEMPORAL_NAMESPACE` | `default` | 初始化和 UI 默认 Namespace |
| `TZ` | `Asia/Shanghai` | 容器时区 |

`scripts/setup-postgres.sh` 保留已有数据库并应用 Schema 迁移；`scripts/create-namespace.sh` 保留已有 Namespace，新 Namespace 的保留时间为 3 天。动态配置使用 `dynamicconfig/development-sql.yaml`。Server 使用 4 个 History 分片、1 GB 内存上限，UI 使用 256 MB 上限；常驻服务自动重启并轮转日志。

Schema 初始化优先使用已有数据库；不存在时尝试创建，创建失败会退出并输出错误。账号没有创建数据库权限时，由管理员执行以下 SQL，替换 `your_existing_user` 为该账号；无需改变其全局权限：

```sql
CREATE DATABASE temporal OWNER your_existing_user;
CREATE DATABASE temporal_visibility OWNER your_existing_user;
```

默认接口用于本地开发，没有配置认证/TLS。镜像升级前检查 Schema 兼容性并备份数据库，已迁移的数据库不能直接降级 Server；初始化后不要随意修改 History 分片数。

## 验证与常用命令

在本目录执行：

```bash
docker compose ps -a
docker compose logs --tail=50 temporal-schema-setup temporal-namespace-setup
docker compose logs -f temporal temporal-ui
docker compose run --rm --no-deps --entrypoint temporal temporal-namespace-setup operator cluster health --address temporal:7233
docker compose run --rm --no-deps --entrypoint temporal temporal-namespace-setup operator namespace describe --address temporal:7233 --namespace default
curl -fsS http://127.0.0.1:18080/api/v1/namespaces

docker compose stop
docker compose down
```

PowerShell API 检查使用 `Invoke-RestMethod http://127.0.0.1:18080/api/v1/namespaces`。自定义 Namespace 时替换命令中的 `default`。初始化失败先查日志，确认数据库网络、凭据及权限，不要删除数据掩盖错误。

## 数据持久化与清理

Temporal 数据存储在现有 PostgreSQL 的 `temporal` 和 `temporal_visibility` 数据库，持久化位置为 `pgsql-start-local/pgsql-data`。本目录 `stop`、`down` 或容器重建均保留这些数据，也不会停止 PostgreSQL。

本项目没有数据卷，`docker compose down -v` 也不会清空外部数据库。需要重置时明确操作上述两个数据库；不要删除共享 PostgreSQL 数据目录或影响其他业务数据库。
