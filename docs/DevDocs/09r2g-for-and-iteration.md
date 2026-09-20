# 09R2G. `for` 与迭代执行闭环交接文档

> **本文对应的执行批次已完成。** `for` 的类型检查、生命周期事实、TAC 降低和三种研究
> VM 载体已经闭环；`Len`/`IndexGetDynamic` 追加为 opcode 36/37，动态可迭代检查使用
> `X06-RUNTIME-024`。本文保留设计取舍、释放边界和验证证据，供后续表声明与 R3 冻结复用。

## Agent 交接上下文

### 接手前提

1. [09R2F. 集合运算执行闭环交接文档](09r2f-set-operations.md) —— **第三节的顺序契约、
   第六节的三处翻车点**直接适用于本批；其中的检查接线手法（白名单 + `check_value` +
   `runtime_check_code` 三处）本批照用。
2. [09R2F1. 集合运算续交接文档](09r2f1-set-operations-continuation.md) —— **本批的前置**。
   **已完成**（`a1a0938`），七个问题全部关掉、四类集合检查接通、59 条共享向量就位。
   它同时给本批留下了两样可直接复用的东西：`emit_runtime_checks_for` 那个
   **精确消费整个 span** 的发射入口，以及那条遍历全部 `RuntimeCheckKind` 的
   **穷尽性守卫测试**——本批接通 `iterable` 时要把它的名字从「显式未支持清单」移出。
3. [09R2D. 两种机型与指令编码器交接文档](09r2d-machines-and-encoder.md) —— 开发规定主表。
4. [09R2B. 选择器全量执行交接文档](09r2b-selector-execution.md) —— **新增 `TacOp` 的
   8 处改动点清单**在 `:373`（标题是「照单改，别凭记忆」），其中 `decode_op` 与
   `cfg.rs::jump_targets` 两处**编译器不强制**。本批要再走一遍。
5. [09. 字节码运行模式](09-bytecode-runtime.md) —— 09 阶段最小执行骨架的边界。

### 本批交付与不负责

**交付（已完成）**：两条基础指令与编码标签、`for` 的 TAC 降低、`iterable` 运行时检查、
共享向量、直接 VM 夹具、释放边界修复与文档同步。

**不负责**：表声明（`09R2H`）、集合的增删与跨后端 lowering、`0..n` 这类 range 语法
（见第二节末），以及 09R3 的基准设施。

---

## 一、为什么必须赶在 09R3 之前

09R3 结束后要冻结**指令编码和版本字段**。而 `for` 的落地需要**索引来自寄存器**，
这与现有设计直接冲突：

`PathStep::Index(i128)` 携带的是**字面常量**（`tac.rs`），全仓没有任何一处能让索引来自
寄存器；选择器侧同样如此（`SelectionModel` 的 `Index { raw: i128 }`）。这个限制编码在
`IndexGet` 的操作数格式里。

本批用**新增一条指令**的方式绕开它（不改 `PathStep` 的编码，见第三节），但**新增指令本身
就是指令集的一部分**。R3 之后再补，等于在已冻结的指令集上开新口子——那正是 09R2F 排在第
一位的同一理由。

---

## 二、语言语义：已经冻结的，不要重新设计

`docs/UseDocs/language/control-flow/conditions-and-loops.md:50` 明确规定：

> 数组、元组、集合、字典表和字典列可以作为已知可迭代对象。集合和字典表的遍历顺序不保证，
> 字典列保留其定义顺序。

**是迭代器式（`for x in container`），不是索引式。** 对应实现是
`xiao-types/src/control_checker.rs` 的 `iterable_element_type`，实际接受：
`str`、`array`（三种形态）、`tuple`、`set`、`DictTable`/`DictColumn`、`Dynamic`。

### 两处必须同批裁定的不一致

1. **`str` 可迭代，但文档清单没列**。代码（`control_checker.rs:281`）接受 `str`，
   且元素类型也是 `str`；而 `conditions-and-loops.md:52` 的清单里没有 `str`。
   **裁定其一并同批改**——不要留着一个「文档说不行、代码能做」的口子。
2. **`0..n` 这种 range 完全不存在**。类型系统里没有 `Type::Range`，
   `iterable_element_type` 也没有对应分支，`for i in 0..3` 会直接吃 `X04-TYPE-006`。
   **本批不要顺手加**——那是新增语法，不是补执行。

`break` / `continue` 已由类型层的 `loop_depth` 守住（`control_checker.rs:211-232`），
本批只需保证运行时的跳转把它们接到循环出口，不需要新增语法。

---

## 三、设计：两条基础指令 + 循环降低，**不是**一条 `IterNext`

```
TacOp::Len             { source: VReg }          → dst 是 int   （opcode 36）
TacOp::IndexGetDynamic { source: VReg, index: VReg } → dst 是元素（opcode 37）
```

编号**必须是 36 与 37**：34/35 已被 09R2F 的 `SetOp`/`SetCompare` 占用。这张表
**只能追加、不得重排**——`all_ops_program` 的计数守卫会拦住插在中间的新变体，
而重排会让旧字节被读成另一种指令，往返测试**看不出来**（编解码用同一张表）。

### 为什么不用一条 `IterNext`

一条「取下一个元素」的指令必须携带**游标状态**，而 `RuntimeValue` 没有迭代器变体；
引入它要连带处理 `PartialEq`、`Hash`、`is_hashable`、`type_name`、编码标签一整圈，
还要为不可哈希的迭代器定义新的相等语义。代价远大于收益。

两条基础指令的方案与仓内既有范式一致——**复合语义用多条基础指令降低**
（`try/finally` 用 `CallSub`/`RetFromSub`，`while` 用 `Jump`/`BranchIf` 拼 CFG）。

### `IndexGetDynamic` 必须复用既有索引语义，不要重新实现

`ops.rs` 的 `index_step` 已经处理了数组/元组/`str`/字典表/字典列的索引，
包括**负索引**与 **`str` 的 Unicode 码点单位**（选择器已把这条冻结在 `00-decisions.md` 的
P1 项里）。`IndexGetDynamic` 应当把**动态索引解析成同一个 `raw` 后走同一条路径**，
而不是另写一份。另写一份就会让「`x[-1]` 是什么」在两个地方各有一套答案。

### 降低形状（CFG 抄 `lower_while`，`lower/stmt.rs:440`）

```text
src  = <iterable 表达式>
len  = Len(src)
i    = 0
header:  cond = i < len
         BranchIf(cond, body, exit)
body:    item = IndexGetDynamic(src, i)
         <绑定 target>
         <循环体>
         i = i + 1
         Jump(header)
exit:
```

`i < len` 用现有 `Compare`，`i + 1` 用现有 `Arith`——**除两条新指令外不需要更多指令**。
游标 `i` 是隐藏临时值，走正常的活跃区间分配；`src` 若是表达式（如 `for x in (a + b)`），
它的临时容器释放要挂在**循环出口之后**，不能每轮触发一次。

---

## 四、`iterable` 运行时检查

动态 iterable 必须在运行期确认可迭代。三处照 09R2F 的手法：

| 位置 | 改什么 |
| --- | --- |
| `lower/mod.rs` 的白名单（`matches!` 列表，当前 `:761-770`） | 加 `iterable` |
| `semantics/exec.rs:1329` `check_value`（兜底 `_ => true` 在 **`:1367`**） | 加 `iterable` 分支；**不加就静默恒真** |
| `semantics/exec.rs:1300` `runtime_check_code`（兜底 `_ => None` 在 **`:1312`**） | 加分支；不加就没有稳定码 |

> **行号随每次接线下移**：09R2F 落地后它下移过 12 行，09R2F1 落地后又下移了更多。
> 上面的数字是本文定稿时实测的；接手时**先按函数名定位，不要按行号跳转**。

错误码：**`X06-RUNTIME-024`**（`021`–`023` 已被 09R2F 的集合三类占用），
`message_id` 走 `runtime.*` 命名空间，错误类型名走 `TypeError`。

### 4.1 本批同时认领的债：`set_membership` 的**类型兼容半**

09R2F 与 09R2F1 **两份文档都把这一笔交给 09R2G**，而 09R2G 原文没有认领它——
**债不能掉在地板缝里**，所以在这里正式接下。

现状（`semantics/exec.rs` 的 `check_value`）：`set_membership` 只覆盖**可哈希半**
（值是集合则查全部成员可哈希，否则查自身）。缺的一半是：

> 动态成员必须满足**声明的** `set<T>` 成员类型。

`Check { kind, value }` 只带一个字符串 kind 和一个寄存器，**带不了期望成员类型**。
本批已把 `Check` 扩为 `expected: Option<IrType>`，由类型层的 `RuntimeCheck.expected`
经 IR 原样镜像到 TAC，再由 VM 的 `check_set_membership` 消费。选择可选类型载荷而不是
新增指令，是因为它只改变检查的声明边界，不改变控制流或值计算；新增指令会重复现有
`Check` 的失败边和错误路由，也无法复用其他检查的统一编码/验证路径。

**绝不能用 `record_unsupported` 给它记账**——`encode/validate.rs:40` 对非空
`TacProgram.unsupported` **直接拒绝编码**，一记就把 09R3 的**编码体积**指标整条堵死，
而且要到 R3 才流血。

---

## 五、顺序契约：跨批次的硬约束

语言**不承诺**集合与字典表的遍历顺序（第二节的引文）。但本批一旦把它们写进共享向量，
**物理顺序就变成了可观察期望值**——那不是缺陷，是契约，而且是**单向**的。

- 集合的物理顺序由 `SetHandle` 的结果顺序契约决定（09R2F 第三节：只依赖操作数，
  不依赖哈希、不依赖排序）。
- **不许为了让 `for` 的输出好看而改那个顺序。** 改了要连带改向量，那是掩盖而不是修复。
  09R2F 第三节已把这条写进了 `set.rs` 的模块 doc。

字典表与字典列同理：顺序来自构造顺序，不要引入排序。

---

## 六、最可能翻车的前三处

1. **顺序**（第五节）。最容易的做法是"顺手排个序让输出稳定"——那会新增一个待冻结契约，
   并且与集合侧的顺序契约打架。
2. **空容器与边界**：`len == 0` 时循环体必须**一次都不进**（`BranchIf` 的条件方向写反会
   变成死循环或越界）。`str` 的迭代单位是**码点**，不是字节——直接用字节长度会让多字节
   字符被切碎。
3. **临时容器的释放时机**：`for x in <表达式>` 的来源只求值一次，临时来源在循环
   `exit` 块释放；循环体的元素若是堆句柄，则由循环体作用域的正常/`break`/`continue`
   计划按轮次释放。标量循环变量不会因跨作用域读取而被错误提升为堆值。

---

## 七、测试与区分度

### 7.1 共享向量（`tests/spec/09-bytecode/iteration.json`，三载体共用一份期望）

按 09R2B1 的写法：断言 `RunOutcome::value` 的**实际值**，不是 `is_success()`；
错误码引用常量而非字面量。

覆盖：数组、元组、字符串（**含多字节字符**，验证码点单位）、集合、字典表、字典列各一条；
`break` / `continue` 各一条；**空容器**（循环体零次）；嵌套 `for`；
动态 iterable 的错误码；循环体里对循环变量的重新赋值不影响下一轮的绑定。另有
`xiao-vm/tests/r2_iteration_values.rs` 手工 TAC 夹具，直接覆盖三种载体的两条新指令。

### 7.2 区分度（每组都要实做一次撤掉实验）

- 撤掉 `IndexGetDynamic` 的动态索引（退回只认字面索引）→ **必须失败**。
- 把 `BranchIf` 的条件方向写反 → **必须失败**（能抓出死循环或空循环）。
- 把 `str` 的迭代单位改成字节长度 → **多字节字符那条必须失败**。
- 撤掉 `check_value` 的 `iterable` 分支 → 动态边界错误向量**必须失败**。

### 7.3 穷尽性守卫

把 `iterable` 从 09R2F 建的那份「显式未支持清单」里**移出**——那份测试断言
「每个 kind 要么在降低白名单里、要么在显式未支持清单里」，本批接通后它应当落在前者。

## 八、完成证据

- `TacOp::Len`/`IndexGetDynamic` 使用 opcode 36/37，已完成 TAC 定义、LEB128/定宽
  `u16` 编解码、标签、引用验证、活跃分析、CFG 目标遍历和 VM `step` 接线。
- `lower_for` 生成来源求值、`Len`、游标比较、`BranchIf`、动态索引、绑定、更新桥和
  出口块；`continue` 指向更新桥，空容器不会进入循环体。
- `Len` 覆盖数组、元组、字符串、集合、字典表和字典列；字符串按 Unicode 码点计数。
  `IndexGetDynamic` 复用统一 `index_step`，保留负索引和集合/字典表物理顺序。
- `iteration.json` 由栈式、分类型寄存器式和混合式载体复用；`r2_iteration_values.rs`
  直接构造 TAC 验证两条新指令。
- 动态 iterable 失败稳定报告 `X06-RUNTIME-024`；`set_membership` 的声明类型载荷
  通过 `Check.expected` 传递，没有新增平行检查指令；可哈希但不符合 `set<int>` 的动态
  字符串由三载体共享向量锁定为 `X06-RUNTIME-023`。
- `Check.expected` 为既有 opcode 25 增加存在位与类型载荷，研究编码布局版本已从 1
  升至 2；解码器会在读取函数体前拒绝版本 1 字节，避免按新布局错读旧 `Check`。
- 四组受控撤回均被测试捕获：动态索引固定为字面 `0` 时
  `dynamic_index_executes_on_all_carriers` 得到 `Int(1)` 而非 `Int(2)`；对调 `for`
  的 `BranchIf` 目标时 `iteration_vectors_are_stable` 的非空循环结果归零，空容器触发
  `X06-RUNTIME-014`；字符串长度改用 UTF-8 字节数时 `len_executes_on_all_carriers`
  得到 `7` 而非 `3`，且 `unicode-code-point-iteration` 触发越界；`iterable` 检查恒真时
  `dynamic-iterable-error` 从预期 `X06-RUNTIME-024` 退化为 `X06-RUNTIME-002`。

---

## 硬性约束

门禁、区分度验证、工具规定、单一来源原则、解耦约束**全部沿用 09R2D 文档第二章**。

1. **不要给三种载体各加方法**：指令由 `step` 统一实现，载体只管值放在哪里。
2. **新增的每个 `pub` 项都要有文档注释**（公共 API 100%、全仓 ≥90% 是硬门槛）。
3. **新增 `TacOp` 必须走完 09R2D 的 8 处清单**，其中 `decode_op` 与 `cfg.rs::jump_targets`
   编译器不强制。本批的 `Len` 没有控制流边，但 `IndexGetDynamic` 也没有——**仍要逐处改**，
   不要因为"没有边"就跳过 `jump_targets`。
4. **不要改 09R2F 已定的集合顺序契约**。

## 原计划提交切分（已完成）

1. **两条指令与接线**：`Len` / `IndexGetDynamic` + 8 处清单 + 编码标签 + 往返用例。
   此时降低器还不发射它们，靠编码往返与手工 TAC 验证。
2. **`for` 的降低与 `iterable` 检查**：CFG、绑定、`break`/`continue` 接出口、
   三处检查接线、错误码 `024`。
3. **向量与文档**：`iteration.json`、区分度四组、穷尽性守卫的迁移、连带文档。

三步均已在同一工作批次完成；`cargo test --workspace`、全目标 Clippy、格式检查、
`bun test`、仓库检查和文档覆盖率门禁均已通过，本交接记录随实现一并提交。

## 连带影响（必须同批）

- `tests/spec/09-bytecode/README.md` 的向量计数与清单（本批共 73 条共享向量）。
- `docs/module-registry.json` 里 `rust.xiao-bytecode-research` 与 `rust.xiao-vm-research`
  的 `tests` 数组要加 `iteration.json`。
- **opcode 计数与布局版本的同一批文档**（`xiao-bytecode/README.md`、
  `research/README.md`、`09r-bytecode-machine-research.md`）——09R2F 已经因为漏改它们
  被记过一次，本批不要重演。
- `docs/UseDocs/language/control-flow/conditions-and-loops.md:52` 的可迭代清单
  （第二节的不一致之一）。
- **顺手修（零风险）**：`xiao-vm/src/research/machine/README.md` 已记录三种载体。

## 不要重复做的事

- **不要重做集合的指令与检查**：那是 09R2F/09R2F1 的账。
- **不要为了让 `for` 输出好看而改集合的结果顺序**（第五节）。
- **不要顺手加 `0..n` range 语法**（第二节）。
- **不要为迭代器新增 `RuntimeValue` 变体**（第三节）。
- **不要重写索引语义**：复用 `index_step` / `resolve_index`。

## 相关页面

- [09R2F. 集合运算执行闭环交接文档](09r2f-set-operations.md)
- [09R2F1. 集合运算执行闭环续交接文档](09r2f1-set-operations-continuation.md)
- [09R2D. 两种机型与指令编码器交接文档](09r2d-machines-and-encoder.md)
- [09. 字节码运行模式](09-bytecode-runtime.md)
- [条件与循环](../UseDocs/language/control-flow/conditions-and-loops.md)
