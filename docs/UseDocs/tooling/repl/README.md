---
id: tooling.repl
title: 单行交互会话
status: verified
audience: learner
module: ts.xiao-cli
stage: 11B-I0
related:
  - ../cli/README.md
  - ../../../DevDocs/11bi0-terminal-skeleton.md
---

# 单行交互会话

直接运行 `xiao` 进入单行模式；每次按 Enter 提交一行 Xiao 源码，执行出错后仍可输入下一行。
输入结束（如终端中的 EOF）即可离开会话。当前每行独立通过 Rust `run` 协议执行，
尚不提供跨行共享的用户变量或延迟加载的环境包。

启动时显示版权与版本行，提示符显示当前路径及可选的 `$venv$` 环境段；
`[X>` 是输入标记。环境名来自 `XIAO_ACTIVE_ENV` 的路径末段，渲染提示符不读取包代码。

Git 摘要默认关闭；在项目中执行 `xiao config CLI.git.summary true`，或通过
`xiao config --global CLI.git.summary true` 设置全局默认值。项目级配置优先。
启用后每次显示提示符前刷新分支和领先/落后计数；没有跟踪上游时只显示分支名。
Git 缺失、不是仓库、状态错误或超时时保留基本提示符；可用 `xiao -debug`
查询失败时的稳定诊断编号。非 TTY、`NO_COLOR`、`TERM=dumb` 和
`--color=never` 均不输出提示符 ANSI 色。

## 前置知识

建议先阅读[命令行参考](../cli/README.md)和[开始使用](../../getting-started/README.md)。

## 后续批次

`--inLF` 目前明确提示多行模式尚未实现；`!inLF!`、`!outLF!`、`!save!`、
`!panel!`、Esc 和快捷键也留给后续批次，不把占位参数写成可用命令。
