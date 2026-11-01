# `math` 模块

工程期：20O-B1-IMP。目录职责：提供固定 64 位 `int` 的六个纯 Xiao 数学入口，
规范实现为 [math.xiao](math.xiao)。不新增 intrinsic、ABI、私有 helper 或数值重载。

| 签名 | 行为 |
| --- | --- |
| `abs(int value) -> int` | 非负绝对值；MIN 不可表示，报算术溢出 |
| `min(int left, int right) -> int` | 二元最小值 |
| `max(int left, int right) -> int` | 二元最大值 |
| `clamp(int value, int lower, int upper) -> int` | 先检查闭区间合法性，再钳位 |
| `gcd(int left, int right) -> int` | 非负最大公约数；支持 MIN 输入，只在最终结果不可表示时失败 |
| `pow(int base, int exponent) -> int` | 非负整数指数；0^0=1，不执行多余平方 |

倒置区间为 `Error / MATH_INVALID_BOUNDS`，负指数为 `Error / MATH_NEGATIVE_EXPONENT`；
不可表示的结果沿用 `ArithmeticError / X06-RUNTIME-009`。参数全部必填，名称属于契约。
`gcd` 在非正数域做余数迭代，`pow` 使用平方求幂；其它入口常量时间，不使用浮点或 lint。

公共路径为 `std.math`，固定身份 `stdlib.math`，接口版本 1；不进入普通包依赖或复制进归档。
完整契约见 [B1 冻结](../docs/DevDocs/20o-b1-freeze-draft.md#31-math六项)，
教程见 [整数计算](../docs/UseDocs/library/math.md)，当前仍为 planned，待独立 CLOSE 验收升级。
