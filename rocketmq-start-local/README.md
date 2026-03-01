<div align="center">

# 🚀 RocketMQ 本地启动服务

[![Docker](https://img.shields.io/badge/Docker-Ready-blue.svg)](https://www.docker.com/)
[![RocketMQ](https://img.shields.io/badge/RocketMQ-5.3.1-orange.svg)](https://rocketmq.apache.org/)

**Apache RocketMQ 单机环境快速部署方案**

</div>

---

## 📖 目录

- [简介](#简介)
- [服务架构](#服务架构)
- [快速开始](#快速开始)
- [配置说明](#配置说明)
- [应用接入](#应用接入)
- [常见问题](#常见问题)

---

## 简介

[Apache RocketMQ](https://rocketmq.apache.org/) 是一款出身阿里的开源分布式消息中间件，经历了多年双十一的考验。

### 核心特性

- ✅ **高吞吐量**：单机可达十万级 TPS
- ✅ **高可用**：支持主从复制，故障自动切换
- ✅ **消息可靠性**：同步刷盘保证消息不丢失
- ✅ **消息追溯**：支持消息轨迹查询
- ✅ **定时消息**：支持延时投递
- ✅ **事务消息**：支持分布式事务

---

## 服务架构

```
┌─────────────┐      ┌─────────────┐      ┌─────────────┐
│   NameServer │◄─────┤   Broker    │◄─────┤  Console    │
│    (注册中心)  │      │  (消息代理)   │      │  (控制台)    │
└─────────────┘      └─────────────┘      └─────────────┘
      ▲                                            │
      └────────────────────────────────────────────┘
                  生产者 / 消费者 连接
```

### 端口说明

| 组件 | 端口 | 说明 |
|------|:----:|------|
| NameServer | **15876** | 服务注册与发现 |
| Broker | 15909 | Broker 内部通信 |
| Broker | **15911** | 消息收发主端口 |
| Broker | 15912 | HA 高可用端口 |
| Console | **18180** | Web 管理控制台 |

> 使用非常用端口，避免与系统服务冲突

---

## 快速开始

### 前置要求

- Docker Desktop 已安装并运行

### 启动服务

```bash
cd rocketmq-start-local
./start.sh
```

### 验证启动

```bash
# 查看服务状态
docker compose ps

# 查看日志
docker compose logs -f
```

### 访问控制台

打开浏览器访问: **http://localhost:18180**

---

## 配置说明

### Broker 配置

配置文件位置: `./conf/broker.conf`

| 配置项 | 默认值 | 说明 |
|--------|--------|------|
| `brokerClusterName` | DefaultCluster | 集群名称 |
| `brokerName` | broker-a | Broker 名称 |
| `brokerId` | 0 | 0=Master, >0=Slave |
| `autoCreateTopicEnable` | true | 自动创建 Topic |
| `autoCreateSubscriptionGroup` | true | 自动创建订阅组 |
| `brokerRole` | ASYNC_MASTER | 异步复制 Master |
| `flushDiskType` | ASYNC_FLUSH | 异步刷盘 |

### 数据持久化

```
./data/broker/
├── logs/    # 运行日志
└── store/   # 消息存储
```

---

## 应用接入

### Spring Boot 配置

**1. 添加依赖**
```xml
<dependency>
    <groupId>org.apache.rocketmq</groupId>
    <artifactId>rocketmq-spring-boot-starter</artifactId>
    <version>2.3.1</version>
</dependency>
```

**2. 配置文件**
```yaml
rocketmq:
  name-server: localhost:15876
  producer:
    group: my-producer-group
    send-message-timeout: 3000
    retry-times-when-send-failed: 2
```

**3. 发送消息**
```java
@Service
public class MessageService {
    @Resource
    private RocketMQTemplate rocketMQTemplate;

    public void sendMessage(String message) {
        rocketMQTemplate.convertAndSend("test-topic", message);
    }
}
```

**4. 消费消息**
```java
@Service
@RocketMQMessageListener(
    topic = "test-topic",
    consumerGroup = "my-consumer-group"
)
public class MessageListener implements RocketMQListener<String> {
    @Override
    public void onMessage(String message) {
        System.out.println("收到消息: " + message);
    }
}
```

### 原生客户端

```java
// 生产者
DefaultMQProducer producer = new DefaultMQProducer("my-producer-group");
producer.setNamesrvAddr("localhost:15876");
producer.start();

Message msg = new Message("test-topic", "Hello RocketMQ".getBytes());
SendResult result = producer.send(msg);

// 消费者
DefaultMQPushConsumer consumer = new DefaultMQPushConsumer("my-consumer-group");
consumer.setNamesrvAddr("localhost:15876");
consumer.subscribe("test-topic", "*");
consumer.registerMessageListener((MessageListenerConcurrently) (msgs, context) -> {
    System.out.println("收到消息: " + msgs);
    return ConsumeConcurrentlyStatus.CONSUME_SUCCESS;
});
consumer.start();
```

---

## 常见问题

### Q: Broker 无法连接 NameServer？

**A**: 检查容器网络是否正常:
```bash
docker compose logs broker
```

### Q: 消息发送失败？

**A**: 确认应用连接的 NameServer 地址为 `localhost:15876`

### Q: 如何清理数据？

**A**:
```bash
# 停止服务
docker compose down

# 删除数据
rm -rf ./data/broker/*

# 重新启动
./start.sh
```

### Q: 控制台无法访问？

**A**: 等待约 30 秒，让服务完全启动后再访问

---

## 管理命令

```bash
# 启动服务
./start.sh

# 停止服务
./stop.sh

# 查看日志
docker compose logs -f

# 重启服务
docker compose restart

# 完全删除
docker compose down -v
```

---

## 参考资源

- [RocketMQ 官方文档](https://rocketmq.apache.org/docs/)
- [RocketMQ GitHub](https://github.com/apache/rocketmq)
- [Spring Cloud RocketMQ](https://github.com/apache/rocketmq-spring)

---

<div align="center">

**Made with ❤️ using Docker & RocketMQ**

</div>
