# 后端开发：MySQL / Redis / MinIO

范围：数据库、缓存和对象存储基础设施；应用API由您提供。先在工作箱启动本场景并分别通过内置连接测试，确认三者的已应用generation。Redis推荐入门只做内置真实PING，不需要安装Java。

业务教学使用MySQL Connector/J26.10.0与MinIO Java9.0.4（OkHttp JVM5.5.1），要求JDK17与Maven3.9.16。依[共同输入](../examples/README.md)设置当前连接与唯一run_id；必须先创建/授权仅供教学的`codeark_tutorial_<run_id>`数据库，不能使用已有业务库。

从受控教学目录构建后执行：

```sh
java -cp "examples/java/sql-mysql/target/*" io.codeark.tutorial.Main
java -cp "examples/java/s3/target/*" io.codeark.tutorial.Main
```

SQL示例创建独立表、插入一条记录并查询内容；S3示例拒绝复用同名桶，创建新教学桶/对象后读回。两者仅在真实结果相符时分别打印`MYSQL_QUERY_OK`、`S3_WRITE_READ_OK`。这些是**预期输出，当前运行验收pending**，不是已经成功的证明。

清理：示例成功/常规异常会删除本次教学表与新桶/对象，不删除数据库、不触碰已有桶或其他记录。异常退出后先核对run_id和归属，再按[恢复指南](storage.md)处理。若需容器执行，改用[实际网络/别名](network.md)，不能复制宿主机127.0.0.1。

返回[指南目录](README.md)。
