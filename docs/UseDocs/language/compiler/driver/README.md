---
id: language.compiler.driver
title: 前端到 VM 内部驱动器
status: verified
audience: contributor
module: rust.xiao-driver
stage: "09-B0-C"
version: "0.1.0"
related:
  - ../README.md
  - ../frontend/README.md
  - ../bytecode-runtime/README.md
  - ../../../../DevDocs/09b0c-frontend-to-vm-driver.md
  - ../../../../DevDocs/09b0b-production-vm.md
---

# 前端到 VM 内部驱动器

状态：`verified`，对应 09-B0-C。该页面描述 Rust 内部库 ABI，不代表用户可见的
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

选择单一枚举是为了同时保留前端诊断和已执行结果的完整形状；整数退出码不在本批冻结，
继续留给 11/X0。消费方应读取 `code`、`phase` 和报告字段，不解析人类可读消息判断成败。

## 取消与超时

`CancellationToken` 和 `RunControl` 支持从驱动调用开始计时的超时，以及跨线程取消信号。
本批在驱动器边界采样：开始、前端完成、降低完成和 VM 调用前后。VM 指令循环中途检查点
记录为 `B0-C-CANCEL-001`，出口批次为 `11/X0`；当前接口不会声称可以中途打断正在运行的 VM。

## 相关实现

- [前端流水线](../frontend/README.md)
- [生产 VM 执行闭环](../../../../DevDocs/09b0b-production-vm.md)
- [B0-C 交接文档](../../../../DevDocs/09b0c-frontend-to-vm-driver.md)
