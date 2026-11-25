# 驱动器附属可执行程序

工程期包括 20D 和 20O-B3-2。目录提供分发入口与进程回归夹具。

## B3-2 分发启动器

`xiao-launcher` 作为分发包的 `xiao` 入口，在运行 Bun 前读取宿主参数的原始编码，随后启动同目录的 `xiao-cli`。开发时与同目标的 `xiao-core` 一起构建。[分发说明](../../../../../../docs/UseDocs/tooling/cli/packaging.md)

## 20D 进程 API 测试子程序

`process_child_exit7` 默认退出 7；隔离测试可通过继承的环境变量启用等待放行和 stdio/cwd
继承验证。`process_child_abort` 在 Unix 触发 SIGTERM，在 Windows 返回访问异常状态码。
它们不解析命令行参数，不属于 Xiao 的公开命令行接口；集成测试通过 `CARGO_BIN_EXE_*`
定位二进制，同一子程序用于 VM 与原生差分。
