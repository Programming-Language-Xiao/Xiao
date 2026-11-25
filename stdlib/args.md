# args 程序参数

`std.args` 和 `stdlib.args` 从本次运行的快照读取程序名和有序参数。登记状态为 `planned`。

- `program_name() -> str` 返回拥有的程序名。
- `argv() -> [str]` 每次返回独立数组，保留空项、顺序和重复项。

两个入口均检查 `process` 能力。本批的接纳上限为 1,024 个参数、每项和程序名各 4,096 个 UTF-8 字节、合计 65,536 个 UTF-8 字节。嵌入执行省略快照时读到空程序名和空数组。[冻结契约](../docs/DevDocs/assets/20o-b3/entry-20261123-b3-0b/contract-draft.md) · [实施记录](../docs/DevDocs/20o-b3-2-args-cli-implementation-plan.md)
