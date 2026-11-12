# 贡献指南 · Contributing

欢迎贡献服务模板、问题修复、文档与使用反馈。当前仓库提供 17 套本地 Docker Compose 配置，桌面工作箱仍处于设计阶段。请先阅读 [README](README.md)、[项目结构](PROJECT.md) 和 [协作规范](AGENTS.md)。

English Summary: Use `type(scope): description` for new commits and PR titles; scope is required. Keep changes focused, preserve local data and secrets, and report only checks actually performed. Original contributions use the existing MIT license; third-party licenses remain applicable. See [sharing](SHARING.md), [security](SECURITY.md), and [community conduct](CODE_OF_CONDUCT.md).

## 提交与合并流程

1. 检查分支和工作区，确认范围，保留其他任务和用户已有修改。
2. 外部贡献通过 fork 和 PR；有仓库权限时使用主题分支，自动化代理创建分支使用 `codex/<topic>`。用户明确授权直接提交 `main` 时可直接提交、推送，无需再次确认。
3. 一次提交只处理一个主题。同步相关 README、结构说明和必要的设计/需求/计划；不顺带重命名历史目录、升级无关服务或格式化整个仓库。
4. 完成下述验证，使用明确文件路径暂存，再检查 `git diff --cached --check` 和暂存差异。只提交任务相关文件。
5. PR 填写模板，说明实际验证与未验证项。优先 squash 合并，合并提交标题也使用同一规范；维护者合并前核对内容和标题。

不要强推 `main`、重写既有历史或修改他人的提交。若远端发生变化，先获取并检查差异，按团队约定整合后重新验证。

## 提交消息：`type(scope): 描述`

采用 [Conventional Commits 1.0.0](https://www.conventionalcommits.org/en/v1.0.0/)，并在其基础上**要求 scope 必填**。从本规范落地后的新提交开始执行，历史提交保持原样。

```text
type(scope): 描述

可选正文：原因、主要行为变化、实际验证。

可选页脚：关联 Issue；破坏性变更说明。
```

- `type` 和 `scope` 使用小写；冒号后一个空格。scope 以字母开头，只用小写字母、数字和连字符。
- 描述具体且非空，中文或英文均可，建议整个标题不超过 72 个字符，不以句号结尾。避免 `update`、`修改一下` 等模糊标题。
- 破坏性变更使用 `type(scope)!: 描述`，并补 `BREAKING CHANGE: 影响与迁移方法` 页脚。例如端口、数据布局、凭据或目录不兼容时，说明备份、迁移及恢复步骤。

| type | 用途 |
| --- | --- |
| `feat` | 新能力或服务 |
| `fix` | 修复已有行为 |
| `docs` | 文档与协作规范 |
| `style` | 不影响行为的格式调整 |
| `refactor` | 保持行为的结构调整 |
| `perf` | 性能优化 |
| `test` | 测试用例或测试资产 |
| `build` | 依赖、构建和制品配方 |
| `ci` | CI 工作流 |
| `chore` | 其他维护工作 |
| `revert` | 回退，正文注明原提交和原因 |

scope 表示主要受影响模块。根规范用 `project`，知识库用 `docs`；服务用 `mysql`、`pgsql`、`redis`、`temporal` 等稳定服务名，不必复制历史目录拼写。未来工作箱可用 `toolbox`、`catalog`、`runtime`、`frontend`、`probes`、`release`；这些范围名不表示代码已经存在。跨模块选择主要职责，必要时拆分提交。

```text
docs(project): 明确目录职责与贡献流程
fix(temporal): 修复命名空间初始化的重复执行
build(minio): 固定镜像构建来源
feat(runtime)!: 调整工作区数据布局

BREAKING CHANGE: 说明旧布局、迁移步骤和备份恢复方法。
```

可在仓库根目录临时使用提交模板：

```bash
git -c commit.template=.gitmessage commit
```

当前通过文档、提交/PR 模板和维护者审查执行规范，尚未安装 commit hook、commitlint 或 CI 标题门禁。模板不等于自动校验；本次不修改全局 Git 配置。

## 服务与配置约定

- 新服务使用 `<service>-start-local/`，至少包含 `docker-compose.yml` 和 `README.md`；使用环境变量时提供 `.env.example`。初始化 SQL、配置和脚本放在当前服务目录内。
- 保留 `mysql-start-lcoal`、`rabbitmq-start-lcoal` 的历史名称。复用现有数据库/网络，明确依赖、启动顺序及当前服务专属数据库，不操作其他业务数据。
- 使用 Compose v2，不新增顶层 `version`。新增镜像固定已验证版本，配置健康检查、资源限制、日志轮转；常驻服务 `unless-stopped`，初始化任务成功后退出。
- 默认宿主机入口绑定 `127.0.0.1`，检查仓库默认端口与本机占用；说明容器内部地址和宿主机地址。
- 配置、脚本只读挂载；Shell 脚本使用 LF。不得全局 prune、删除无关卷或覆盖用户配置。
- `.env.example` 只放公开默认值和凭据占位符；必要变量加必填校验。只在首次、确认没有 `.env` 时复制示例，已有文件保留；不提交真实凭据、数据目录、原始诊断日志或数据库导出。

## 文档约定

服务 README 使用 UTF-8，中文优先，包含 English Summary、简介、组件/端口、前置条件、启动、配置、持久化/清理、常用命令和返回根 README 的链接。命令可公开复现，能力描述以实际配置为准。

新增或删除服务时，同步中英文根 README 的服务表、数量和现有目录树（如有），并更新 `PROJECT.md`。工作箱技术变化沿 `requirement → design → prd → plan` 追溯，详见 [知识库维护说明](docs/01-知识库维护说明.md)。计划路径和未执行任务必须标注状态。

## 提交前验证

- 所有改动：`git diff --check`、暂存差异检查、相关文档链接和路径核对，确认未跟踪真实 `.env`、凭据或运行数据。
- 纯文档：检查 UTF-8、导航、数量和实现状态；无需启动容器或安装工作箱依赖。
- Compose/配置改动：阅读目标服务 README、Compose 和 `.env.example`，在目标目录运行 `docker compose config --quiet`，避免打印展开后的配置与密码。说明采用了公开样例还是本机配置。
- 用户要求启动或改动初始化行为时：仅操作目标服务及明确依赖，检查 `ps -a`、健康状态、初始化退出码及真实客户端/API；验证重复启动与保留数据的重建。首页 200 不代表后台可用。
- 普通停止用 `stop` 或 `down`；`down -v` 删除当前项目命名卷，不删除绑定目录或外部数据库，执行前确认范围和数据处理授权。

PR 和交付报告列出实际命令、结果与未执行项；不能将静态配置通过写成真实环境或最终安装包验收通过。

## Issue、许可与共享

普通问题使用 [Issue 模板](https://github.com/StephenQiu30/code-ark/issues/new/choose)，给出服务目录、操作系统、Docker/Compose 版本、最小复现和脱敏日志。安全漏洞按 [SECURITY.md](SECURITY.md) 处理，社区交流遵守 [行为准则](CODE_OF_CONDUCT.md)。

贡献者应确认有权公开内容，并同意以本仓库 [MIT 许可证](LICENSE) 提供原创贡献。第三方引用、补丁与分发需保留来源及其许可要求，详见 [SHARING.md](SHARING.md)。
