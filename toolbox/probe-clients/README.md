# 协议就绪客户端

工作箱把每种协议的客户端放在独立 classpath 中。运行时先验证包内文件白名单与 SHA，再复制到当前 control 的内容摘要缓存，重新验证后只读挂载给镜像锁中的 `probe-jre17`；已发布的缓存目录不原地替换，原随包文件仍是唯一来源。用户电脑只需 Docker。探针通过当前托管组件的网络命名空间连接服务，输入经 stdin 送入，固定结果写入 stdout。结果包含探针 ID、组件 ID、采样时间、成功状态和固定错误码。

维护者用 JDK 17 和 Maven 3.9.16 从当前目录构建发行资源。Maven 在编译前校验这两项；`--release 17` 不能保证其他 JDK 的编译字节及 JAR manifest 与已审查锁一致，切换 JDK 后应重新执行 `clean package`。

```sh
mvn -B -ntp clean package
python3 bundle.py
python3 tests/client_contract.py
```

`bundle.py` 校验 `artifacts.lock.json` 中逐个 JAR 的 SHA-256 后将九个模块放入 `target/bundle/`。安装包将该目录映射为 `probe-clients/`。维护者更换 SDK、编译器或客户端源码后，应检查依赖和实际协议结果，再通过 `python3 bundle.py --record` 写入新锁。使用 BuildKit 构建时，通过 `BUILDER_IMAGE` 传入对应平台的 Maven/JDK17 manifest，导出 `artifacts` 阶段并使用同一份锁验证结果。镜像引用见 [构建配方](../image-builds/xxjob/recipe.json)。

| 模块 | 只读连接或检查 |
| --- | --- |
| amqp | 通过账号登录指定 vhost 并打开 AMQP channel |
| s3 | 使用签名请求读取授权桶列表 |
| kafka | 读取 broker 元数据及 cluster ID |
| rocket-route | 通过 NameServer 获取已注册 broker 地址 |
| rocket-proxy | SDK 按本地预设的明文 HTTP/2 入口建立 gRPC producer 会话，显式等待服务端 settings（PRODUCER/publishing）和心跳均返回 OK 后关闭；不访问系统 Topic、不创建 Topic 或发送消息，Broker 路由由独立 NameServer 探针检查 |
| canal | 订阅专用 `codeark_probe` destination，取一个未确认批次；仅回滚本客户端取得的批次 |
| seata | 与协调器完成 TM 注册握手并断开 |
| nacos | 认证、读取配置并确认 SDK 连接状态 |
| mysql | 在已验证的绑定容器网络中，用下游账号核对数据库身份、专属表/种子或复制前提 |

Temporal 使用服务锁中的同版本 Admin Tools 执行 cluster health 和 namespace describe。原生 Rust 执行 PostgreSQL、MySQL、Redis、HTTP 及各管理端的后台 API 探针。探针的进程状态、连接范围和成功条件见[编排设计](../../docs/design/02-跨平台Docker探测与自动化编排设计.md)。
