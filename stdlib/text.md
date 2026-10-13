# `text` 模块

工程期：20C 首批与第二批；规范源码为 [`text.xiao`](text.xiao)。

## 接口

`contains_char(str text, str needle) -> bool` 按迭代器式 `for c in text` 扫描文本，发现与 `needle`
相等的字符后返回 `true`，否则返回 `false`。该接口只使用已探测的循环、条件和布尔赋值，不引入 intrinsic。

`length(str text) -> int` 同样按 `for c in text` 逐字符计数，返回 Unicode 码点数量；不引入新的长度 intrinsic。

参数必须是 `str`；其它类型由现有类型层拒绝。本批不提供切分、查找子串、切片、大小写转换、格式化或集合操作。

## 兼容身份

这是标准库接口版本 1 的新增接口，属于兼容扩展，不提升 `STDLIB_INTERFACE_VERSION`。模块由工具链专用
标准库命名空间提供，不进入普通包依赖，也不写入 `.xar` 兼容字段。
