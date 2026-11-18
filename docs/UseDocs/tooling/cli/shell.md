---
id: tooling.cli.shell
title: xiao run 与 CLI 外壳
status: verified
audience: learner
module: ts.xiao-cli
stage: 11X0-B
related:
  - README.md
  - protocol.md
  - ../../../DevDocs/11x0b-cli-shell.md
---

# xiao run 与 CLI 外壳

X0-B 提供 TypeScript `xiao` 命令入口。`xiao run <file.xiao>` 和直接写文件名的快捷方式
读取真实 Xiao 源码，通过 `xiao-core` 的长度前缀 JSON 协议执行，再把结构化结果写到终端。
核心路径可以用 `XIAO_CORE_PATH` 指定；独立分发目录和生产发现顺序见[独立打包与核心发现](packaging.md)。
当前平台证据仍以 Windows 原生为主，Linux/macOS 的原生复现状态见该页。

## 运行结果

CLI 不从人类可读文案猜测成败。进程码直接取协议响应的 `exit_code`，诊断使用稳定的
`code`、`message_id`、`report` 和 `next_step` 字段。`--json` 把完整响应写到标准输出，
适合脚本消费；人类文本诊断写到标准错误。

`NO_COLOR` 或非 TTY 输出会自动去色，`--color=always` 可用于快照测试。中文和 emoji 的
表格宽度按终端显示列计算。管道接收端提前关闭时，CLI 会忽略 `EPIPE`，不会额外打印异常。

## 已知限制：没有 `print`

本阶段没有实现内置函数和标准库。`print("hello")` 仍会得到 Runtime 的结构化错误，成功
脚本的入口值也不会自动显示；因此一个没有诊断的 `xiao run hello.xiao` 没有用户程序输出
是预期行为，不表示 CLI 丢失了输出。内置函数契约属于 20 阶段，CLI 不会在这里添加替代
实现。

## 其他入口

`xiao config [--global] CLI.git.summary true|false` 和
`xiao config [--global] language.locale zh|zh-CN|en|en-US` 使用结构化、原子写回，保留
注释和无关配置。`xiao test` 的项目文件发现、结果和边界见[`xiao test`](test.md)；Rust 使用
`cargo test`，TypeScript 使用 `bun test`，两者仍是各自 workspace 的工程测试。`xiao build` 的输入、输出和工具链要求见
[`xiao build`](build.md)；无参数 REPL 和 `--inLF` 仍给出稳定未实现诊断。`-debug` 已接入
`run` 与源码快捷运行，诊断窗口行为见[-debug 诊断窗口](debug.md)。

项目测试失败时，CLI 直接使用协议 `test_result.exit_code`；只有参数、项目发现或源码读取等
CLI 自身错误才使用 usage/infrastructure 退出码。人类输出写标准错误，`--json` 输出完整协议
结果。

## 平台说明

本批在 Windows 原生开发树完成端到端回环，并在 Docker Linux 容器完成构建与仓库外回环；
Linux 原生、WSL 和 macOS 仍是待复现清单。WSL/容器数字不能替代原生平台验证。
