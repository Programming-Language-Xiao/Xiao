# `xiao-types/src`

放置类型表示、HM 统一/泛化、作用域环境、转换矩阵、标量数值检查、容器类型/路径约束和 AST 类型检查实现。
对应工程期 02–04、03-C0、08；不得依赖平台宽度、Runtime、VM、LLVM 或 CLI。

模块边界：`types.rs` 不依赖 AST；`unify.rs` 只消费类型和环境接口；`environment.rs`
只保存绑定状态；`conversion.rs`/`numeric.rs` 是纯规则函数；`path_constraints.rs` 和
`materialization.rs` 是无状态路径/计划辅助；`checker.rs` 是 AST 分派入口，具体容器
逻辑在 `container_checker.rs` 中实现。新增容器语义必须继续放入独立模块，禁止把检查器
堆成跨层中心或形成循环依赖。
