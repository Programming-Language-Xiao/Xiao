# `xiao-runtime/src/errors`

这里提供 `xiao-diagnostics` 统一错误核心的 Runtime 兼容重导出，包括稳定错误码、
`XiaoError`/`FatalError`、消息键、参数、原因链和 `suppressed` 清理错误。对应 07-A
的 Runtime 迁移；错误本体不在本目录重复实现。本目录不负责本地化目录、终端渲染或
调试窗口。

## 工程期

06-B 保留兼容入口，07-A 完成统一错误模型迁移；后续 07-B/07-C/11C 分别消费传播、日志
和本地化接口。
