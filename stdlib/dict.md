# `dict` 模块

20K 提供开放键集字典的自由函数接口。函数参数和返回值使用
`dict<str, T>`，固定字典字面量只要所有值都能赋给 `T` 就可以传入。

- `get<T>(values, key) -> T`：读取已有键；键不存在报告 `X06-RUNTIME-015`。
- `set<T>(values, key, value) -> none`：更新已有键，不新增键；缺失键报告
  `X06-RUNTIME-015`。
- `remove<T>(values, key) -> none`：删除已有键；缺失键报告
  `X06-RUNTIME-015`。
- `contains<T>(values, key) -> bool`：键存在返回 `true`，缺失返回 `false`。
- `keys<T>(values) -> [str]`：返回当前键数组，顺序沿用字典句柄迭代顺序。
- `values<T>(values) -> [T]`：返回与 `keys` 对应顺序的值数组。
- `size<T>(values) -> int`：返回当前条目数。
- `is_empty<T>(values) -> bool`：等价于 `size(values) == 0`。

本模块不允许通过 `set` 新增键，也不改变字典表与字典列原有的顺序约定。
