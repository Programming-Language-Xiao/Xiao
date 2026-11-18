# `xiao-doc-coverage-rust/src`

## 目录职责

实现 A0 文档覆盖率工具的 Rust 原生 AST 适配器。库负责把 Rust 声明转换为稳定 JSON 记录，二进制入口只提供给 TypeScript 编排器调用，不是面向 Xiao 用户的独立 CLI。

## 工程期

A0.3 建立 `syn` 解析和 JSON 协议；A0.4 接入 Bun 检查器与 CI。后续只在协议版本变更时扩展字段。

## 协议与规则

请求和响应都必须携带 `protocol_version`（当前为 2）；请求中的 `outline` 开关缺省为
`false`，响应始终带有 `outlines` 数组，未请求时为空。解析失败必须返回结构化错误，不得
以正则扫描替代 AST。覆盖率判定只读取 `declarations`，结构大纲是独立的按需通道。所有
公共 Rust 项都必须有 Rustdoc，并同步维护 `tools/doc-coverage` 的测试和 UseDocs。
