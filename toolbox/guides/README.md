# 离线指南与教学源码

当前是源码候选，尚未发布安装包；以下预期输出不是已执行证据。先完成应用内置连接测试，再选择是否安装业务开发工具。基础设施启动不要求 Git、Node、Rust、JDK 或 Maven。

- [安装与首次打开](environment.md)
- [网络、实际连接与容器示例](network.md)
- [存储、备份、恢复与工作区](storage.md)
- [MySQL / Redis / S3](backend-mysql.md)
- [PostgreSQL / Redis / S3](backend-pgsql.md)
- [Nacos / Sentinel](java-microservices.md)
- [Seata / XXL-Job](java-distributed.md)
- [Temporal Worker](workflow-temporal.md)
- [Canal 与消息系统](cdc-messaging.md)
- [Elastic / Prometheus](search-observability.md)
- [MediaCrawler 宿主机工具](mediacrawler.md)
- [教学源码构建与输入](../examples/README.md)
- [维护者CI与发行门禁](maintainers-ci.md)

`manifest.json` 使用与 Rust 相同的 GuideSpec；`status=pending` 表示对应真实 SDK、服务和最终安装包测试尚未完成。candidate 校验只证明资源完整，release 必须提供同一份 manifest/sourceDigest 的外部逐目标运行证据。不要把编译通过改写成真实服务通过。
