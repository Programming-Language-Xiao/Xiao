# `xiao-runtime-abi`

## 目录职责

本 crate 保存 Xiao 原生程序与 Rust Runtime 之间的稳定 C ABI 契约。公开层只使用固定宽度
标量、长度参数和不透明句柄，不暴露 `xiao-runtime` 的 Rust 枚举、引用计数对象或容器布局。

## 工程期

10A 先登记 ABI 版本和最小句柄操作；动态值、容器、错误展开和释放钩子在 N0-B/N0-C 接入。

## 允许与禁止依赖

当前不依赖任何工作区 crate，也不反向依赖 LLVM 后端。后续 Runtime 实现可以实现这些 ABI
入口，但不得把内部布局写入本 crate 的公开类型。
