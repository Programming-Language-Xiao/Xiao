---
id: library.set
title: 官方 `set` 模块
status: verified
audience: user
module: stdlib.xiao-standard-library-set
stage: 20H
related:
  - README.md
  - ../../DevDocs/20f-container-parameters-and-collections.md
  - ../../DevDocs/20h-set-methods-and-20g-closeout.md
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

values.add(5)
print(values.size())
merged = values.union({6})
```

所有入口的元素类型由调用点决定，同一份程序可以同时处理整数集合和字符串集合。
`add`/`remove` 原地修改共享集合；`remove` 找不到元素时抛出稳定错误码
`SET_ELEMENT_MISSING`。集合运算保持左侧顺序，新增右侧元素按首次出现顺序追加。
方法式调用与自由函数一一对应，只允许直接调用静态集合的方法；动态接收者和方法值分别
使用 `X03-TYPE-024` 与 `X03-TYPE-025` 拒绝。
