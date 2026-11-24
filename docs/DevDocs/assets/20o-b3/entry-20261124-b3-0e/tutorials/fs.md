# 文件系统路径操作教程草案

状态为 `planned`。本例使用 `std.fs` 执行路径操作，通过 `std.os` 获取和关闭读取句柄。运行需要 `filesystem` 与 `stdout`。别名 `stdlib.fs` 使用同一官方身份。标准库接口目标为 1。[冻结契约](../../entry-20261123-b3-0c/contract-draft.md)

## 快速开始

从 `std.fs` 导入路径操作函数。准备一个独立的工作目录，并在里面创建空的 `demo` 目录。

## 完整示例

从独占任务目录运行。任务入口在当前工作目录预先创建空的 `demo` 子目录，示例结束后由任务入口回收目录。

```xiao
from std.fs import write_text, list_dir, remove as remove_entry, rename, metadata
from std.os import open_file, read_text, close

write_text("demo/z.txt", "hello", overwrite = false)
write_text("demo/a.txt", "other", overwrite = false)
for name in list_dir("demo")
    print(name)

try
    write_text("demo/z.txt", "ignored", overwrite = false)
catch failure as ResourceError
    print(error_code(failure))

rename("demo/z.txt", "demo/m.txt", overwrite = false)
info = metadata("demo/m.txt")
print(info["kind"])
print(info["size"])
handle = open_file("demo/m.txt")
try
    print(read_text(handle))
finally
    close(handle)

remove_entry("demo/m.txt")
remove_entry("demo/a.txt")
```

```text
a.txt
z.txt
FS_EXISTS
1
5
hello
```

枚举只返回直接子项的名称，按 UTF-8 字节排序。`metadata` 的 `kind=1` 表示文件，`size=5` 是文本的 UTF-8 字节数。示例在删除前关闭句柄。后续验收会核对精确输出、退出码 0 和 `demo` 为空。[源码夹具](../candidates/tests/fixtures/docs/fs/tutorial.xiao) · [独立黄金](../candidates/tests/fixtures/docs/fs/tutorial.stdout)

## 路径与错误

相对路径以进程工作目录为基准，中间链接和 `..` 由宿主处理。写入按目标同目录原子提交，`overwrite = false` 遇到已有目录项报 `FS_EXISTS`。删除只处理指定的文件、链接或空目录。每条路径最多 4,096 字节，文本最多 8 MiB。`FS_TIMEOUT` 表示操作期限已到，运行取消沿执行控制通道交付。三平台验收在各自的独占目录内检查宿主差异。[资源与错误表](../../entry-20261123-b3-0c/contract-draft.md)
