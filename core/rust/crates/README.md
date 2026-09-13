# `core/rust/crates/`

## 目录职责

存放 Xiao Rust 核心的可独立编译单元，以及 A0 文档工具使用的内部 AST 适配 crate。新增 crate 前必须先登记工程期、依赖方向和公共 API，并添加 crate 根 README 与 `src/README.md`。

## 依赖原则

依赖从源码/诊断层逐步流向 IR、后端和驱动层；禁止 CLI 反向依赖内部 crate 实现细节，禁止平台 crate 改写语言语义。

## 工程期

01–19 按 [00A. 工程框架与目录布局](../../../docs/DevDocs/00a-project-layout.md) 的表格启用。

`xiao-doc-coverage-rust` 只在 A0.3–A0.4 为文档覆盖率工具提供 Rust AST 记录，不得被语言核心依赖。
