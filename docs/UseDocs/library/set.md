---
id: library.set
title: 官方 `set` 模块
status: verified
audience: user
module: stdlib.xiao-standard-library-set
stage: 20F
related:
  - README.md
  - ../../DevDocs/20f-container-parameters-and-collections.md
---

# `set`

```xiao
from stdlib.set import size, is_empty

count = size({1, 2, 3})
empty = is_empty(set())
```

`size<T>` 和 `is_empty<T>` 的元素类型由调用点决定，同一份程序可以同时处理整数集合和字符串集合。接口只读，不提供 `add` 或 `remove`。
