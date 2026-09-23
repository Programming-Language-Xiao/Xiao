# `xiao-types`

## 目录职责

提供静态类型、动态值边界、类型推断、显式转换、固定宽度数值、溢出检查、容器元素约束和布尔加减规则的编译期表示。当前 P2 首批实现覆盖标量声明、`const`、名称环境、HM 统一/泛化/实例化、转换矩阵和静态检查器；C0 又加入数组、元组、字典表、字典列的结构化类型、声明路径约束和单项精确索引检查；C2-A 加入单一元素类型集合、静态可哈希性、唯一性和成员判断检查；C2-B 加入异构集合成员并集、显式 `set<T | U>`、动态尾标和成员兼容检查；C2-C 加入集合代数、集合比较、`Empty` 结果类型和动态检查计划；05-C 加入 `TableType`、`TableSignature`、字段纯初始化、成员可见性、`new` 和 `init`/`drop` 静态契约；07-B 加入错误控制流静态恢复边界。

## 工程期

02–04 建立基础类型；03-C0/C1/C2-A/C2-B/C2-C 负责容器和集合静态闭环；08 接入统一前端和 IR；后续由两个后端共同消费。C0/C1/C2-A/C2-B/C2-C 只检查语义，不创建 Runtime 容器、执行修改或实现集合代数。

## 模块放置

类型表示放在 `types.rs`，容器形状和约束放在 `containers.rs`，集合类型和可哈希规则放在 `set_types.rs`，集合 AST 检查放在 `set_checker.rs`，集合运算/比较分派放在 `set_operations.rs`，表检查放在独立的 `table_checker.rs`，路径解析放在 `path_constraints.rs`，空容器计划放在 `materialization.rs`，容器 AST 检查放在 `checker.rs` 的 `container_checker.rs` 子模块；统一和 HM 操作放在 `unify.rs`，作用域放在 `environment.rs`，转换放在 `conversion.rs`，数值规则放在 `numeric.rs`，稳定编号放在 `diagnostics.rs`。后端布局类型放在各后端 crate，禁止把 Runtime 或 CLI 依赖倒灌到类型层。

## 检查器门面

`checker.rs` 只保留 `TypeChecker` 状态、顶层声明注册、语句分派和结果装配；标量检查按
职责拆在 `src/checker/` 的 `result.rs`、`constant.rs`、`conversion.rs`、`diagnostic.rs`、
`expression.rs` 和 `statement.rs`。`RuntimeCheckKind`、`TypeCheckResult` 和 `TypedNode` 的
公开路径与字段保持不变，架构边界由 `checker_architecture_tests.rs` 锁定。允许依赖和门面
契约详见 `src/checker/README.md`；本拆分不新增类型规则、诊断码或 Runtime 能力。

## 禁止事项

不编码 LLVM 指令、不执行用户代码、不把平台 C 类型宽度当作 Xiao 类型定义。

## 05-C 表模块

`tables.rs` 保存 `TableType`、成员签名和可见性值对象；`table_checker.rs` 负责表预登记、字段
静态初始化、成员访问、构造参数以及 `init`/`drop` 签名检查。它只输出静态结果，不创建实例、
不执行生命周期。表相关正反规格测试位于 `tests/c05_tables.rs`，后续 Runtime 必须复用这些
稳定类型和诊断编号，不能在后端重复解析表语法。

## 07-B 错误控制流

`control_checker.rs` 负责检查错误对象、`FatalError` 禁止捕获以及具体到宽泛的处理器顺序；它不执行
`try` 主体，也不决定 Runtime 的错误文本。`tests/f04_functions.rs` 中的 07-B 用例只断言稳定诊断和动态
检查标记；错误码匹配、模式匹配和 `Result` 泛型留给后续阶段。
