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

请求和响应都带有 `protocol_version`，当前版本为 `2`。请求形如
`{"protocol_version":2,"files":["..."],"outline":false}`，其中 `outline` 是可选布尔开关，
省略时按 `false` 处理。响应始终包含 `declarations`、`outlines` 与 `errors` 三个数组；即使
没有请求大纲，`outlines` 也必须存在并为空数组。源文件语法错误会通过结构化错误返回，
编排器不会退回正则扫描。

`declarations` 是覆盖率判定的唯一输入。每条声明含 `file`、`line`、`kind`、`name`、
`is_public`、`has_doc` 和 `end_line`；`line` 指向声明自身起始行，`end_line` 指向整个项的
结束行。`outlines` 是独立的结构通道，每个 `FileOutline` 含 `file` 与 `nodes`，节点
`OutlineNode` 含以下字段：

| 字段 | 含义 |
| --- | --- |
| `kind` / `name` | 节点类别与名称 |
| `line` / `end_line` | 声明自身起始行与整个项结束行 |
| `lines` | 从 `line` 到 `end_line` 的行数，满足 `line + lines - 1 == end_line` |
| `signature` / `source_line` | 去名后的定义句与声明起始行源码 |
| `children` | 字段、变体、方法或嵌套项的递归节点 |

大纲只在调用方确实需要判断文件结构时生成。`A0-SIZE-001` 是该通道的第一个消费者；它只在
文件超过 2500 物理行后请求大纲，覆盖率路径仍不承担大纲成本。调用约定如下：

| 调用方 | `outline` | 原因 |
| --- | --- | --- |
| 覆盖率检查 `checker.ts` | 不传（默认 `false`） | 只读取 `declarations`，避免正常检查承担大纲体积 |
| 单文件过长门禁发现超标后 | `true` | 需要渲染该文件的结构大纲以指导解耦 |

Rust 大纲与 TypeScript 的 `TypeScriptOutlineNode` 字段同形，均使用 `kind`、`name`、行号、
签名、源码行与递归 `children`；两者类型保持独立，Rust 的版本化响应校验边界不因
TypeScript 大纲扩张。适配器由覆盖率命令和尺寸门禁按仓库 Rust workspace 构建和调用。它只
处理传入的 Rust 文件，不执行 Xiao 源码，也不负责生成用户程序。

## 失败处理

看到 `A0-PROTOCOL-001` 时，先确认适配器和 TypeScript 编排器来自同一版本；看到
`A0-PARSER-001` 或文件读取错误时，检查源文件语法、路径和 Rust 工具链，再重新运行
[覆盖率检查](doc-coverage.md)。尺寸门禁请求大纲时，适配器等待上限为 120 秒；并发 Cargo
构建可能持锁，超时会保留 `A0-SIZE-001` 并追加 `A0-PARSER-001`。可先运行
`cargo build --manifest-path core/rust/Cargo.toml -p xiao-doc-coverage-rust`，或设置
`XIAO_RUST_DOC_ADAPTER` 指向预编译二进制。

## 相关页面

- [文档覆盖率检查](doc-coverage.md)
- [命令行参考](README.md)
- [故障排查](../../troubleshooting/README.md)
