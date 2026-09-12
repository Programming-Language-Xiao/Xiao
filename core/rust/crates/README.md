# `core/rust/crates/`

## 目录职责

存放 Xiao Rust 核心的可独立编译单元。新增 crate 前必须先登记工程期、依赖方向和公共 API，并添加 crate 根 README 与 `src/README.md`。

## 依赖原则

依赖从源码/诊断层逐步流向 IR、后端和驱动层；禁止 CLI 反向依赖内部 crate 实现细节，禁止平台 crate 改写语言语义。

## 工程期

01–19 按 [00A. 工程框架与目录布局](../../../docs/DevDocs/00a-project-layout.md) 的表格启用。
