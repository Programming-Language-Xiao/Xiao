---
id: library.text
title: 官方 text 模块
status: verified
audience: learner
module: stdlib.xiao-standard-library-first-modules
stage: 20C/20I
version: "0.1.0"
related:
  - README.md
  - path.md
  - ../../DevDocs/20c-first-modules.md
  - ../../DevDocs/20c-second-batch-probe.md
  - ../../DevDocs/20c-fifth-batch-reprobe-and-library.md
  - ../../DevDocs/20c-sixth-batch-case-conversion-and-api-closeout.md
  - ../../DevDocs/20c-seventh-batch-str-to-int-and-ci-coverage.md
  - ../../DevDocs/20i-nonliteral-indexing-and-splitting.md
---

# 官方 `text` 模块

`text` 是 20C 首批官方 Xiao 模块，规范实现位于 [`stdlib/text.xiao`](../../../stdlib/text.xiao)。
适用版本：Xiao `0.1.0`、Runtime ABI `1.13`。首批/第二批受控双路径证据：维护回归
`38550284761`（Windows 作业 `115806283501`、Linux 作业 `115806283515`）。

第五批新增的 `is_ascii_upper` 由维护回归 `38682801761` 覆盖：**VM 路径**为 Linux 作业
`116250888293`（`text_module_vm_result_and_removal_boundary_are_stable ... ok`）；
**原生路径**为 Linux 作业 `116250888144` 与 Windows 作业 `116250888212`
（均 `first_modules_match_native_results ... ok`）。三条作业全部成功；
第六批的复探夹具 `sixth-case-conversion-reprobe` 也在同一次运行的两个原生作业上跑到，两平台输出逐字一致。
取证与覆盖面说明见 [20C-第六批](../../DevDocs/20c-sixth-batch-case-conversion-and-api-closeout.md) §十。

第五批候选复探的 VM/LLVM 原生结果见 [20C-第五批](../../DevDocs/20c-fifth-batch-reprobe-and-library.md) §四；
第五批的 `format_int` 候选已按第六批收口撤回。

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

## `lower_ascii`

```xiao
lower_ascii(str value) -> str
```

只把 ASCII `A`–`Z` 转成对应的小写码点，其他字符原样保留。该接口冻结 ASCII 范围，
不承诺 Unicode 或 locale 大小写规则。

```xiao
from stdlib.text import lower_ascii

print(lower_ascii("Az-9"))
```

本批不提供填充/对齐/进制格式化、完整 Unicode 大小写转换或集合操作。官方库不通过普通包依赖安装，
`stdlib.` 是工具链保留命名空间，接口兼容身份由 `StandardLibraryIdentity` 提供。

第七批的 `lower_ascii` 已由标准库独立 push 的维护回归 `38753601939` 覆盖：Linux 原生作业
`116486098841`、Windows 原生作业 `116486098536` 均通过；VM 由同次运行的默认测试作业
`116486098784` 覆盖。

## `find`、`slice` 与 `split`

```xiao
find(str text, str needle) -> int
slice(str text, int start, int end) -> str
split(str text, str separator) -> [str]
```

这些接口按 Unicode 码点工作。`find` 未找到返回 `-1`，空 `needle` 返回 `0`；`slice` 使用
`[start, end)`，越界抛出 `TEXT_SLICE_OUT_OF_RANGE`，`start > end` 返回空串；`split` 保留首、
中、尾空段，空分隔符抛出 `TEXT_SPLIT_EMPTY_SEPARATOR`，空文本返回一个空字符串元素。
