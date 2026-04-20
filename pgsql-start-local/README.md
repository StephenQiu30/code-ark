# PostgreSQL 本地启动

## 功能

- 本地开发数据库环境（PostgreSQL）
- 数据持久化：`./pgsql-data`
- 初始化脚本：`./pgsql-init`

## 端口

| 服务 | 端口 |
|------|:----:|
| PostgreSQL | 5432 |

## 快速启动

```bash
cd pgsql-start-local
docker compose up -d
```

## 连接参数

```yaml
host: localhost
port: 5432
username: see .env
database: see .env
password: see .env
```

## 常用命令

```bash
docker compose logs -f
docker compose down
docker compose down -v
```
