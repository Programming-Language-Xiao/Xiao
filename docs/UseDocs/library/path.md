---
id: library.path
title: 官方 path 模块
status: verified
audience: learner
module: stdlib.xiao-standard-library-first-modules
stage: 20C
version: "0.1.0"
related:
  - README.md
  - text.md
  - ../../DevDocs/20c-first-modules.md
---

# 官方 `path` 模块

`path` 是 20C 首批官方 Xiao 模块，规范实现位于 [`stdlib/path.xiao`](../../../stdlib/path.xiao)。
适用版本：Xiao `0.1.0`、Runtime ABI `1.12`。受控双路径证据：维护回归
`38550284761`（Windows 作业 `115806283501`、Linux 作业 `115806283515`）。

## `join`

```xiao
join(str base, str leaf) -> str
```

用 `/` 连接两个路径片段并返回新文本。接口只承诺已探测的字符串拼接形状；平台分隔符、绝对路径、
规范化、解析、比较和文件系统访问不在本批。参数必须是 `str`，模块为纯 Xiao 实现，不引入 intrinsic。

```xiao
from stdlib.path import join

print(join("lib", "core"))
```

官方库通过工具链保留的 `stdlib.` 命名空间导入，不加入项目或包依赖图。
