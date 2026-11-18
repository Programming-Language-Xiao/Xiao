# `tests/benchmarks/programs`

工程期 09R3；这里存放四族真实 Xiao 源码基准。清单覆盖运行时溢出、布尔奇偶加减、递归/循环、
字符串索引、表实例与 `drop`、`try/finally`、数组/元组/字典/集合及高级选择；程序必须避开
`string_boolean`、表方法值/动态派发和 `*args`/`**kwargs` 展开实参。入口、固定输入和期望值在
同级 `manifest.json` 中登记。
