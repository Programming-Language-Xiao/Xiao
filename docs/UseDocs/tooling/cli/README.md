# 命令行参考

本页是 CLI 命令的分层入口。每个命令页面都要说明输入、输出、退出码、失败恢复和 Windows/Linux/macOS 差异。

## 页面规划

`run`、`build`、`test`、`config`、`venv`、`sync`、`install`、`-debug`、`.xiaoc` 和 `.xar` 会在相应实现完成后分别建立页面。优化级别未实现前不把占位参数写成可用命令。

X0-B 已提供 [xiao run 与 CLI 外壳](shell.md)；其中记录 `print` 尚未实现、退出码映射、
非 TTY 输出和当前 Windows 原生验证边界。

A0 工程检查工具先提供：[仓库完整性检查](repo-check.md)、[文档覆盖率检查](doc-coverage.md)和其内部的 [Rust AST 适配器说明](doc-coverage-rust.md)。

X0-A 已验证核心协议的[长度前缀与结构化结果](protocol.md)；这页只描述 Rust 核心与
后续 TypeScript CLI 的机器边界，不代表 `xiao run`、`xiao build` 或独立打包已经交付。

## 下一步

需要逐行试验代码时阅读[交互式解释器](../repl/README.md)。
