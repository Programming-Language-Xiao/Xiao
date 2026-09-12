# `core/`

## 目录职责

这里存放 Xiao 的语言核心、编译器后端、Rust 字节码 VM 和执行时 Runtime。核心代码不负责终端提示符、命令行参数或 Shell 状态。

## 工程期

01–10 建立语言前端、IR、Runtime、字节码和 LLVM 后端；13–17 逐步加入优化、产物缓存和 `.xar` 支持。

## 子目录

- `rust/`：Rust workspace 及所有核心 crate。

## 依赖边界

核心可以依赖标准库和明确登记的第三方库，不能依赖 `cli/`。平台差异只能通过 `xiao-platform` 进入。

