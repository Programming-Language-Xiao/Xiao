# `xiao-bytecode/src/research`

这是 09R3 冻结后的兼容重导出层，不再承载实质实现。三地址模型、调用签名表、降低器、
验证器与内存编码器均位于父级 `src/` 生产模块；`research::...` 路径继续供 09R 共享向量和
基准设施使用。新生产代码应直接依赖 crate 根路径。

工程期：09R2（兼容层维护）。

`tac.rs` 只描述算什么，`sig.rs` 补 `IrProgram` 缺失的调用 ABI 描述，
`lower/` 按关注点拆分降低规则，`verify.rs` 做自校验与释放序列对账，`encode.rs` 作为
研究编码门面，内部实现拆在 `encode/` 下，提供布局版本 3 的内存研究编码、解码、41 个
稳定 opcode、两种操作数宽度和 `pc -> IrSpan` 目录；
其中 `SelectorApply`、`BroadcastAssign`、`RandomSeed` 携带选择器、广播和种子计划。
09R2H 的 `table_definitions` 保存前端表签名镜像与函数索引，`LoadTable`、`MemberGet`、
`MemberSet` 执行表构造和字段访问；静态方法复用 `Call`，旧函数编号不变。
当前生产模块提供 `MakeError`/`Raise`/`Check`、`TacHandler` 和按子程序复用的 `finally`
降低；`TacProgram.unsupported` 非空由生产验证入口报告为内部一致性错误。禁止在这里重新
推断类型、重算生命周期或重排释放顺序。编码器不是公开 `.xiaoc` 文件格式，产物只在内存
中流转。别名层的移除条件是 B0-C 交付且生产驱动器成为唯一消费方。
