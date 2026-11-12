# 可执行教学源码（真实服务验收 pending）

这是业务客户端示例，不是工作箱启动基础设施的宿主机前置。先让应用内置连接测试通过，再自愿安装JDK17或更高的兼容JDK与Maven3.9.16。源码使用Java17 release，SDK精确版本见[锁表](clients.lock.json)。各Maven模块独立classpath，避免Canal/Seata等不同Netty或日志依赖相互污染。

从包含`examples/`与`guides/`的受控教学目录开始：

```sh
mvn -B -ntp -f examples/java/pom.xml package
java -cp "examples/java/sql-mysql/target/*" io.codeark.tutorial.Main
```

`target/*`跨Windows/macOS/Linux使用同一写法；只运行所选模块，不自动运行其他示例。构建/单测不会访问服务；只有显式运行Main才可能连接和写教学数据。未设置确认或输入时必须非零退出，不输出伪成功。

## 共同输入

所有示例都要求`CODEARK_TUTORIAL_CONFIRM=YES`和唯一`CODEARK_RUN_ID`（8–24位小写字母/数字）。这是您对本次教学数据操作的明确选择。每次使用新run_id；资源重名会拒绝采用既有数据。标识前缀为`codeark_tutorial_<run_id>`；SQL数据库名必须恰好等于该值。只使用已应用连接视图的当前端口/网络/凭据。

在您自己的终端会话中设置环境变量。POSIX使用`export 变量名=值`，PowerShell使用`$env:变量名=值`。真实密码不写入源码、可提交文件、命令参数或诊断；有秘密的终端输出不要直接分享。下面列的是变量名，不是可冒充实际连接的默认凭据。

| 模块 | 必须设置的连接输入 |
| --- | --- |
| sql-mysql / sql-pgsql | CODEARK_SQL_HOST、CODEARK_SQL_PORT、CODEARK_SQL_DATABASE、CODEARK_SQL_USERNAME、CODEARK_SQL_PASSWORD |
| s3 | CODEARK_S3_ENDPOINT（http URL）、CODEARK_S3_ACCESS_KEY、CODEARK_S3_SECRET_KEY |
| nacos-sentinel | CODEARK_NACOS_HOST/PORT/USERNAME/PASSWORD、CODEARK_SENTINEL_HOST/PORT、CODEARK_APP_HOST |
| seata-rollback | SQL全部变量及CODEARK_SEATA_HOST/PORT；只在独立空教学数据库操作 |
| xxl-executor | CODEARK_ENDPOINT_MODE=container、CODEARK_DOCKER_NETWORK、CODEARK_XXL_ADMIN_ENDPOINT、CODEARK_XXL_EXECUTOR_TOKEN、CODEARK_XXL_EXECUTOR_HOST/PORT |
| temporal | CODEARK_TEMPORAL_HOST/PORT、CODEARK_TEMPORAL_NAMESPACE |
| kafka | CODEARK_MQ_HOST/PORT |
| amqp | CODEARK_MQ_HOST/PORT/USERNAME/PASSWORD/VHOST |
| rocketmq | CODEARK_MQ_HOST/PORT（5.x Proxy），预建同run_id的Topic/消费组 |
| canal | CODEARK_CANAL_HOST/PORT/USERNAME/PASSWORD/DESTINATION、CODEARK_SQL_DATABASE；专属destination必须与教学前缀相等 |
| http-query | CODEARK_HTTP_ENDPOINT，ES启用认证时还需CODEARK_HTTP_USERNAME/PASSWORD；模式参数elastic或prometheus |

宿主机HOST仅允许127.0.0.1或localhost。容器模式另需明确`CODEARK_DOCKER_NETWORK`和详情中的同网络别名，见[网络指南](../guides/network.md)。端口不硬编码；草稿值不能替代已应用值。示例不会停止容器、删除卷、变更数据库账号或自动搭建CDC桥接。

## 数据与退出

SQL只在精确匹配run_id的独立教学数据库创建/删除教学表；S3只创建新测试桶/对象并清理；Kafka只创建/删除本次独占Topic；RabbitMQ使用当前连接独占的临时队列；Nacos只写新教学配置/注册并清理。Seata失败时保留教学表供恢复，确认全局事务已终结后才清理；不吞掉失败。Temporal只创建唯一Workflow/task queue，历史按Namespace保留时间清理；不删除Namespace。

Canal只能消费专属教学destination，不能连接应用或`codeark_probe`的固定client_id；只确认本客户端取得的教学批次。RocketMQ的Topic/消费组和XXL教学任务由您按相应指南显式创建/删除。所有SDK真实成功输出目前都未实测：`pending`不能被编译通过或README中的预期文字改成`passed`。

返回[离线指南](../guides/README.md)。
