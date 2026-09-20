# `tests/benchmarks/src`

工程期 09R3；这里提供 Windows 原生基准、三机型差分、统计和报告落盘代码。实现必须使用
`std::time::Instant`，只依赖仓内前端/IR/VM crate，不引入 `criterion`；平台与布局元数据由清单
和报告固定，Linux/macOS 结果保持待复现。
