# `cli/ts/src/commands`

放置 `run`、`build`、`test`、`config`、`venv`、`sync`、`install` 和优化/归档命令路由。工程期 11、11A、18；只解析参数并调用 Rust 驱动器。

X0-B 已实现 `parser.ts`、`index.ts` 和 `main.ts` 的核心路由；X0-E 已接入真实源码 `build`。
`test`、REPL、环境与包管理仍只返回稳定的后续批次诊断，其中项目测试语义归 X0-T。
