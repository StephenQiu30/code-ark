# Code Ark 桌面工作箱（源码预览）

Tauri 2 + Rust + React 本地基础设施工作台，复用 [services/](../services/README.md) 中的模板。已进入源码实施阶段，尚无公开安装包。当前普通运行入口仅在 macOS arm64 开放变更操作；其他原生平台须完成验收后再开放，不能把跨平台编译或界面测试当作支持承诺。

English Summary: This is an unreleased source preview. The backend and desktop interface are implemented in part; an isolated native macOS arm64 Redis lifecycle has passed. Other native targets, all 17 services, database-dependent workflows and final installers are not accepted yet. Runtime configuration and data stay outside source control.

## 已落地范围

- 17 服务、7 场景、1 宿主机指南的清单；77 个受跟踪来源资产白名单同步、哈希及路径校验，不复制本机 `.env`、数据或日志。
- 工作区和实例身份、配置保留与随机秘密、资源隔离覆盖层、升级冲突保护、固定 Docker 目标、端口/内存/容量预检、不可变镜像和 OCI 归档身份校验。
- 持久操作与跨进程锁、共享引用、部分创建资源恢复、启停、就绪探针、候选配置预检及就绪后提交、按归属清单重置。周期状态检查使用有期限的内存观察结果，不能把读取磁盘时间当成新探针证据。
- 推荐入门、场景和服务列表、环境助手、任务、详情与连接、配置、日志、教程、工作区设置及白名单诊断。前端只调用类型化后端命令；没有通用 Shell/文件 IPC。
- 11 份离线指南、13 个教学模块、9 个隔离协议客户端，以及源码 CI / 原生候选构建流程。随包使用锁定 JRE17 镜像运行协议客户端，基础设施用户无需宿主机 Java；维护者构建与自愿运行教学源码需要相应工具。

## 当前验证和限制

[执行记录](../docs/research/06-工作箱首阶段实施与验证记录.md) 区分源码检查、真实服务和最终制品。Redis 已验证写读、保留数据重建、端口改动、容量确认、登记缺失后的停止恢复和精确重置。XXL ARM 镜像独立登录/数据库探测通过；不代表主工作箱的 XXL 依赖链或任务调度验收通过。

Canal、Temporal、XXL、Seata 的数据库初始化尚未接入主编排，启动显式返回 `DependencyInitializationRequired`；MySQL/PG 未完成全部业务资源枚举前拒绝整实例重置。日志的原生 Channel/ACK 联调、诊断日志预览、完整场景、非 root Linux 数据目录权限、Windows 进程树及 ACL、各系统安装包仍待验证。不要用于承载未备份的业务数据。

镜像锁共 56 个平台条目：50 个注册表条目已解析，6 个 MinIO/XXL/ES-IK 原生归档仍未具备完整可发行来源。development 用于源码开发；candidate 要求所选目标所有来源可解析且架构匹配；release 还要求匹配同 SHA 候选包的真实服务与安装验收。当前不放行候选/公开发行，不能绕过门禁。

## 开发与本地运行

使用 Node `24.22.3`、pnpm `12.18.0`、Rust `1.99.1`、JDK17、Maven `3.9.16` 和对应系统的 Tauri 构建前置。在仓库根目录执行：

```bash
pnpm --dir toolbox install --frozen-lockfile
mvn -B -ntp -C -f toolbox/examples/java/pom.xml package
mvn -B -ntp -C -f toolbox/probe-clients/pom.xml package
python3 toolbox/probe-clients/bundle.py
pnpm --dir toolbox test:scripts
pnpm --dir toolbox sync
node toolbox/scripts/prepare-desktop.mjs
cargo test --locked --manifest-path toolbox/src-tauri/Cargo.toml --features desktop
pnpm --dir toolbox test
pnpm --dir toolbox typecheck
pnpm --dir toolbox build
pnpm --dir toolbox tauri dev
```

首次启动只打开工作台，不自动启动服务。运行工作区默认 `~/.code-ark`，公共控制目录按系统隔离；检测 Docker 后再选择服务。全新开发检出须先构建并校验协议 JAR，再准备桌面资源；不提交这些生成物。教学模块自行显式确认后才连接/写入，详见 [示例说明](examples/README.md)。

真实服务测试默认忽略，必须提供固定 CLI/context/daemon、专用工作区、run ID、端口范围及删除授权的 `CODE_ARK_ACCEPTANCE_CONFIG`。测试只处理该范围的可证实资源；不要复用现有业务环境或执行全局 prune。`resources/blueprints/`、随包教程/客户端和资源 manifest 均由构建生成，不手工维护。

[项目结构](../PROJECT.md) · [设计与计划](../docs/00-文档导航.md) · [贡献规范](../CONTRIBUTING.md)
