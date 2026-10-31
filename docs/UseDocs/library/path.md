---
id: library.path
title: 官方 path 模块
status: verified
audience: learner
module: stdlib.xiao-standard-library-first-modules
stage: 20C/20M
version: "0.1.0"
related:
  - README.md
  - text.md
  - ../../DevDocs/20c-first-modules.md
  - ../../DevDocs/20m-text-and-path-expansion.md
---

# 官方 `path` 模块

`path` 是 20C 首批官方 Xiao 模块，规范实现位于 [`stdlib/path.xiao`](../../../stdlib/path.xiao)。
适用版本：Xiao `0.1.0`、Runtime ABI `1.17`、标准库接口版本 `1`。20M 新增接口已通过
三平台双路径与归档验证，证据见 [20M 验收记录](../../DevDocs/20m-text-and-path-expansion.md#97-产品提交与远端验收)。
首批历史受控双路径证据：维护回归
`38550284761`（Windows 作业 `115806283501`、Linux 作业 `115806283515`）。

## `join`

```xiao
join(str base, str leaf) -> str
```

用 `/` 连接两个路径片段并返回新文本，保持既有拼接规则，不合并重复分隔符。
参数必须是 `str`，模块为纯 Xiao 实现，不引入 intrinsic。

```xiao
from stdlib.path import join

print(join("lib", "core"))
```

## `basename`、`dirname` 与 `extension`

```xiao
basename(str path) -> str
dirname(str path) -> str
extension(str path) -> str
```

`basename` 取最后一个 `/` 后的文本；没有 `/` 时返回原串，尾随 `/` 时返回空串。
`dirname` 取最后一个 `/` 前的文本；没有 `/` 时返回空串，也不把空父段补成 `/` 或 `.`。
`extension` 只检查末段，取最后一个点及其后的文本，包含点；没有点时返回空串。
开头点和结尾点都参与匹配，因此 `.env` 的扩展文本是 `.env`，`file.` 的扩展文本是 `.`。

只识别 `/`；反斜杠、盘符、`.` 和 `..` 均按普通文本处理。不合并重复 `/`，不规范化路径，
不访问文件系统。三个接口都接收 `str`，不增加错误编号；空输入均返回空串。

```xiao
from stdlib.path import basename, dirname, extension

print(basename("目录/小😀.x"))
# 输出：小😀.x
print(dirname("a//b"))
# 输出：a/
print(extension("a.tar.gz"))
# 输出：.gz
```

下表逐格固定边界，`""` 表示空字符串：

| 输入 | basename | dirname | extension |
| --- | --- | --- | --- |
| `""` | `""` | `""` | `""` |
| `name` | `name` | `""` | `""` |
| `/` | `""` | `""` | `""` |
| `/name` | `name` | `""` | `""` |
| `a/` | `""` | `a` | `""` |
| `a/b` | `b` | `a` | `""` |
| `a//b` | `b` | `a/` | `""` |
| `//` | `""` | `/` | `""` |
| `a//` | `""` | `a/` | `""` |
| `.env` | `.env` | `""` | `.env` |
| `dir.x/.env` | `.env` | `dir.x` | `.env` |
| `a.tar.gz` | `a.tar.gz` | `""` | `.gz` |
| `file.` | `file.` | `""` | `.` |
| `.` | `.` | `""` | `.` |
| `..` | `..` | `""` | `.` |
| `a/../b` | `b` | `a/..` | `""` |
| `C:\tmp\file.txt` | `C:\tmp\file.txt` | `""` | `.txt` |
| `目录/小😀.x` | `小😀.x` | `目录` | `.x` |

官方库通过工具链保留的 `stdlib.` 命名空间导入，不加入项目或包依赖图。
归档只记录 `stdlib.path` 的宿主要求，不复制库函数体；本批是接口版本 1 的兼容扩展。
