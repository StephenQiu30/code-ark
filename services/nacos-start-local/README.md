# Nacos 本地开发环境

启动单机 Nacos 3.3.0，在本地验证服务注册、配置管理和客户端接入。Compose 自动创建固定名称持久卷。

[项目首页](../../README.md) · [服务索引](../README.md) · [快速启动](#快速启动) · [连接入口](#端口与访问入口) · [配置说明](#配置说明)

<details>
<summary>English summary</summary>

This directory provides a standalone Nacos server for local service discovery and config testing.
It uses `.env` for token-related configuration, a Docker named volume for database data, and a local directory for logs.
Create `.env` from `.env.example` before the first startup, keeping any existing `.env`. Compose creates the explicitly named volume; it is not declared external.
Nacos 3.3 does not ship a default administrator password. Initialize the `nacos` administrator once through the console or the bootstrap API; existing administrator passwords remain unchanged across restarts.

</details>

## 服务简介

该目录提供单机模式 Nacos，适合本地开发时验证：

- 服务注册与发现
- 配置管理
- Nacos 客户端接入

## 端口与访问入口

| 组件 | 端口 | 说明 |
| --- | --- | --- |
| Nacos Console | `8840` | Web 控制台映射到容器 `8080` |
| Nacos Main Port | `8848` | 主服务端口 |
| Nacos gRPC | `9848` | 客户端通信 |
| Nacos gRPC | `9849` | 客户端通信 |
| Nacos gRPC | `9850` | 客户端通信 |

- 控制台地址：`http://localhost:8840/`
- 管理员账号：`nacos`；新数据卷首次启动后需自行初始化密码，没有内置的默认密码

## 前置条件

- 已安装 Docker 和 Docker Compose
- 首次启动前准备 `.env`，使用有效的 Base64 编码认证密钥
- Compose 自动创建固定名称卷 `nacos-start-local-data`；当前配置没有 `external: true`，不同路径运行也会引用同名卷，启动前确认没有其他实例正在使用它

## 快速启动

以下进入目录的命令从仓库根目录执行；已有 `.env` 时保留原文件。

```bash
cd services/nacos-start-local
# 仅在不存在 .env 时首次复制；已有配置请保留
cp .env.example .env
# 编辑 .env，设置 NACOS_AUTH_TOKEN
docker compose up -d
```

首次使用新数据卷时，打开控制台完成管理员密码初始化；也可使用官方首次初始化 API：

```bash
curl -X POST 'http://127.0.0.1:8848/nacos/v3/auth/user/admin' \
  --data-urlencode 'password=<your-password>'
```

将占位值替换为自行保存的强密码，不要提交到仓库。该 API 的成功响应包含密码，避免将响应收集进日志。已有全局管理员时，API 返回错误码 `409`，不会覆盖密码；保留数据卷的重启继续使用已有密码。

## 配置说明

请在 `.env` 中设置：

- `NACOS_AUTH_TOKEN`

当前 `docker-compose.yml` 已固定以下行为：

- 单机模式启动：`MODE=standalone`
- 认证开启：`NACOS_AUTH_ENABLE=true`
- 默认鉴权管理员用户名：`nacos`；密码由首次初始化设置，保存在 Nacos 数据卷中

Compose 中的 `NACOS_AUTH_USERNAME`、`NACOS_AUTH_PASSWORD` 不能代替上述首次初始化步骤。不要通过重建容器、修改这些环境变量或重复调用初始化 API 尝试重置已有密码。

行为依据：[Nacos 3.3 鉴权手册](https://nacos.io/docs/latest/manual/admin/auth/)、[Nacos 3.3.0 管理员初始化源码](https://github.com/alibaba/nacos/blob/3.3.0/plugin-default-impl/nacos-default-auth-plugin/src/main/java/com/alibaba/nacos/plugin/auth/impl/controller/v3/UserControllerV3.java)。

## 数据持久化与清理

- 数据卷：`nacos-start-local-data`（Docker 管理，避免 Derby 数据库放在 Windows bind mount 上）
- 日志目录：`./nacos/logs`

`docker compose down` 不会删除数据卷。当前卷不是外部卷，`docker compose down -v` 会请求删除它。需要重置 Nacos 时，先确认该固定名称卷的归属、其他使用者并备份；删除会清空其中的 Nacos 配置及注册数据。未来桌面工作箱的实例专属卷方案尚未实现，不改变此处 CLI 配置的行为。

```bash
docker compose down
```

## 常用命令

```bash
docker compose up -d
docker compose logs -f
docker compose stop
docker compose down
```

## 使用说明

- 宿主机应用通常连接 `localhost:8848`
- 控制台登录与客户端 token 是两类配置，请分别处理
- 不要把实际 token 提交到仓库
