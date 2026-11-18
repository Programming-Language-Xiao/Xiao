# `set` 模块

20G/20H 提供通用集合入口和等价的方法式调用：

- `size<T>(set<T> values) -> int`：逐元素计数，空集合返回 `0`。
- `is_empty<T>(set<T> values) -> bool`：等价于 `size(values) == 0`。
- `add<T>(set<T> values, T value) -> none`：原地插入，重复值无操作，别名可见。
- `remove<T>(set<T> values, T value) -> none`：原地删除；元素不存在时抛出
  `Error(code = "SET_ELEMENT_MISSING")`。
- `contains<T>`、`union<T>`、`intersection<T>`、`difference<T>`：分别提供成员判断
  与既有集合运算的泛型只读包装，保持集合的首次出现顺序。

同一组入口也支持静态集合接收者的方法式写法：

```xiao
values = {1, 2}
values.add(3)
print(values.size())
merged = values.union({4})
```

方法调用只接受静态 `set<T>` 接收者，方法值和动态接收者在类型层拒绝；方法实现直接
复用本模块的自由函数，因此别名、顺序和 `SET_ELEMENT_MISSING` 错误身份保持一致。

调用点为每次使用创建独立类型实例；官方 Runtime 和标准库对象由宿主提供，模块不会写入普通包或归档对象。
