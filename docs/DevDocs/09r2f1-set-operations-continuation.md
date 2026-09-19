# 09R2F1. 集合运算执行闭环续交接文档

> 本文记录 09R2F 接续批次的审查问题、实现决策和验证证据。七个交接问题、四类
> RuntimeCheck、集合指令执行和共享向量均已完成；文末保留后续阶段的明确边界。
>
> 当前状态：集合代数、比较和成员判断已在三种研究 VM 载体上真实执行；四类检查已接入
> 降低器和语义核，含集合运算的 TAC 可在 LEB128 与定宽 `u16` 下编码。显式集合成员
> 类型的动态兼容半、集合增删和迭代仍不属于本批。

## Agent 交接上下文

### 接手前提

1. [09R2F. 集合运算执行闭环交接文档](09r2f-set-operations.md) —— **本批的总设计与理由在此**，
   第三节的顺序契约、第四节的检查设计、第六节的三处翻车点一律沿用，本文不复制。
2. [09R2D. 两种机型与指令编码器交接文档](09r2d-machines-and-encoder.md) —— 开发规定主表；
   新增 `TacOp` 的 8 处改动点清单在 `:373-399`（本批已按它走过一遍）。
3. [03E. C2-C 集合运算静态闭环](03e-c2c-set-operations.md) —— 本批要执行的语义就是它冻结的。
4. [09R2B1. 选择器执行验证缺口修复](09r2b1-selector-verification.md) ——
   **结果值可观察性**（`RunOutcome::value`）与共享向量的写法，本批的向量要照它写。

### 已落地（可依赖的事实）

| 提交 | 内容 |
| --- | --- |
| `17d36b1` | `SetHandle` 的四种代数与六种关系判定 + 6 个单测；模块 doc 补了顺序契约与「不得引入哈希索引」 |
| `68199e9` | `TacOp::SetOp`(34)/`SetCompare`(35) 两条指令与 8 处接线；三个新错误码 |

**已实测的结论**（接手时不必重做）：

- `SetHandle::equals` 是**无序双向包含**；把实现退回 `same_object`（即修复前的行为）
  会让 `set_equality_is_unordered_mutual_inclusion` 立刻变红——区分度已验证。
- 往返测试在 LEB128 与定宽 `u16` 两种宽度下都过；`all_ops_program` 的计数守卫已更新到 36。
- 集合代数与比较**没有控制流边**，`cfg.rs::jump_targets` 不需要新分支。
- Windows 原生与 Linux 容器（`Dockerfile.dev`）两侧门禁都是绿的。

### 交接前未落地点（本批均已关闭）

四个 RuntimeCheck 的接线、共享向量、区分度用例、连带文档。详见下文。

---

## 一、交接前查出的七个问题（本批已全部关闭）

前四条来自原作者的实现后自审，后三条来自交付前的独立复核。两类都要修，
**第 5 条（新指令零运行时用例）优先级最高**——它是本仓已被记录过一次的病。

### 1. 三个新错误码没有经 `xiao-runtime` 重导出

`SET_OPERATION_CODE` / `SET_COMPARISON_CODE` / `SET_MEMBERSHIP_CODE` 定义在
`xiao-diagnostics/src/lib.rs` 的 crate 根，但**没有加进 `xiao-runtime/src/lib.rs:23` 的
重导出列表**（`CONTAINER_HASHABILITY_CODE` 等都在那里）。

后果：共享向量与测试**只能写字面量错误码**，而这正是 09R2B1 专门清理过的漂移形态
（`e900f3b` 把 18 个测试文件从字面量改成常量引用）。必须同批补上。

### 2. 三处文档仍写「34 个 opcode」

| 文件 | 位置 |
| --- | --- |
| `core/rust/crates/xiao-bytecode/README.md` | `:20`「提供 34 个稳定 opcode」 |
| `core/rust/crates/xiao-bytecode/src/research/README.md` | `:10`「34 个稳定 opcode」 |
| `docs/DevDocs/09r-bytecode-machine-research.md` | `:536`「当前覆盖 34 个 `TacOp` 变体」 |

现在是 36。三处的措辞各不相同，改的时候要**逐处对照上下文**，不要机械替换。

### 3. `apply_set_op` / `apply_set_compare` 没有单测

`xiao-runtime` 层有 6 个 `SetHandle` 单测，但 **ops 层没有**。ops 层多出来的是两件事：
非集合操作数的**错误路径**（三个新错误码各自在什么条件下报）与 `Member` 的**方向**
（`x in s` 里左操作数是被包含的一方，写反了在 `SetHandle` 单测里看不出来）。

### 4. 09R2F 文档里的命名与实现不符

09R2F 第五节写的是 `Contains` / `NotContains`，实现改用了 `Member` / `NotMember`——
原命名读起来是「左包含右」，而 `x in s` 里左操作数是被包含的一方，方向正好相反。
**实现是对的，文档要跟上**，并在其中写明改名理由。

### 5. 两条新指令**没有任何运行时用例**，而且降低器从不发射它们

`grep SetOp` 在 `core/rust/crates/{xiao-vm,xiao-bytecode}/tests/` 下命中 **0**。
`semantics/exec.rs` 里 `step` 的两个新分支**从来没有被执行过**；编码往返测试只覆盖
「编出来、解回去」，不覆盖「执行对不对」。

更完整的事实是：**降低器也从不发射它们**——`&`/`^` 仍走 `lower_unsupported`
（`lower/expr.rs:357`），`+`/`-` 仍映射到 `Arith`。所以 `68199e9` 那批的准确定性是
**管线接好了，但没通电，也没人测过通电后的样子**。

这正是 09R2B1 存在的理由（「跑得通，但没人知道对不对」）。修复成本不高：照
`r2_selector_values.rs` 的手工 TAC 范式加一条运行时用例即可，**不需要等 RuntimeCheck
接通**——手工 TAC 直接构造 `SetOp` 就能跑到那两个分支。

### 6. 8 个变体只覆盖了 1 个，而往返测试**抓不到标签互换**

`SetCompareOp` 有 8 个变体，往返测试只用了 `ProperSubset` 一个。

要紧的是：**枚举声明顺序与标签表顺序是两处必须互相一致的地方**，而往返测试
**用同一张表正反走**——把 `Subset` 与 `ProperSubset` 的标签对调，往返照样通过。
而这两个变体恰恰是 09R2F 专门警告过的那对（`<` 是真子集、`<=` 是非真子集）。

仓内 `scalar_and_release_tags_have_one_bidirectional_mapping`
（`encode/tests.rs:621`）就是为这类问题存在的——它钉的是**显式数值**，不是「能转回来」。
给 `SetOpKind` 与 `SetCompareOp` 补同形的测试。

### 7. 用户可见的错误码表缺三个新码

`docs/UseDocs/language/collections/errors.md:55` 的错误码表列到 `X06-RUNTIME-017` 一带，
`021`–`023` 不在其中。交接时检查尚未接通，但 `apply_set_op` / `apply_set_compare`
已经能抛出这三个码（手工 TAC 可达）；本批已将它们接入语义核并补入用户错误码表。

---

## 二、已完成工作：RuntimeCheck 接线

### 2.1 三处接线

| # | 位置 | 改什么 |
| --- | --- | --- |
| 1 | `lower/mod.rs:738` 的白名单（`matches!` 列表到 `:745`） | 加 `set_operation`、`set_comparison`、`set_membership`、`set_hashability` 四个 kind |
| 2 | `semantics/exec.rs:1312` `check_value`，兜底 `_ => true` 在 **`:1342`** | 加四个分支；**不加就全部静默恒真** |
| 3 | `semantics/exec.rs:1287` `runtime_check_code`，兜底 `_ => None` 在 **`:1295`** | 加四个分支；不加就没有稳定错误码 |

`set_hashability` 可以**几乎免费**：元素的 `lower_expression` 走完通用尾巴时会用元素自己的
寄存器消费该 span 的检查，正好是 `is_hashable` 要的操作数——只需加上面三处，不必写发射逻辑。

### 2.2 两个操作数的检查发射

`set_operation` 与 `set_comparison` **必须检查两个操作数，不能只检查结果**：类型层允许
「一侧静态集合、另一侧 `Dynamic`」进入集合语义，运行时那个 `Dynamic` 完全可能不是集合；
而结果若成功必然是集合，对结果做检查近乎恒真。

需要一个新入口 `emit_runtime_checks_for(span, &[left_reg, right_reg])`，
并在 `lower_binary`（`lower/expr.rs:335`）的集合分支里用它。

> **必须把该 span 的检查取干净**（`emit_runtime_checks` 用 `remove`）。
> 半消费比不消费更糟：只对 left 发 `Check`、忘了 right，剩下的会被**选择器的范围清扫**
> 捞起来，用一个语义不相干的寄存器（选择器源容器）发 `Check{set_operation}`，
> 选择器源是数组/元组 → **检查失败，把合法程序打死**。这不是「少检查」，是「错检查」。

### 2.3 `set_membership` 缩小到「可哈希」半（**本批口径**）

09R2F 第四节已定：它的登记点里有一类的语义是「动态成员必须满足**声明的** `set<T>` 成员类型」，
而 `Check { kind, value }` **带不了期望成员类型**，这一半在本批的数据结构下无法表达。

本批口径：`value` 是 `Set` → 所有成员 `is_hashable`；否则 → `is_hashable(value)`。
类型兼容半记为具名债项交给 `09R2G`。

**绝不能用 `record_unsupported` 给它记账**——`encode/validate.rs:40` 对非空
`TacProgram.unsupported` **直接拒绝编码**，一记就把 09R3 四个指标之一的**编码体积**整条堵死，
而且要到 R3 才流血。

---

## 三、测试与区分度

### 3.1 共享向量（`tests/spec/09-bytecode/sets.json`，三载体共用一份期望）

按 09R2B1 的方式写：`run` 的返回值经 `RunOutcome::value` 断言，**不是** `is_success()`。
错误码引用常量，不写字面量（见问题 1）。

最终向量覆盖：四种代数各一条（断言结果值与释放序列）；**六种比较**，其中两条是前后对照锚点——
`{1} == {1}` 必须为 true（修复前是 false）、`{1,2} < {1,2}` 必须为 false；
空集方向性（`set() <= {1}` true 而 `{1} <= set()` false、`set() < set()` false）；
重复元素去重（`{1,1,2} + {2,2,3}` 的 `len` 为 3）；**异构集合**
（`{1,"a"} & {1,true}` → `{1}`，且 `{true} == {1}` 为 false）；动态边界错误码；
`in` / `not in` 命中与不命中。

动态边界必须**经形参**构造——字面量会被类型层常量折叠拒绝。

### 3.2 区分度（已实做并通过）

- 把 `check_value` 的集合分支改成恒真 → 动态边界错误向量**必须失败**（对应 2.1 第 2 条）。
- 把 `lower_binary` 改成半消费 → `(a + b)[0, 1]` 形状的用例**必须失败**（对应 2.2）。
- `jump_targets(&SetOp{..})` 与 `jump_targets(&SetCompare{..})` **断言为空**——
  这不是防漏改（没有边可漏），而是**钉死「集合运算没有控制流边」这个前提本身**：
  将来有人给 `SetOp` 加 `on_failure` 边，这条会立刻失败并强制他去改 `cfg.rs`。
- 一个使用集合运算的程序 `encode()` **必须 `Ok`**（现有测试只覆盖了反向的拒绝路径）。

### 3.3 穷尽性守卫（已通过）

遍历 `xiao_types::RuntimeCheckKind` 的**全部 14 个变体**，经 `runtime_check_kind_name`
转名后断言：「要么在降低白名单里、要么在一份**显式列出的未支持清单**里」。
以后新增 kind 必须二选一，无法静默漏。这是对 2.1 第 1 条那个「字符串匹配、编译期零联系」
的白名单的防护——写错一个字母会掉进 `record_unsupported`。

## 四、完成证据与后续债项

- `cargo test --workspace`、`cargo clippy --workspace --all-targets -- -D warnings`、
  `cargo fmt --all -- --check` 已通过；`r2_tac`、`r2_set_values` 和 `r2_vectors` 分别
  覆盖降低、手工 TAC 执行和 59 条三机型共享向量。
- `sets.json` 的动态运算/比较/成员错误向量经入口形参注入数组，能够区分检查分支是否
  仍然生效；集合指令编码在 LEB128 与定宽 `u16` 下均通过。
- `SetOp`/`SetCompare` 不产生 CFG 跳转边，集合结果顺序只依赖操作数顺序；实现没有引入
  哈希索引。
- `set_membership` 当前只消费可哈希半：集合值检查全部元素可哈希，其他值检查自身
  可哈希性。声明 `set<T>` 的动态成员类型兼容半是后续债项，不能在本批伪装成已完成。
- 集合增删、`for`/迭代、表声明、正式生产字节码和 `xiao run` 不在 09R2F1 范围内。

---

## 硬性约束

门禁、区分度验证、工具规定、单一来源原则、解耦约束**全部沿用 09R2D 文档第二章**。

1. **不要给三种载体各加方法**：指令由 `step` 统一实现，载体只管值放在哪里。
2. **新增的每个 `pub` 项都要有文档注释**（公共 API 100%、全仓 ≥90% 是硬门槛）。
3. **改动 `xiao-runtime/src/containers/set.rs` 会触碰一个 `verified` 模块**，
   其 UseDocs 与模块登记要同批复核。
4. **保持 09R2F 第三节的顺序契约**：代数结果顺序只依赖操作数，不依赖哈希也不依赖排序。

## 原计划提交切分（已执行）

1. **修交接前查出的七个问题**（第一节）：重导出三个错误码、三处 opcode 计数、ops 层单测、
   09R2F 文档改名、**新指令的运行时用例**、**两组标签表的显式数值双向测试**、
   错误码表补三个新码。这一步都很小且独立，**先做完再动检查**。
2. **RuntimeCheck 接线**：2.1 三处 + 2.2 的发射入口 + 2.3 的口径。
3. **向量与文档**：`sets.json`、区分度四组、穷尽性守卫、连带文档（见下）。

## 连带影响（已同步）

- `tests/spec/09-bytecode/README.md` 的向量计数与清单（现为 59 条，集合 28 条）。
- `docs/module-registry.json` 已在 `rust.xiao-bytecode-research` 与 `rust.xiao-vm-research`
  的 `tests` 数组登记 `sets.json`。
- `docs/UseDocs/language/collections/sets.md` 是 `status: verified`，正文写着
  「Runtime 尚未创建或修改真实集合对象」——本批已改为研究 VM 执行边界，`module`/`stage`/正文同批同步；
  `collections/errors.md` 同理。
- **顺手修（零风险）**：`xiao-vm/src/research/machine/README.md` 已改为记录三种机型；
  `00-decisions.md` 已按实现与类型层口径说明**元组当前不可哈希**，递归哈希证明保留为后续债项。

## 不要重复做的事

- **不要重做指令接线**：两条指令与 8 处改动点已经走完并通过往返测试。
- **不要改 09R2F 已定的顺序契约与检查口径**。
- **不要用 `record_unsupported` 给 `set_membership` 的类型兼容半记账**。
- **不要为实现集合代数引入哈希索引**。
- **不要把集合的增删与跨后端 lowering 一起做了**——那是 C2-D 的其余部分。

## 相关页面

- [09R2F. 集合运算执行闭环交接文档](09r2f-set-operations.md)
- [09R2D. 两种机型与指令编码器交接文档](09r2d-machines-and-encoder.md)
- [09R2B1. 选择器执行验证缺口修复](09r2b1-selector-verification.md)
- [03E. C2-C 集合运算静态闭环](03e-c2c-set-operations.md)
