# `cli/ts/src/commands`

放置 `run`、`build`、`test`、`config`、`venv`、`sync`、`install` 和优化/归档命令路由。工程期 11、11A、18；只解析参数、发现项目测试文件并调用 Rust 驱动器。

X0-B 已实现 `parser.ts`、`index.ts` 和 `main.ts` 的核心路由；X0-E 已接入真实源码 `build`。
X0-T 已接入递归发现 `tests/**/*.xiao`，按项目相对路径稳定排序后交给项目测试协议执行，
并按协议返回逐用例结果与退出码；11A-E0 已接入 `venv`、`shell-init` 和 `deactivate`，
环境目录与 Shell 状态边界由 `../environments` 负责；11A-E2B 已接入 `sync`、
`install`/`i`，CLI 只解析显式项目路径；环境目标选择、锁文件和映射更新仍由 Rust 包模块唯一实现。
