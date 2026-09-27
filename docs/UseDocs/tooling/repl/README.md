---
id: tooling.repl
title: 交互会话与多行编辑
status: verified
audience: learner
module: ts.xiao-cli
stage: 11B-I0/I1a/I1c
related:
  - ../cli/README.md
  - ../../../DevDocs/11bi0-terminal-skeleton.md
  - ../../../DevDocs/11bi1a-multiline-buffer.md
  - ../../../DevDocs/11bi1c-cursor-selection-clipboard.md
---

# 交互会话与多行编辑

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

I1a 多行模式：输入独占一行的 `!inLF!`，或直接运行 `xiao --inLF`，进入 raw mode 编辑。
EOF/Ctrl+D 可退出空缓冲，Ctrl+C 清空当前编辑并回到单行提示符；多行缓冲保留真实换行和
缩进，五字符行号栏与软换行只影响画面，中文和 emoji 按显示列宽、光标按 Unicode 字素处理。
括号粘贴保留内部换行；基础空白/标点按词移动已实现。`Ins` 或独占一行的 `!ovr!` 可切换覆盖模式，
覆盖模式在当前逻辑行替换字素，行尾只追加，不吞掉下一行。按住 Shift 加方向键、Home 或 End 可扩展选区；
无 Shift 的方向键会把光标移到选区相应端点。输入、粘贴、Backspace 和 Delete 在有选区时作用于整个选区。

`Ctrl+Shift+C`、`Ctrl+Shift+X` 和 `Ctrl+Shift+A` 使用程序内编辑器剪贴板；它与终端或系统剪贴板不同，
本版本不使用 OSC 52。传统终端无法上报这些组合键时，终端自身仍可处理复制操作，不会产生 Xiao 错误。
`Ctrl+←/→` 是逻辑行首/行尾，`Ctrl+↑/↓` 是缓冲区首行/尾行，不是按词移动。

多行模式会开启 SGR 鼠标点击和按住拖动。点击行号栏不移动光标，双宽字符后半格归到该字素之前；
按住 Shift 的本地选择由终端处理。鼠标滚轮在编辑器中被忽略，按住 Shift 滚动可交回终端。

`!outLF!` 独占逻辑行时会从缓冲区和源码行号映射移除并分派到运行准备；`!save!`、
`!panel!` 已接入共享分派但分别留给 I2、I3。传统终端无法区分修饰键时，普通 Enter
不会被误判为 Shift+Enter，后者仅在 Kitty“所有按键上报”能力确认后启用。

建议先阅读[命令行参考](../cli/README.md)和[开始使用](../../getting-started/README.md)。

## 后续批次

I1b 的运行确认、完整缓冲区执行及输出摘要尚未实现；`!save!` 的实际写盘留给 I2，
`!panel!` 的面板界面留给 I3。`xiao --inLF <file.xiao>` 的文件参数当前明确拒绝，
文件打开行为留给 I2。环境包延迟加载和跨行共享运行时状态留给 I4。
