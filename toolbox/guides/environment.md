# 安装、首次打开与环境助手

当前尚无公开发行安装包。本页放在应用外也能阅读；未来候选包必须附带实际文件名、SHA-256、源提交、签名/公证状态和支持矩阵。缺少某一目标验收，不能声明全部三端可用。

## 安装前

只选择与系统和原生 CPU 相符的候选。macOS14是界面技术基线，不代表当前 Docker 仍支持该版本；同时满足对应 Docker 版本的官方系统范围。当前计划验证 macOS原生arm64/x64、Windows11 x64（WSL2/Linux containers）、Ubuntu22.04/24.04 x64。Windows10、其他架构与发行版没有自动获得支持。

下载后用系统工具比较公开的候选SHA-256：macOS的`shasum -a 256`、Windows的`Get-FileHash -Algorithm SHA256`、Linux的`sha256sum`。文件名与公开校验值来自实际候选记录，不用本文的占位输出代替。签名、发布者或校验不符时停止安装。

## macOS

安装官方支持本机系统和CPU的[Docker Desktop](https://docs.docker.com/desktop/setup/install/mac-install/)，手动启动并等待引擎就绪。工作箱不捆绑Docker引擎。首次打开若被系统拦截，先核对候选校验和、签名状态及发布者，再按系统“隐私与安全性”中针对该应用的入口处理；不全局关闭Gatekeeper，不执行通用去隔离脚本。未公证候选必须明确标注，不能伪称已签名。

## Windows11

按[官方说明安装WSL2](https://learn.microsoft.com/windows/wsl/install)，检查虚拟化并完成要求的管理员授权/重启。安装[Docker Desktop](https://docs.docker.com/desktop/setup/install/windows-install/)，切换Linux containers；不要把Windows containers或远程context当作已验证目标。WebView2缺失时使用[微软官方运行库](https://developer.microsoft.com/microsoft-edge/webview2/)，安装后重新打开应用。工作箱不会后台提权、修改系统虚拟化或代替系统安装器。

## Ubuntu22.04 /24.04

按[Docker Engine官方Ubuntu步骤](https://docs.docker.com/engine/install/ubuntu/)安装Engine及Compose插件。对照[Linux安装后步骤](https://docs.docker.com/engine/install/linux-postinstall/)设置socket访问并重新登录；不要给`docker.sock`开放所有用户权限。安装包需要对应系统的WebKitGTK4.1/GTK3运行库；开发包则按[Tauri官方前置条件](https://v2.tauri.app/start/prerequisites/)安装，二者不能混为用户运行依赖。必须分别记录22.04和24.04的真实WebKitGTK版本与界面结果。

## 应用内流程与修复

打开工作箱 → 检测所选Docker目标 → 按错误码修复 → “重新检测” → “先试运行Redis”。未安装时打开官方安装指引；daemon未启动时手动启动；Compose能力缺失时更新Compose v2；权限、目标或CPU不符时保留选择并说明原因，不静默切换全局context。

端口冲突在服务配置草稿中修改并验证；内存不足在运行时设置中调整。磁盘容量未知时每次操作单独确认，不用一个全局开关永久跳过预检。首次镜像拉取需要网络；超时可以取消/重试，不能承诺固定一分钟完成。

镜像下载失败时检查Docker自身DNS、代理、网络和剩余空间，按[Docker代理说明](https://docs.docker.com/desktop/settings-and-maintenance/settings/#proxies)处理。工作箱不设置未知镜像站、不修改全局daemon代理、不借维护者镜像缓存宣称匿名分发完成。离线仅能使用已验证缓存；缺归档或架构未验证会显示阻断，不能让用户先装Go/JDK/Maven构建基础设施。

返回[指南目录](README.md)；继续[网络与连接](network.md)。
