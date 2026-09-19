# 09R2F. 集合运算执行闭环交接文档

> **本文对应的执行批次已完成。** 开发前的基线问题仍保留在第一节作为历史证据：当时
> `{1} == {1}` 返回 `false`，`{1} + {2}` 报误导性的宽度错误；09R2F1 已修复并补上
> 三机型运行时、编码和动态检查验证。
>
> 交付结果：C2-C 冻结的集合语义在三种研究 VM 载体上执行，四个检查中的集合部分已接通；
> 集合显式成员类型兼容、增删和迭代仍由后续阶段负责。

> 验证证据：`r2_set_values.rs` 直接执行两条新指令，`sets.json` 提供 28 条共享集合向量，
> `r2_tac.rs` 覆盖双操作数检查、CFG 边界和两种编码宽度；总共享向量为 59 条。

## Agent 交接上下文

### 接手前提

1. [09R2D. 两种机型与指令编码器交接文档](09r2d-machines-and-encoder.md) —— **开发规定的主表在这里**：
   门禁命令、区分度验证、工具规定、单一来源原则、解耦约束、**新增 `TacOp` 的 8 处改动点清单**
   （`:373-399`），本文不复制。
2. [09R2B. 选择器全量执行交接文档](09r2b-selector-execution.md) —— 「类型层规范化成计划 →
   一条 TAC 指令索引该计划」的范式，以及「不需要改载体」的判据。
3. [03E. C2-C 集合运算静态闭环](03e-c2c-set-operations.md) —— **本批要执行的语义
   就是它冻结的**：四种代数、六种比较、成员判断的静态规则。
4. [09R. 字节码寄存器机型特别研究](09r-bytecode-machine-research.md) —— 指令分组表（`:90-101`）、
   R1-AC 门槛、R1-AD 基准协议；`:612-621` 是 09R3 之后的冻结范围。
5. [09. 字节码运行模式](09-bytecode-runtime.md) —— 09 阶段最小指令集，其中点名「集合运算」。

### 本批交付与不负责

**交付**：`TacOp::SetOp` 与 `TacOp::SetCompare` 两条指令及其编码标签；`SetHandle` 的四种代数
与六种关系判定、成员判断；四个 RuntimeCheck 中的集合部分；共享向量与区分度用例；文档同步。

**不负责**：

- **集合的增删（`append`/`insert`/`remove`）与跨后端 lowering**——同属 C2-D 的其余部分，
  **不要在文档里合并宣称**。
- **哈希索引**。本批所有算法建在现有有序 `Vec` 上（理由见第二节）。
- `for` 与迭代（**09R2G**）、`table` 声明（**09R2H**）、`LoadFunc`/`Box`/`Unbox`/`CallDynamic`。
- 09R3 的基准设施（`tests/benchmarks/`、`tests/differential/`，目前只有 README）。

---

## 一、为什么集合排在最前（三条依据）

### 1. 交接前集合是「在算错」而不是「没实现」（决定性依据，均已实测）

`core/rust/crates/xiao-runtime/src/value/mod.rs` 的 `PartialEq`：

```rust
(Self::Set(left), Self::Set(right)) => left.same_object(right),
```

**这是身份比较，不是值相等**。于是 `{1} == {1}`（两个不同对象）返回 `false`。
而类型层 `xiao-types/src/set_operations.rs` 判定它合法——**错误结果一路静默通过**。

`core/rust/crates/xiao-runtime/src/value/ops.rs` 的 `add`：Bool 与 Str 各有一条快路径，
之后**直接进 `NumericPair::new`**。集合落到那里，报出的是一条**误导性**的
「宽度不一致；后端必须先插入显式转换」，而不是「集合不支持加法」。

C2-A / C2-B / C2-C 三份文档都是 `verified` 静态闭环，运行时却相反。**先修会算错的。**

### 2. 集合不引入控制流边

新增 `SetOp`/`SetCompare` 没有跳转目标，因此**完全绕开全仓最危险的静默漏改点**：
`xiao-bytecode/src/research/cfg.rs:24` 的 `jump_targets` 兜底 `_ => Vec::new()`——
漏改会被静默当成「没有后继」，**活跃区间算错 → 寄存器复用踩掉还要用的值**。
`09R2D` 文档把这一条列为「最危险的一条」，本批恰好不触发它。

### 3. 两处验收点名

09R3 退出条件第 2 条点名「数组/元组/字典/**集合**」；09 阶段最小指令集点名「集合运算」。

---

## 二、两条新指令，不是一条

```
TacOp::SetOp      { op: SetOpKind,    left: VReg, right: VReg }  → dst 是集合
TacOp::SetCompare { op: SetCompareOp, left: VReg, right: VReg }  → dst 是 bool

SetOpKind    = Union | Intersection | Difference | SymmetricDifference
SetCompareOp = Equal | NotEqual | Subset | ProperSubset | Superset | ProperSuperset
             | Member | NotMember
```

**为什么拆两条**：结果类别不同（集合句柄 vs 布尔）；R1 的分组表把「比较（结果恒为布尔）」
单列成一组（`09r-bytecode-machine-research.md:96`）；C2-C 在类型层本就是**两条独立路径、
两个 RuntimeCheck、两个诊断码**（`X03-TYPE-021`/`022`）。合成一条等于把已经分开的东西
在 TAC 层重新黏起来。

**为什么 `Member`/`NotMember` 归比较而不是代数**：`in` 在类型层就被路由到集合语义
（`xiao-types/src/checker.rs` 的 `SetMembership` 分支），且强制右侧必须是集合，
它天然是「关系」而非「代数」。实现刻意不用 `Contains` 命名：后者读起来像「左侧
包含右侧」，而 `x in s` 的左操作数其实是被包含的成员，`Member` 能固定这个方向。

### 明确拒绝的第三条路

**不要**复用 `Arith`/`Compare` 加枚举变体。成本确实最低（`ArithOp`/`CompareOp` 有独立追加表，
不触发 `all_ops_program` 的 `ops.len()` 守卫），但有实质隐患：那会让 `Compare{Less}` 在两个
整数上意味着「小于」、在两个集合上意味着「真子集」。**优化器与 LLVM 后端看到 `Compare{Less}`
时无法安全做强度削减或常量折叠**——语义重载发生在指令层而非算子层时，没有安全 fallback。

### 编码

照 `encode/tags.rs` 的 `arith_tag`/`arith_from_tag` 写 `set_op_tag`/`set_op_from_tag`（4 个标签）
与 `set_compare_tag`/`set_compare_from_tag`（8 个标签）。两者都必须是
**穷尽 match + 未知标签显式报 `InvalidEnum`，不得兜底**（照 `compare_from_tag`）。
opcode **追加在末尾**，同步更新「0–N 连续」的表述与 `all_ops_program` 的计数守卫。

**新增 `TacOp` 的 8 处改动点见 09R2D 文档 `:373-399`**，其中 `decode_op` 与 `cfg.rs::jump_targets`
**编译器不强制**。本批的 `SetOp`/`SetCompare` 没有控制流边，但**仍要按清单逐处改**，
不要因为「没有边」就跳过 `jump_targets`——顺带在测试里断言它返回空（见验收第 2 组）。

---

## 三、`SetHandle`：算法建在现有 `Vec` 上，不引入哈希

交接前的 `SetHandle`（`xiao-runtime/src/containers/set.rs`）只有
`new`/`len`/`is_empty`/`contains`/`with_elements`/`same_object` 等，**没有任何代数或值比较**。

本批新增（算法写成模块内 `&[RuntimeValue]` 私有纯函数，`SetHandle` 只暴露薄壳）：

```
union / intersection / difference / symmetric_difference            → RuntimeResult<SetHandle>
equals / is_subset / is_proper_subset / is_superset / is_proper_superset → RuntimeResult<bool>
```

- 成员判定**复用 `RuntimeValue::PartialEq`**——它已按判别式隔离
  （`Bool(true)` ≠ `Int(1)` ≠ `Float(1.0)` ≠ `Sint(1)`），正是 C2-B 要的口径。
  **不要用 `type_name()` 或 `as_bool()` 之类的近似判定**：`Bool(true)` 会命中 `Int(1)`。
- 交集/差集**按 `left` 原序过滤，不构造中间集合再去重**。

### 结果顺序契约（必须写进 `set.rs` 模块 doc 与本文件）

| 运算 | 结果顺序 |
| --- | --- |
| 并集 | `left` 原序，随后 `right` 中不在 `left` 者按 `right` 原序 |
| 交集 / 差集 | `left` 原序过滤 |
| 对称差 | `left` 独有按 `left` 序，随后 `right` 独有按 `right` 序 |
| 六种比较 | 与顺序无关 |

理由：**只依赖操作数，不依赖哈希、不依赖排序**。`RuntimeValue` 没有全序，
引入排序等于新增一个待冻结契约。

> **给 09R2G 的警告**：`for` 一旦落地，这个顺序就成为共享向量的**可观察期望值**。
> 届时**不许为了让 `for` 的输出好看而改这里的顺序**——改了要连带改向量，那是掩盖而不是修复。

### `RuntimeValue::Hash` 的去留

**本批保留。** 已核实它零消费者，但删除一个公共 trait impl 属于范围外，
还会牵动重导出复核与 doc-coverage 门槛。

本批真正要钉死的规则是：**「集合成员判定与去重一律走 `PartialEq` + 有序 `Vec`，
不得为实现代数而引入哈希索引」**——写进 `set.rs` 的模块 doc 顶部。
它保护的正是 R2a「释放序列不被哈希序污染」的原始理由。

---

## 四、四个 RuntimeCheck：其中一个已**显式缩小范围**

机制前提：检查按 `(span.start, span.end)` **精确匹配消费**（`lower/mod.rs:221-234`），
所以「在哪个 span、拿哪个寄存器去消费」就是全部设计。

| kind | 发射方式 | `check_value` 判定 | 错误码 |
| --- | --- | --- | --- |
| `set_hashability` | **通用尾巴自动消费**（元素寄存器正好是操作数）→ 只需加白名单 | `is_hashable(value)` | **复用** `X06-RUNTIME-016` |
| `set_operation` | 集合分支里对 **left、right 各发一条 `Check`** | `matches!(value, Set(_))` | 新增 `X06-RUNTIME-021` |
| `set_comparison` | 同上 | 同上 | 新增 `X06-RUNTIME-022` |
| `set_membership` | 见下 | 见下 | 新增 `X06-RUNTIME-023` |

`X06-RUNTIME-020` 已被占用，**本批使用的后续编号是 021–023**。`set_hashability` 复用 `-016`
（`CONTAINER_HASHABILITY_CODE`）——`RuntimeError::unhashable_element` 已经是这个身份，复用才一致。

**错误类型名走 `TypeError`**：`emit_runtime_check_kinds` 只把 `arithmetic`/`numeric_range`
映射成 `ArithmeticError`，集合三类落进 `else` 正好。**不要动那个三元表达式的结构。**
三个 `pub const *_CODE` 要经 `xiao-runtime` 的重导出列表导出（`semantics/exec.rs` 就是从那里引的）。

### 代数与比较必须检查**两个操作数**，不能只检查结果

类型层允许「一侧静态集合、另一侧 `Dynamic`」进入集合语义。运行时那个 `Dynamic`
完全可能不是集合——**只有检查它才能拦住**。而结果若成功必然是集合，对结果做检查近乎恒真。

这需要一个新入口 `emit_runtime_checks_for(span, &[left_reg, right_reg])`，
且**必须把该 span 的检查取干净**（`emit_runtime_checks` 用 `remove`）。

### `set_membership` 缩小到「可哈希」半（**本批最重要的范围决定**）

它的登记点分两类：

- **类 A（4 处）**：语义是「这个集合的动态成员必须满足**声明的** `set<T>` 成员类型」。
  `Check { kind, value }` 只带一个字符串 kind 和一个寄存器，**带不了期望成员类型**——
  这一半**在本批的数据结构下无法表达**。
- **类 B（2 处，`x in s` 的左操作数）**：只要求可哈希 → 可表达。

**本批口径**：`set_membership` 定义为单操作数的可哈希性质——
`value` 是 `Set` → 所有成员 `is_hashable`；否则 → `is_hashable(value)`。
覆盖类 A、类 B 的**可哈希半**。

**类型兼容半记为具名债项，交给 09R2G**（那一批无论如何要为 `for` 引入新的操作数形态，
正好复用同一次格式变更）。**绝不能用 `record_unsupported` 给它记账**——
`encode/validate.rs:40` 对非空 `TacProgram.unsupported` **直接拒绝编码**，
一记就把 09R3 四个指标之一的**编码体积**整条堵死，而且要到 R3 才流血。

---

## 五、执行层（已接通）

`semantics/exec.rs` 的 `step` 加两个 arm，逻辑下沉到 `research/ops.rs`
（照 `apply_arith` 的先例）：

```rust
pub fn apply_set_op(op: SetOpKind, left: &RuntimeValue, right: &RuntimeValue) -> RuntimeResult<RuntimeValue>
pub fn apply_set_compare(op: SetCompareOp, left: &RuntimeValue, right: &RuntimeValue) -> RuntimeResult<RuntimeValue>
```

非集合操作数一律返回带新错误码的 `RuntimeError`；三种新码已在 `xiao-runtime` 重导出并由
ops 单测和共享向量覆盖。
**不要给三种载体各加方法**——载体只管「值放在哪里」，指令由 `step` 统一实现（R2D 已定）。

`use_def` 与 `Arith` 同形（`uses = {left, right}`，`dst` 由通用尾巴加），照 `liveness.rs` 对应分支改。

---

## 六、最可能翻车的前三处（审查记录与已采取的防护）

### 1. `check_value` 的 `_ => true`（`semantics/exec.rs:1330`）

官方「新增 TacOp 的 8 处清单」**不含这一条**，因为它不是新增变体的漏改点。
你会看到：白名单加了、`Check` 指令发了、这里忘了 → **所有集合检查静默恒真**。
只要向量以「静态集合 + 期望 success」为主，**全绿**。
这与 `cfg.rs::jump_targets` 的兜底是同一类 bug，只是换了个位置。
配套的第二处是 `runtime_check_code` 的 `_ => None`（`:1283`）——忘了它错误就没有稳定码。

**防守**：加一条穷尽性守卫测试，遍历 `xiao_types::RuntimeCheckKind` 的**全部 14 个变体**，
断言它要么在降低白名单（`lower/mod.rs:736-746`）里、要么在一份**显式列出的未支持清单**里。
以后新增 kind 必须二选一，无法静默漏。

### 2. `set_membership` 的类型兼容半被 `record_unsupported` 记账

见第四节。症状是语义向量全绿、`r2_tac` 全绿，然后 `encode()` 失败——
R3 的编码体积指标没有任何集合程序可测。

### 3. 子集/真子集与 `==` 的无序语义写反

- `<` 是**真**子集、`<=` 是**非真**子集：`{1,2} < {1,2}` 必须为 `false`。
  最自然的写法是让两者共用 `is_subset`——**那就错了**。
- `==` **必须是无序双向包含**。集合物理表示是有序 `Vec`，
  顺 `left.elements == right.elements` 是错的，**复用 `RuntimeValue::PartialEq`
  （即今天的行为）也是错的**——那正是 `same_object` 的 bug。
- 空集方向性：`set() <= {1}` 为 true，`{1} <= set()` 为 false，`set() < set()` 为 false；
  且 `set()`（`SetType::Unknown`）与 `{1} - {1}`（`SetType::Empty`）在运行时**必须是同一种值**。
- 异构集合命中：`{1, "a"} & {1, true}` → `{1}`，且 `{true} == {1}` 必须为 **false**。
  用 `type_name()` 或 `as_bool()` 做判定就会写成 true。

### 另一处机制问题：**半消费比不消费更糟**

集合检查挂在二元表达式的 span 上，而选择器用**范围清扫**（`emit_runtime_checks_in`）消费检查。
若 `lower_binary` 只消费一半（只对 left 发 `Check`、忘了 right），剩下的会被选择器捞起来，
**用一个语义不相干的寄存器**（选择器源容器）发 `Check{set_operation}`。
选择器源是数组/元组 → **检查失败，把合法程序打死**。这不是「少检查」，是「错检查」。

---

## 硬性约束

门禁、区分度验证、工具规定、单一来源原则、解耦约束**全部沿用 09R2D 文档第二章**（不重复）。
以下是本批额外注意的三条：

1. **不要给三种载体各加方法**：指令由 `step` 统一实现，载体只管值放在哪里。
2. **新增的每个 `pub` 项都要有文档注释**——`SetOpKind`/`SetCompareOp` 的每个变体、
   `SetHandle` 的每个新方法、三个 `pub const`。公共 API 100% 与全仓 ≥90% 是硬门槛。
3. **改动 `xiao-runtime/src/containers/set.rs` 会触碰一个 `verified` 模块**——
   它的 UseDocs 页面与模块登记要同批复核。

---

## 原计划提交切分（已执行）

按可独立验证的单元分三次，**每次提交后门禁全绿**：

1. **集合运行时与语义**：`SetHandle` 的代数与比较 + `apply_set_op`/`apply_set_compare` +
   `set.rs` 的顺序契约与「不引入哈希」的模块 doc。此时**不接指令**，靠 `xiao-runtime` 单测验证。
   **这是核心**，后两次是接线与收尾。
2. **指令与检查接线**：两条 `TacOp` 与编码标签（按 8 处清单逐处改）、`step` 两个 arm、
   四个 RuntimeCheck（含 `set_membership` 的缩小口径）、穷尽性守卫测试。
3. **向量与文档**：`tests/spec/09-bytecode/sets.json` 共享向量、区分度用例、第七节的连带影响。

**若中途必须停**：第 1 次提交本身完整可验证。

---

## 验收（已通过）

沿用 09R2D 的「撤掉实现 → 用例必须失败 → 还原 → 通过」。本批区分度实验已完成，**关键验收
不是只看一次测试通过**：

### 1. 语义向量（`sets.json`，三载体共用一份期望，不改任何机型的期望值）

- 四种代数各一条，断言**结果值与释放序列**。
- **六种比较**，其中两条是**前后对照锚点**：`{1} == {1}` 必须 **true**（修复前为 false）、
  `{1,2} < {1,2}` 必须 **false**。
- 空集方向性：`set() <= {1}` true、`{1} <= set()` false、`set() < set()` false、`set() < {1}` true。
- 重复元素去重：`{1,1,2} + {2,2,3}` 的 `len` 必须是 3。
- **异构集合**：`{1,"a"} & {1,true}` → `{1}`；`{true} == {1}` false。
- 动态边界错误向量（须经**形参**构造，字面量会被类型层常量折叠拒绝），断言错误码。
- `x in s` / `x not in s` 命中与不命中各一条。

### 2. 区分度（四组撤掉实验均已实做并通过）

- 把 `check_value` 的集合分支改成恒真 → 动态边界错误向量已验证会失败。
- 把 `lower_binary` 改成半消费 → `(a + b)[0, 1]` 形状的用例已验证会失败（见第六节末）。
- `jump_targets(&SetOp{..})` 与 `jump_targets(&SetCompare{..})` **断言为空**——
  这不是防漏改（没有边可漏），而是**钉死「集合运算没有控制流边」这个前提本身**：
  将来有人给 `SetOp` 加 `on_failure` 边，这条会立刻失败并强制他去改 `cfg.rs`。
- 一个使用集合运算的程序 `encode()` 已验证返回 `Ok`（同时保留反向拒绝路径）。

### 3. 穷尽性守卫

遍历 `RuntimeCheckKind` 全部 14 个变体，断言「要么在降低白名单里、要么在显式未支持清单里」。

### 4. 门禁全绿

`cargo test --workspace`、`cargo clippy --workspace --all-targets -- -D warnings`、
`cargo fmt --all -- --check`、`bun test`、`bun run check`、`bun run check:coverage`。

---

## 已同步的连带影响

1. **`docs/UseDocs/language/collections/sets.md` 与 `errors.md` 已同步 09R2F1**，正文区分
   研究 VM 的集合执行与生产命令边界，`module`/`stage` 已更新。
2. `tests/spec/09-bytecode/README.md` 已登记 59 条向量（集合 28 条），并保留未进向量的
   容器越界/键缺失由 ops 单测覆盖的说明口径。
3. `docs/module-registry.json` 已为 `rust.xiao-bytecode-research` 与
   `rust.xiao-vm-research` 的 `tests` 数组登记 `sets.json`；`rust.xiao-runtime` 覆盖
   `src/containers`，因此 `set.rs` 的公共方法同步更新了集合 UseDocs 段与
   `objects-and-handles.md`。
4. opcode 计数已同步到三处：`xiao-bytecode/src/research/README.md`、
   `09r-bytecode-machine-research.md:522/:536`、`encode/README.md`。
5. **顺手修（零风险）**：
   - `xiao-vm/src/research/machine/README.md` 已改为记录三种机型，不再声称只有
     `stack.rs`。
   - `00-decisions.md` 已按 `containers/mod.rs` 与类型层的实现口径改为「元组当前不可哈希」，
     并保留递归哈希证明作为后续债项。
6. `09r-bytecode-machine-research.md` 已在 `09R2b` 之后登记本批；R3 退出条件文本保持不变，
   因为它描述的是验收条件而不是当前事实。

---

## 不要重复做的事

- **不要改 09R2D/R2B 已冻结的语义**：三机型分工、`stack_map_entries` 口径、
  选择器计划格式一律不动。
- **不要为实现集合代数引入哈希索引**（理由见第三节）。
- **不要用 `record_unsupported` 给 `set_membership` 的类型兼容半记账**（它会堵死编码）。
- **不要把集合的增删与跨后端 lowering 一起做了**——那是 C2-D 的其余部分。
- **不要给三种载体各加方法**。

---

## 后续批次的约束传递

- **09R2G（`for` 与迭代）**：必须继承第三节的**结果顺序契约**，且要接手
  `set_membership` 的**类型兼容半**。`for` 需要索引来自寄存器，会改动
  `PathStep::Index(i128)` 这个已冻结语义的边界——**必须在 09R3 冻结指令编码之前完成**。
- **09R2H（`table` 声明）**：若只在 `TacProgram.functions` 里追加条目并追加三条 opcode，
  可在 R3 之后做（纯追加不破坏格式）；**若需新增程序级区段，则必须提前**，
  否则等于换 `FORMAT_VERSION` 重新冻结。

---

## 相关页面

- [09R2D. 两种机型与指令编码器交接文档](09r2d-machines-and-encoder.md)
- [09R2B. 选择器全量执行交接文档](09r2b-selector-execution.md)
- [03E. C2-C 集合运算静态闭环](03e-c2c-set-operations.md)
- [09R. 字节码寄存器机型特别研究](09r-bytecode-machine-research.md)
- [09. 字节码运行模式](09-bytecode-runtime.md)
- [集合](../UseDocs/language/collections/README.md)
