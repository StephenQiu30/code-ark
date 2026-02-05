# Stephen Cloud 本地开发环境

一键启动微服务开发所需的全部中间件

## 包含服务

| 服务 | 端口 | 用户名 | 密码 | 说明 |
|------|------|--------|------|------|
| MySQL | 3306 | root | stephenqhd30. | 数据库 |
| Redis | 6379 | - | - | 缓存 |
| Nacos | 8840, 8848 | nacos | nacos | 服务注册/配置中心 |
| RabbitMQ | 5672, 15672 | stephen | stephenqhd30. | 消息队列 |
| Elasticsearch | 9200 | elastic | 20x4R1nO | 搜索引擎 |
| Kibana | 5601 | - | - | ES可视化 |
| MinIO | 9000, 9001 | minio | stephenqhd30. | 对象存储 |

## 快速启动

```bash
# 进入目录
cd stephen-cloud-local

# 启动所有服务
docker-compose up -d

# 查看服务状态
docker-compose ps

# 查看日志
docker-compose logs -f

# 停止所有服务
docker-compose down

# 停止并删除数据卷（谨慎使用）
docker-compose down -v
```

## 服务访问

### Nacos 控制台
- URL: http://localhost:8840/nacos
- 用户名: nacos
- 密码: nacos

### RabbitMQ 管理界面
- URL: http://localhost:15672
- 用户名: stephen
- 密码: stephenqhd30.

### Kibana
- URL: http://localhost:5601
- ES用户名: elastic
- ES密码: 20x4R1nO

### MinIO 控制台
- URL: http://localhost:9001
- 用户名: minio
- 密码: stephenqhd30.

## 配置说明

所有配置都在 `.env` 文件中，可以根据需要修改：
- 数据库密码
- 服务端口
- JVM参数
- 存储路径

## 数据持久化

所有服务数据都存储在对应的服务目录下：
- `./mysql-data` - MySQL数据
- `./redis-data` - Redis数据
- `./nacos/data` - Nacos配置数据
- `./rabbitmq-data` - RabbitMQ数据
- `./es-data` - Elasticsearch数据
- `./kibana-data` - Kibana数据
- `./minio-data` - MinIO数据

## 健康检查

所有服务都配置了健康检查，确保服务启动顺序和可用性：

1. MySQL 和 Redis 首先启动
2. Nacos 依赖 MySQL
3. Kibana 依赖 Elasticsearch
4. 其他服务独立启动

## 注意事项

- 确保端口未被占用
- 确保 Docker 有足够的内存（建议至少 4GB）
- ES 版本使用的是 ARM64 版本，如果是 Intel 芯片请修改 `.env` 中的 `ES_LOCAL_VERSION`