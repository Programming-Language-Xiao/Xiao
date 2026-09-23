# `xiao-codegen-llvm`

## 目录职责

把已验证的类型化 IR 降低为 LLVM IR，并通过调用方注入的 `clang`/`llvm-as` 生成原生产物。
静态标量继续走不链接 Runtime 的 N0-A 路径；N0-B 为字符串、动态值、容器和表生成固定
`xiao-runtime-abi` 调用，并在正常退出边消费所有权释放计划。统一可恢复错误展开仍留给 N0-C。

## 工程期

10 建立未优化原生闭环；15 接入 `-O0`–`-O3`、Runtime 裁剪、链接和性能基线；平台顺序为 Windows → Linux → macOS。

## 模块放置

`src/ir.rs` 放置标量 LLVM 降低，`src/dynamic.rs` 是 N0-B 动态降低门面，具体实现拆在
`src/dynamic/` 的谓词、文本、Runtime ABI、入口、槽、释放、控制流、表达式和容器模块中；
`src/target.rs` 放置规范化目标，`src/toolchain.rs` 放置显式工具链适配，`src/build.rs` 放置
内部构建/运行观察面。稳定 Runtime ABI 位于独立的 `xiao-runtime-abi` crate；`.app`/签名
编排由平台与发布工具负责。

## N0-B 动态降低器

`dynamic.rs` 只保留生成器状态、生成流程和 LLVM 发射原语；职责实现见
`src/dynamic/README.md`。动态路径可以依赖 `xiao-ir`、`xiao-runtime-abi` 和 crate 级
`text.rs`，静态 `ir.rs` 不得反向依赖 `dynamic/`。源码级依赖回归由
`src/dynamic_architecture_tests.rs` 锁定，拆分不新增 Runtime 能力、不改变所有权释放顺序，
也不升 `CODEGEN_VERSION`。

## 允许与禁止依赖

后端只依赖 `xiao-ir` 和零运行时实现依赖的 `xiao-runtime-abi` 声明，不依赖 VM、完整
Runtime 或 CLI，也不引入 `inkwell`/`llvm-sys`。
不在后端改变类型、溢出或错误语义，不打包 `.xar`，不解析终端命令行，不自动发现工具链。
