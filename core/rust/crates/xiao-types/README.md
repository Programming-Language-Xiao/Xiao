# `xiao-types`

## 目录职责

提供静态类型、动态值边界、类型推断、显式转换、固定宽度数值、溢出检查、容器元素约束和布尔加减规则的编译期表示。当前 P2 首批实现覆盖标量声明、`const`、名称环境、HM 统一/泛化/实例化、转换矩阵和静态检查器；C0 又加入数组、元组、字典表、字典列的结构化类型、声明路径约束和单项精确索引检查；C2-A 加入单一元素类型集合、静态可哈希性、唯一性和成员判断检查；C2-B 加入异构集合成员并集、显式 `set<T | U>`、动态尾标和成员兼容检查；C2-C 加入集合代数、集合比较、`Empty` 结果类型和动态检查计划。

## 工程期

02–04 建立基础类型；03-C0/C1/C2-A/C2-B/C2-C 负责容器和集合静态闭环；08 接入统一前端和 IR；后续由两个后端共同消费。C0/C1/C2-A/C2-B/C2-C 只检查语义，不创建 Runtime 容器、执行修改或实现集合代数。

## 模块放置

类型表示放在 `types.rs`，容器形状和约束放在 `containers.rs`，集合类型和可哈希规则放在 `set_types.rs`，集合 AST 检查放在 `set_checker.rs`，集合运算/比较分派放在 `set_operations.rs`，路径解析放在 `path_constraints.rs`，空容器计划放在 `materialization.rs`，容器 AST 检查放在 `checker.rs` 的 `container_checker.rs` 子模块；统一和 HM 操作放在 `unify.rs`，作用域放在 `environment.rs`，转换放在 `conversion.rs`，数值规则放在 `numeric.rs`，稳定编号放在 `diagnostics.rs`。后端布局类型放在各后端 crate，禁止把 Runtime 或 CLI 依赖倒灌到类型层。

## 禁止事项

不编码 LLVM 指令、不执行用户代码、不把平台 C 类型宽度当作 Xiao 类型定义。
