# `tests/fuzz`

存放 Token、AST/IR、字节码、Protobuf、ZIP、配置和协议解析的模糊测试入口与种子。工程期 14、16、17、19；每个 fuzz harness 必须有模块文档和资源上限。

19A 使用普通 `cargo test` 的确定性变异器，不引入 `cargo-fuzz` 或其他新依赖。当前入口为：

- `xiao-ir/tests/fuzz_snapshots.rs`：IR JSON 快照，seed `0x19a01`，256 轮，输入上限 4 KiB；
- `xiao-bytecode/tests/fuzz_xiaoc.rs`：`.xiaoc` 容器，seed `0x19a02`，256 轮，输入上限 1 MiB；
- `xiao-artifacts` 单元测试：Protobuf 归档/全局索引，固定 `fuzz-index` 摘要，256 轮，输入上限 64 KiB；
- `xiao-xar` 单元测试：ZIP/ZIP64 归档，seed `0x19a03`，256 轮，输入上限 64 KiB。

重放单组解析器模糊测试：

```text
cargo test -p xiao-ir deterministic_ir_mutations_never_panic_or_escape_input_bound
cargo test -p xiao-bytecode deterministic_xiaoc_mutations_never_panic_or_allocate_unbounded_input
cargo test -p xiao-artifacts deterministic_index_mutations_never_panic_or_exceed_input_bound
cargo test -p xiao-xar deterministic_archive_mutations_never_panic_or_exceed_input_bound
```
