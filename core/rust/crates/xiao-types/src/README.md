# `xiao-types/src`

放置类型表示、HM 统一/泛化、作用域环境、转换矩阵、标量数值检查、容器类型/路径约束、
有序选择器计划和 AST 类型检查实现。对应工程期 02–04、03-C0、03-C1、08；不得依赖
平台宽度、Runtime、VM、LLVM 或 CLI。

模块边界：`types.rs` 不依赖 AST；`unify.rs` 只消费类型和环境接口；`environment.rs`
只保存绑定状态；`conversion.rs`/`numeric.rs` 是纯规则函数；`path_constraints.rs` 和
`materialization.rs` 是 C0 无状态路径/计划辅助；`selection_model.rs`、
`selection_random.rs`、`selection_shape.rs` 和 `selector_checker.rs` 负责 C1 选择计划、
抽样辅助和结果形状；`checker.rs` 是 AST 分派入口，具体容器逻辑在 `container_checker.rs`
和 C1 选择器模块中实现。新增容器语义必须继续放入独立模块，禁止把检查器堆成跨层中心或
形成循环依赖。

## C1 模块登记

- `selection_model.rs`：后端无关的选择、步长、广播和随机种子计划数据结构。
- `selection_random.rs`：可注入随机源、可复现种子和无放回/放回抽样辅助；不管理全局 Runtime 状态。
- `selection_shape.rs`：根据静态路径投影数组、元组和字典列结果类型；不创建运行时值。
- `selector_checker.rs`：将 P1 选择器 AST 检查为上述计划，并发出 `X03-TYPE-008` 至
  `X03-TYPE-014`；不执行容器读写、随机抽取或后端 lowering。

后续 Runtime/IR 只能通过这些稳定值对象消费 C1 结果；不得把 VM、LLVM、CLI 或平台代码
反向引入本目录。若单个文件继续膨胀，应按职责拆分并保持门面只做装配与重导出。
