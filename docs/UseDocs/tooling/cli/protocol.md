---
id: tooling.cli.protocol
title: Rust 核心进程协议
status: verified
audience: CLI 集成开发者
module: rust.xiao-driver
stage: 11X0
related:
  - ../../../DevDocs/11x0-cli-protocol-and-toolchain.md
  - README.md
---

# Rust 核心进程协议

X0-A 的核心入口是 `xiao-core` 子进程。调用方先发送一个 `hello` 帧完成版本协商，
再发送 `run`、`build`、`cancel` 或 `shutdown`。本页描述已经验证的机器边界；用户可见的
`xiao` 命令、打包和平台发现仍属于后续 X0 批次。

## 帧格式

每帧由 8 字节大端无符号长度和 UTF-8 JSON 负载组成。长度只计算 JSON 字节，最大负载是
16 MiB。干净 EOF 表示输入结束；部分长度、部分负载、非法 UTF-8、非法 JSON 和超长帧
都会返回 `X11-PROTOCOL-001`。调用方应读取稳定 `code`，不解析人类可读消息。

## 版本与结果

`protocol_version = 1` 和统一 `core_version = 1` 决定兼容性。前端、驱动器、字节码格式、
Runtime ABI 和 LLVM 版本只在 `versions` 中用于诊断。失配返回 `X11-PROTOCOL-004`，
不会执行用户源码。

`run` 和 `build` 响应始终包含 `request_id`、冻结的 `exit_code`（0 到 4）和 `exit_name`。
运行响应还提供结构化诊断、错误报告、事件和指标；执行前拒绝的 `error` 响应也保留可选
`report`；构建响应提供产物路径与工具链指纹。
取消通过同一请求 ID 绑定 `CancellationToken`，其结果使用 `ArtifactRejected` 的进程码 2。

## 共享契约

Rust 与 TypeScript 当前采用共享 JSON fixture 的窄类型策略。样本位于
`tests/spec/11x0-protocol/`，两侧测试都必须读取同一批样本并验证回环。新增字段先更新
样本和两侧显式类型，再更新协议版本或核心版本。
