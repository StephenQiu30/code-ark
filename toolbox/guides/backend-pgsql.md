# 后端开发：PostgreSQL / Redis / MinIO

范围：PG、缓存、对象存储；应用API、Agent和向量扩展由业务应用提供。启动后分别完成内置探针，先复制已应用连接，再运行源码。

客户端：PostgreSQL JDBC42.7.8与MinIO Java9.0.4（OkHttp JVM5.5.1），JDK17/Maven3.9.16。按[共同输入](../examples/README.md)创建/授权仅供教学的`codeark_tutorial_<run_id>`数据库，使用PG真实端口，不能把MySQL端口或草稿地址填入。

```sh
java -cp "examples/java/sql-pgsql/target/*" io.codeark.tutorial.Main
java -cp "examples/java/s3/target/*" io.codeark.tutorial.Main
```

PG示例创建表、插入并查询后删除该表；S3示例在全新教学桶完成对象写读并清理。只有真实验证相符才打印`PGSQL_QUERY_OK`、`S3_WRITE_READ_OK`。当前逐目标运行结果为pending，不能把本文输出当验收日志。

数据库本身保留，示例不安装扩展、不修改已有表，不把PG就绪说成Agent业务链路就绪。停止场景保留持久化数据；共享PG仍由其他场景使用时按后端操作计划保留。见[数据保护](storage.md)、[实际网络](network.md)和[指南目录](README.md)。
