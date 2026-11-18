# `tests/fuzz`

存放 Token、AST/IR、字节码、Protobuf、ZIP、配置和协议解析的模糊测试入口与种子。工程期 14、16、17、19；每个 fuzz harness 必须有模块文档和资源上限。

19A 使用普通 `cargo test` 的确定性变异器，不引入 `cargo-fuzz` 或其他新依赖。每组先确认基线输入能解码，再按种子做 512 轮多步变异（翻转、删除、插入、截断、填充 0xFF）。失败信息带 `seed`、`round` 和变异步骤，可直接重放。

| 入口 | 目标 | 种子 | 输入上限 | 拒绝断言 |
| --- | --- | --- | --- | --- |
| `xiao-ir/tests/fuzz_snapshots.rs` | IR JSON 快照（基线为 `IrProgram::new(..).to_json()`） | `0x19a001` | 64 KiB | 只能是 `Decode` 或 `UnsupportedVersion` |
| `xiao-bytecode/tests/fuzz_xiaoc.rs` | `.xiaoc` 容器 | `0x19a002` | 1 MiB | 错误文本必须以 `XIAOC-nnn` 稳定码开头 |
| `xiao-xar/tests/fuzz_archive.rs` | ZIP/ZIP64 归档 | `0x19a003` | 64 KiB | 内存输入不得产生 `Io` 错误 |
| `xiao-artifacts` 单元测试 | Protobuf 归档索引 / 全局索引 | `0x19a004` / `0x19a005` | 64 KiB | 内存输入不得产生 `Io` 错误 |

每组还断言被拒绝的变异占多数（索引组为四分之一以上），防止变异器退化成只产生合法输入。

这些测试证明的是：解析器遇到坏输入不 panic，并给出有类别的拒绝。输入长度在变异后被断言不超过上限，但并没有测量解析过程的内存分配，所以不能据此声称“不会无界分配”。

重放单组解析器模糊测试：

```text
cargo test -p xiao-ir --test fuzz_snapshots
cargo test -p xiao-bytecode --test fuzz_xiaoc
cargo test -p xiao-xar --test fuzz_archive
cargo test -p xiao-artifacts seeded_index_mutations
```
