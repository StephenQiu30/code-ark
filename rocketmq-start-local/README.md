# RocketMQ 本地启动服务

使用 Docker Compose 快速启动 RocketMQ 单机环境。

## 服务组件

| 服务 | 端口 | 说明 |
|------|------|------|
| NameServer | 15876 | 服务注册发现中心 |
| Broker | 15909, 15911, 15912 | 消息代理服务 |
| Console | 18180 | 可视化管理控制台 |

## 快速开始

### 启动服务

```bash
./start.sh
```

或者直接使用：

```bash
docker compose up -d
```

### 停止服务

```bash
./stop.sh
```

### 查看日志

```bash
docker compose logs -f
```

### 完全删除（包括数据卷）

```bash
docker compose down -v
```

## 访问地址

- **Console 控制台**: http://localhost:18180
- **NameServer**: localhost:15876

## 配置说明

### Broker 配置

配置文件位于 `./conf/broker.conf`，主要配置项：

- `brokerClusterName`: 集群名称
- `brokerName`: Broker 名称
- `brokerRole`: ASYNC_MASTER（异步复制 Master）
- `flushDiskType`: ASYNC_FLUSH（异步刷盘）

### 数据持久化

数据存储在 `./data/broker/` 目录下：

```
./data/broker/
├── logs/    # 日志文件
└── store/   # 消息存储
```

## 应用程序连接配置

### Spring Boot 配置示例

```yaml
rocketmq:
  name-server: localhost:15876
  producer:
    group: my-producer-group
```

### 原生客户端示例

```java
DefaultMQProducer producer = new DefaultMQProducer("my-producer-group");
producer.setNamesrvAddr("localhost:15876");
producer.start();
```

## 常见问题

### Broker 无法连接 NameServer

检查网络连接状态：

```bash
docker compose logs broker
```

### 端口冲突

如果端口被占用，可以修改 `docker-compose.yml` 中的端口映射：

```yaml
ports:
  - "15876:9876"  # 左侧端口可改为其他值
```

### 重置数据

```bash
# 停止服务
docker compose down

# 删除数据
rm -rf ./data/broker/*

# 重新启动
./start.sh
```
