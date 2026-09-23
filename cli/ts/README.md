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
调用 Rust `xiao-core`。X0-C 通过 `bun run build` 生成 `dist/<目标>/xiao[.exe]`，并把
同目标的 `xiao-core[.exe]` 和 `xiao-package.json` 放在同一目录；构建时需要 Bun，生成物
运行时不需要 Bun 或 Node.js。

X0-E 已接入 `xiao build`：`src/platform/toolchain.ts` 按固定来源发现并探测 clang、Runtime
和诊断组件，`src/protocol/client.ts` 把真实源码、配置与输出路径交给 Rust 原生驱动器。
