---
id: library.text
title: 官方 text 模块
status: draft
audience: learner
module: stdlib.xiao-standard-library-first-modules
stage: 20C
version: "0.1.0"
related:
  - README.md
  - path.md
  - ../../DevDocs/20c-first-modules.md
---

# 官方 `text` 模块

`text` 是 20C 首批官方 Xiao 模块，规范实现位于 [`stdlib/text.xiao`](../../../stdlib/text.xiao)。

## `contains_char`

```xiao
contains_char(str text, str needle) -> bool
```

按迭代器式 `for c in text` 扫描文本；存在与 `needle` 相等的字符时返回 `true`，否则返回 `false`。
参数必须是 `str`，类型错误由现有前端诊断；本接口不引入 intrinsic。

本批不提供子串查找、切分、切片、大小写转换、格式化或集合操作。官方库不通过普通包依赖安装，
接口兼容身份由 `StandardLibraryIdentity` 提供。
