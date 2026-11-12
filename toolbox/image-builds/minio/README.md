# MinIO 原生制品

使用 `recipe.json` 锁定的 MinIO commit、源码归档 SHA、Go 1.27.4 与各架构基础 digest，在原生 Linux-containers daemon 构建。不得通过 QEMU 代替目标 CPU 验收。

在仓库根目录执行，先用只读 `docker --context <context> info` 核对 daemon ID 和架构：

```sh
node toolbox/scripts/producer.mjs --recipe minio --context <context> --expected-daemon <daemon-id> --platform linux/arm64
```

amd64 目标必须改为 `linux/amd64` 并在原生 amd64 daemon 执行。producer 校验 Dockerfile SHA；修改配方后应审查并更新 `recipeSha256`，不会自动接受漂移。构建使用 Go 模块/编译缓存，模块下载最多三次、每次 240 秒，编译最多 900 秒；Docker 子命令总上限 30 分钟，超时后先取消、再强制结束客户端。错误保留最后一段上游诊断，不把超时写成成功。

产物位于忽略目录 `.tools/artifacts/`，包含 image-save 归档及实际 SHA、image ID、内容 tag、原生 daemon 和构建输入 receipt。归档生成只代表构建完成，`functionalAcceptance` 仍为 `pending`；还需独立验证 S3 认证、对象读写和数据保留，并完成许可审查及提供不可变公开下载来源。没有真实来源时 `archiveUrl` 为 `null`，不可写入已解析发行锁或宣称目标 release 通过。

镜像以 UID/GID 1000 运行，预建 `/data`。外部绑定目录必须可由该用户写入；不修改用户已有数据目录的权限。构建不启动 MinIO，也不操作现有服务。
