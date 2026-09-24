# 05-containers 规格快照

本目录按阶段承载容器语法、结构化类型和选择器正反例。JSON 只描述输入与稳定诊断编号，
不执行 Runtime，也不把静态计划伪装成真实容器值。

对应 JSON 执行入口为：

- `core/rust/crates/xiao-types/tests/c0c1_snapshots.rs`

`core/rust/crates/xiao-syntax/tests/c0_containers.rs`、
`core/rust/crates/xiao-syntax/tests/c0_snapshots.rs` 和
`core/rust/crates/xiao-types/tests/c0_containers.rs` 是不读取本目录 JSON 的内联规格测试，
因此不把它们列为快照 harness。

后续 C1/C2 必须新增独立目录或明确版本字段，不能改写 C0 快照的语义。

## C1 快照

`c1-valid.json` 和 `c1-errors.json` 对应工程期 03B，覆盖多选、闭区间/单边范围、负索引、
步长、随机数量与种子、字典列重复键以及标量广播赋值。`X03-TYPE-008` 至 `X03-TYPE-014`
是 C1 的稳定诊断编号。C1 快照与 C0 分工独立，后续 Runtime 接入时应复用这些输入并增加
执行结果字段，不得修改已有 C0 期望。

## C2-A 快照

`c2a-valid.json` 和 `c2a-errors.json` 对应工程期 03C，固定最小集合静态闭环：非空集合、
`set()` 与 `{}` 的消歧，单一元素类型、独立 `bool`、`none`、静态重复、可哈希性、成员判断
和不可索引边界。错误快照中的每条诊断同时记录 `code`、`message_id` 和语言无关参数；不
记录中文或英语展示句子。集合元素顺序只属于快照输入顺序，不构成集合语义。

C2-A 快照不要求 Runtime 集合、集合增删、异构集合、集合代数或 `frozenset`；这些必须在
后续 C2 子阶段新增版本化快照，不能改写本目录已有期望。

## C2-B 快照

`c2b-valid.json` 和 `c2b-errors.json` 对应工程期 03D，覆盖默认异构集合的静态成员并集、
`set<T>`/`set<T | U>` 显式注解、`none`、严格跨类型成员判断、动态尾标和非集合初始化器
诊断。快照状态固定为 `verified-static`：它只验证 AST 类型检查结果、`code`、`message_id`
和结构化参数，不创建 Runtime 集合，也不执行增删或集合代数。

C2-B 不改写 C2-A 的历史快照；集合成员顺序仍只属于输入源码，类型并集的稳定显示顺序
不代表运行时迭代顺序。动态成员的 `SetHashability`/`SetMembership` 只是后端待消费的
检查标记，不能被快照解释为运行时检查已经执行。

## c0/c1 快照的执行入口

`c0-valid.json`、`c0-errors.json`、`c1-valid.json`、`c1-errors.json` 在 09R2 的
跨层审计中被发现**从未被任何测试加载**：它们在模块登记里是有效契约，却因为缺少
harness 而长期没有执行，快照与实现的分歧因此被静默掩盖。现在由
`core/rust/crates/xiao-types/tests/c0c1_snapshots.rs` 加载。

补上执行入口后立即暴露了两处分歧，已分别处理：

- `c0-errors.json` 的 `unsupported-range`（数组上的 `items[0~1]` 期望报错）已被
  C1 取代——C1 让有序容器支持区间。该用例改写为 `unsupported-dict-range`，用
  字典表上的高级选择继续覆盖「不受支持的容器」，期望编号随之改为 `X03-TYPE-008`。
  这是对 03A 期期望语义的**有意例外**：原期望已不可能成立，保留它只会让契约腐烂。
- `c1-errors.json` 的 `random-seed-invalid` 源码含**两个**非法 `random.seed`
  调用，实现正确地报了两条诊断，是快照当初只记了一条。期望已补全为两条。

两处都不是实现缺陷，但只有把它们接回执行才能发现。
