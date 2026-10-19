# `set` 模块

20G 提供通用集合入口：

- `size<T>(set<T> values) -> int`：逐元素计数，空集合返回 `0`。
- `is_empty<T>(set<T> values) -> bool`：等价于 `size(values) == 0`。
- `add<T>(set<T> values, T value) -> none`：原地插入，重复值无操作，别名可见。
- `remove<T>(set<T> values, T value) -> none`：原地删除；元素不存在时抛出
  `Error(code = "SET_ELEMENT_MISSING")`。
- `contains<T>`、`union<T>`、`intersection<T>`、`difference<T>`：分别提供成员判断
  与既有集合运算的泛型只读包装，保持集合的首次出现顺序。

调用点为每次使用创建独立类型实例；官方 Runtime 和标准库对象由宿主提供，模块不会写入普通包或归档对象。
