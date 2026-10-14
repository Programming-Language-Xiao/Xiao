---
id: library.text
title: 官方 text 模块
status: verified
audience: learner
module: stdlib.xiao-standard-library-first-modules
stage: 20C
version: "0.1.0"
related:
  - README.md
  - path.md
  - ../../DevDocs/20c-first-modules.md
  - ../../DevDocs/20c-second-batch-probe.md
  - ../../DevDocs/20c-fifth-batch-reprobe-and-library.md
---

# 官方 `text` 模块

`text` 是 20C 首批官方 Xiao 模块，规范实现位于 [`stdlib/text.xiao`](../../../stdlib/text.xiao)。
适用版本：Xiao `0.1.0`、Runtime ABI `1.12`。首批/第二批受控双路径证据：维护回归
`38550284761`（Windows 作业 `115806283501`、Linux 作业 `115806283515`）；第五批候选复探的
VM/LLVM 原生结果见 [20C-第五批](../../DevDocs/20c-fifth-batch-reprobe-and-library.md) §四；第五批的
`format_int` 候选已按第六批收口撤回。

第五批新增的 `is_ascii_upper` 尚未取得覆盖它的 CI 运行号；当前模块状态沿用首批/第二批的历史验证，
待下一次维护回归覆盖后再补运行证据。

## `contains_char`

```xiao
contains_char(str text, str needle) -> bool
```

按迭代器式 `for c in text` 扫描文本；存在与 `needle` 相等的字符时返回 `true`，否则返回 `false`。
参数必须是 `str`，类型错误由现有前端诊断；本接口不引入 intrinsic。

```xiao
from stdlib.text import contains_char

if contains_char("lib/core", "/")
    print("has separator")
```

## `length`

```xiao
length(str text) -> int
```

按 `for c in text` 逐字符计数，返回文本的 Unicode 码点数量；接口是纯 Xiao 实现，不新增 intrinsic。

```xiao
from stdlib.text import length

print(length("小雪"))
```

## `is_ascii_upper`

```xiao
is_ascii_upper(str value) -> bool
```

判断文本是否位于 ASCII 大写字母区间 `"A"` 到 `"Z"`，不依赖 `str` → `int` 码点转换。

```xiao
from stdlib.text import is_ascii_upper

print(is_ascii_upper("A"))
```

本批不提供填充/对齐/进制格式化、子串查找、切分、切片、完整大小写转换或集合操作。官方库不通过普通包依赖安装，
`stdlib.` 是工具链保留命名空间，接口兼容身份由 `StandardLibraryIdentity` 提供。
