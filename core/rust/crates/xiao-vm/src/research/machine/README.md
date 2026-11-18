# `xiao-vm/src/research/machine`

放置三地址解释器各候选机型的载体实现，对应工程期 09R2。实质实现位于父级生产
`src/machine/`，本目录通过 `research::machine` 兼容暴露；每个文件是一种机型，只实现
`carrier.rs` 的接口，不含任何语言语义。

当前提供 `stack.rs`、`register.rs` 和 `hybrid.rs` 三种载体。它们都只实现
`carrier.rs` 的接口，集合、选择器和错误语义统一由父级 `src/semantics/` 执行；禁止把
机型条件分支写进语义核。
