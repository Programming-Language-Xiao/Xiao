---
id: tooling.cli.doc-coverage-rust
title: Rust AST 覆盖率适配器
status: verified
audience: contributor
module: rust.xiao-doc-coverage-rust
stage: "A0"
related:
  - doc-coverage.md
  - ../README.md
  - ../../troubleshooting/README.md
---

# Rust AST 覆盖率适配器

这是文档覆盖率工具内部使用的 Rust 组件说明。它使用 `syn` 原生 AST 识别 Rust 声明，输出给 Bun/TypeScript 编排器消费的版本化 JSON；它不是面向 Xiao 用户的独立命令。

## 何时需要阅读

只有在维护 Rust 文档覆盖率规则、协议或适配器构建失败时才需要阅读本页。日常检查请先看[文档覆盖率检查](doc-coverage.md)。

## 协议边界

请求和响应都带有 `protocol_version`，当前版本为 `1`。响应固定包含 `declarations` 与 `errors` 数组；源文件语法错误会通过结构化错误返回，编排器不会退回正则扫描。

适配器由覆盖率命令按仓库 Rust workspace 构建和调用。它只处理传入的 Rust 文件，不执行 Xiao 源码，也不负责生成用户程序。

## 失败处理

看到 `A0-PROTOCOL-001` 时，先确认适配器和 TypeScript 编排器来自同一版本；看到 `A0-PARSER-001` 或文件读取错误时，检查源文件语法、路径和 Rust 工具链，再重新运行[覆盖率检查](doc-coverage.md)。

## 相关页面

- [文档覆盖率检查](doc-coverage.md)
- [命令行参考](README.md)
- [故障排查](../../troubleshooting/README.md)
