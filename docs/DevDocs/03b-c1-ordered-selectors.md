# 03B. C1 有序容器选择器实现交接记录

> 状态：已完成静态类型阶段。本文记录的是选择器规范化、结果类型投影、随机计划和
> 选择器左值广播的后端无关实现；它不表示 Runtime、字节码或 LLVM 已经能够执行真实
> 容器值。后续代理必须先读本文，再读 [03. 容器、集合与索引路径](03-collections.md)、
> [03A. C0 交接记录](03a-c0-containers.md) 和 [12. 测试与开发里程碑](12-tests-and-milestones.md)。

> 本子工程承接 C0 的结构化 `Type` 和 P1 的选择器 AST，把语法层的选择项降低为一份
> 可供解释器、字节码后端和 LLVM 后端共同消费的计划。类型层只计算“已知事实”和
> “需要 Runtime 验证的边界”，不得创建或修改运行时容器。

## 一级工程目标：冻结选择器的共享语义

### 输入与输出

输入是 `xiao-syntax` 的 `Expression::Selector`、`SelectorItem`、`IndexPath`，以及
`xiao-types` 当前环境中的结构化 `Type`。输出是 `TypeCheckResult` 中的三类记录：

- `SelectionPlan`：选择项、规范化路径、结果类型、步长、重复和 Runtime 检查需求；
- `BroadcastAssignmentPlan`：确定性选择器左值的标量广播写入计划；
- `RandomSeedPlan`：`random.seed(value)` 的静态值或 Runtime 检查记录。

计划是值对象，不能持有 AST 引用或运行时容器引用。后端必须消费这些记录，而不是重新
解析源码、重新推断类型或各自实现一套范围/随机算法。

### 已冻结规则

1. 同一对方括号内使用逗号表达多选；选择项按源码顺序保留，重复精确项和重叠范围不去重。
2. `start~end` 是双端闭区间；`<`、`<=`、`>`、`>=` 表示相应的开闭单边范围。
3. 数字索引从零开始并支持 Python 风格负索引；`str` 的索引单位是 Unicode 码点。
4. `{step}` 对每个选择项独立应用；正步长正向，负步长反向，零步长始终报错。
5. `[?x]` 是无放回抽取，`[!?x]` 是放回抽取；零数量返回来源类型的空结果，负数报错，
   无放回超量报错，空来源不能抽取正数。
6. 数组、元组、`str` 和字典列支持高级选择；字典表只能保留单个精确键路径，集合完全
   不可索引。范围路径不得穿过无序字典表。
7. 多选/范围结果保留来源根容器和必要的嵌套形状。字典列直接键重复时不能伪造重复键，
   必须表示为按选择顺序排列的元组。
8. 选择器左值只允许直接名称作为根容器和标量右值；随机目标、容器右值、复合赋值及
   复杂根表达式均拒绝。Runtime 写入必须先验证全部目标，再一次性提交并在失败时回滚。
9. `random.seed` 接受非负整数语义值；类型阶段可把静态整数规范化为 `u128`，动态值
   由 Runtime 检查非负性和表示范围。

## 二级工程任务

### C1-A：选择计划模型

实现位置：`core/rust/crates/xiao-types/src/selection_model.rs`。

1. `SelectionPathSegment` 保存数字段的原始有符号索引、可选的静态规范化位置，或规范化
   键名；`SelectionPath` 只表示从根开始的路径段。
2. `SelectionItemPlan` 覆盖 `Exact`、`Range`、`All` 和 `Random`，并保留随机模式、数量
   是否动态等信息。
3. `StepPlan` 区分静态非零整数和需要 Runtime 求值的表达式。
4. `SelectionPlan` 保存来源类型、结果类型、按源码顺序的选择项、静态路径、目标叶子类型、
   Runtime 标志、放回标志和重复提示。
5. `BroadcastAssignmentPlan` 和 `RandomSeedPlan` 只描述后续执行所需事实，不实现写入或
   随机状态推进。

公共类型通过 `xiao-types` 门面重新导出。新增字段时必须同时更新构造处、单元测试、
模块登记和交接文档；禁止把 Runtime 值、VM 指针或 LLVM 类型塞进这些结构。

### C1-B：静态路径与结果形状

实现位置：`selection_shape.rs`、`selector_checker.rs`。

1. 对固定长度数组、异构数组、元组和字典列枚举直接子项；未知长度数组只生成 Runtime
   边界检查，不臆造长度。
2. 解析每个精确路径的数字/键名段，静态已知越界和缺失键立即报错，未知边界登记
   `RuntimeCheckKind::SelectorBounds`。
3. 范围端点按有序容器的深度优先节点顺序展开。端点切入的嵌套分支只保留命中后缀，
   未切入的中间分支保留为完整容器；同一父节点的后缀在投影时合并。
4. 空路径结果调用 `empty_selection_type`，静态路径结果调用 `project_selection_type`。
   单项精确路径直接返回叶子类型，多项结果保留根容器类型。
5. 字典列重复直接键（包括数字键名混合选择）投影为保序元组；普通不重复选择仍保留
   字典列结构。

### C1-C：步长、随机与种子

实现位置：`selection_random.rs` 和 `selector_checker.rs`。

1. 步长必须是整数语义值；静态零值使用 `X03-TYPE-009`，动态步长登记 `SelectorStep`。
2. 随机数量必须是非负整数；负数、非整数和平台无法表示的数量使用 `X03-TYPE-010`，
   无放回超量或空来源正数使用 `X03-TYPE-011`，动态数量登记 `RandomCount`。
3. `RandomSource` 是后端和测试的最小注入接口；`SeededRandom` 使用跨平台可复现的
   xorshift64* 状态；`sample_indices` 统一实现无放回/放回抽样。
4. 类型阶段不推进随机状态。`random.seed(value)` 只生成 `RandomSeedPlan`；参数个数错误
   使用 `X03-TYPE-014`，非法类型、负数或超范围使用 `X03-TYPE-013`，动态参数登记
   `RuntimeCheckKind::RandomSeed`。
5. 放回随机的具体路径不能在类型阶段猜测。非零随机结果使用来源类型作为保守上界，
   字典列放回结果使用元组类型，避免把潜在重复键表示成非法字典列。

### C1-D：选择器左值广播

实现位置：`selector_checker.rs` 与 `checker.rs` 的语句分派接线。

1. 先检查目标选择器和值表达式，再检查目标诊断；目标已经失败时不生成写入计划，也不
   提前改变绑定状态。
2. 右值必须是标量，且能赋给每一个静态目标叶子类型；数组、元组、字典或 `none` 不做
   逐项配对，使用 `X03-TYPE-012` 拒绝。
3. 只允许可变、已初始化绑定的直接名称根；随机选择和 `+=` 等复合赋值拒绝。
4. 成功时生成 `BroadcastAssignmentPlan { transactional: true }`。静态目标可直接交给后端，
   动态目标还必须保留边界检查；Runtime 必须采用全量验证后提交的事务策略。

### C1-E：测试与文档同步

交付位置：

- `core/rust/crates/xiao-types/tests/c1_selectors.rs`：类型计划、诊断和回归测试；
- `tests/spec/05-containers/c1-valid.json`、`c1-errors.json`：跨后端可复用快照；
- `docs/UseDocs/language/collections/`：自然人可读的高级选择、随机和广播页面；
- `docs/DevDocs/03-collections.md`、`12-tests-and-milestones.md`、`00-decisions.md`：
  阶段状态和冻结决策；
- `docs/module-registry.json` 及 `src/README.md`、`tests/README.md`：模块追踪和交接边界。

测试只验证 AST 输入经过静态检查后得到的计划、类型和诊断，不宣称执行真实容器值。
每个新增公共类型/函数和测试辅助函数都必须有 Rustdoc，UseDocs 页面必须达到 `verified`
后才能把本子工程登记为已完成。

## 稳定诊断映射

| 编号 | 触发条件 |
| --- | --- |
| `X03-TYPE-008` | 对无序字典表/不支持容器使用高级选择，或范围穿过无序字典表 |
| `X03-TYPE-009` | 步长不是整数或静态值为零 |
| `X03-TYPE-010` | 随机数量不是非负整数或超出可表示范围 |
| `X03-TYPE-011` | 无放回抽取超量，或从空来源抽取正数 |
| `X03-TYPE-012` | 选择器左值不满足标量广播、可变根或确定性写入约束 |
| `X03-TYPE-013` | `random.seed` 参数不是合法非负整数语义值 |
| `X03-TYPE-014` | `random.seed` 参数个数不是一个 |

这些编号在字节码和 LLVM 后端之间共享；本阶段不定义本地化文案、Runtime 堆栈格式或
窗口输出协议。

## 验收与交接

### 已通过验收

1. `c1_selectors.rs` 的 18 项测试覆盖多选、范围、负索引、Unicode 字符串边界、正负步长、
   随机边界、种子、字典列重复键、字典表拒绝、广播事务边界和嵌套结果形状。
2. `xiao-types` 单元测试、格式检查和 Clippy 均通过；计划 API 可由后续后端直接读取。
3. 文档、规格快照、目录 README 和模块登记与代码同步，UseDocs 不把静态计划写成可执行
   Runtime 功能。

### 接手代理必须确认

1. 先读本文、`03-collections.md`、`03a-c0-containers.md` 和 `12-tests-and-milestones.md`。
2. 不在 `xiao-types` 创建运行时数组、字典、随机全局状态或写入操作；这些属于 Runtime/IR。
3. 不把集合实现、`frozenset`、默认值自动填充、字节码指令或 LLVM lowering 混入 C1。
4. 新增选择器语义先更新 `00-decisions.md` 和规格快照，再修改模型和检查器；不得通过
   临时实现偷偷冻结待定行为。
5. 后续 Runtime 必须消费 `SelectionPlan`、`BroadcastAssignmentPlan`、`RandomSeedPlan`，
   并使用 `RandomSource`/`sample_indices`，不能复制一套近似算法。

## 当前不负责的后续工作

- Runtime 对真实数组、元组、字符串和字典列执行读取、写入、自动扩容和默认值填充；
- 字节码/LLVM 指令编码、优化和双模式运行时一致性；
- 集合、可哈希值、`set()`、集合运算和 `frozenset`；
- 函数、控制流、模块导入、并发、调试窗口和本地化文案。
