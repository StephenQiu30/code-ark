# 维护者：源码、候选构建与最终运行证据

两个工作流各有独立含义：`toolbox-ci.yml`在Ubuntu22.04检查源码、锁、生成DTO、Rust、组件、TS和教学源码；`toolbox-native.yml`只在显式dispatch后使用自建原生机器构建候选。编译成功不会填写运行验收，缺runner会排队，不用跳过来伪装通过。

原生runner必须具备标签`self-hosted`、`codeark-native-build`以及四者之一：`codeark-mac-arm64`、`codeark-mac-x64`、`codeark-win11-x64`、`codeark-ubuntu2204-x64`。实际process.platform/arch、Windows11 build、macOS14内核基线和Ubuntu22.04构建基线仍被再次检查，不仅相信label。macOS要满足当前Docker系统支持范围；Ubuntu24.04另做最终包运行，不能在新glibc上构建后声称兼容22.04。

构建机需要系统原生编译器/Tauri前置库、Rustup和bash（Windows使用受控Git Bash/MSVC环境）；源码job安装锁定Node24.22.3、pnpm12.18.0、Rust1.99.1、Maven3.9.16与Java17。它构建并核对只读协议探针JAR，再将同源SHA的白名单artifact传给原生job，原生job重新按SHA锁核对。官方Actions均固定commit，不引用可移动的v4/latest标签。

候选镜像门禁要求每个目标的registry descriptor或原生image-save归档已解析且架构匹配。MinIO/XXL/ES-IK producer必须先在独立原生CPU生成公开可获取归档、SHA与image ID；缺失时候选构建按设计失败。开发模式不能作为候选/发行绕过手段。

原生job要求DMG（两种Mac CPU各自构建）、Windows NSIS、Linux deb与AppImage实际存在；缺一种格式就是失败。收据记录候选文件SHA、源提交、pnpm/Cargo/image锁摘要、实际OS/CPU。签名配置未提供时写`not-configured`，不伪造已签名/公证。

## 最终运行仍需外部证据

从同SHA候选包安装，在没有Git/Node/Rust/源码的原生用户环境分别验证17服务、7场景、无Docker/已有Docker、首启、SDK源码教学、故障恢复、升级和卸载保留数据。用户安装业务语言工具只发生在选择源码教学后。Ubuntu22.04与24.04分别记录WebKitGTK/CSS和运行结果。CI源代码双替身、hosted runner构建或QEMU模拟不计原生验收。

`verify-guides.mjs --mode candidate --check-clients`核对源码完整性、直依赖版本与已下载第三方JAR SHA。release还需`--target`、`--candidate`与`--evidence`：证据应匹配命令输出中的manifestSha256/sourceDigest、实际candidateSha256、目标ID、每场景guideId/clientVersion/真实status=passed，以及installer.status=passed。它与blueprint release gate一起执行，不能只通过其中一个：

```sh
node toolbox/scripts/verify-guides.mjs --mode release --target "$CODEARK_TARGET" --candidate "$CODEARK_CANDIDATE" --evidence "$CODEARK_GUIDE_EVIDENCE"
node toolbox/scripts/verify-blueprints.mjs --mode release --target "$CODEARK_TARGET" --candidate "$CODEARK_CANDIDATE" --evidence "$CODEARK_BLUEPRINT_EVIDENCE"
```

这些环境变量由维护者选定的真实文件填写，本文没有现成“passed”证据。候选原样推广，不回写包内pending状态改变已验收SHA。本轮工作流仅做本地YAML/actionlint/源码校验；没有在GitHub运行，没有创建安装包或补填原生矩阵。

返回[指南目录](README.md)。
