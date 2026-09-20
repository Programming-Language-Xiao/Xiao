# 09R2H. 表声明执行闭环交接文档（**方向稿**）

> **本文是方向稿。** 它定下本批的范围、入场条件与那个**可能阻塞 09R3 的问题**；
> 具体指令形状、函数表扩展方式与降低形状留给接手者设计，**设计定稿前不要动代码**。
>
> 现状：`table` 声明在前端与静态检查上是闭环的（类型、可见性、`new/init/drop` 契约），
> 但**运行期完全不可执行**——`LoadTable` 与 `TableInstance::` 在 `xiao-vm` 与
> `xiao-bytecode` 里**零命中**，没有任何 TAC 指令能造出表实例。

## Agent 交接上下文

### 接手前提

1. [09R2G. `for` 与迭代执行闭环交接文档](09r2g-for-and-iteration.md) —— **本批的前置**，
   已完成。它的 8 处接线手法、共享向量写法与区分度要求直接适用。
2. [09R2D. 两种机型与指令编码器交接文档](09r2d-machines-and-encoder.md) —— 开发规定主表。
   新增 `TacOp` 的 **8 处改动点清单**在 [09R2B](09r2b-selector-execution.md) `:373`
   （标题「照单改，别凭记忆」），其中 `decode_op` 与 `cfg.rs::jump_targets` **编译器不强制**。
3. [05C. 表语法与静态生命周期闭环交接记录](05c-table-static-closure.md) —— 本批要执行的
   语义就是它冻结的。
4. [09R3. 跨平台基准与冻结](09r3-benchmarks-and-freeze.md) —— **第二节第 5 条就是本文的
   那个问题**，先读完再动手。
5. [06B. Runtime 对象与表生命周期执行闭环](06b-runtime-objects-and-tables.md) ——
   表对象与状态机已经在 `xiao-runtime/src/tables/` 实现，本批是**接线**不是重写。

### 本批交付与不负责

**交付**：表声明的 TAC 降低与执行、表实例的构造/字段读写、共享向量与区分度、
文档同步。

**不负责**：`Result` 泛型、并发、正式 `.xiaoc`、09R3 的基准设施。**也不重写**
`xiao-runtime/src/tables/` 的既有状态机——它已交付且被 06B 的测试锁定。

---

## 一、那个**可能阻塞 09R3** 的问题（**动手前必须先回答**）

09R3 结束后要冻结七项，其中一项是**指令编码与版本字段**，其载体是 `TacProgram` 的
程序级布局。于是：

| 如果本批… | 那么 |
| --- | --- |
| **只追加指令与函数条目**（在 `TacProgram.functions` 里加表方法，追加 `LoadTable`/`MemberGet`/`MemberSet` 三条 opcode） | ✅ **纯追加，可以在 R3 之后做**——追加不破坏已冻结的格式 |
| **需要新增程序级区段**（例如 `table_definitions`） | ❌ **必须赶在 R3 之前**：那会改动 `TacProgram` 的编码布局，等于换 `FORMAT_VERSION` 重新冻结，本轮全部基准数字作废 |

**这是本批的第一个交付物，而且是一个判断，不是一段代码**：先确定走哪条路，
把结论写进本批交接文档，再动实现。

### 已知的相关事实

- `IrStatementKind::Table { name: IrName, table_kind: String, body: Vec<IrStatement> }`
  （`xiao-ir/src/model.rs:211-218`）——`table_kind` 是 `"singleton"`/`"instance"` 字符串。
- 表体的静态白名单是**闭合的**：`Assignment`/`Declaration`/`ConstDeclaration`/`Function`，
  其余一律 `X05-TYPE-015`。**表体内不能再出现 `for` 或嵌套 `table`**——这条影响测试取材。
- 表方法体是 `body` 里的**嵌套 `IrStatementKind::Function`**，而该变体在
  `lower/stmt.rs` 的程序级遍历里是**被忽略**的（函数只在程序级被收集成 `FuncId`）。
  **表方法需要一条不同的收集路径**——这是本批最可能低估工作量的一处。
- `xiao-runtime/src/tables/` 已有完整的 `TableDefinition`/`TableState`/`TableObject`/
  `TableInstance` 与状态机（`Allocated -> FieldsInitializing -> InitCompleted -> Usable
  -> Dropping -> Released`）。

---

## 二、需要设计并写进交接文档的

1. **指令形状**：`LoadTable`/`MemberGet`/`MemberSet` 三条是否够？字段访问要不要区分
   singleton 与 instance？方法调用走现有 `Call` 还是新指令？
2. **函数表扩展**：表方法如何进入 `TacProgram.functions` 并获得稳定 `FuncId`，
   以及它与程序级函数的编号关系（**编号一旦分配就要稳定**——参见 opcode 表的同款理由）。
3. **降低形状**：表声明是语句、不产生值。注意它与第 9 条变量的交互，以及
   表体内 `const` 字段与 `init` 的求值时机。
4. **释放**：表实例是堆对象，必须参与既有的确定性释放。**09R2G 刚修的释放边界问题
   就在这里附近**——标量/栈值逃逸不得被伪装成堆句柄，表实例则**确实**是堆句柄，
   两者的门控不能写反。
5. **共享向量与区分度**：至少覆盖 singleton 与 instance 的构造、字段读写、
   `init` 失败回滚、`drop` 只执行一次。

---

## 硬性约束

门禁、区分度验证、工具规定、单一来源原则、解耦约束**全部沿用 09R2D 文档第二章**。

### ★ 提交正文必须写「为什么」

这条单列出来，因为它在本仓**反复复发**，而最近一笔的代价可以量化。

**事实**：`c239948`（09R2G）**正文 0 行**，而它同时做了两件最需要解释的事——
把既有共享向量的期望值改成空（`control` 2→0、`errors` 7→6、`sets` 6→5），
以及把 `FORMAT_VERSION` 从 1 升到 2。

**代价**：审核者只能读 diff 反推。**审核者先误判成「改期望去迁就实现」**——
那是最典型的假绿形态——直到翻到 `xiao-lifetime/src/escape.rs` 里那五行
（`if value.storage.is_heap() && ...`）才纠正过来：旧向量断言的**正是那个 bug**，
改动是修复的正确后果。

**规则**：正文说明「为什么」。当改动属于下面三类之一时，正文**必须**写清：

1. **改了既有共享向量的期望值**——要说明旧期望断言的是什么错，或者为什么旧期望过时；
2. **升了 `FORMAT_VERSION`**——要说明是哪个指令/字段的布局变了；
3. **删除了断言**（把非空断言改成空、去掉字段）——要说明删掉它之后**还剩什么**能抓住回归。

**反过来说**：如果一笔提交同时做了上面两件，而正文是空的，审核者**没有义务**
替你还原意图——他会先按最坏假设看。

### 其他

- **新增的每个 `pub` 项都要有文档注释**（公共 API 100%、全仓 ≥90% 是硬门槛）。
- **改动 `xiao-runtime/src/tables/` 会触碰一个 `verified` 模块**，其 UseDocs 与模块登记
  要同批复核。
- **不要重写** `xiao-runtime/src/tables/` 的既有状态机。

## 验收

沿用 09R2D 的「撤掉实现 → 用例必须失败 → 还原 → 通过」。**关键验收不是「测试通过」**：

1. **第一节那个判断有明确结论**，并写清了依据；
2. 表实例能被构造、字段可读写、`init` 失败回滚、`drop` 恰好一次；
3. **释放行为有断言**——表实例是堆对象，向量必须锁定它的释放序列，
   不能只在标量路径上通过；
4. 门禁全绿：`cargo test --workspace`、clippy `-D warnings`、`fmt`、`bun test`、
   `bun run check`、`bun run check:coverage`。

## 不要重复做的事

- **不要重写表状态机**：06B 已交付并锁定。
- **不要顺手加表体内的 `for`/嵌套 `table`**：静态白名单是闭合的，那是新增语义。
- **不要把「表声明」与「字典表」混为一谈**：`NewDictTable` 是字典表**字面量**，
  与 `table` 声明是两回事。
- **不要重复 09R2G 的释放边界修复**：它已交付，直接复用那个门控。

## 相关页面

- [09R2G. `for` 与迭代执行闭环交接文档](09r2g-for-and-iteration.md)
- [09R2D. 两种机型与指令编码器交接文档](09r2d-machines-and-encoder.md)
- [09R3. 跨平台基准与冻结](09r3-benchmarks-and-freeze.md)
- [05C. 表语法与静态生命周期闭环交接记录](05c-table-static-closure.md)
- [06B. Runtime 对象与表生命周期执行闭环](06b-runtime-objects-and-tables.md)
