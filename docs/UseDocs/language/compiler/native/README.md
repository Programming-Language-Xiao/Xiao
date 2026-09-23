---
id: language.compiler.native
title: 前端到 LLVM 原生内部驱动器
status: verified
audience: contributor
module: rust.xiao-codegen-llvm
stage: "10B"
version: "0.1.0"
related:
  - ../README.md
  - ../frontend/README.md
  - ../driver/README.md
  - ../../../../DevDocs/10-native-backend.md
  - ../../../../DevDocs/10a-n0-native-closure.md
  - ../../../../DevDocs/10b-n0-runtime-abi.md
---

# 前端到 LLVM 原生内部驱动器

状态：`verified`，对应 N0-A/N0-B。调用方把真实 Xiao 源码交给统一
`FrontendCompiler`，再把同一份已验证的 `IrProgram` 交给 `xiao-codegen-llvm`；本页面描述
Rust 内部接口。用户可见的 `xiao build` 已由 X0-E 接入，用法见
[原生构建命令](../../../tooling/cli/build.md)。

## 已接入能力

静态路径降低 `int`/`sint`、`float`/`sfloat`、`bool`、函数调用、分支和循环。整数算术使用
LLVM checked intrinsic，浮点结果检查非有限值；失败边调用 `llvm.trap`，统一 Xiao 错误
Runtime 留给 N0-C。含字符串、动态值、数组、元组、字典、集合或表的程序切换到 N0-B
的固定 tagged value ABI；静态标量程序继续保持不链接 Runtime。动态路径支持字符串、数组、
元组、字典、集合、表字段默认值和字段读写，以及已类型检查的
布尔 `if`/`elif`/`else` 和无异常转移的 `while`，当前仍明确拒绝函数体、`for`、表方法、
构造参数、异常、动态派发和 `*args`/`**kwargs`，避免静默丢弃语义，待后续批次接入。若分支或
循环作用域声明了需要释放的动态局部值，也会在降低前结构化拒绝；当前释放器只消费程序根
作用域计划，块级释放计划留给后续批次。表方法、构造参数和其他未接入函数表的表体语句
仍会结构化拒绝。

N0-B 的正常作用域退出、`return` 释放按前端 `IrOwnership.release_plans` 发射；异常展开
与 `try`/`catch`/`finally` 的错误路径属于 N0-C。

## 工具链与指纹

`Toolchain` 的路径由调用方显式注入；N0 不搜索 PATH。`llvm-as`（或 clang 的 IR 编译模式）
先验证文本，再由 clang 生成并链接目标文件。动态模块还必须由调用方通过
`Toolchain::with_runtime_library` 注入与目标 triple 匹配的 `xiao-runtime` 静态库，并通过
`Toolchain::probe_native_static_libraries` 让 Rust 工具链报告该 staticlib 的原生依赖；已经
由调用方取得的规范化清单也可用 `Toolchain::with_native_static_libraries` 注入。缺少原生库
清单时后端拒绝动态链接，不会硬编码平台库名。后端不自动发现或下载 Runtime。目标 triple、
固定宽度描述、后端版本、ABI 编码版本、Rust/LLVM 版本首行和原生库清单进入构建指纹，绝对
安装路径只保留在调用方日志中。版本登记位置为 `core/rust/llvm-toolchain.toml`。

## ABI 边界

静态标量程序的产物不链接完整 `xiao-vm` 或 `xiao-runtime`。动态能力使用独立
`xiao-runtime-abi` 的不透明强/弱句柄、固定布局 tagged value、容器入口和表描述符；
不能把 Rust 内部枚举布局当作语言契约。Runtime ABI 主版本为 1，当前次版本为 1；布局、
标签或所有权契约改变时必须升主版本。

动态模块的 `LlvmModule::runtime_components` 是生成器声明的 ABI 组件清单，供构建诊断解释
产物组成；它不承诺未使用的 Runtime 实现会被链接器裁剪。
