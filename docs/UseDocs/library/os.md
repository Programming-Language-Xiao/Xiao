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
  - ../../../stdlib/os.xiao
---

# 官方 `os` 模块

`os` 提供首批文件和目录资源 API。公开面是纯 Xiao 模块函数：

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
```

四个接口各接收一个位置参数；关键字或展开实参在编译期拒绝。

相对路径以进程当前工作目录为基准并原样交给宿主。`read_text` 读取整个文件，文件必须是合法 UTF-8。
文件和目录资源可以放进数组或元组；集合元素和字典键不接受资源。资源值不能用于 `const`。

四个入口要求 `filesystem` 能力。能力缺失是 `X06-RUNTIME-026`，读取已关闭资源是
`X06-RUNTIME-025`，路径、权限、IO 或 UTF-8 失败是 `X06-RUNTIME-012`，并带 `reason` 参数。
`close` 对已关闭资源及其别名保持幂等。

以上名称是模块公开函数；内部绑定名称不属于公开契约。
