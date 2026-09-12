# `tools/`

## 目录职责

存放工程编排、文档覆盖率、Schema 校验和发布辅助工具。工具可以调用 Rust/TypeScript 的稳定接口，但不能复制语言语义。

## 工程期

A0 建立目录检查和文档门禁；A1–A3 接入接口、构建矩阵和覆盖率；13–19 服务优化与发布。

## A0 检查入口

- [`repo-check/`](repo-check/README.md)：workspace、目录、README、模块登记和 UseDocs 完整性。
- [`doc-coverage/`](doc-coverage/README.md)：Rust/TypeScript/测试辅助代码的 docstring 覆盖率。

具体命令、诊断码和报告格式见 [A0 工作区与质量门禁实现方案](../docs/DevDocs/00a-a0-workspace-and-checkers.md)。
