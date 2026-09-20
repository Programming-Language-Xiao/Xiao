# `xiao-bytecode/src/lower`

工程期 09；这里把已验证的 `IrProgram` 单向降低为冻结的统一三地址模型。`expr.rs`、
`stmt.rs`、`plan.rs` 和 `tables.rs` 按职责拆分，生产路径与旧 `research::lower` 通过同一
实现保持一致。

职责：消费类型、控制流和释放计划事实，不重新解析源码、推断类型、重算生命周期或重排
释放动作。所有已接通的 RuntimeCheck 都进入 TAC；生产验证器负责拦截非空 `unsupported`。
