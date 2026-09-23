---
id: tooling.cli.debug
title: -debug 诊断窗口
status: verified
audience: developer
module: ts.xiao-cli
stage: 11X0-D
related:
  - README.md
  - shell.md
  - packaging.md
  - ../../../DevDocs/11x0d-debug-diagnostics-window.md
---

# `-debug` 诊断窗口

`xiao run <file.xiao> -debug` 和源码快捷运行会把调试位放入 Rust 核心协议。核心在
前端/VM 执行前创建独立诊断进程，诊断进程再打开新的终端 TUI；用户程序的标准输出仍由
原运行进程拥有，诊断文本不会混入它。

诊断进程使用本机回环套接字和一次性令牌握手。事件以八字节大端长度前缀的结构化 JSON
发送，窗口显示模块、函数、作用域、释放、处理器、错误、堆栈和指标事件。窗口最后一行
固定显示运行时间、当前/峰值内存、运行时错误数、断点命中数和钩子数。默认等级为 `info`；
逐指令追踪需要显式的 `trace` 配置。

文件目标使用 JSONL。`[debug]` 的终端等级、文件等级、日志路径、堆栈详细程度和聚焦规则
通过协议的 `optimization.diagnostics` 传递；聚焦规则默认从总日志分流，只有 `mirror = true`
才同时写入总日志。错误计数不受等级过滤影响。

启动阶段没有可用终端或独立诊断组件时，命令返回 `X11-DIAGNOSTIC-START-001` 并且不执行
用户代码。窗口在运行中关闭、套接字中断或日志目标失败时，Runtime 继续执行，退出码和
用户程序语义保持不变。

## 平台边界

终端候选顺序固定为：Windows 的 `wt.exe`、`cmd.exe start`、PowerShell；Linux 的
`x-terminal-emulator`、`gnome-terminal`、`konsole`、`xterm`；macOS 的 `osascript`、
`open -a Terminal`。候选来源会进入结构化启动失败详情。Windows 原生是当前可宣称的
端到端平台；Linux Docker 只提供功能/构建证据，Linux 原生、WSL 和 macOS 仍待复现。

独立分发目录随 `xiao-core` 一并携带 `xiao-diagnostics`。调试原生构建的旁置
`<可执行文件>.xiao-debug.json` 是持久激活位；普通构建不会生成该文件，也不会因 `[debug]`
配置自行开窗。激活位格式和读取 API 位于 `xiao-diagnostics::window`。X0-E 已接入
`xiao build`：调试构建会把 `xiao-diagnostics` 复制到可执行文件目录，并把启动 shim 链接
进原生入口。Windows shim 使用新控制台并等待诊断进程写入一次性就绪标记；创建失败、诊断
进程提前退出或就绪超时都会在执行用户代码前返回 `X11-DIAGNOSTIC-START-001`。构建用法和
工具链环境变量见 [`xiao build`](build.md)。
