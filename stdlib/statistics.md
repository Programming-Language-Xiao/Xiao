# `statistics` 模块

工程期：20O-B1-IMP。目录职责：提供 `[int]` 的三项只读归约，规范实现为
[statistics.xiao](statistics.xiao)。不新增 intrinsic、ABI 或 helper。

| 签名 | 行为 |
| --- | --- |
| `sum([int] values) -> int` | 从左到右从 0 累加；空数组为 0；中间溢出立即失败 |
| `min([int] values) -> int` | 数值最小项；空数组失败 |
| `max([int] values) -> int` | 数值最大项；空数组失败 |

最多 65,536 项；先有界预检到上限+1，再开始归约，超限为
`ResourceError / STATISTICS_LIMIT_EXCEEDED`。空 min/max 为 `Error / STATISTICS_EMPTY`；
算术溢出沿用 `ArithmeticError / X06-RUNTIME-009`。输入不排序、不复制或修改，结果为 int。
算法时间 O(n)、归约状态 O(1)，不包含浮点、高精度或均值/方差入口。

公共路径为 `std.statistics`，固定身份为 `stdlib.statistics`，接口版本 1。
契约见 [B1 冻结](../docs/DevDocs/20o-b1-freeze-draft.md#37-statistics三项)，
教程见 [整数统计](../docs/UseDocs/library/statistics.md)。教程与 registry 保持 planned。
