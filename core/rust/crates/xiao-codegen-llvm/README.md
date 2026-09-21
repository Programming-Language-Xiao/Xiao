# `xiao-codegen-llvm`

## 目录职责

把已验证的类型化 IR 降低为 LLVM IR，并通过调用方注入的 `clang`/`llvm-as` 生成原生产物。
N0-A 只覆盖静态标量、函数调用、分支、循环和显式溢出失败边；动态值、容器、表和统一
错误 Runtime 留给 N0-B/N0-C。

## 工程期

10 建立未优化原生闭环；15 接入 `-O0`–`-O3`、Runtime 裁剪、链接和性能基线；平台顺序为 Windows → Linux → macOS。

## 模块放置

`src/ir.rs` 放置标量 LLVM 降低，`src/target.rs` 放置规范化目标，`src/toolchain.rs` 放置
显式工具链适配，`src/build.rs` 放置内部构建/运行观察面。稳定 Runtime ABI 位于独立的
`xiao-runtime-abi` crate；`.app`/签名编排由平台与发布工具负责。

## 允许与禁止依赖

后端只依赖 `xiao-ir`，不依赖 VM、完整 Runtime 或 CLI，也不引入 `inkwell`/`llvm-sys`。
不在后端改变类型、溢出或错误语义，不打包 `.xar`，不解析终端命令行，不自动发现工具链。
