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
调用 Rust `xiao-core`。B3-2 的分发入口 `xiao[.exe]` 先读取原始宿主参数，再启动相邻的
`xiao-cli[.exe]`。先构建同目标的 `xiao-core`、`xiao-launcher` 和 `xiao-diagnostics`，
然后运行 `bun run build` 生成分发目录。用户可以直接启动 `xiao`。[分发说明](../../docs/UseDocs/tooling/cli/packaging.md)

X0-E 已接入 `xiao build`：`src/platform/toolchain.ts` 按固定来源发现并探测 clang、Runtime
和诊断组件，`src/protocol/client.ts` 把真实源码、配置与输出路径交给 Rust 原生驱动器。
X0-T 已接入 `xiao test`：`src/commands/test-discovery.ts` 递归发现并排序项目测试文件，
`src/protocol/client.ts` 发送批量源码请求，结果协议直接决定项目测试退出码。
