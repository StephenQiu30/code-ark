# 搜索与观测：Elastic / Prometheus

启动时明确选择Elastic、Monitoring或两者。业务日志采集器、真实采集目标由应用提供；控制台页面可打开不表示后台已连接。ES/IK原生归档未具备匿名可获取SHA/image ID/平台证据时仍会阻断候选，不能要求新手在宿主机安装Maven构建搜索引擎。

教学HTTP客户端使用JDK17内置java.net.http（release17），Maven3.9.16构建。清单的17.0.0表示Java SE17 API基线，并非本次机器的JDK补丁号；真实运行证据必须另记实际JDK补丁版本。读取已应用服务API endpoint，按需设置认证输入，见[共同输入](../examples/README.md)。

```sh
java -cp "examples/java/http-query/target/*" io.codeark.tutorial.Main elastic
java -cp "examples/java/http-query/target/*" io.codeark.tutorial.Main prometheus
```

只执行对应服务的一行。Elastic示例创建全新`codeark_tutorial_<run_id>`索引、写入一条教学日志、执行真实_search并核对结果，最后仅删除这个新索引；真实验证后输出`ELASTIC_QUERY_OK`。同名已存在索引会拒绝，不覆盖业务索引。

Prometheus通过`/api/v1/query`执行`vector(1)`并核对成功结果后输出`PROMETHEUS_QUERY_OK`；这是指标查询API测试，不声称业务采集目标已经配置。两条预期输出当前均pending，不能用文案当执行证据。

清理只针对本次新教学索引；指标查询不写入数据、不删除采集目标或Grafana仪表盘。查看[数据保护](storage.md)、[网络](network.md)和[指南目录](README.md)。
