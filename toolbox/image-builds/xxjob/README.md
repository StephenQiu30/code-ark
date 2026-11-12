# XXL-Job 原生制品

使用 `recipe.json` 锁定的 XXL-Job 3.5.0 commit、源码归档 SHA、Maven 3.9.16/JDK17 与各架构 JRE digest 原生构建。保留源 LICENSE；Dockerfile 和 entrypoint 都由 SHA 校验，漂移必须经审查后更新配方锁。

在仓库根目录执行，先只读核对 daemon ID/CPU：

```sh
node toolbox/scripts/producer.mjs --recipe xxjob --context <context> --expected-daemon <daemon-id> --platform linux/arm64
```

amd64 必须在原生 amd64 daemon 改用 `linux/amd64` 执行，不允许模拟架构。每个 Docker 子命令上限 30 分钟，错误保留上游诊断，超时不会生成成功 receipt。

产物写入忽略目录 `.tools/artifacts/`。receipt 中的实际 archive SHA、image ID、内容 tag 和原生构建输入不能代替功能验收或公开来源；`functionalAcceptance` 默认 `pending`、`archiveUrl` 默认 `null`。需隔离初始化 MySQL、验证 SHA256 管理员登录、认证后的数据库 API 及数据保留，再另行审查许可和公开不可变下载来源。不得为了填锁伪造 URL 或以本地镜像缓存代替可获取制品。

源码默认根路径；镜像固定 `SERVER_SERVLET_CONTEXT_PATH=/xxl-job-admin` 保持现有服务入口。3.5 登录页为 `/xxl-job-admin/auth/login`，认证为 `POST /xxl-job-admin/auth/doLogin`（`userName`/`password` 表单）；不是旧版 `/login`。

镜像以 UID/GID 1000 运行，预建 `/data/applogs`；绑定日志目录须可由该用户写入。支持 Spring 环境变量，也兼容旧 `PARAMS`（按空白拆分、不执行 `eval`）；包含空白的值应通过 Spring 环境变量传入。生产者不会读取真实 `.env` 或启动现有 MySQL。
