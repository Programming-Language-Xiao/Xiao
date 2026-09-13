# `xiao-types/src`

放置类型表示、HM 统一/泛化、作用域环境、转换矩阵、标量数值检查和 AST 类型检查实现。
对应工程期 02–04、08；不得依赖平台宽度、Runtime、VM、LLVM 或 CLI。

模块边界：`types.rs` 不依赖 AST；`unify.rs` 只消费类型和环境接口；`environment.rs`
只保存绑定状态；`conversion.rs`/`numeric.rs` 是纯规则函数；`checker.rs` 是唯一消费
`xiao-syntax` AST 的入口。新增容器语义应放入独立模块，禁止把检查器继续堆成跨层中心。
