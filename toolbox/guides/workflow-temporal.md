# Temporal：基础设施与业务Worker分开

本场景复用PG并启动Temporal；Namespace可见仅表示基础设施可访问，不表示Workflow已有人执行。应用自己的Worker由应用提供。

教学源码锁定Temporal Java SDK1.31.0，要求JDK17/Maven3.9.16。按[输入说明](../examples/README.md)设置真实gRPC HOST/PORT与已存在的Namespace；新Namespace保留时间默认3天，以实际后端配置为准。不能把UI端口当gRPC端口。

```sh
java -cp "examples/java/temporal/target/*" io.codeark.tutorial.Main
```

本示例同时运行测试Worker、Workflow和Activity。任务队列与Workflow ID均为`codeark_tutorial_<run_id>`。它让Worker真实执行Activity并返回`Hello, CodeArk`，仅在客户端收到匹配结果后打印`TEMPORAL_WORKER_OK`。运行验收当前pending；仅Namespace describe成功不能替代该输出。

示例退出时关闭测试Worker/客户端，不删除Namespace、Temporal/PG容器或数据库。教学Workflow历史按Namespace保留时间回收；UI中只筛选本次ID核查，不清空整个Namespace。服务重开后确认历史与已有Namespace保留，Schema初始化与Server版本一致，不能随意修改已初始化分片数或降级迁移后的Schema。

参见[网络](network.md)、[恢复](storage.md)、[指南目录](README.md)。
