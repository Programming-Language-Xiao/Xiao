---
id: library.set
title: 官方 `set` 模块
status: draft
audience: user
module: stdlib.xiao-standard-library-set
stage: 20G
related:
  - README.md
  - ../../DevDocs/20f-container-parameters-and-collections.md
---

# `set`

```xiao
from stdlib.set import add, contains, difference, is_empty, size, union

values = {1, 2, 3}
add(values, 4)
count = size(values)
empty = is_empty(set())
present = contains(values, 2)
merged = union(values, {4, 5})
removed = difference(merged, {1})
```

所有入口的元素类型由调用点决定，同一份程序可以同时处理整数集合和字符串集合。
`add`/`remove` 原地修改共享集合；`remove` 找不到元素时抛出稳定错误码
`SET_ELEMENT_MISSING`。集合运算保持左侧顺序，新增右侧元素按首次出现顺序追加。
