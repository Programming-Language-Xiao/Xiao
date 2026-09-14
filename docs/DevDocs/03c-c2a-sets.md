# 03C. C2-A 最小集合静态闭环交接记录

> 状态：已完成。本次全仓库质量门禁已通过；本文是 03 阶段的 C2-A 子工程交接
> 契约；它只闭合“集合语法 AST → 静态类型/诊断”，不把集合 Runtime、集合代数或
> 后端指令提前塞进类型层。

## Agent 交接上下文

### 接手前必须阅读

1. [00. 决策基线](00-decisions.md)：`bool` 独立类型、Python 风格可哈希规则和国际化字段边界。
2. [02A. P2-B/S0 静态标量类型](02a-p2-static-types.md)：类型环境、转换矩阵和动态检查标记。
3. [03. 容器、集合与索引路径](03-collections.md)：容器分类、集合拆分和不可索引语义。
4. [03A. C0 基础容器](03a-c0-containers.md) 与 [03B. C1 有序选择器](03b-c1-ordered-selectors.md)：已有 AST、容器类型和选择器边界。
5. [11C. 国际化](11c-localization.md)：`code`、`message_id`、`params` 与展示文本分离规则。
6. [12. 测试与开发里程碑](12-tests-and-milestones.md)：C2-A 退出条件和 UseDocs 同步门禁。

### 前置输入

- `xiao-syntax` 已提供带 `SourceSpan` 的字面量、调用、容器和选择器 AST；`{}` 的字典表
  语义及花括号错误恢复已稳定。
- `xiao-types` 已提供标量类型、`Type::Dynamic`、容器结构类型、作用域/绑定、赋值兼容
  矩阵和 `RuntimeCheckKind`。
- `xiao-diagnostics` 已提供不可变 `Diagnostic`，并能保存语言无关的 `DiagnosticParam`。
- A0 的目录 README、模块登记、规格快照和 UseDocs 检查器已经可执行。

## 一级工程目标：闭合 C2-A 静态集合语义

### 冻结规则

1. 非空花括号中只出现值条目时形成 `Expression::SetLiteral`；花括号中出现 `键 = 值`
   条目时形成字典表；`{}` 永远是空字典表。`set()` 是唯一的空集合构造式，C2-A 只接受零参数。
2. 集合无序。AST 可以按源码顺序保存元素以便定位、诊断和测试，但类型层、后端和用户代码
   不能观察这一次书写顺序。
3. C2-A 的集合必须是单一元素类型。显式前缀（例如 `int ids = {1, 2}`）约束每一个成员
   的类型，不表示位置，也不会给无序集合增加索引。异构集合留给 C2-B。
4. `int`、`sint`、`lint`、`float`、`sfloat`、`lfloat`、`str`、`bool` 和 `none` 可作为
   C2-A 的静态元素类型；`bool` 永远不与整数或浮点统一。
5. 只有静态可证明的标量和 `none` 在本阶段判定为可哈希。数组、字典表、字典列、集合、
   函数和当前阶段的元组判定为不可哈希；动态值登记 Runtime 可哈希检查。元组的递归哈希
   证明属于后续阶段，不得在 C2-A 中凭实现方便偷偷放开。
6. 静态常量重复元素产生诊断；比较按静态类型和值进行，不把 `true` 当作 `1`，也不把
   不同数值族的值隐式合并。动态表达式的相等和哈希留给 Runtime。
7. `value in values` 与 `value not in values` 返回 `bool`。右侧必须是集合，左侧必须满足
   已知元素类型和可哈希约束；动态边界只登记 Runtime 检查。
8. 集合不支持数字索引、键名索引、范围、多选、步长、全选或随机选择；选择器检查遇到
   集合时报告集合专属诊断且不生成可执行选择计划。
9. 本批次不创建 Runtime 集合、不实现增删、集合代数（包括 `+` 并集）、比较运算或
   `frozenset`。这些功能必须在新的 C2 子阶段先冻结规则再实现。

集合嵌套在数组或字典中时，路径可以定位到“集合这个容器”并对其成员施加类型约束，
但不能继续用数字或键名定位某个成员；进入集合的路径统一使用 `X03-TYPE-020`。

### 国际化边界

每条新增集合诊断都保存稳定 `code`、`message_id` 和结构化参数；`Diagnostic.message()`
只是当前语言的预览文本。类型检查和测试不得从中文/英语句子反解析参数，也不得把翻译
目录依赖倒灌到 `xiao-types`。当前阶段仍使用中文预览，真正的目录选择和渲染由 11C 接入。

## 二级实现任务

### C2-A.1：语法与 AST

交付位置：

- `core/rust/crates/xiao-syntax/src/ast.rs`：`Expression::SetLiteral`、源码区间和节点遍历；
- `core/rust/crates/xiao-syntax/src/parser.rs`：花括号消歧、集合元素解析、混合条目恢复；
- `core/rust/crates/xiao-syntax/tests/c2a_sets.rs`：集合、`set()`、嵌套元素和错误恢复回归。

验收重点：非空纯值花括号与键值花括号不能混淆；空花括号必须继续产生空字典表；解析器
不得在语法层按哈希或排序重排元素；每个集合节点和元素保留准确 `SourceSpan`。

### C2-A.2：静态类型与可哈希能力

交付位置：

- `core/rust/crates/xiao-types/src/set_types.rs`：`SetType`、`Hashability`、集合赋值兼容；
- `core/rust/crates/xiao-types/src/set_checker.rs`：集合字面量、`set()`、成员判断、静态
  唯一性和 Runtime 检查标记；
- `core/rust/crates/xiao-types/src/types.rs`、`unify.rs`、`conversion.rs`：集合类型递归
  遍历、统一和容器赋值接线；
- `core/rust/crates/xiao-types/src/container_checker.rs`、`selector_checker.rs`：显式成员
  类型约束和集合不可索引边界。

类型层只返回 `Type::Set(SetType)` 和旁路计划，不保存运行时成员、不执行哈希、不实现变更。
未知/动态集合元素不得被假装成静态合法值；应登记 `SetHashability` 或 `SetMembership`。

### C2-A.3：结构化诊断

交付位置：

- `core/rust/crates/xiao-diagnostics/src/lib.rs`：`DiagnosticParam::{Text,Integer,Boolean}`、
  `DiagnosticParams` 和只读访问器；
- `core/rust/crates/xiao-types/src/diagnostics.rs`：C2-A 稳定编号；
- `docs/UseDocs/language/collections/errors.md`：面向使用者的触发条件和参数说明。

稳定映射如下：

| 编号 | `message_id` | 参数 | 触发条件 |
| --- | --- | --- | --- |
| `X03-TYPE-015` | `x03.type.set_element_type_mismatch` | `actual_type`, `expected_type` | 单一元素类型或显式成员约束冲突 |
| `X03-TYPE-016` | `x03.type.set_unhashable_element` / `x03.type.set_unhashable_membership` | `actual_type` | 元素或成员查询值不可哈希 |
| `X03-TYPE-017` | `x03.type.set_duplicate_element` | `element` | 静态常量重复 |
| `X03-TYPE-018` | `x03.type.set_constructor_arity` | `actual_count`, `expected_count` | `set()` 参数不是零个 |
| `X03-TYPE-019` | `x03.type.set_membership_requires_set` / `x03.type.set_membership_element_mismatch` | `operator`, `actual_type`，必要时 `expected_type` | 成员判断右侧或左侧类型不满足规则 |
| `X03-TYPE-020` | `x03.type.set_index_unsupported` | 无 | 对集合使用任意索引或高级选择器 |

同一编号可能有不同的消息键以区分元素位置和成员查询上下文；调用方仍以编号、键和参数
作为机器接口。旧 C0/C1 诊断允许参数为空，但新增集合诊断不得把插值值只拼进展示句子。

### C2-A.4：测试、快照与 UseDocs

交付位置：

- `core/rust/crates/xiao-types/tests/c2a_sets.rs`：类型推导、哈希、唯一性、成员判断、
  动态标记和索引拒绝；
- `core/rust/crates/xiao-types/tests/c2a_snapshots.rs`：跨后端快照，比较稳定身份和参数，
  不比较本地化展示文本；
- `tests/spec/05-containers/c2a-valid.json`、`c2a-errors.json`：可复用正反例；
- `docs/UseDocs/language/collections/sets.md`、`errors.md` 及主题 README：自然人阅读路径；
- `docs/module-registry.json` 和各源码/测试目录 README：模块登记与交接说明。

每个新增测试辅助函数必须有注释；公共 Rust API 和模块文档覆盖率继续满足 100%，全仓库
声明项不低于 90%。代码、测试和 UseDocs 必须同一可审计变更集中完成。

## 验收与交接

### 定向命令

```text
cargo test --manifest-path core/rust/Cargo.toml -p xiao-syntax -p xiao-types
cargo test --manifest-path core/rust/Cargo.toml -p xiao-types --test c2a_snapshots
cargo clippy --manifest-path core/rust/Cargo.toml -p xiao-types --all-targets -- -D warnings
bun run check:usedocs
bun run check:coverage
```

### 已验证的退出条件

1. 语法能稳定区分集合、空字典表和 `set()`，并保留无序语义。
2. 类型层能推断/检查单一元素类型，`bool` 与数值严格分离，`none` 可作为元素。
3. 重复、不可哈希、类型冲突、构造器参数错误、成员判断错误和集合索引错误都有稳定身份。
4. `in`/`not in` 返回 `bool`；动态值只产生后续 Runtime 检查标记。
5. 快照、Rustdoc、UseDocs、模块登记和国际化字段已经同步；没有集合 Runtime 或代数实现。

### 接手代理不得做的事

- 不要把异构集合、元组递归哈希、集合增删、集合代数或 `frozenset` 混入 C2-A 修补。
- 不要在 `xiao-types` 创建集合对象、执行哈希、排序成员或访问 VM/LLVM/CLI。
- 不要把集合索引降级成“第一个元素”，也不要为失败的集合选择生成 `SelectionPlan`。
- 不要用本地化展示文本作为测试断言；先更新 `00-decisions.md` 和消息目录契约，再扩展编号。
- 不要继续把集合逻辑堆回超过单一职责的中心文件；新增语义应进入独立模块并同步目录 README。

## 后续交接边界

### C2-B：异构与动态成员（已完成静态阶段）

异构集合的静态成员并集、跨类型相等/哈希隔离、动态尾标和成员检查已在
[03D C2-B 交接记录](03d-c2b-heterogeneous-sets.md) 中完成。后续代理必须复用本阶段的
`DiagnosticParam` 和稳定编号，不修改 C2-A 快照语义；真实 Runtime 插入检查仍由后续阶段
消费 `SetHashability`/`SetMembership` 标记。

### C2-C：集合运算

在独立规格中冻结 `+` 并集、`&` 交集、`-` 差集、`^` 对称差和比较运算的结果类型、显式类型
冲突及操作数错误，再由 IR/Runtime 消费计划。C2-A 不预留伪造的运算计划。

### Runtime/IR 与国际化

Runtime/IR 负责真实集合存储、哈希表、增删和执行期错误；11C 负责根据 `message_id`、参数
和有效语言目录渲染文案。两者都必须保留本阶段的源码位置和机器诊断字段，不能重新解析
`message` 文本或建立第二套集合类型模型。
