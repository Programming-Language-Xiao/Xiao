# 09R2G. `for` 与迭代执行闭环交接文档

> **本文是待执行的交接文档。** `for` 在前端是**静态闭环**（类型检查与生命周期逃逸分析
> 都已实现，且复用同一个元素类型推导），但降低器把它整条记入 `unsupported`
> （`lower/stmt.rs:129`），所以它**不可执行、也无法编码**。
>
> 本批补齐迭代，并把它排在 09R3 冻结指令编码**之前**——理由见第一节。

## Agent 交接上下文

### 接手前提

1. [09R2F. 集合运算执行闭环交接文档](09r2f-set-operations.md) —— **第三节的顺序契约、
   第六节的三处翻车点**直接适用于本批；其中的检查接线手法（白名单 + `check_value` +
   `runtime_check_code` 三处）本批照用。
2. [09R2F1. 集合运算续交接文档](09r2f1-set-operations-continuation.md) —— **本批的前置**。
   它列出的七个问题要**先修完**，否则本批会在同一批未验证的管线上继续叠加。
3. [09R2D. 两种机型与指令编码器交接文档](09r2d-machines-and-encoder.md) —— 开发规定主表；
   **新增 `TacOp` 的 8 处改动点清单**在 `:373-399`，本批要再走一遍。
4. [09. 字节码运行模式](09-bytecode-runtime.md) —— 09 阶段最小执行骨架的边界。

### 本批交付与不负责

**交付**：两条基础指令与编码标签、`for` 的 TAC 降低、`iterable` 运行时检查、
共享向量与区分度用例、文档同步。

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
TacOp::Len             { source: VReg }          → dst 是 int
TacOp::IndexGetDynamic { source: VReg, index: VReg } → dst 是元素
```

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

### 降低形状（CFG 抄 `lower_while`，`lower/stmt.rs:438`）

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
| `lower/mod.rs:738` 的白名单 | 加 `iterable` |
| `semantics/exec.rs:1312` `check_value`（兜底在 `:1342`） | 加 `iterable` 分支；**不加就静默恒真** |
| `semantics/exec.rs:1287` `runtime_check_code`（兜底在 `:1295`） | 加分支；不加就没有稳定码 |

错误码：**`X06-RUNTIME-024`**（`021`–`023` 已被 09R2F 的集合三类占用），
`message_id` 走 `runtime.*` 命名空间，错误类型名走 `TypeError`。

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
3. **临时容器的释放时机**：`for x in <表达式>` 的临时值要在循环结束后释放**一次**，
   而不是每轮一次。释放计划挂在错误的块上会让引用计数在循环里被反复减。

---

## 七、测试与区分度

### 7.1 共享向量（`tests/spec/09-bytecode/iteration.json`，三载体共用一份期望）

按 09R2B1 的写法：断言 `RunOutcome::value` 的**实际值**，不是 `is_success()`；
错误码引用常量而非字面量。

覆盖：数组、元组、字符串（**含多字节字符**，验证码点单位）、集合、字典列各一条；
`break` / `continue` 各一条；**空容器**（循环体零次）；嵌套 `for`；
动态 iterable 的错误码；循环体里对循环变量的重新赋值不影响下一轮的绑定。

### 7.2 区分度（每组都要实做一次撤掉实验）

- 撤掉 `IndexGetDynamic` 的动态索引（退回只认字面索引）→ **必须失败**。
- 把 `BranchIf` 的条件方向写反 → **必须失败**（能抓出死循环或空循环）。
- 把 `str` 的迭代单位改成字节长度 → **多字节字符那条必须失败**。
- 撤掉 `check_value` 的 `iterable` 分支 → 动态边界错误向量**必须失败**。

### 7.3 穷尽性守卫

把 `iterable` 从 09R2F 建的那份「显式未支持清单」里**移出**——那份测试断言
「每个 kind 要么在降低白名单里、要么在显式未支持清单里」，本批接通后它应当落在前者。

---

## 硬性约束

门禁、区分度验证、工具规定、单一来源原则、解耦约束**全部沿用 09R2D 文档第二章**。

1. **不要给三种载体各加方法**：指令由 `step` 统一实现，载体只管值放在哪里。
2. **新增的每个 `pub` 项都要有文档注释**（公共 API 100%、全仓 ≥90% 是硬门槛）。
3. **新增 `TacOp` 必须走完 09R2D 的 8 处清单**，其中 `decode_op` 与 `cfg.rs::jump_targets`
   编译器不强制。本批的 `Len` 没有控制流边，但 `IndexGetDynamic` 也没有——**仍要逐处改**，
   不要因为"没有边"就跳过 `jump_targets`。
4. **不要改 09R2F 已定的集合顺序契约**。

## 提交切分

1. **两条指令与接线**：`Len` / `IndexGetDynamic` + 8 处清单 + 编码标签 + 往返用例。
   此时降低器还不发射它们，靠编码往返与手工 TAC 验证。
2. **`for` 的降低与 `iterable` 检查**：CFG、绑定、`break`/`continue` 接出口、
   三处检查接线、错误码 `024`。
3. **向量与文档**：`iteration.json`、区分度四组、穷尽性守卫的迁移、连带文档。

**若中途必须停**：第 1 次提交本身完整可验证。

## 连带影响（必须同批）

- `tests/spec/09-bytecode/README.md` 的向量计数与清单。
- `docs/module-registry.json` 里 `rust.xiao-bytecode-research` 与 `rust.xiao-vm-research`
  的 `tests` 数组要加 `iteration.json`。
- **opcode 计数的同一批三处文档**（`xiao-bytecode/README.md`、
  `research/README.md`、`09r-bytecode-machine-research.md`）——09R2F 已经因为漏改它们
  被记过一次，本批不要重演。
- `docs/UseDocs/language/control-flow/conditions-and-loops.md:52` 的可迭代清单
  （第二节的不一致之一）。
- **顺手修（零风险）**：`xiao-vm/src/research/machine/README.md` 仍写「当前只有 `stack.rs`」。

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
