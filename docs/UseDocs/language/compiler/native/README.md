---
id: language.compiler.native
title: 前端到 LLVM 原生内部驱动器
status: verified
audience: contributor
module: rust.xiao-codegen-llvm
stage: "10A"
version: "0.1.0"
related:
  - ../README.md
  - ../frontend/README.md
  - ../driver/README.md
  - ../../../../DevDocs/10-native-backend.md
  - ../../../../DevDocs/10a-n0-native-closure.md
---

# 前端到 LLVM 原生内部驱动器

状态：`verified`，对应 N0-A。调用方把真实 Xiao 源码交给统一
`FrontendCompiler`，再把同一份已验证的 `IrProgram` 交给 `xiao-codegen-llvm`；本页面描述
Rust 内部接口，不代表用户可见的 `xiao build` 已接入。

## N0-A 范围

当前只降低 `int`/`sint`、`float`/`sfloat`、`bool`、函数调用、分支和循环。整数算术使用
LLVM checked intrinsic，浮点结果检查非有限值；失败边调用 `llvm.trap`，统一 Xiao 错误
Runtime 留给 N0-C。字符串、容器、表、异常、动态派发和 `*args`/`**kwargs` 会在降低前
返回结构化拒绝。

## 工具链与指纹

`Toolchain` 的路径由调用方显式注入；N0 不搜索 PATH。`llvm-as`（或 clang 的 IR 编译模式）
先验证文本，再由 clang 生成并链接目标文件。目标 triple、固定宽度描述、后端版本和工具链
版本首行进入构建指纹，绝对安装路径只保留在调用方日志中。版本登记位置为
`core/rust/llvm-toolchain.toml`。

## ABI 边界

静态标量程序的产物不链接完整 `xiao-vm` 或 `xiao-runtime`。后续动态能力使用独立
`xiao-runtime-abi` 的不透明句柄和固定宽度 C ABI，不能把 Rust 内部枚举布局当作语言契约。
