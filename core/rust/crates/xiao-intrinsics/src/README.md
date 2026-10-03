# `xiao-intrinsics/src`

## 目录职责

保存契约 crate 的 Rust 类型、生成表入口和契约一致性校验；生成产物来自同 crate 的
`intrinsics.json` 与 `build.rs`，不在源码目录手工维护副本。

## 工程期

20AB：为类型层、IR、字节码、VM 和 LLVM 提供后端无关的 intrinsic 声明消费接口。

## 依赖边界

本目录不得依赖 Runtime、VM、LLVM 或前端实现；实现层只能反向依赖本契约目录。
