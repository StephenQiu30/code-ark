# MediaCrawler 本地版本与补丁

本目录只保存 HotKey 使用的固定版本、安全补丁和安装步骤，不提供容器服务。HotKey 适配器以宿主机子进程运行 `$HOME/Desktop/StephenQiu/MediaCrawler/.venv/bin/python main.py`；MediaCrawler 通过 CDP 驱动本机 Chrome 的独立用户资料。上游仓库另有 `Dockerfile.local` 和 `LOCAL-DOCKER.md`，用于无头 Chromium 容器入口；HotKey 当前不使用该入口。

上游项目：[NanmiCoder/MediaCrawler](https://github.com/NanmiCoder/MediaCrawler)。固定在提交 `380b426000aac3d612837ed72c99808347dc94c9`，依次应用 `hotkey-safe.patch` 的 3 个提交及 `hotkey-safety-protocol.patch` 的 2 个提交后应得到 `1bd07bc783963acef092ff00771207b7792b8cb6`。第二组包含本地 Docker 入口提交和 HotKey 受控退出码提交，HotKey 仍只使用宿主机子进程。上游采用非商业学习许可，仅用于个人研究；使用时须遵守上游许可与目标平台规则。

在全新目录安装（需要 Git 和 `uv`）：

```bash
git clone https://github.com/NanmiCoder/MediaCrawler.git "$HOME/Desktop/StephenQiu/MediaCrawler"
cd "$HOME/Desktop/StephenQiu/MediaCrawler"
git checkout 380b426000aac3d612837ed72c99808347dc94c9
git switch -c hotkey-safe
git -c user.name=HotKey -c user.email=noreply@hotkey.local am --committer-date-is-author-date "$HOME/Desktop/Docker/mediacrawler-start-local/hotkey-safe.patch"
git -c user.name=StephenQiu30 -c user.email=popcornqhd@gmail.com am --committer-date-is-author-date "$HOME/Desktop/Docker/mediacrawler-start-local/hotkey-safety-protocol.patch"
git rev-parse HEAD  # 应为 1bd07bc783963acef092ff00771207b7792b8cb6
uv sync --frozen
umask 077
mkdir -p browser_data/cdp_bili_user_data_dir
chmod 700 browser_data browser_data/cdp_bili_user_data_dir
```

HotKey 适配器会校验补丁后的提交。补丁禁用 `stealth.min.js` 注入，抖音滑块校验会直接停止；B 站安全模式将登录缺失或验证要求以退出码 70 报告，将 HTTP 429 限流以退出码 71 报告，其他非零退出由 HotKey 单独分类，不从日志关键词推断风控。运行中出现滑块或验证码应停止并由本人核查，不自动破解。CDP 调试端口只绑定 `127.0.0.1`（上游为 `0.0.0.0`），也不会连接日常使用的 Chrome。各平台使用 `browser_data/cdp_<平台>_user_data_dir` 独立资料，其中含登录态，不得入库或同步。启用其他平台前也应分别创建资料目录并设为 `700`；运行 HotKey 的进程应使用 `umask 077`，保证新建文件仅本机用户可读写。

默认低频配置为请求间隔 8 秒、并发 1、条数上限 10、每条最多 20 条一级评论，不采楼中楼。B 站搜索只对请求条数内的视频继续抓取详情和评论，关键词结果按最新发布时间排序。

HotKey 侧另限制同一来源最短间隔 6 小时，`00:00—08:00` 不采；每轮最多 3 个关键词、每词 5 条，遇到验证或限流会自动暂停该来源。开发阶段只启用 B 站。
