# `cli/ts/src/commands`

放置 `run`、`build`、`test`、`config`、`venv`、`sync`、`install` 和优化/归档命令路由。工程期 11、11A、18；只解析参数、发现项目测试文件并调用 Rust 驱动器。

X0-B 已实现 `parser.ts`、`index.ts` 和 `main.ts` 的核心路由；X0-E 已接入真实源码 `build`。
X0-T 约定递归发现 `tests/**/*.xiao`，按项目相对路径稳定排序后交给项目测试协议执行；
REPL、环境与包管理仍只返回稳定的后续批次诊断。
