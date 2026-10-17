# Xiao 官方 `os` 模块（20D 首批）

规范源码位于 [`stdlib/os.xiao`](os.xiao)。用户通过 `from stdlib.os import open_file` 等导入公开函数；
`os_open_file`、`os_open_directory`、`os_close`、`os_read_text` 是模块内部使用的 intrinsic 名称，不属于公开契约。

## 接口

```xiao
open_file(str path) -> file
open_directory(str path) -> directory
close(resource value) -> none
read_text(file value) -> str
```

相对路径按进程当前工作目录解释，并原样交给宿主。`open_file` 打开文件，`open_directory` 验证目录路径；
`read_text` 一次读取整个文件且要求合法 UTF-8。`close` 对同一资源的重复调用成功，别名共享关闭状态。

四个接口都需要 `filesystem` 能力。能力缺失报告 `X06-RUNTIME-026`；已关闭资源读取报告
`X06-RUNTIME-025`；宿主路径、权限、IO 和非 UTF-8 失败报告 `X06-RUNTIME-012` 并带 `reason` 参数。

资源可以放入数组和元组，不能作为集合元素或字典键；资源不能出现在 `const` 或产物常量池中。
