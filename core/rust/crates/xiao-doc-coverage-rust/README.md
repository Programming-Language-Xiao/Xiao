# `xiao-doc-coverage-rust`

## 目录职责

提供 A0 文档覆盖率工具所需的 Rust `syn` AST 适配器，把 Rust 声明转换为稳定 JSON 记录。它是开发工具内部 crate，不实现 Xiao 语言、Runtime 或用户可见命令。

## 工程期

A0.3 建立原生 Rust AST 扫描；A0.4 由 Bun/TypeScript 覆盖率编排器调用并纳入 CI。

## 依赖边界

只允许依赖 Rust AST 和 JSON 序列化库；不得被任何 Xiao 语义 crate 反向依赖。公共 API 使用 Rustdoc，协议版本当前为 1；协议变更需同步更新 TypeScript 适配器、测试和 UseDocs。
