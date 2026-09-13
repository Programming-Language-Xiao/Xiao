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

本工具检查 Xiao 仓库的 Bun/Cargo workspace、代码目录 README、模块登记和文档链接。它只读取仓库，不自动修改 manifest、源码或文档。

## 运行方式

在仓库根目录执行：

```text
bun tools/repo-check/src/cli.ts layout
bun tools/repo-check/src/cli.ts docs
bun tools/repo-check/src/cli.ts usedocs
bun tools/repo-check/src/cli.ts all
```

`all` 按固定顺序执行 workspace、目录、DevDocs、UseDocs 和文档覆盖率检查。需要脚本化时可添加 `--format json` 或 `--format sarif`，并用 `--out` 保存报告。

## 失败时怎么处理

报告会给出稳定错误码、路径和修复提示。常见类别包括 `A0-WORKSPACE-001`（成员漂移）、`A0-LAYOUT-001`（README 缺失）和 `A0-DOCS-001`（断链或登记路径错误）。修复对应文件后重新运行同一命令；工具不会替你覆盖配置。

## 相关页面

- [文档覆盖率检查](doc-coverage.md)
- [命令行参考](README.md)
- [故障排查](../../troubleshooting/README.md)
