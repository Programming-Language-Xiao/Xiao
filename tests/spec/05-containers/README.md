# 05-containers 规格快照

本目录承载 03A/C0 的容器语法、结构化类型和精确路径正反例。JSON 只描述输入与稳定
诊断编号，不执行 Runtime，也不覆盖集合、范围、随机选择或默认值填充。

对应实现测试：

- `core/rust/crates/xiao-syntax/tests/c0_containers.rs`
- `core/rust/crates/xiao-syntax/tests/c0_snapshots.rs`
- `core/rust/crates/xiao-types/tests/c0_containers.rs`

后续 C1/C2 必须新增独立目录或明确版本字段，不能改写 C0 快照的语义。
