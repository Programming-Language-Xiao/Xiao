# 进程 API 测试子程序

工程期：20D-进程 API。职责是为进程等待、退出码和关闭语义提供确定性夹具。
边界：只服务回归测试，不接入生产驱动器协议或公开 CLI 命令。

`process_child_exit7` 默认退出 7；隔离测试可通过继承的环境变量启用等待放行和 stdio/cwd
继承验证。`process_child_abort` 在 Unix 触发 SIGTERM，在 Windows 返回访问异常状态码。
它们不解析命令行参数，不属于 Xiao 的公开命令行接口；集成测试通过 `CARGO_BIN_EXE_*`
定位二进制，同一子程序用于 VM 与原生差分。
