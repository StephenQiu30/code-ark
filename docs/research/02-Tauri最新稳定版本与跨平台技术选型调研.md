# Tauri 最新稳定版本与跨平台技术选型调研

核查日期：2026-11-12（Asia/Shanghai）。状态：**版本核验与技术选型调研**。

## 1. Tauri 最新版本实测结果（O）

通过联网检索及直接查询 GitHub `tauri-apps/tauri` Releases 与 npm 官方源（`npm view`），截至 **2026-11-12** 的版本事实如下：

### 1.1 最新稳定版系列（推荐用于生产/发布给新手的工具箱）

| 组件 / 包名 | 生态来源 | 最新稳定版本 | 发布日期（UTC） | 核验来源 |
| --- | --- | --- | --- | --- |
| `tauri` | crates.io / GitHub | **`2.12.5`** | 2026-10-21 | GitHub `tauri-v2.12.5`（`prerelease: false`） |
| `tauri-build` | crates.io / GitHub | **`2.7.4`** | 2026-10-21 | GitHub `tauri-build-v2.7.4` |
| `tauri-cli` | crates.io / GitHub | **`2.12.4`** | 2026-10-21 | GitHub `tauri-cli-v2.12.4` |
| `tauri-bundler` | crates.io / GitHub | **`2.10.4`** | 2026-10-21 | GitHub `tauri-bundler-v2.10.4` |
| `@tauri-apps/cli` | npm (`latest`) | **`2.12.4`** | — | `npm view @tauri-apps/cli version` |
| `@tauri-apps/api` | npm (`latest`) | **`2.12.4`** | — | `npm view @tauri-apps/api version` |
| `@tauri-apps/plugin-opener` | npm (`latest`) | **`2.7.3`** | — | `npm view @tauri-apps/plugin-opener version` |
| `@tauri-apps/plugin-dialog` | npm (`latest`) | **`2.8.4`** | — | `npm view @tauri-apps/plugin-dialog version` |
| `@tauri-apps/plugin-fs` | npm (`latest`) | **`2.7.1`** | — | `npm view @tauri-apps/plugin-fs version` |

### 1.2 前瞻预览版对照（不建议用于当前交付基线）

- GitHub Releases 已出现 `tauri-v3.0.0-alpha.6`（2026-11-06 发布，同时包含 `tauri-cli-v3.0.0-alpha.6` marked `prerelease: true` 与 `@tauri-apps/cli-v3.0.0-alpha.6`）。
- **结论（I）**：3.x 仍处于早期 alpha 阶段，API 与插件生态尚未稳定；Code Ark 工作箱面向编程新手强调开箱稳定与三端打包成熟度，应**锁定 Tauri 2.12.x 稳定系列（`tauri = "2.12.5"`、`tauri-build = "2.7.4"`、`@tauri-apps/api = "2.12.4"`、`@tauri-apps/cli = "2.12.4"`）**。

## 2. 跨平台桌面技术方案对照（I）

| 维度 | Tauri 2.12.5（Rust + Web） | Electron（TS + Chromium） | 原生 SwiftUI | 纯 Rust GUI（`egui`/`iced`） |
| --- | --- | --- | --- | --- |
| **平台覆盖** | macOS / Windows / Linux | macOS / Windows / Linux | 仅 macOS | macOS / Windows / Linux |
| **自身内存与包体** | 复用系统 WebView，轻量低内存，不与 Elastic（需 6 GB）/ Kafka 抢内存 | 内置 Chromium，基础包体与常驻内存显著偏高 | 极低 | 极低 |
| **系统调度与兼容性** | Rust 原生处理 PATH 探测、Windows 无弹窗进程（`CREATE_NO_WINDOW`）、端口 TCP 探测、`OsRng` 密钥生成与 LF 字节写文件 | Node 子进程亦可实现，但需要额外打包 Node 运行时 | 原生极佳，但不跨平台 | Rust 系统能力强 |
| **工作箱复杂 UI 研发效率** | 高（React + Tailwind 易实现双栏导航、日志流、中文代码片段高亮与配置表单） | 高 | 高（但仅限 Mac） | 低（中文文本排版、表单与图文教程组件开发成本高） |
| **主要权衡** | 需维护 Rust + TS 双栈；Linux 打包需关注 WebKitGTK 依赖 | 对新手低配电脑内存更不友好 | 丢失 Windows/Linux 用户 | 界面精致度难以对齐 `mac-gaming-toolbox` |

## 3. 本机现有工具链状态实测（O）

- 2026-11-12 本机实测输出：
  - `node`: `v24.19.0`（`/opt/homebrew/opt/node@24/bin/node`）
  - `npm` / `pnpm`: 已安装
  - `docker`: `Docker version 29.9.3`（`/usr/local/bin/docker`）
  - `rustc` / `cargo`: **当前本机未安装**（`cargo not found`、`~/.cargo/bin` 不存在）。
- **影响说明**：代码实施阶段开始前，需先通过 `rustup` 安装 Rust 工具链，或由 CI（`macos-latest` / `windows-latest` / `ubuntu-22.04`）完成 Rust 编译验证。

[返回调研导航](00-调研导航.md)
