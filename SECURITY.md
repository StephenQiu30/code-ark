# 安全政策 · Security Policy

当前维护范围是 `main` 分支上的服务配置、脚本和文档；桌面工作箱尚未发行。本项目面向本地开发与学习，不承诺生产安全认证、固定响应时限或尚不存在的发行版本支持周期。

## 报告安全问题

若问题可能暴露凭据、执行未授权命令或破坏其他服务数据，请避免在公开 Issue/PR 中发布漏洞细节、利用代码或真实配置。

1. 在 [仓库 Security 页面](https://github.com/StephenQiu30/code-ark/security) 检查是否提供 **Report a vulnerability** 私密报告入口；仅在维护者已启用时可使用。本政策不表示功能已经开启。
2. 若没有私密入口，先通过 [Issue](https://github.com/StephenQiu30/code-ark/issues/new) 仅请求维护者提供私密联系渠道，不填写漏洞细节或秘密；获得渠道后再私下提交完整材料。不要在公开讨论补充敏感细节。
3. 私密报告提供受影响提交/服务、系统及工具版本、合成数据的最小复现、影响范围和建议修复。不要上传真实 `.env`、业务数据库或未脱敏日志。

普通配置错误、文档问题和功能需求可按 [贡献指南](CONTRIBUTING.md) 使用公开 Issue 模板。验证漏洞只在自己有权操作的隔离环境进行，不测试他人的数据或服务。

维护者核查影响范围，协调修复、回归验证与适当披露；无法复现或缺少环境时如实记录。若凭据已泄露，先撤销或轮换；不要把删除 Git 文件当作恢复秘密安全。

English Summary: Use GitHub private vulnerability reporting if enabled. Otherwise, request a private contact route without disclosing details publicly. Share sanitized reproductions. Current scope is `main`; no desktop releases or response SLA are claimed.
