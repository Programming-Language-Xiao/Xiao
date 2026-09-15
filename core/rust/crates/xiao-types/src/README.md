# `xiao-types/src`

放置类型表示、HM 统一/泛化、作用域环境、转换矩阵、标量数值检查、容器类型/路径约束、
有序选择器计划、C2-A/C2-B/C2-C 集合静态检查和 AST 类型检查实现。对应工程期 02–04、03-C0、
03-C1、03-C2A、03-C2B、03-C2C、08；不得依赖
平台宽度、Runtime、VM、LLVM 或 CLI。

模块边界：`types.rs` 不依赖 AST；`unify.rs` 只消费类型和环境接口；`environment.rs`
只保存绑定状态；`conversion.rs`/`numeric.rs` 是纯规则函数；`path_constraints.rs` 和
`materialization.rs` 是 C0 无状态路径/计划辅助；`selection_model.rs`、
`selection_random.rs`、`selection_shape.rs` 和 `selector_checker.rs` 负责 C1 选择计划、
抽样辅助和结果形状；`checker.rs` 是 AST 分派入口，具体容器逻辑在 `container_checker.rs`
和 C1 选择器模块中实现。新增容器语义必须继续放入独立模块，禁止把检查器堆成跨层中心或
形成循环依赖。

集合职责：`set_types.rs` 保存 `SetType`、C2-A/C2-B 可哈希判定、静态成员并集和集合赋值兼容规则；
`set_checker.rs` 负责集合字面量、`set<T | U>`、`set()`、成员判断及相关 Runtime 检查标记；
`set_operations.rs` 负责 `+`、`&`、`-`、`^`、集合比较和四种原地运算的静态分派、结果类型及检查标记。
本阶段不创建 Runtime 集合、不执行哈希或集合代数；元组递归哈希留给后续集合阶段。

## C1 模块登记

- `selection_model.rs`：后端无关的选择、步长、广播和随机种子计划数据结构。
- `selection_random.rs`：可注入随机源、可复现种子和无放回/放回抽样辅助；不管理全局 Runtime 状态。
- `selection_shape.rs`：根据静态路径投影数组、元组和字典列结果类型；不创建运行时值。
- `selector_checker.rs`：将 P1 选择器 AST 检查为上述计划，并发出 `X03-TYPE-008` 至
  `X03-TYPE-014`；不执行容器读写、随机抽取或后端 lowering。

后续 Runtime/IR 只能通过这些稳定值对象消费 C1/C2-B 结果；不得把 VM、LLVM、CLI 或平台代码
反向引入本目录。若单个文件继续膨胀，应按职责拆分并保持门面只做装配与重导出。

## 04 阶段职责

`functions.rs` 只保存 `FunctionSignature` 和参数签名值对象；`function_checker.rs` 负责函数预登记、
参数匹配、返回统一、递归/前向引用和未解析类型诊断；`control_checker.rs` 负责条件、可迭代性、
循环深度、`break`/`continue` 和入口相关静态约束。`checker.rs` 仅做 AST 分派与结果装配，不能重新承载
这些模块的实现细节。

04 阶段的检查器可以生成 `BooleanCondition`、`Iterable` 等 `RuntimeCheckKind`，但不执行它们；不创建
调用栈、闭包、资源释放或字节码。函数参数作用域退出后才回写外层函数绑定，防止同名参数遮蔽签名。
类型诊断的 `code`、`message_id`、结构化参数和源码区间是稳定接口，中文文本仅为预览译文。
