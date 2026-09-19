---
id: tooling.cli.repo-check
title: 仓库完整性检查
status: verified
audience: contributor
module: ts.repo-check
stage: "A0"
related:
  - ../README.md
  - ../../troubleshooting/README.md
---

# 仓库完整性检查

本工具检查 Xiao 仓库的 Bun/Cargo workspace、代码目录 README、单文件行数、模块登记和文档链接。它只读取仓库，不自动修改 manifest、源码或文档。

## 运行方式

在仓库根目录执行：

```text
bun tools/repo-check/src/cli.ts layout
bun tools/repo-check/src/cli.ts docs
bun tools/repo-check/src/cli.ts usedocs
bun tools/repo-check/src/cli.ts all
```

`layout` 会扫描清单登记的项目维护源码；文件超过 2500 物理行后，它才请求该文件的结构
大纲。超标文件是 Rust 时会调用 Rust 工具链，因此首次运行前可执行：

```text
cargo build --manifest-path core/rust/Cargo.toml -p xiao-doc-coverage-rust
```

也可以设置 `XIAO_RUST_DOC_ADAPTER` 指向已经构建的适配器二进制；**这是推荐做法**：适配器默认
经 `cargo run` 启动，而 cargo 进程创建本身约需 10 秒（实测本机 `cargo --version` 即 10.2 秒），
直调已构建二进制只需约 64 毫秒。适配器等待上限为 120 秒；
并发 Cargo 构建持锁时，检查会以 `A0-PARSER-001` 失败而不是无限等待。`all` 按固定顺序执行
workspace、目录与尺寸、DevDocs、UseDocs 和文档覆盖率检查，所以 `bun run check:layout` 与
`bun run check` 都可能调用 Rust 工具链。需要脚本化时可添加 `--format json` 或
`--format sarif`，并用 `--out` 保存报告。

## 单文件行数门禁

`A0-SIZE-001` 的上限是 2500 物理行，注释和测试模块也计入。超过上限会给出行数摘要与
树形结构大纲；文本、JSON 和 SARIF 都保留大纲的结构化详情。大纲解析失败不会放过尺寸
错误：原 `A0-SIZE-001` 仍是 `error`，同时追加 `A0-PARSER-001` 说明恢复方式。

确因硬耦合无法拆分时，可以在源文件同目录创建
`<文件名>的硬耦合需要的说明.md`。文件必须分别包含四个非空章节：

- `边界`：为什么不能拆，以及拆分会破坏什么；
- `理由`：为什么不采用替代方案；
- `替代方案`：已经尝试或考虑的拆法；
- `移除计划`：满足什么条件后删除豁免。

四段齐全只会把诊断降为仍然可见的 `warning`；缺段或空文件不生效。没有目录级或全局
白名单。当前 `core/rust/crates/xiao-syntax/src/parser.rs`（3040 行）必须由后续批次拆分，
不得添加豁免；`xiao-bytecode` 研究编码器已拆为门面与 `research/encode/` 子模块。在
`parser.rs` 拆分完成前，`bun run check:layout` 和 `bun run check` 按设计返回失败。

已知限制：符号链接形式的源文件目前不会被扫描。仓内源码树没有此类链接；若新增链接，
不能把它当作绕过尺寸门禁的方式。

## 失败时怎么处理

报告会给出稳定错误码、路径和修复提示。常见类别包括 `A0-WORKSPACE-001`（成员漂移）、
`A0-LAYOUT-001`（README 缺失）、`A0-SIZE-001`（单文件超过 2500 行或豁免说明不完整）、
`A0-PARSER-001`（AST/大纲不可用）和 `A0-DOCS-001`（断链或登记路径错误）。修复对应文件后
重新运行同一命令；工具不会替你覆盖配置。

## 相关页面

- [文档覆盖率检查](doc-coverage.md)
- [Rust AST 覆盖率适配器](doc-coverage-rust.md)
- [命令行参考](README.md)
- [故障排查](../../troubleshooting/README.md)
