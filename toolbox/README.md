# 代码方舟 · Code Ark 桌面工作箱（源码预览）

Tauri 2 + Rust + React 本地基础设施工作台，复用 [services/](../services/README.md) 中的模板。已进入源码实施阶段，尚无公开安装包。当前普通运行入口仅在 macOS arm64 开放变更操作；其他原生平台须完成验收后再开放，不能把跨平台编译或界面测试当作支持承诺。

English Summary: This is an unreleased source preview. The backend and desktop interface are implemented in part; an isolated native macOS arm64 Redis lifecycle has passed. Sixteen presets have passed two isolated startup, protocol and data-preserving stop rounds on macOS arm64, including the database-dependent startup chains. The full Elastic stack was rejected by the actual memory preflight; complete business workflows, other native targets and final installers still require acceptance. Runtime configuration and data stay outside source control.

## 已落地范围

- 17 服务、7 场景、1 宿主机指南的清单；77 个受跟踪来源资产白名单同步、哈希及路径校验，不复制本机 `.env`、数据或日志。
- 工作区和实例身份、配置保留与随机秘密、资源隔离覆盖层、升级冲突保护、固定 Docker 目标、端口/内存/容量预检、不可变镜像和 OCI 归档身份校验。
- 持久操作与跨进程锁、共享引用、部分创建资源恢复、启停、就绪探针、候选配置预检及就绪后提交、按归属清单重置。周期状态检查使用有期限的内存观察结果，不能把读取磁盘时间当成新探针证据。
- 推荐入门、场景和服务列表、环境助手、任务、详情与连接、配置、日志、教程、工作区设置及白名单诊断。前端只调用类型化后端命令；没有通用 Shell/文件 IPC。
- 11 份离线指南、13 个教学模块、9 个隔离协议客户端，以及源码 CI / 原生候选构建流程。随包使用锁定 JRE17 镜像运行协议客户端，基础设施用户无需宿主机 Java；维护者构建与自愿运行教学源码需要相应工具。

## 前端组件与主题

页面按[当前Figma](https://www.figma.com/design/iHxycwPLgUZAWbos7wNh0F?node-id=4-4349)实现，统一shadcn Radix（radix-nova）与Tailwind CSS 4。官方Sidebar负责折叠/移动端导航；`components/ui/`维护基础组件，Brand、PageHeader、ResourceList、ServiceRow、ScenarioItem、StatusBadge复用页面结构与样式。Logo来自Figma原始矢量，界面图标使用Lucide；字体自托管，无图片占位。

ThemeProvider与ModeToggle提供浅色、深色、跟随系统，偏好存入`code-ark-theme`；侧栏和设置复用同一菜单。语义色统一维护在`src/styles.css`，页面不单独覆盖明暗颜色。新增交互控件先复用`components/ui/`，遵循Radix的asChild、焦点和键盘行为，避免重复实现通用组件。

全局异常与消息使用唯一shadcn Sonner Toaster（`GlobalFeedback`）。页面通过`useWorkspace().feedback`发布消息，类型化命令由`lib/notifying-api.ts`集中处理失败与提交/导出结果；`lib/errors.ts`集中维护安全文案。禁止在页面/弹窗内另行渲染Alert、FieldError、复制/导出结果段落或导入Sonner。字段保留aria-invalid标记，操作授权继续使用Dialog；服务状态、体检资源数据保留结构化展示。快照故障按事件去重、恢复后清除，React和窗口异常也进入同一消息入口。

`pnpm --dir toolbox dev`只启动Vite前端，不提供浏览器模拟后端；普通浏览器缺少Tauri IPC时显示连接错误。真实服务操作需要桌面入口，测试夹具不进入生产路径。

## 当前验证和限制

[首阶段记录](../docs/research/06-工作箱首阶段实施与验证记录.md) 与[本轮验证](../docs/research/08-桌面全局消息与预设启动验证.md) 区分源码检查、真实启动、业务功能和最终制品。Redis 已验证写读、保留数据重建、端口改动、容量确认、登记缺失后的停止恢复和精确重置。数据库依赖组已接入并完成分组启动；XXL主编排初始化及认证通过，执行器调度等业务功能仍单独验收。

Canal、Temporal、XXL、Seata 的数据库认证、范围授权和初始化现已接入主编排，实际结果以隔离验收记录为准；MySQL/PG 未完成全部业务资源枚举前拒绝整实例重置。日志的原生 Channel/ACK 联调、诊断日志预览、完整场景、非 root Linux 数据目录权限、Windows 进程树及 ACL、各系统安装包仍待验证。不要用于承载未备份的业务数据。

镜像锁共 56 个平台条目：50 个注册表条目已解析，MinIO/XXL/ES-IK 的 3 个 ARM64 归档已完成独立功能及公开下载校验并[发布制品](../docs/research/07-ARM64预构建镜像制品验收与发布.md)，3 个 AMD64 归档仍 pending。development 用于源码开发；candidate 要求所选目标所有来源可解析且架构匹配；release 还要求匹配同 SHA 候选包的真实服务与安装验收。镜像预发布不代表桌面安装包发行，不绕过安装包门禁。

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

`cargo run --locked --manifest-path toolbox/src-tauri/Cargo.toml --example prepare_acceptance -- .tools/<new-run>/config.json` 创建隔离配置；`cargo run --locked --manifest-path toolbox/src-tauri/Cargo.toml --example preset_startup -- .tools/<new-run>/config.json redis` 走真实编排路径，保留数据停止后再次启动并复验全部必要探针。只有显式选中的服务及其依赖参与启动；`--stop` 只停止该隔离工作区中已核验归属的容器，保留数据。

验收增加 `--live` 可在每轮启动后失效启动证据，强制完成桌面同款后台观察；`--rebuild` 为本次明确选择的服务与依赖产生新代际，保留数据重建。Nacos需要包含主端口及+1000/+1001/+1002的组，验收窗口保持显式、有界（跨度最多2048）。运行时协议JAR从随包资源校验后进入当前control的不可变缓存，避免直接绑定会被开发构建替换的目录。

[项目结构](../PROJECT.md) · [设计与计划](../docs/00-文档导航.md) · [贡献规范](../CONTRIBUTING.md)
