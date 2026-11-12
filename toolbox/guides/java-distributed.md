# Java分布式事务与调度：Seata / XXL-Job

基础设施为MySQL、Seata协调器、XXL调度中心；业务`undo_log`、执行器和业务代码由应用提供。下面是两个独立的教学应用，不会在启动基础设施时偷偷部署。

## Seata实际回滚

客户端Seata1.8.0匹配当前1.8服务协议，MySQL Connector/J26.10.0、Druid1.2.21。要求JDK17/Maven3.9.16。只在**全新的独立**`codeark_tutorial_<run_id>`数据库中执行，拒绝业务数据库和已有`undo_log`。设置SQL输入与协调器真实HOST/PORT，见[共同输入](../examples/README.md)。

```sh
java -cp "examples/java/seata-rollback/target/*" io.codeark.tutorial.Main
```

示例创建余额100的教学表和本库undo_log，建立真实TM/RM连接，用DataSourceProxy提交分支更新，再请求全局rollback，最后从数据库直接查询余额确实恢复100才打印`SEATA_ROLLBACK_OK`。该预期结果尚待逐目标实测。失败时保留教学表/undo_log以便恢复，不能自动删除未完成事务的数据；成功后仅删除本次教学表/undo_log，保留数据库。

## XXL真正执行一次任务

SDK为XXL-Job Core3.5.0。此版本使用**执行器级**AccessToken，不能套用旧全局Token方案；控制台地址已按当前版本去掉旧context-path时，以已应用入口为准。教学执行器强制容器模式，加入调度中心返回的真实网络、用自身容器别名注册并绑定实际可用端口；不能注册宿主机127.0.0.1给调度中心容器。

先在XXL管理界面显式创建本run_id的执行器，配置同一执行器Token和专属AppName`codeark_tutorial_<run_id>`；创建BEAN任务`codearkTutorial`，只绑定这个教学执行器。设置说明中的`CODEARK_XXL_*`输入后，在[容器网络](network.md)中运行：

```sh
java -cp "examples/java/xxl-executor/target/*" io.codeark.tutorial.Main
```

它先打印`XXL_EXECUTOR_WAITING_FOR_EXPLICIT_ADMIN_TRIGGER`，这**不是成功**。在控制台显式触发教学任务，120秒内实际执行才打印`CODEARK_XXL_TASK_OK`；超时非零退出。最后停止/删除本次教学任务与执行器登记；只处理本次临时教学日志，勿删除其他执行器记录或业务调度。当前真实调度验收pending。

返回[指南目录](README.md) · [备份恢复](storage.md)。
