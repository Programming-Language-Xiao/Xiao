# `text` 模块

工程期：20C、20I 与 20M；规范源码为 [`text.xiao`](text.xiao)。第五批的零增量 `format_int` 候选已由第六批撤回。

## 接口

`contains_char(str text, str needle) -> bool` 按迭代器式 `for c in text` 扫描文本，发现与 `needle`
相等的字符后返回 `true`，否则返回 `false`。该接口只使用已探测的循环、条件和布尔赋值，不引入 intrinsic。

`length(str text) -> int` 同样按 `for c in text` 逐字符计数，返回 Unicode 码点数量；不引入新的长度 intrinsic。

`is_ascii_upper(str value) -> bool` 判断文本是否落在 ASCII 大写字母区间 `"A"` 到 `"Z"`；只使用字符串比较与
`and`，不依赖 `str` → `int` 码点转换。

`lower_ascii(str value) -> str` 只把 ASCII `A`–`Z` 转为对应的小写码点，其他字符原样保留；
它明确不做 Unicode 或 locale 大小写映射。

`find(str text, str needle) -> int` 返回按 Unicode 码点计数的首次匹配位置；空子串返回 `0`，
未找到返回 `-1`。`slice(str text, int start, int end) -> str` 使用左闭右开区间，越界抛出
`TEXT_SLICE_OUT_OF_RANGE`，`start > end` 返回空串。`split(str text, str separator) -> [str]`
保留首、中、尾空段，空分隔符抛出 `TEXT_SPLIT_EMPTY_SEPARATOR`；结果数组通过 `array_push`
原地构造。

20M 兼容新增四项，全部由纯 Xiao 实现：

| 接口 | 冻结语义 |
| --- | --- |
| `trim(str text) -> str` | 只剥除首尾 ASCII 空格、tab、CR、LF、VT、FF；内部空白和其他 Unicode 空白保留 |
| `replace_all(str text, str needle, str replacement) -> str` | 从左到右、非重叠；空 needle 返回原串；替换结果不重扫 |
| `repeat(str text, int count) -> str` | 重复 count 次；count ≤ 0 返回空串 |
| `upper_ascii(str text) -> str` | 仅把 ASCII a–z 转为 A–Z，其余字符原样保留 |

`repeat` 不额外限制正计数或结果大小；极端输入可能耗尽可用内存，调用方应按资源预算限制输入。
本接口不新增专用资源错误编号。

四项不增加错误编号。前后缀判断按 20M 的一行组合门槛剔除，分别使用
`find(text, needle) == 0` 和带长度守卫的 `slice` 比较；示例见
[公共文档](../docs/UseDocs/library/text.md)。本批验证与证据见
[20M](../docs/DevDocs/20m-text-and-path-expansion.md)。

文本参数必须是 `str`；其它类型由现有类型层拒绝。本批不提供填充/对齐/进制格式化、数字格式化接口、
完整 Unicode 大小写转换或集合操作。

## 兼容身份

这是标准库接口版本 1 的新增接口，属于兼容扩展，不提升 `STDLIB_INTERFACE_VERSION`。模块由工具链专用
标准库命名空间提供，不进入普通包依赖。归档通过既有 `stdlib_modules` 记录 `stdlib.text`，
运行时由宿主加载规范源码，不复制库函数体，也不新增归档字段。
