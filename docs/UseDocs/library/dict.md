---
id: library.dict
title: dict 标准库
status: draft
audience: learner
module: stdlib.dict
stage: 20K
version: "0.1.0"
---

# `dict` 标准库

`stdlib.dict` 为开放键集字典提供六个自由函数。函数签名使用
`dict<str, T>`，固定字典字面量可以按共同值类型传入。

- `get<T>(values, key) -> T`：读取已有键；缺失键为 `X06-RUNTIME-015`。
- `set<T>(values, key, value) -> none`：更新已有键；不新增键，缺失键为 015。
- `remove<T>(values, key) -> none`：删除已有键；缺失键为 015。
- `contains<T>(values, key) -> bool`：键存在时返回 `true`，缺失时返回 `false`。
- `keys<T>(values) -> [str]`：返回键数组。
- `values<T>(values) -> [T]`：返回值数组，顺序与 `keys` 对应。

字典表不承诺键的排序；数组顺序沿当前句柄的迭代顺序保留。
