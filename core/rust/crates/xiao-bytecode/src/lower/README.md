# `xiao-bytecode/src/lower`

工程期 09；这里把已验证的 `IrProgram` 单向降低为冻结的统一三地址模型。`expr.rs`、
`stmt.rs`、`plan.rs` 和 `tables.rs` 按职责拆分，生产路径与旧 `research::lower` 通过同一
实现保持一致。

职责：消费类型、控制流和释放计划事实，不重新解析源码、推断类型、重算生命周期或重排
释放动作。所有已接通的 RuntimeCheck 都进入 TAC；生产验证器负责拦截非空 `unsupported`。

18B 的 `imports.rs` 让函数引用已经初始化且未被覆盖的模块顶层导入，复用现有模块导入/
成员指令和 VM 单次初始化缓存，不复制官方函数。局部遮蔽与赋值覆盖仍走原有绑定路径。
