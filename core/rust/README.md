# `core/rust/`

## 目录职责

Rust workspace 根目录。编译器前端、类型化 IR、Rust VM、Runtime、LLVM 后端、包/产物核心逻辑，以及 A0 文档覆盖率 AST 适配器都以独立 crate 放在 `crates/` 下。

## 工程期

F0/A0 建立 workspace；01–10 按顺序启用前端和执行核心；13–17 启用优化与产物链。

## 规则

- 每个 crate 保持单一职责和可独立测试的 API；`xiao-doc-coverage-rust` 是工具内部 crate，不属于 Xiao Runtime。
- `src/` 内的公共项必须有 100% Rustdoc；全部函数/方法/类型按全仓库 90% 门槛检查。
- 这里不放 TypeScript、终端 UI 或用户项目代码。

## 子目录

- `crates/`：具体 Rust crate，见其各自 README。
- `llvm-toolchain.toml`：N0-A 外部 LLVM 工具链版本策略与构建指纹字段；工具路径由调用方注入。
