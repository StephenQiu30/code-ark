# 实际连接、宿主机与容器网络

所有业务示例先从服务详情的“连接与入口”读取**已应用generation**。草稿里的新端口尚未应用；Ready是后端真实协议验证结果，控制台首页200或按钮点击不能代替后台连接测试。

## 宿主机

设置教学环境变量时复制实际宿主机host/port、账号和显式揭示的秘密。入口默认只发布到127.0.0.1。源码例子只接受localhost/127.0.0.1，拒绝把任意远端数据库当教学目标。不把密码放在命令参数、Git文件或诊断中；SDK运行输出另行复核，不能承诺任意第三方日志完全脱敏。

## 容器

容器内localhost是该容器自己。使用详情返回的真实`requiredNetworks`、容器别名和容器端口；不能照抄宿主机127.0.0.1，也不默认借`host.docker.internal`访问Linux回环发布端口。将`CODEARK_DOCKER_NETWORK`设为实际网络，`CODEARK_ENDPOINT_MODE=container`，各HOST设为同网络别名。示例守卫仅接受单标签别名，不接受外部域名。

下面是需要填写已验证运行输入的命令模板，不是已完成验收的输出。`CODEARK_JAVA_IMAGE`必须取候选镜像锁`probe-jre17`对应本机原生架构的不可变引用；不是`latest`。`CODEARK_EXAMPLE_DIR`为受控教学源码目录，`CODEARK_MODULE`为本页列出的模块名，`CODEARK_DOCKER_NETWORK`来自当前已应用连接视图。

```sh
docker run --rm --network "$CODEARK_DOCKER_NETWORK" \
  --mount "type=bind,source=$CODEARK_EXAMPLE_DIR,target=/tutorial,readonly" \
  --workdir /tutorial --env CODEARK_TUTORIAL_CONFIRM --env CODEARK_RUN_ID \
  --env CODEARK_ENDPOINT_MODE --env CODEARK_DOCKER_NETWORK \
  --env CODEARK_SQL_HOST --env CODEARK_SQL_PORT --env CODEARK_SQL_DATABASE \
  --env CODEARK_SQL_USERNAME --env CODEARK_SQL_PASSWORD \
  "$CODEARK_JAVA_IMAGE" java -cp "java/$CODEARK_MODULE/target/*" io.codeark.tutorial.Main
```

不同模块另需其指南列出的环境变量，逐项用`--env 变量名`透传，不把秘密直接写进命令。XXL执行器必须在调度中心可达的真实网络运行，并用自身容器别名注册；调度中心容器不能回连宿主机的127.0.0.1执行器。

RocketMQ使用5.x Proxy及Java SDK5.2.2，不能以旧Broker SDK直连示例替代。Kafka的advertised listener、Nacos gRPC偏移端口也必须与当前拓扑一致；复制一个无法解析的别名不算连接配置可用。

返回[指南目录](README.md) · [源码输入说明](../examples/README.md)。
