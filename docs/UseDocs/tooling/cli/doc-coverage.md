---
id: tooling.cli.doc-coverage
title: 文档覆盖率检查
status: verified
audience: contributor
module: ts.doc-coverage
stage: "A0"
related:
  - ../README.md
  - repo-check.md
  - ../../troubleshooting/README.md
---

# 文档覆盖率检查

本工具统计 Rust、TypeScript、TSX、测试辅助和构建工具中的声明文档。A0 的门槛是全仓库至少 90%，公共 API/导出项 100%；单行 Rustdoc/JSDoc 只要直接关联声明也会计入，UseDocs 页面不能替代代码 docstring。

## 运行方式

```text
bun tools/doc-coverage/src/cli.ts
bun tools/doc-coverage/src/cli.ts --format json --out coverage/docs.json
bun tools/doc-coverage/src/cli.ts --format sarif --out coverage/docs.sarif
```

Rust 文件由内部 `syn` AST 适配器解析，TypeScript 文件由 Compiler API 解析。解析失败会直接失败，不会用正则猜测结果。

## Rust 编译门禁

文档覆盖率检查和 Rust 编译器 lint 是两道独立门槛。所有 Rust workspace 成员都在自己的
`Cargo.toml` 声明 `[lints] workspace = true`，共享配置中的 `missing_docs` 由标准命令
`cargo clippy --workspace --all-targets -- -D warnings` 提升为失败；因此新增公共声明时，
必须同时补 Rustdoc 并运行这条命令。`bun run check:coverage` 负责更宽的全仓声明统计，不能由
Clippy 或 UseDocs 页面替代。

## 阅读报告

终端输出包含总体、公共 API 和每个 workspace 成员的分子/分母及百分比；JSON/SARIF 报告还列出缺口声明的文件、行号、类别和稳定错误码。`A0-COVERAGE-001` 表示阈值未达，`A0-COVERAGE-002` 表示具体声明缺少文档，`A0-PARSER-001` 表示解析器失败。

## 相关页面

- [仓库完整性检查](repo-check.md)
- [Rust AST 覆盖率适配器](doc-coverage-rust.md)
- [命令行参考](README.md)
- [故障排查](../../troubleshooting/README.md)
