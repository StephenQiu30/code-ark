# MediaCrawler宿主机工具

这是一份宿主机工具指南，不是第18个Compose服务，也不提供工作箱内启动容器按钮。需要Git与uv；站点登录、验证或限流另行处理，不能把工作箱中间件就绪当作采集业务成功。

唯一上游为[NanmiCoder/MediaCrawler](https://github.com/NanmiCoder/MediaCrawler)。代码固定在`380b426000aac3d612837ed72c99808347dc94c9`，使用导出教学源码中的[第一份补丁](../blueprints/mediacrawler/hotkey-safe.patch)的3个提交、[第二份补丁](../blueprints/mediacrawler/hotkey-safety-protocol.patch)的2个提交依次应用，应得到`1bd07bc783963acef092ff00771207b7792b8cb6`。补丁摘要与蓝图manifest一起核验；不从本文下载另一份未知补丁。没有这两份已校验资源时停止安装，不跳过补丁校验。

在用户选择的全新目录克隆上游并checkout上述提交，依次`git am`应用两份只读补丁后，用`git rev-parse HEAD`比较完整结果，再执行`uv sync --frozen`。具体安装目录由您选择，指南不硬编码某个用户主目录、不覆盖已有克隆。上游与补丁的许可独立适用，不把第三方内容当成本仓库原创MIT授权。

单独Chrome用户资料可能包含登录态，应只对本机用户可读写，不能放进Git或诊断。只绑定本机CDP入口，不连接日常浏览器资料。当前只记录源码与补丁合同，真实登录、站点抓取和宿主机安装验收pending；遇验证/限流按上游与站点规则暂停并由本人处理。

清理只删除您本次新建且确认归属的教学克隆/浏览器资料；不清空日常Chrome资料。返回[指南目录](README.md)。
