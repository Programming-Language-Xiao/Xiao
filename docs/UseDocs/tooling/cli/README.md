# 命令行参考

本页是 CLI 命令的分层入口。每个命令页面都要说明输入、输出、退出码、失败恢复和 Windows/Linux/macOS 差异。

## 页面规划

`run`、`build`、`test`、`config`、`venv`、`sync`、`install`、`.xiaoc` 和 `.xar` 分别维护页面；
当前已提供 [`xiao test`](test.md)、[`xiao build`](build.md)、[`xiao venv`](environments.md)、[`sync`/`install`](sync.md) 和 `-debug` 页面。优化级别未实现前
不把占位参数写成可用命令。

X0-B 已提供 [xiao run 与 CLI 外壳](shell.md)；X0-C 的[独立打包与核心发现](packaging.md)
补充了分发目录、构建时/运行时依赖和平台证据边界。两页都记录 `print` 尚未实现、退出码
映射、非 TTY 输出和当前 Windows 原生验证边界。

A0 工程检查工具先提供：[仓库完整性检查](repo-check.md)、[文档覆盖率检查](doc-coverage.md)和其内部的 [Rust AST 适配器说明](doc-coverage-rust.md)。

X0-A 已验证核心协议的[长度前缀与结构化结果](protocol.md)；X0-B/C 已分别接入用户命令
和独立分发，X0-E 已接入 [`xiao build`](build.md) 与主机工具链发现。

X0-D 已提供 [`-debug` 诊断窗口](debug.md)：诊断事件走独立 Rust 进程和本机回环通道，
不会污染用户程序标准输出。

X0-T 已接入 [`xiao test`](test.md)：它递归发现项目 `tests/**/*.xiao`，按稳定路径顺序执行，
并返回逐用例结构化结果；它仍不替代 `cargo test` 或 `bun test`。

平台功能复现和环境门禁见[平台复现工具](platform-reproduction.md)。它只验证构建、核心发现、
协议协商和项目测试链路，不把 Docker/WSL 功能证据写成裸机性能验收。

11A-E0 的项目环境创建、一次性 Shell 钩子、提示符前缀和取消激活规则见[`xiao venv` 与环境激活](environments.md)。

11A-E1 的本地依赖快照、`XIAO_HOME` 共享缓存和环境逻辑映射见[本地包缓存与环境映射](package-cache.md)。
E1 只提供核心 API 和测试；E2B 已接入 `sync`、`install`/`i`。

11A-E2A 的项目根 [`xiao.lock.json` 与原子环境映射](package-lockfile.md)已提供核心生成、
校验和复用 API；E2B 的命令接线见 [`sync`/`install`](sync.md)。

## 下一步

需要逐行试验代码时阅读[交互式解释器](../repl/README.md)。
