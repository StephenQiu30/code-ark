# 数据变更订阅与消息收发

本场景默认MySQL、Canal和Kafka；RabbitMQ或RocketMQ由您明确选择，一次不自动启动所有替代队列。**Canal订阅与消息收发是分别验证的结果，默认不部署Canal→队列桥接。** 可选桥接代码是额外业务应用，当前未随基础设施自动创建，不能宣称完整CDC消息链路已跑通。

客户端锁定Canal1.1.8、Kafka4.4.0、RabbitMQ AMQP5.37.1、RocketMQ5.x Java SDK5.2.2。源码工具为JDK17/Maven3.9.16，真实逐目标SDK运行均pending。共同run_id/网络/实际端口见[源码输入](../examples/README.md)。

## Canal专属订阅

先准备独立教学数据库`codeark_tutorial_<run_id>`与同名专属Canal destination，仅过滤该数据库；ROW binlog和复制账号由已授权初始化配置检查。不要连接业务destination或`codeark_probe`：Canal固定client_id可能影响另一个消费者。`CODEARK_CANAL_DESTINATION`必须精确匹配教学前缀；无认证时显式设置空账号/密码，不能默默使用猜测凭据。

```sh
java -cp "examples/java/canal/target/*" io.codeark.tutorial.Main
```

在另一个终端运行同run_id的SQL教学示例产生变更。Canal仅在30秒内真实收到该教学schema事件并确认自己的批次后打印`CANAL_EVENT_OK`，没有事件则失败。退出只断开此客户端；专属destination与教学表按登记范围显式清理，不清空其他订阅游标。

## 明确选择的队列

```sh
java -cp "examples/java/kafka/target/*" io.codeark.tutorial.Main
java -cp "examples/java/amqp/target/*" io.codeark.tutorial.Main
java -cp "examples/java/rocketmq/target/*" io.codeark.tutorial.Main
```

只运行您选择的一行。Kafka创建全新教学Topic、发送/消费独特消息并删除本次Topic，预期`KAFKA_SEND_RECEIVE_OK`。RabbitMQ使用当前连接独占的临时队列，预期`AMQP_SEND_RECEIVE_OK`。它们不会向业务Topic/队列发送测试数据。

RocketMQ使用当前实际**5.x Proxy** endpoint，不用旧Broker回环广播开关。先在专属教学范围显式创建与run_id相同的Topic和消费组，设置Broker上对应配置；SDK只向该教学Topic发消息并从该组收到相同载荷后打印`ROCKETMQ_PROXY_SEND_RECEIVE_OK`。最后显式删除本次教学Topic/消费组，不能删除系统Topic或其他消费者组。旧Broker SDK限制需另行验证，不作为此示例的兼容承诺。

返回[指南目录](README.md) · [网络与连接](network.md)。
