---
id: library.os
title: 官方 os 模块
status: draft
audience: learner
module: stdlib.xiao-standard-library-os
stage: 20D
version: "0.1.0"
related:
  - README.md
  - ../../DevDocs/20d-first-api.md
  - ../../DevDocs/20d-process-api.md
  - ../../../stdlib/os.xiao
---

# 官方 `os` 模块

`os` 提供文件、目录和进程资源 API。公开面是纯 Xiao 模块函数：

```xiao
from stdlib.os import open_file, read_text, close

handle = open_file("notes.txt")
text = read_text(handle)
close(handle)
```

接口签名为：

```xiao
open_file(str path) -> file
open_directory(str path) -> directory
close(resource value) -> none
read_text(file value) -> str
spawn(str program) -> process
wait(process value) -> int
close_process(process value) -> none
```

每个接口各接收一个位置参数；关键字或展开实参在编译期拒绝。

函数别名（`from stdlib.os import open_file as opener`）和模块限定调用
（如 `import stdlib.os as platform` 后调用 `platform.open_file(...)`）同样保留上述静态签名。
`read_text` 只接受 `file`，`close` 接受 `file`、`directory` 或 `resource`；
`resource` 只包含文件和目录，不接收 `process`，也不会隐式收窄成 `file`。这些导入形式可在 VM 执行；当前 LLVM 路径使用
无别名的 `from stdlib.os import ...` 直接函数导入。

相对路径以进程当前工作目录为基准并原样交给宿主。`read_text` 读取整个文件，文件必须是合法 UTF-8。
文件和目录资源可以放进数组或元组；集合元素和字典键不接受资源。资源值不能用于 `const`。
变量和表字段的资源注解均不支持容器路径约束，例如 `file handles[1]` 会在编译期拒绝；
包含资源的表常量及其嵌套容器同样报 `X02-TYPE-008`。
来自模块等动态边界的值进入资源声明、赋值、形参或返回槽时，会验证实际资源种类，
不匹配报 `X06-RUNTIME-002`；整数不能伪装成资源。关闭后的资源仍保留原来的类型。

文件/目录的四个入口要求 `filesystem` 能力。能力缺失是 `X06-RUNTIME-026`，读取已关闭资源是
`X06-RUNTIME-025`，路径、权限、IO 或 UTF-8 失败是 `X06-RUNTIME-012`，并带 `reason` 参数。
`close` 对已关闭资源及其别名保持幂等。

以上名称是模块公开函数；内部绑定名称不属于公开契约。

## 子进程

```xiao
from stdlib.os import spawn, wait, close_process

process child = spawn("worker")
int exit_code = wait(child)
close_process(child)
```

`spawn` 只接受一个程序路径或名称，原样交给宿主按 PATH 解析；不经过 shell，也不拆分
命令行字符串。子进程继承父进程的 stdin、stdout、stderr、环境变量和当前工作目录。
创建成功即返回进程句柄，子进程继续运行。本批不提供命令行参数、显式环境/目录、输出捕获或管道。
Windows 的 `.cmd`/`.bat` 批处理需要 shell 解释，因此 `spawn` 会在启动前拒绝，
报 `X06-RUNTIME-012` 并带 `reason`。Unix 无解释器脚本也不会回退到 shell。

`wait` 阻塞到子进程结束并回收它，缓存退出码；重复等待同一资源或其别名返回同一结果。
正常退出返回宿主退出码；Unix 信号终止返回 `128 + signal`，例如 SIGTERM 返回 143。
Windows 异常按故障类别映射为 139、132、136 或 134。

`close_process` 只释放句柄，不终止、不阻塞、不等待；最后一个强引用释放也采用相同语义。
关闭任何别名后再次 `wait` 报 `X06-RUNTIME-025`，重复关闭仍成功。
在 Unix 上，未 `wait` 的子进程在其退出后会成为僵尸，直到父进程结束；关闭句柄不会回收它。
需要退出码并回收子进程时，应先 `wait`，再 `close_process`。

这三个入口只要求 `process` 能力（位 2），`filesystem` 位不能替代它；缺位报
`X06-RUNTIME-026`，且不会启动、等待或关闭进程。启动/等待的宿主失败报 `X06-RUNTIME-012`
并带 `reason`。`close(process)` 和 `close_process(file)` 都是编译期错误。
进程可以进入数组、元组和表字段，但不能成为集合元素、字典键、`const` 或产物常量。
