# CLI 参数分组

`std.cli` 和 `stdlib.cli` 提供 `split_terminator([str] values) -> [[str]]`。登记状态为 `planned`。

它在第一个 `--` 处分成两个新数组，移除该项，保留后续的 `--`。无分隔符时，第二组为空。输入最多 1,024 项，每项最多 4,096 个 UTF-8 字节，总计最多 65,536 个 UTF-8 字节。超限错误为 `ResourceError/CLI_LIMIT_EXCEEDED`。[冻结契约](../docs/DevDocs/assets/20o-b3/entry-20261123-b3-0b/contract-draft.md) · [实施记录](../docs/DevDocs/20o-b3-2-args-cli-implementation-plan.md)
