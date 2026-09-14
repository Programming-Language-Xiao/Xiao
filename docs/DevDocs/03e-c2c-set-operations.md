# 03E. C2-C 集合运算静态闭环交接记录

> 状态：已完成静态阶段。本子阶段在 C2-B 的异构集合类型之上闭合集合运算的前端静态契约。
> 只生成 AST、类型结果、诊断和 Runtime 检查标记；不创建集合对象，不执行哈希、增删、
> 字节码或 LLVM 指令。代码、测试、UseDocs、目录 README 和模块登记均已同步并通过质量门禁；
> 后续 Runtime/IR 仍须消费本记录定义的结果类型和检查标记。

## Agent 交接上下文

### 接手前必须阅读

1. [00. 决策基线](00-decisions.md)：集合无序/可哈希边界、国际化字段和类型锁定规则。
2. [02A. P2-B/S0 静态标量类型](02a-p2-static-types.md)：`Type`、赋值兼容和动态检查接口。
3. [03. 容器、集合与索引路径](03-collections.md)：集合不可索引和容器职责边界。
4. [03C. C2-A](03c-c2a-sets.md)：集合诊断、`set()` 与空字典消歧。
5. [03D. C2-B](03d-c2b-heterogeneous-sets.md)：异构成员并集、动态尾标和成员判断。
6. [11C. 国际化](11c-localization.md)：稳定 `code`、`message_id`、参数与展示文本分离。
7. [12. 测试与开发里程碑](12-tests-and-milestones.md)：快照、UseDocs 和覆盖率门禁。

### 已有输入

- `xiao-syntax` 已有可恢复 Pratt 解析器、二元表达式 AST、`+`/`-` 和比较 Token，
  `Pipe` 只在 `set<...>` 类型注解中使用。
- `xiao-types` 已有 `SetType::Unknown`、同构/异构集合、动态尾标、集合赋值兼容和
  `in`/`not in` 检查；集合 Runtime 尚未实现。
- `RuntimeCheckKind` 已有 `SetHashability`、`SetMembership`；C2-C 新增的检查标记只
  作为后续 Runtime/IR 的输入，不在此阶段执行。

## 一级工程目标：闭合集合代数的静态语义

### 冻结规则

1. 两侧都是集合时，`+` 表示并集；`&` 表示交集；`-` 表示差集；`^` 表示对称差。
   `|` 与 `|=` 永远不是集合并集运算符，仍只用于类型注解。
2. 运算符优先级从高到低为算术、比较、`&`、`^`、逻辑与、逻辑或；`&` 高于 `^`，
   两者都低于比较和算术。集合和标量共用同一语法 Token，具体语义由类型检查决定。
3. 结果类型按静态成员可证明集合计算：
   - `A + B`：两侧已知成员类型并集。
   - `A ^ B`：两侧已知成员类型并集。
   - `A - B`：保留左侧已知成员类型；右侧动态边界只追加动态尾标。
   - `A & B`：两侧已知成员类型交集。
4. 新增 `SetType::Empty` 表示已经静态证明为空的运算结果，并以 `set<never>` 格式化。
   `SetType::Unknown` 仍表示未知约束（例如无上下文的 `set()`），不能与 `Empty` 混用。
5. 交集有静态共同类型时返回共同类型；没有共同类型且两侧没有动态边界时返回 `Empty`。
   只要任一侧未知或带动态尾标，就保留可证明成员并设置动态尾标，不能把不确定情况
   错误降成静态空集合。
6. 动态或未统一的集合操作数保留已知成员并集并登记动态检查；已知为标量的操作数
   不能被动态值掩盖而伪装成集合。集合与标量混用产生稳定类型诊断。
7. 集合比较 `==`、`!=`、`<`、`<=`、`>`、`>=` 均返回 `bool`，允许同构和异构集合
   互相比较。`<`/`<=` 表示真子集/子集，`>`/>=` 表示真超集/超集。动态边界登记
   `SetComparison`。
8. `+=`、`-=`、`&=`、`^=` 全部支持，但左值必须可变；运算结果必须能写回左值当前
   锁定类型。不能自动扩大左值类型。已知成员冲突在编译期报错，动态尾标则登记
   `SetMembership` 后交给 Runtime。
9. 非集合标量的 `+`、`-` 和既有比较语义保持不变。`&`、`^` 不开放标量位运算；非集合
   操作数使用集合运算符时报告集合专属诊断。

### 国际化边界

新增诊断必须保存稳定 `code`、`message_id` 和结构化参数；当前中文只作预览。建议编号：

| 编号 | `message_id` | 参数 | 触发条件 |
| --- | --- | --- | --- |
| `X03-TYPE-021` | `x03.type.set_operation_requires_sets` | `operator`、`left_type`、`right_type` | 集合代数两侧不是可确认的集合形状 |
| `X03-TYPE-022` | `x03.type.set_comparison_requires_sets` | `operator`、`left_type`、`right_type` | 集合比较两侧不是可确认的集合形状 |

快照只能比较这些机器字段，不能匹配中文或英语译文。类型层不加载语言目录，也不从
展示文本反解析类型。

## 二级实现任务（SOP）

### C2-C.1：Token、AST 与 Pratt 优先级

1. 在 `xiao-syntax/src/token.rs` 增加 `Ampersand`、`Caret`、`AmpersandEqual`、
   `CaretEqual`，补充稳定名称、运算符判定和文档注释。
2. 在 `lexer.rs` 使用最长匹配扫描 `&`/`&=` 与 `^`/`^=`；不得影响字符串、注释、
   路径或 `|` 类型注解。
3. 在 `ast.rs` 增加 `Intersect`、`SymmetricDifference`、`IntersectAssign`、
   `SymmetricDifferenceAssign`，保持 `SourceSpan` 和既有二元节点布局。
4. 在 `parser.rs` 将 `&` 和 `^` 接入既有 Pratt 表，并把四种复合 Token 映射到赋值枚举。
   增加优先级回归，确认 `a + b & c ^ d` 的树形为 `((a + b) & c) ^ d`。

### C2-C.2：集合结果类型模块

1. 在 `xiao-types/src/set_types.rs` 增加 `SetType::Empty`、`empty()`、`is_empty()` 和
   动态边界查询；保持 `Unknown` 的 `set()` 语义不变。
2. 实现独立的 `union`、`intersection`、`difference`、`symmetric_difference` 纯类型
   辅助，统一成员规范化和动态尾标传播；不把运算实现塞回 `checker.rs`。
3. 更新 `can_assign_set`、统一器和类型展示，保证空结果能写入任意兼容集合，而未知
   或动态集合不能伪装成静态空集合。
4. 为纯类型辅助补充同构、异构、空、未知、动态和不同整数/布尔类型隔离测试。

### C2-C.3：类型检查与动态检查计划

1. 新增 `xiao-types/src/set_operations.rs`，提供集合运算/比较分派、操作数诊断和
   `SetOperationCheck` 内部结果；`checker.rs` 只负责调用分派，不复制成员算法。
2. `+`/`-` 在任一侧已知集合时进入集合语义；`&`/`^` 始终按集合运算检查；比较在
   任一侧为集合时进入集合比较检查。已知标量和动态值的组合不能绕过操作数诊断。
3. 静态集合操作按 `SetType` 纯函数生成结果；动态边界登记 `SetOperation`，集合比较
   动态边界登记 `SetComparison`。
4. 将四种原地赋值映射到对应二元运算，检查左值可变性、初始化状态、结果可赋回性和
   已锁定显式类型；动态结果另登记 `SetMembership`。
5. 扩展数值辅助的穷举分支，使集合运算不会误落入标量数值提升或常量折叠；集合常量
   求值继续留给 Runtime/后续常量系统。

### C2-C.4：规格快照与文档同步

1. 新增 `core/rust/crates/xiao-syntax/tests/c2c_set_operations.rs`，覆盖 Token、
   优先级、四种复合赋值和 AST 形状。
2. 新增 `core/rust/crates/xiao-types/tests/c2c_set_operations.rs`、`c2c_snapshots.rs`
   及 `tests/spec/05-containers/c2c-valid.json`、`c2c-errors.json`，覆盖结果类型、
   `Empty`/`Unknown`、动态标记、比较、混合操作数和原地锁定冲突。
3. 更新 `docs/UseDocs/language/collections/sets.md`、`errors.md` 和主题 README，
   只把已验证的静态运算写成可用内容，并明确 Runtime 尚未执行集合运算。
4. 更新模块登记、源码/测试目录 README、`03-collections.md`、`00-decisions.md`、
   `12-tests-and-milestones.md` 和本页状态；代码、测试、DevDocs、UseDocs 必须同一提交。

## 已交付内容

- 词法层新增 `&`/`^` 及其复合赋值 Token，Pratt 优先级固定为算术、比较、`&`、`^`、逻辑运算。
- 类型层新增 `SetType::Empty` 和独立 `set_operations.rs` 分派；四种运算、六种比较及四种原地形式均有稳定结果或诊断。
- 动态边界分别输出 `SetOperation`、`SetComparison` 和写回时的 `SetMembership` 计划；本阶段不执行 Runtime 集合。
- Rust 集成测试、JSON 规格快照、源码/测试目录 README、模块登记和集合 UseDocs 已同步。

## 验收命令与退出条件

```text
cargo fmt --manifest-path core/rust/Cargo.toml --all -- --check
cargo check --manifest-path core/rust/Cargo.toml --workspace
cargo test --manifest-path core/rust/Cargo.toml --workspace
cargo clippy --manifest-path core/rust/Cargo.toml --workspace --all-targets --all-features -- -D warnings
cargo doc --manifest-path core/rust/Cargo.toml --workspace --no-deps
bun run check
bun run check:usedocs
bun run check:coverage
bun test
```

退出条件：

1. 新 Token、AST、优先级和复合赋值语法测试通过，既有语法快照不变。
2. 四种集合运算和全套比较返回稳定静态类型；空交集明确为 `Empty`，未知边界明确保留动态尾标。
3. 原地运算遵守可变性和锁定类型，不自动拓宽；混合操作数与显式冲突拥有稳定诊断。
4. `SetOperation`、`SetComparison`、`SetMembership` 标记可被后续 Runtime/IR 消费，当前不执行。
5. Rustdoc 公共 API 100%、全仓库声明注释覆盖率至少 90%，UseDocs 状态为 `verified`，模块登记同步。

## 后续交接边界

### C2-D / Runtime

后续阶段负责真实集合存储、哈希、增删、集合值运算和运行时比较；必须复用本阶段的
`SetType` 结果规则与 `RuntimeCheckKind`，不能建立第二套成员并集或动态传播逻辑。元组
递归哈希、`frozenset` 和方法式 API 仍不属于 C2-C。

### IR、字节码与 LLVM

本阶段不生成集合指令或常量值。第 08、09、10 阶段消费类型结果和检查标记时，必须保留
源码位置、稳定错误身份与国际化参数；后端不得自行拼接或解析本地化文本。
