---
id: tooling.cli.test
title: xiao test 项目测试
status: verified
audience: developer
module: ts.xiao-cli
stage: 11X0-T
related:
  - README.md
  - shell.md
  - protocol.md
  - ../../../DevDocs/11x0t-project-test-semantics.md
---

# `xiao test` 项目测试

`xiao test` 是 Xiao 项目测试运行器，不是 `cargo test`、`bun test` 或 CLI 自身规格测试的
包装器。它读取项目源码，通过 Rust 核心协议逐个执行测试文件，并依据退出码和结构化诊断
报告结果；不会读取或比较用户程序的标准输出。

## 输入与发现

```text
xiao test [project] [--timeout <ms>] [--json] [--color=auto|always|never]
```

省略 `project` 时使用当前工作目录；指定值必须是项目目录。CLI 递归发现项目根下的
`tests/**/*.xiao`，只读取普通文件，不跟随符号链接越过项目边界。每个 `.xiao` 文件都是
一个独立测试用例，不需要新增语言关键字、属性或 `config.xiao` 字段。

发现结果转换为项目相对路径的正斜杠形式，并按稳定字典序执行。例如，
`tests/nested/a.xiao` 一定排在 `tests/z.xiao` 之前。`--timeout` 是每个用例的驱动器期限，
而不是整个测试批次的总期限。

## 输出

`--json` 直接输出协议的 `test_result` 响应。响应包含整体 `exit_code`、`total`、`passed`、
`failed`，以及按执行顺序排列的 `tests` 数组；每个用例保留路径、模块名、退出码、诊断、
报告、事件、指标和协议错误。人类模式把整体统计和每个用例路径写到标准错误，失败用例的
结构化诊断也会显示；测试源码本身没有标准输出契约。

## 退出码

- 所有用例退出码为 `0` 时，命令返回 `0`。
- 只要有失败，整体返回按执行顺序遇到的首个非零协议退出码（冻结范围 `1..=4`）。
- 单个用例失败不会提前隐藏后续用例；所有已发现用例仍按顺序执行并出现在结果中。
- 参数错误、项目目录不可读、没有测试文件或测试源码不可读是 CLI 自身错误，分别使用
  usage/infrastructure 退出码，不伪造 `test_result`。

## 失败恢复与边界

普通源码或运行失败只标记当前用例，后续用例继续执行；协议响应中的诊断和报告按用例保留。
`SIGINT` 沿 `AbortSignal` 和既有 `cancel` 帧传入核心；超时与取消使用现有 VM 检查点，并在
驱动器边界映射为退出码 `2`。检查点是协作式机制，不是操作系统强制杀死核心进程。

每个用例在核心服务的 worker 线程中执行，但 `xiao test` 不提供子进程沙箱，也不隔离全局
状态、文件系统或外部副作用；不承诺在前一个用例破坏宿主状态后自动恢复环境。

## 平台差异

路径排序和协议字段不依赖宿主目录枚举顺序；Windows、Linux 和 macOS 使用相同的发现规则、
正斜杠路径和退出码语义。核心发现仍遵循 CLI 的平台布局、`XIAO_CORE_PATH` 和开发目录
规则；当前仓库的原生端到端证据以 Windows 为主，其他平台的复现状态见[命令行参考](README.md)
和[独立打包与核心发现](packaging.md)。
