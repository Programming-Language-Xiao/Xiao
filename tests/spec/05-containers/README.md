# 05-containers 规格快照

本目录按阶段承载容器语法、结构化类型和选择器正反例。JSON 只描述输入与稳定诊断编号，
不执行 Runtime，也不把静态计划伪装成真实容器值。

对应实现测试：

- `core/rust/crates/xiao-syntax/tests/c0_containers.rs`
- `core/rust/crates/xiao-syntax/tests/c0_snapshots.rs`
- `core/rust/crates/xiao-types/tests/c0_containers.rs`

后续 C1/C2 必须新增独立目录或明确版本字段，不能改写 C0 快照的语义。

## C1 快照

`c1-valid.json` 和 `c1-errors.json` 对应工程期 03B，覆盖多选、闭区间/单边范围、负索引、
步长、随机数量与种子、字典列重复键以及标量广播赋值。`X03-TYPE-008` 至 `X03-TYPE-014`
是 C1 的稳定诊断编号。C1 快照与 C0 分工独立，后续 Runtime 接入时应复用这些输入并增加
执行结果字段，不得修改已有 C0 期望。
