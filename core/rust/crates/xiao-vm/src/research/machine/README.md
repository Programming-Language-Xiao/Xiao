# `xiao-vm/src/research/machine`

放置三地址解释器各候选机型的载体实现，对应工程期 09R2。每个文件是一种机型，
只实现 `carrier.rs` 的接口，不含任何语言语义。

当前提供 `stack.rs`、`register.rs` 和 `hybrid.rs` 三种载体。它们都只实现
`carrier.rs` 的接口，集合、选择器和错误语义统一由 `../semantics/` 执行；禁止把
机型条件分支写进语义核。
