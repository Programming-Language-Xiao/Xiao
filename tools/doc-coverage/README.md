# `tools/doc-coverage`

扫描 Rust/TypeScript AST，统计全仓库函数、方法、类、模块文档覆盖率和公共 API 缺口。工程期 A0.3 建立统一报告契约，A0.4 接入 CI；门槛为总覆盖率 ≥90%、公共 API 100%。实现、测试和 UseDocs 必须同步交付，源码位于 [`src/`](src/README.md)。
