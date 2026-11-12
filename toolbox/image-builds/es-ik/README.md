# ES/IK 原生制品

只读复用 `services/elastic-start-local/elasticsearch/Dockerfile` 配方；producer 校验源配方哈希，在隔离构建上下文中将 ES 基础引用替换为锁定平台 digest，将 IK 下载替换为已校验本地 ZIP。不会改动 CLI 配方或在用户普通启动时执行 build。

需要分别在原生 amd64/arm64 的本地 Linux-containers daemon 构建、验证 IK API，再导出 Docker image-save 归档与校验和。当前源码支持配方生成；实际归档、公开下载来源和目标系统运行验证未齐全，发行继续阻断。
