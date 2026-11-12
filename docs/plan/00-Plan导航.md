# 实施计划导航

本目录按核心依赖链拆解实施步骤：**底座与蓝图（Plan 01） → 跨平台探测与编排（Plan 02） → 前端工作台、诊断与跨平台 CI（Plan 03）**。每份 Plan 仅从属于唯一一份 `prd`，技术结构体与协议定义沿来源 PRD 指向的 `design` 读取。

> **当前执行状态（核查于 2026-11-12）**：规划文档已齐备，**代码开发尚未启动，所有构建、单测与端到端测试均处于“未执行”状态**。本机实测工具链现状：`node v24.19.0`、`npm`、`pnpm`、`Docker version 29.9.3` 已就绪；`rustc` / `cargo` 当前未安装。

## 1. 实施计划总表

| 实施计划 | 唯一来源 PRD | 核心职责与包含模块 |
| --- | --- | --- |
| [01 服务清单与蓝图同步实施计划](01-服务清单与蓝图同步实施计划.md) | [PRD 01](../prd/01-CodeArk工作箱底座与蓝图管理PRD.md) | `services.json`、`sync-blueprints.mjs` / `verify-blueprints.mjs`、Tauri `2.12.5` 工程骨架、`catalog.rs`、`env_store.rs`、`blueprint.rs` |
| [02 跨平台 Docker 探测与编排引擎实施计划](02-跨平台Docker探测与编排引擎实施计划.md) | [PRD 02](../prd/02-跨平台环境预检与服务自动化编排PRD.md) | `docker_cli.rs`、`preflight.rs`、`provisioner.rs`、`seata_1_8_0_mysql.sql`、`orchestrator.rs` |
| [03 工作箱前端工作台与跨平台 CI 实施计划](03-工作箱前端工作台与跨平台CI实施计划.md) | [PRD 03](../prd/03-工作箱可视化工作台与新手引导PRD.md) | `diagnostics.rs`、`commands.rs`、React 19 + TS + Tailwind 前端工作台、Vitest、`.github/workflows/toolbox-ci.yml`、三端集成验证 |

## 2. 需求与场景验收归属

| 所属 PRD | 需求 ID | 测试场景 | 承接实施计划 |
| --- | --- | --- | --- |
| [PRD 01](../prd/01-CodeArk工作箱底座与蓝图管理PRD.md) | `PRD01-R01`–`PRD01-R06` | `TS-01`、`TS-02`、`TS-03`、`TS-04` | [Plan 01](01-服务清单与蓝图同步实施计划.md) |
| [PRD 02](../prd/02-跨平台环境预检与服务自动化编排PRD.md) | `PRD02-R01`–`PRD02-R06` | `TS-05`、`TS-06`、`TS-07`、`TS-08`、`TS-09` | [Plan 02](02-跨平台Docker探测与编排引擎实施计划.md) |
| [PRD 03](../prd/03-工作箱可视化工作台与新手引导PRD.md) | `PRD03-R01`–`PRD03-R06` | `TS-10`、`TS-11`、`TS-12`、`TS-13` | [Plan 03](03-工作箱前端工作台与跨平台CI实施计划.md) |

## 3. 执行约定

1. **开工前置阅读**：实施任一任务前，必读唯一来源 `prd`、其唯一 `design`，以及仓库根目录 [AGENTS.md](../../AGENTS.md) 与 [CONTRIBUTING.md](../../CONTRIBUTING.md)。
2. **版本锁定基线**：桌面框架锁定 **Tauri v2 最新稳定版**：`tauri = "2.12.5"`、`tauri-build = "2.7.4"`、`@tauri-apps/cli = "2.12.4"`、`@tauri-apps/api = "2.12.4"`、`@tauri-apps/plugin-opener = "2.7.3"`、`@tauri-apps/plugin-dialog = "2.8.4"`、`@tauri-apps/plugin-fs = "2.7.1"`（不使用 `3.0.0-alpha.6` 预览版）。
3. **本机既有容器保护**：调试时严禁直接停止或销毁本机已有非工作箱容器（实测本机存在 `nacos-standalone` 等已有容器），全流程 E2E 验证一律使用临时工作目录（如 `CODE_ARK_WORKSPACE=$(mktemp -d)`）。
4. **根仓库只读原则**：根目录现有 `*-start-local` / `*-start-lcoal` 目录只做构建期只读同步来源，实施过程中不得改写原业务 YAML 或 `.env.example`。
5. **如实报告检查结果**：每项任务按“先写失败测试 → 实现代码 → 复跑验证”推进；缺少工具链（如本地无 `cargo`）时据实写明未执行，不得以静态代码或 Mock 替代真实容器实测。

[返回文档中心](../00-文档导航.md)
