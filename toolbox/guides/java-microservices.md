# Java微服务：Nacos注册/配置与Sentinel限流

基础设施：MySQL、Redis、Nacos、Sentinel。真实微服务由应用提供。本教学额外运行一个短期Java HTTP服务，不能把启动控制台当作微服务已经接入。

客户端精确锁定Nacos3.3.0、Sentinel1.8.10。构建要求JDK17/Maven3.9.16，当前SDK真实运行验收pending。按[输入说明](../examples/README.md)设置Nacos端口/认证、Sentinel Dashboard实际端口和应用广告地址；本机示例的`CODEARK_APP_HOST=127.0.0.1`，容器模式必须用同网络真实别名。

```sh
java -cp "examples/java/nacos-sentinel/target/*" io.codeark.tutorial.Main
```

示例拒绝覆盖已存在的同run_id配置/服务，在`CODEARK_TUTORIAL`分组发布并读回专属配置，注册实际短期HTTP服务；再用Sentinel的真实入口限制该服务的请求，确认至少一次HTTP429。

预期输出：`NACOS_CONFIG_OK`、`NACOS_REGISTRATION_OK`、`SENTINEL_HTTP_429_OK`。只在各项实际验证后打印，不以Dashboard首页200代替。Nacos gRPC偏移端口必须与部署拓扑一致；控制台可访问并不证明SDK已连接。

清理仅移除本次新教学配置、服务注册与本进程限流规则，关闭短期HTTP服务，不改变其他应用规则/注册。不要复用既有业务服务名。参见[网络](network.md)、[数据与恢复](storage.md)和[指南目录](README.md)。
