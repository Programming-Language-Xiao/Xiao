# Xiao 官方 `os` 模块（20D 首批）

规范源码位于 [`stdlib/os.xiao`](os.xiao)。用户通过 `from stdlib.os import open_file` 等导入公开函数；
内部绑定名称不属于公开契约。

## 接口

```xiao
open_file(str path) -> file
open_directory(str path) -> directory
close(resource value) -> none
read_text(file value) -> str
spawn(str program) -> process
wait(process value) -> int
close_process(process value) -> none
```

每个入口各接收一个位置参数；关键字或展开实参在编译期拒绝。
函数别名和模块限定调用保留资源类型及参数签名，VM 沿用已有模块调用路径；
LLVM 当前仍要求无别名的直接函数导入，未扩展模块动态调用。

相对路径按进程当前工作目录解释，并原样交给宿主。`open_file` 打开文件，`open_directory` 验证目录路径；
`read_text` 一次读取整个文件且要求合法 UTF-8。`close` 对同一资源的重复调用成功，别名共享关闭状态。

文件/目录四个接口都需要 `filesystem` 能力。能力缺失报告 `X06-RUNTIME-026`；已关闭资源读取报告
`X06-RUNTIME-025`；宿主路径、权限、IO 和非 UTF-8 失败报告 `X06-RUNTIME-012` 并带 `reason` 参数。

资源可以放入数组和元组，不能作为集合元素或字典键；资源不能出现在 `const` 或产物常量池中。
来自模块等动态边界的值进入资源声明、赋值、形参或返回槽时，会验证实际资源种类；
错误种类报 `X06-RUNTIME-002`。关闭不改变资源类型。

## 进程

`spawn` 无 shell、无命令行参数，程序名/路径原样交给宿主按 PATH 解析，stdio、环境和 cwd
全部继承；创建后立即返回。`wait` 阻塞回收进程并缓存退出码，重复等待同一资源或其别名
返回相同结果。Unix 信号返回 `128 + signal`；Windows 异常映射到既有 139/132/136/134 类别。
Windows 的 `.cmd`/`.bat` 批处理文件会被拒绝，报 `X06-RUNTIME-012` 并带 `reason`；
这些文件需要 shell 解释，不能由 `spawn` 隐式执行。Unix 无解释器脚本也不会回退到 shell。

`close_process` 和最后引用释放都只释放句柄，不终止、不阻塞、不等待。Unix 上未 `wait`
的子进程退出后会成为僵尸，直到父进程结束。关闭后等待报 `X06-RUNTIME-025`，关闭保持幂等。
`resource` 只接受 file/directory，`process` 是独立名义类型，因此不能传给通用 `close`。
进程三个入口只要求 `process` 位，缺失报 `X06-RUNTIME-026`；启动/等待失败报
`X06-RUNTIME-012` 并带宿主 `reason`。参数、显式 env/cwd、捕获和管道均未开放。
