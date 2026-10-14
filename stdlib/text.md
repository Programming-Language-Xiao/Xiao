# `text` 模块

工程期：20C 首批、第二批与第五批；规范源码为 [`text.xiao`](text.xiao)。第五批的零增量 `format_int` 候选已由第六批撤回。

## 接口

`contains_char(str text, str needle) -> bool` 按迭代器式 `for c in text` 扫描文本，发现与 `needle`
相等的字符后返回 `true`，否则返回 `false`。该接口只使用已探测的循环、条件和布尔赋值，不引入 intrinsic。

`length(str text) -> int` 同样按 `for c in text` 逐字符计数，返回 Unicode 码点数量；不引入新的长度 intrinsic。

`is_ascii_upper(str value) -> bool` 判断文本是否落在 ASCII 大写字母区间 `"A"` 到 `"Z"`；只使用字符串比较与
`and`，不依赖 `str` → `int` 码点转换。

文本参数必须是 `str`；其它类型由现有类型层拒绝。本批不提供填充/对齐/进制格式化、数字格式化接口、
切分、查找子串、切片、完整大小写转换或集合操作。

## 兼容身份

这是标准库接口版本 1 的新增接口，属于兼容扩展，不提升 `STDLIB_INTERFACE_VERSION`。模块由工具链专用
标准库命名空间提供，不进入普通包依赖，也不写入 `.xar` 兼容字段。
