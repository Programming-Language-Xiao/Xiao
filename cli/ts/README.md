# `cli/ts/`

## 目录职责

TypeScript workspace 根目录。所有 CLI/REPL 代码都放在 `src/`，通过稳定协议或库 ABI 调用 Rust `xiao-driver`，不复制词法、类型、VM 或 Runtime 逻辑。

## 工程期

11、11A、11B、11C、18、19。

## 交付规则

- 发布时打包为无需用户另装 Node.js 的独立可执行程序。
- 导出函数、类、接口、类型和模块必须有 100% JSDoc。
- 每个新命令必须有结构化错误和跨平台测试。

## 子目录

- `src/`：命令、REPL、协议、配置、平台和 UI 模块。

X0-B 已接入 `src/main.ts` 的 `xiao` 命令入口，运行链路通过 `src/protocol/client.ts`
调用 Rust `xiao-core`；独立可执行打包仍由 X0-C 负责。
