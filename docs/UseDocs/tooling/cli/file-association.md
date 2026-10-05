---
id: tooling.cli.file-association
title: xiao .xar 文件关联
status: verified
audience: user
module: ts.xiao-cli
stage: 18C
related:
  - README.md
  - packaging.md
  - ../../../DevDocs/18c-file-association-and-scripting.md
---

# xiao .xar 文件关联

关联动作分为安装、检查和移除三种，均只作用于当前用户：

```text
xiao association install [--platform=win32|linux|darwin] [--xiao <path>] [--json]
xiao association check   [--platform=win32|linux|darwin] [--json]
xiao association uninstall [--platform=win32|linux|darwin] [--json]
```

`file-association` 是 `association` 的别名。安装前会按显式 `--xiao`、`XIAO_CLI_PATH`、
PATH 和当前独立可执行位置寻找已安装的 `xiao`；找不到时拒绝安装，不会创建悬空关联。
检查只读取现有注册信息或用户目录文件，不创建目录、不写状态。`--json` 的结果写入标准输出，
`--verbose` 的过程记录写入标准错误；`--non-interactive` 禁止进入 REPL，适合 CI。

Windows 写入当前用户注册表的 `.xar` 类；Linux 写入
`~/.local/share/applications/xiao-xar.desktop` 和用户 MIME 包；macOS 使用用户目录中的
轻量 `.app` 与 `Info.plist`，再请求 LaunchServices 注册。macOS 真实注册/移除必须在 macOS
环境执行；其他系统上的测试需要显式设置 `XIAO_ALLOW_MACOS_ASSOCIATION=1`，并且结果会标记
`gated=true`。

关联启动仍等价于 `xiao -xar <file.xar>`：不依赖桌面当前目录，继承标准流并原样转发退出码。
未安装关联时直接使用该命令；它不会承诺归档脱离 xiao Runtime 独立运行。
