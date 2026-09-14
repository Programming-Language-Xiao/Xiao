# `xiao-types/tests`

这里放置 P2/S0 类型检查器的规格与集成测试。测试只构造 AST、运行静态检查并验证
诊断/运行时检查标记，不执行 Xiao 用户代码；后续容器和函数阶段按主题增加独立文件。

## C1 测试登记

`c1_selectors.rs` 对应工程期 03B，覆盖有序容器高级选择器、负索引、嵌套范围、步长、
随机数量/种子、字典列重复键和选择器左值标量广播。它只验证 `SelectionPlan`、
`BroadcastAssignmentPlan`、`RandomSeedPlan` 与诊断，不执行真实 Runtime 容器，也不要求
随机结果在类型阶段确定。

跨后端可复用的输入/期望快照位于 `tests/spec/05-containers/c1-valid.json` 和
`c1-errors.json`；C1 不改写 C0 快照。新增测试函数必须有简短注释，公共测试辅助 API
仍遵守全仓库文档覆盖率门槛。
