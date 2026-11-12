# Code Ark 桌面工作箱（开发中）

Tauri 2 + Rust + React 的本地基础设施工作台，复用 [services/](../services/README.md) 中的模板。当前已开始实施；尚无公开发行安装包，静态检查通过不等于 17 服务或三端验收通过。

English Summary: The desktop toolbox is under active implementation, not a released product. Service templates are read-only inputs; runtime configuration and data belong in an independent workspace. Candidate and release builds fail closed until immutable image sources and matching acceptance evidence exist.

## 当前已实现

- 17 服务、7 场景、1 宿主机指南的受控清单。
- 只提取 Git 跟踪白名单，拒绝秘密文件、未跟踪资产、越界路径和符号链接；验证挂载引用、脚本 LF 和文件哈希。
- development / candidate / release 分级；未解析镜像不进入候选或发行，未验收的制品不标为通过。
- Rust 公共 DTO 自动生成 TypeScript、带锁工作区身份与实例登记、独立资源前缀、按已选数据库绑定去重的依赖顺序。
- Tauri/React 最小入口。编排与业务界面继续按计划实施。

## 开发环境

锁定 Node `24.22.3`、pnpm `12.18.0`、Rust `1.99.1`，需对应操作系统的 Tauri 原生构建前置条件。开发者需要这些工具；未来安装包用户只需受支持的 Docker 运行环境。

在本目录执行：

```bash
pnpm install --frozen-lockfile
pnpm test:scripts
pnpm sync
pnpm verify --mode development
cargo test --locked --manifest-path src-tauri/Cargo.toml
pnpm typecheck
pnpm build
cargo check --locked --manifest-path src-tauri/Cargo.toml --features desktop
pnpm tauri dev
```

`resources/blueprints/` 为可重复生成的忽略输出，不手工维护。同步不读取真实 `.env` 或数据目录；`catalog/services.json` 的资产列表才是提取范围，不递归复制服务目录。

## 构建与发行边界

当前 `catalog/images.lock.json` 仍有待解析的注册表/原生制品条目。development 仅用于源码开发；candidate 要求所选目标的全部镜像来源解析且架构匹配，release 还要求外部记录的同 SHA 候选包、17 服务、7 场景及最终安装验收证据。不能用 development 绕过门禁打公开安装包。

运行工作区默认 `~/.code-ark`，不使用源码目录；后续真实服务测试只能在明确隔离目标与资源范围内执行。不要删除现有服务数据、轮换已有数据库密码或执行全局 prune。

[项目结构](../PROJECT.md) · [设计与计划](../docs/00-文档导航.md) · [贡献规范](../CONTRIBUTING.md)
