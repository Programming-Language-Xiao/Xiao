---
id: language.compiler.bytecode-runtime.benchmarks-and-freeze
title: 09R3 基准与冻结记录
status: verified
audience: contributor
module: rust.xiao-r3-benchmarks
stage: "09R3"
version: "0.1.0"
related:
  - ./README.md
  - ../../../../DevDocs/09r3-benchmarks-and-freeze.md
  - ../../../../DevDocs/12-tests-and-milestones.md
---

# 09R3 基准与冻结记录

09R3 的基准工具位于 `tests/benchmarks`，以真实 Xiao 源码调用
`xiao-driver::FrontendCompiler`，只降低一次 IR/TAC，再用相同语义输入运行栈式、分类型寄存器式
和混合式载体。清单固定 Rust `1.96.0`、`release`、`opt-level = 3`、`codegen-units = 1`、
关闭 LTO、3 次预热、11 次测量，并按族记录中位数和四分位区间。

Windows 原生报告位于 `tests/benchmarks/reports/`：语义差分、性能、内存和编码体积各一份，
另有 `09r3-freeze.json`。布局固定 `FORMAT_VERSION = 3`、opcode `0..40`；Linux/macOS 仍在
待复现清单，WSL/容器数字不进入验收。冻结记录的 `family_direction_explanation` 按族记录本次
比值和快慢方向；本次样本中四族的寄存器式与混合式均慢于栈式。两种候选的全局中位数均未达到
相对同机栈式基线至少 10% 的吞吐提升，冻结记录据此选择栈式。

本页只记录已验证的基准入口和冻结结果；生产 `xiao run`、LLVM 原生对照和正式 `.xiaoc` 容器
仍由后续里程碑接入。
