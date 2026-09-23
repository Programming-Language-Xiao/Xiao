---
id: language.compiler
title: 前端与中间表示
status: verified
audience: contributor
module: rust.xiao-ir
stage: "08A"
version: "0.1.0"
related:
  - ../README.md
  - frontend/README.md
  - driver/README.md
  - ir/README.md
  - bytecode-runtime/README.md
  - ../../../DevDocs/08a-u0-frontend-implementation.md
---

# 前端与中间表示

本目录说明已经验证的 Xiao 前端产物和内部编译运行边界，面向需要查看编译诊断、前端阶段、
IR 快照和 Rust 驱动器的开发者。用户可见的 `xiao run` 与 `xiao build` 已由 11/X0 接入。

## 阅读顺序

1. [前端流水线](frontend/README.md)
2. [IR 快照](ir/README.md)
3. [前端到 VM 内部驱动器](driver/README.md)
4. [字节码运行路径](bytecode-runtime/README.md)
5. [LLVM 原生内部驱动器](native/README.md)

页面状态：前端流水线、IR 快照和内部驱动器为 `verified`（对应 08A/U0 与 09-B0-C/D）；
字节码运行路径页面仍描述研究与生产边界；命令用法见[命令行参考](../../tooling/cli/README.md)。
