---
id: language.compiler.driver
title: 前端到 VM 内部驱动器
status: verified
audience: contributor
module: rust.xiao-driver
stage: "09-B0-D"
version: "0.1.0"
related:
  - ../README.md
  - ../frontend/README.md
  - ../bytecode-runtime/README.md
  - ../../../../DevDocs/09b0c-frontend-to-vm-driver.md
  - ../../../../DevDocs/09b0b-production-vm.md
  - ../../../../DevDocs/09b0d-exit-codes-and-linux-verification.md
---

# 前端到 VM 内部驱动器

状态：`verified`，对应 09-B0-C/D。该页面描述 Rust 内部库 ABI，不代表用户可见的
`xiao run` 已经接入。

## 运行链

`DriverRequest` 接收真实 `FrontendRequest`，`FrontendVmDriver` 依次调用统一前端、生产
`lower_program` 和 `xiao_vm::run_request`。驱动器只消费前端已经验证的 `FrontendArtifact`，
不重新解析源码、推断类型或生命周期，也不重算释放计划。脚本和 `[main]` 工程都使用 B0-B
固定的函数零入口；请求没有入口函数选择字段或机型选择字段。请求/结果字段版本由
`DRIVER_VERSION = 1` 标识。

## 结构化结果

`DriverOutcome` 用一个枚举覆盖三个阶段：

- `Frontend` 保留完整的 `FrontendError.diagnostics`，表示没有可执行产物。
- `Rejected` 包含阶段、稳定 `code`、路径和可选 `ReportRecord`，表示验证、请求或控制边界
  在进入 VM 前拒绝。
- `Executed` 保留 B0-B 的 `RunOutcome`、事件、指标、报告和前端非错误诊断；其中的
  `Success`、`Error`、`Fatal` 仍由 VM 结构化表达。

`ExitCode` 和 `DriverOutcome::exit_code()` 将这三段结果稳定映射为五种终局：成功为 `0`，
源码检查失败为 `1`，产物/请求/控制边界拒绝（含取消和超时）为 `2`，未捕获可恢复运行时
错误为 `3`，Fatal 为 `4`。派生只读取结构化结果，不读取诊断编号、消息或 locale 展示文本；
第 11/X0 阶段负责把 `as_process_code()` 接到宿主进程。

选择单一枚举是为了同时保留前端诊断和已执行结果的完整形状。消费方应读取 `code`、
`phase` 和报告字段，不解析人类可读消息判断成败。

## 取消与超时

`CancellationToken` 和 `RunControl` 支持从驱动调用开始计时的超时，以及跨线程取消信号；
驱动器把同一个来源和 deadline 注入 VM。`VmOptions.checkpoints_enabled` 可关闭热循环检查点，
`checkpoint_interval` 控制两次轮询之间的指令数。取消不进入用户 `catch`，但会尝试 `finally`
并执行释放计划；取消和超时仍映射为 `ArtifactRejected` 进程码 `2`。

检查点在 `run_blocks` 与 `run_subroutine` 的 `finish_table_effects` 之后执行，避免表析构错误
覆盖控制信号；`metrics.instructions` 不受检查点计数影响。CLI 的 `AbortSignal` 通过协议客户端
发送既有 `cancel` 帧，核心按请求 ID 触发同一取消源。

## 相关实现

- [前端流水线](../frontend/README.md)
- [生产 VM 执行闭环](../../../../DevDocs/09b0b-production-vm.md)
- [B0-C 交接文档](../../../../DevDocs/09b0c-frontend-to-vm-driver.md)
