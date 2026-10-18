# `set` 模块

20F 首版提供两个只读、无 intrinsic 的通用集合入口：

- `size<T>(set<T> values) -> int`：逐元素计数，空集合返回 `0`。
- `is_empty<T>(set<T> values) -> bool`：等价于 `size(values) == 0`。

调用点为每次使用创建独立类型实例；官方 Runtime 和标准库对象由宿主提供，模块不会写入普通包或归档对象。
