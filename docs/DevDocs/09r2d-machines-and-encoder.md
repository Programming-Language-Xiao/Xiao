# 09R2D. 两种机型与指令编码器交接文档

> 本文是 09R2 第四批（R2a 收尾）的实现交接文档。它把 09R2 从「只有栈式参考解释器」推进到
> **三种机型共用同一组语义向量**，并补上指令编码器与源码映射——这两块是 09R3 基准的入场券
> （性能门槛要三种机型对比，编码体积是四个门槛指标之一）。
>
> **接手 Agent 必须先读完「接手前提」与「上一阶段缺陷档案」再动代码。**
> 缺陷档案不是历史回顾：其中 C0 是**本批必须先修的阻断性缺陷**，A 类的病人已复发过一次。

## Agent 交接上下文

### 接手前提

1. [09R. 字节码寄存器机型特别研究](09r-bytecode-machine-research.md) —— 尤其 **R1-E/F/H**
   （三种机型与寄存器类别）、**R1-J**（溢出）、**R1-L/O**（调用约定与保存规则）、
   **R1-Y/Z/AA/AB**（编码、源码映射、版本字段、与 `.xiaoc` 的边界）、**R1-AC/AD**（门槛与基准协议）。
   这些是**已冻结的设计**，本批实现它们，**不得自行改设计**。
2. [09R2C. 异常控制流实现交接文档](09r2c-exception-control-flow.md) —— 上一批的施工图，
   以及它登记的两处与 07-B 的冲突。
3. [07. 错误模型与并发安全边界](07-concurrency-and-errors.md) —— 展开顺序
   `finally -> drop -> 匹配 catch / 继续传播`，清理错误进 `suppressed` 且不覆盖主错误。
4. [06. 内存与运行时语义](06-memory-and-runtime.md) —— 释放顺序的冻结规则。
5. 两个 research README：`core/rust/crates/xiao-bytecode/src/research/README.md`、
   `core/rust/crates/xiao-vm/src/research/README.md`。

### 当前进度

本节保留 `09R2C` 已落地、`0fb62b9` 登记的 R2D 初版交接基线；实现状态以本节末的
“R2D 交付快照”为准。09R2C 的实现提交为 `d733522`、`76e3533`、`ef5b149`，
其完整门禁已经通过。09R2 已交付四批：

| 批次 | 内容 | 提交 |
| --- | --- | --- |
| 批次 1 | TAC 模型与降低、对账验证器、栈式解释器、语义向量 | `51ceaac` `efe9789` `f035279` `a9b68ab` |
| 批次 2 | 容器运行时对象、精确索引、临时值释放 | `1f8d560` `a93fd3f` `885a278` `7dc998a` |
| 跨层审计 | 12 项缺陷 + 契约清理 | `7a67af3` `3f6aa94` `85a11d8` `3d1956f` `59a0317` `2bb5dc8` `ccba90a` `e900f3b` |
| 09R2C | 错误对象、handler 路由、finally 子程序、Raise/Check | `d733522` `76e3533` `ef5b149` |

**R2D 初版交接时的现状（历史基线）**：语义核仍只通过 `Carrier` 使用栈式载体，
`Carrier` 只有六个旧方法，`run.rs` 只有栈式入口，全仓没有编码器和物理 pc。该基线
已由下列交付快照完成，不应再当作当前状态。

09R2C 已经锁定、R2D 必须保持的执行契约如下：

- `RuntimeValue::Error` 保留错误身份；错误类型名由 `xiao-diagnostics` 的单一表提供，
  未知 `FooError` 不得静默通过，`FatalError` 不得被普通 `catch` 捕获。
- VM 先查当前帧 handler，未匹配才逐帧展开；`finally -> drop -> catch/传播` 的顺序、
  `suppressed` 归并和 Fatal 绕过 `catch/finally/drop` 的不对称都已冻结。
- `finally` 以 `CallSub`/`RetFromSub` 子程序执行；正常路径、异常路径以及
  `return`/`break`/`continue` 的覆盖性退出都只能执行一次，并且必须保留作用域释放计划。
- 降低器在生成 finally 子程序时会恢复仍活动的 try 作用域（`remember_scope` 语义），因此
  finally 内的覆盖性退出可以生成正确的 `return`/`break`/`continue` 释放计划；新载体不能
  把这类计划当成普通子程序局部清理而提前或重复执行。
- `Check` 当前真正启用 `boolean_condition`、`arithmetic`、`numeric_range`、
  `dynamic_conversion`；`string_boolean` 目前只预留稳定错误码但不会由降低器发出，选择器、
  集合、随机和迭代器检查继续进入 `TacProgram.unsupported`，
  **不能在 R2D 中把“解释器认识”误写成“降低器已启用”**。
- 共享向量仍是 27 条（`scalar` 6、`control` 4、`errors` 9、`containers` 8），
  `r2_stack.rs` 另有 40 条栈式回归测试；R2D 的三种机型必须复用这些期望，JSON 不改。
  **这组数字在本文、父文档与 `12-tests-and-milestones.md` 各出现一次，属于快照而非来源；
  准数一律以 `tests/spec/09-bytecode/*.json`、`r2_stack.rs` 与 `r2_tac.rs` 为准。**
  本仓的第一号病史就是「同一规则两处各写一份然后漂移」，不要把它变成第三份来源。

R2D 交付后，`TacFunction.categories` 已成为物理分配的正式输入，程序级类别只保留兼容
视图；错误堆栈从只读 pc 表取得物理偏移，映射缺失保留空后端位置并记录事件。仍未交付的
只有本批明确排除的 R2b 选择器全量、`for`/表声明、`Result` 泛型与正式 `.xiaoc` 格式。

**R2D 交付快照（2026-09-18）**：实现提交为 `64691b6`、`351cb12`、`9b7d887`、
`c3bd94c`、`2ac2f2e`、`af779e9`、`1a2b22f`、`b1bab53`、`1605282`。三种载体共用 27 条向量，
`r2_stack.rs` 当前为 42 条回归（其中 40 条为交接时的 R2C 基线，另含机型指标与 pc
映射反例）；编码器单元测试覆盖 31 个 opcode、两种操作数宽度和拒绝路径。H1 已用
`subroutine_faults` 栈化状态收敛；H2 去掉块级 `TacInstr` 克隆，交接基准为中位数
`253.66 ms -> 245.83 ms`（约 `3.1%`）。

### 本批交付与不负责

**交付**：`C0` 逐函数类别映射修复、`Carrier` 接口演进、活跃区间分析、分类型寄存器机型、
混合式机型、`VmMetrics` 机型中立化、三机型共用向量对拍、指令编码器（含两种操作数宽度变体）、
`pc -> IrSpan` 源码映射与接线、文档同步。

**不负责**：R2b 选择器全量（多选/区间/步长/随机）、`for` 与表声明、`Result` 泛型与 `?`
传播、正式 `.xiaoc` 格式
（分段、分区编号、内容寻址、Protobuf 索引归 14/16 阶段）、JIT/PGO/LTO、
**09R3 的基准测量本身**（本批只负责让三种机型可比）。

### 与 R2b 的排序结论：**R2D 先做**

R2B（选择器全量）与本批**无依赖**，但两者**都要改 `xiao-bytecode/src/research/`**
（本批加 `categories`、载体与编码器；R2b 加选择器操作数格式），所以**必须串行**。
排序是 **R2D 先**，理由三条：

1. **本仓自己的阶段划分把 R2a 排在 R2b 之前**，而本批是「R2a 的收尾批次」。
2. **三机型是 09R3 的唯一通路**：R3 的全部内容是「三种机型跑同一组向量、比出四项指标」，
   本批不做，R3 **一步都跑不动**。
3. **R2b 不影响 R3 能否运行，但影响其代表性**：R2 的阶段划分原文写明拆两半是为了
   「避免容器族跑不动导致 +10% 退化成**只在标量上测出来的数字**」。所以 R2b 必须在
   **R3 出报告之前**落地，只是不必排在本批之前。

**已知代价，一并登记（不要当成意外）**：本批的编码器按当前 **31 个 `TacOp` 变体**落地；
R2b 之后新增选择器操作数时，**编码器与三种载体都要跟着扩展**。
反过来先做 R2b 能让指令集先定型再写编码器，但会把 09R3 推后——这是明确的取舍，不是疏漏。

**本节是这条排序的单一来源**；R2b 文档只记录结论并链接到这里，**不复制理由**。

---

## 一、上一阶段缺陷档案

**这一节是本批最重要的部分。** 同类缺陷在上一阶段反复出现，其中一类在「专门修它的那一批」里
**又犯了一次**。以下每条都是已发生的事实，不是推测。

### 1.0 C0：本批必须先修的阻断性缺陷（截至 R2C 仍未修）

**`TacProgram.categories` 是程序级视图，但 `VReg` 编号是逐函数的，两者键空间不匹配。**

证据（全部已核对）：

- `lower/mod.rs:347` 每个函数降低前执行 `self.frame = Frame::default()`，而 `next_vreg` 是
  `Frame` 的字段（`lower/mod.rs:428-429`、`:444-445`）。**所以每个函数的 `VReg` 编号都从 0 重新开始。**
- `self.categories` 是 `Lowering` 的**单个**字段（`lower/mod.rs:119`，初始化于 `:188`，
  在 `finish()` 于 `:291` 整体交给 `TacProgram`）。`new_register`/`new_binding_register`
  在 `:430`/`:446` 往这**同一张表**里 `insert`。
- `CategoryMap::insert` 先 `resize(index + 1, RegisterClass::Poly)` 填空位，再
  `merge_class(self.classes[index], class)`。
- `merge_class` 开头是 **`if matches!(left, Poly) { return right; }`**。

后果**不是「保守退化」，而是非单调的错误收敛**。以「函数 A 的 VReg 5」为例：

| 步骤 | 槽位 5 | 函数 A 的 VReg 5 被读成 |
| --- | --- | --- |
| 函数 A 插入 `Int` | `merge_class(Poly, Int)` = `Int` | `Int` ✅ |
| 函数 B（自己的 VReg 5）插入 `ObjHandle` | `merge_class(Int, ObjHandle)` = `Poly` | `Poly` ❌ 应为 `Int` |
| 函数 C（自己的 VReg 5）插入 `Float` | `merge_class(Poly, Float)` = `Float` | `Float` ❌ 还是错 |

**函数 A 的类被别人改写，且改写方向不确定。** 若 `merge_class` 没有那条 `Poly` 短路，
最坏也只是全体退化成 `Poly`（保守但正确）；正是这条短路让它**报出一个具体而错误的类别**。

**今天为什么没爆**：没有任何生产代码消费 `TacProgram.categories`。唯一消费者是
`xiao-bytecode/tests/r2_tac.rs:108` 的 `assigns_register_classes`，而它用的是**单函数脚本**
（`"text = \"x\"\nnumber = 1\n"`）——**跨函数同编号冲突完全无覆盖**。
栈式机型不看类别，所以整条链路上没人碰过它。

**为什么本批必须先修**：寄存器机型的分配完全依赖类别。而信息在 `merge_class` 里**已经丢失**，
事后无法重建。

**接手决策门**：无论选择逐函数 `CategoryMap` 还是全程序唯一 `VReg`，分配器的正式输入都必须
是「当前 `TacFunction` 的局部编号空间 + 该函数的类别表」。寄存器载体不得继续读取一个
无法区分函数边界的 `TacProgram.categories`；如果保留程序级合并视图，只能用于诊断，不能用于
物理分配。

**两条可选修法**（建议 a，但请自行验证后再定）：

- **(a) 加 `TacFunction.categories`**：把 `categories` 从 `Lowering` 的全局字段改为
  **逐函数**收集，在 `finish()` 时随函数一起放进 `TacFunction`。`TacProgram.categories`
  可以保留为合并视图（供跨函数查询）或直接删除。改动面小，且不改变 `VReg` 身份。
- **(b) 让 `VReg` 编号全程序唯一**：`next_vreg` 提到 `Lowering` 层，不再随函数重置。
  这样程序级表天然正确，且每个 `VReg` 只被插入一次（`merge_class` 可能因此变成多余）。
  代价：编号空间变大，且 `VReg` 的「帧内局部性」含义被削弱。

**验收**：写一条**两个函数各自用到同编号但不同类别**的用例（例如函数 A 用 `int`、
函数 B 用字符串），断言 A 的类别仍是 `Int`。**撤掉修复必须让该用例失败。**

### 1.1 A 类：同一条规则在两层各写一份，然后漂移（**共 7 次，最致命**）

| # | 症状 | 根因 | 修法 | 提交 |
| --- | --- | --- | --- | --- |
| A1 | 退出边字符串在 IR 与后端各拼一份，拼法不同则查不到冻结计划 | 无反向映射，靠手写字符串 | `xiao_lifetime::ExitKind::{as_name,from_name}`；`ReleaseActionKind` 同款 | `bfec618` |
| A2 | 字符串字面量**三份实现**：词法 `is_valid_escape`、类型层 `decode_string`、IR 裸切片（**带引号**） | 转义表散落 | 统一为 `xiao_types::{decode_escape,decode_string_literal}`，IR 复用 | `7a67af3` |
| A3 | 字典字面量键带引号进 IR，与类型层判定不一致 | `dict_entry` 用原始切片 | 改调 `decode_string_literal` | `e72b42c` |
| A4 | 负索引规范化在类型层与运行时各写一份 | 私有重复实现 | 公开 `xiao_types::normalize_index`，删私有副本 | `1f8d560` |
| A5 | 字段写入**静态通过、运行时拒绝** | 静态用 `can_assign`（含加宽），运行时用精确相等 | `runtime_value_matches` 复用 `can_assign` | `7a67af3` |
| A6 | 错误类型名：前端是**无穷集**（`ends_with("Error")`），运行时是 8 名白名单 → `catch err as FooError` 静态过、运行时静默不匹配 | 两侧各一张表 | `xiao-diagnostics` 单一权威表 + `error_kind_of` | `d733522` |
| A7 | **修 A6 的那一批里又犯一次**：`ERROR_TYPE_NAMES: &[&str]` 平行列表与 `error_kind_of` 的 match 各写一份 | 表与查询函数没同源 | 合并为 `[(&str, CatchTypeKind)]`，`error_kind_of` 从表派生；删冗余 `ErrorTypeKind` 别名 | `ef5b149` |

**A7 是本档案里最值得记的一条**：它不是历史遗留，而是**在专门修复单一来源违规的批次里新引入
的同型缺陷**，还躲过了那一批自己的评审，最后靠「审计每个表的消费者」才发现。

**C0 也是 A 类**（一个程序级视图套在逐函数键空间上），只是它还没爆。

**对本次的直接后果**：接手时**先审计你要改的每一张表有没有第二个消费者、第二份副本**。

### 1.2 B 类：静默算错（编译通过，测试没覆盖就没人发现）

| # | 症状 | 根因 | 修法 | 提交 |
| --- | --- | --- | --- | --- |
| B1 | `not x` 编译成 `x == x`，**恒为真** | 一元运算按二元模板生成 | 与 `false` 比较 | `3f6aa94` |
| B2 | 一元 `-` 编译成 `as int`，**截断浮点** | 同上 | 生成 `0 - x`，零常量宽度与被操作数一致 | `3f6aa94` |
| B3 | `a = b` 用 `Move`，**清空源绑定** | 只有 `Move` 没有 `Copy` | 新增 `TacOp::Copy`；临时值 `Move`、具名绑定间 `Copy` | `85a11d8` |
| B4 | 反引号名与普通名混同（`ascii:foo` 与 `backtick:foo` 撞车） | 三处前缀拼法各写一份 | `name_key(name, backticked)` | `3d1956f` |
| B5 | 数值提升的桥不存在：`int + float` 直接拒绝 | 类型层有规则，降低器没接 | `promote_operands` 插显式 `Cast` | `3f6aa94` |
| B6 | `*args`/`**kwargs` 被**静默当作普通实参** | 无分支 | 记入 `unsupported`，验证器拒绝 | `3f6aa94` |

### 1.3 C 类：批次 1 开发期自查出的实现错（**均无独立提交，折在批次 1 里**）

| # | 症状 | 根因 | 修法 |
| --- | --- | --- | --- |
| C1 | `while` 入口跳转发进自身块 → **无限循环**；`if` 条件发进分支块 | `new_block` 把「分配块」与「切换当前块」做成一件事 | 拆成两个操作 |
| C2 | 按名引用形参时分配到位从未写入的新寄存器，读到空槽 | 形参没登记进 `value_regs` | `declare_parameters` 登记 |
| C3 | 命名函数递归调用自己 → 栈溢出 | 脚本入口占 `functions[0]`，`FuncId` 未偏移 | `FuncId::new(index + 1)` |
| C4 | 同名形参在嵌套作用域串味 | 按名查找未按作用域消歧 | 绑定按作用域解析 |

### 1.4 D 类：资源与契约腐烂

| # | 症状 | 根因 | 修法 | 提交 |
| --- | --- | --- | --- | --- |
| D1 | 临时堆值（容器/字符串字面量）**从不释放** | 临时值被排除在释放计划外 | 登记临时值并按其生命周期释放 | `885a278` |
| D2 | 4 个快照注册了却无 harness，其中一个期望**已被 C1 取代**（区间已合法化） | 只注册不加载 | 补 `c0c1_snapshots.rs`，顺带暴露过期期望与漏计诊断 | `59a0317` |
| D3 | 3 个诊断码**无任何产生点**，但文档承诺它们存在 | 死码未清 | 删除并清掉文档承诺 | `2bb5dc8` |
| D4 | 跨 crate 重名常量（`TYPE_MISMATCH_CODE` 等 3 个） | 命名空间未隔离 | 加 `CONFIG_` 前缀 | `2bb5dc8` |
| D5 | 同一诊断重复上报 | 两条检查路径都报 | 同码同跨度只留首次 | `59a0317` |
| D6 | 18 个测试文件用诊断码字面值 | 无可引用常量 | 全改常量引用 | `e900f3b` |

### 1.5 09R2C 已确认的边界（本批必须继承）

09R2C 的异常实现已经有区分度测试，R2D 不得为了适配新载体而改变这些行为：

- `finally` 中的覆盖性 `return`/`break`/`continue` 会先完成所属 `try` 作用域的释放；
  嵌套 `finally` 按内层到外层执行，且同一动态轮次不重复执行。
- 正常路径的 `finally` 故障成为主错误；已有主错误时，清理故障进入 `suppressed`；
  Fatal 故障立即终止，不进入普通错误的合并路径。
- catch 绑定重抛保留原错误身份；未匹配错误逐帧传播；错误堆栈中的每一帧都保留
  `BackendLocation`。R2D 完成后，`bytecode_offset` 来自只读物理 pc 表；映射缺失时
  保留空后端位置并记录 `BackendLocationMissing` 事件。
- `dynamic_conversion` 是只服务 `raise` 错误边界的窄语义，只允许 `RuntimeValue::Error` 通过；
  不得把它扩展宣传为完整动态标量转换。

这些边界由 `r2_stack.rs` 的 40 条 R2C 基线回归（当前文件共 42 条，新增两条区分度测试）
和 `errors.json` 的 9 条向量覆盖。
R2D 的寄存器/混合载体必须复用同一套 `VmEvent` 事件语义，不得只比较最终结果而丢掉
释放、handler、Fatal 和 `suppressed` 证据。

### 1.6 流程性事实（不是代码缺陷，但会影响你）

- **`76e3533` 的提交正文为空**（800 行执行核心），违反「正文说明为什么」的规定。
  **不重写历史无法修复**，接手时不要试图回填。
- 09R2D 的初版交接曾把 R2C 之前的 27 条向量、旧提交基线和“尚待交付”混写；
  本次先按接手时基线标明 27 条共享向量、40 条栈式回归、C0 未修状态和
  `bytecode_offset` 的临时语义，交付快照再登记修复后的状态。
- **教训**：这个仓库的文档承诺会被 `bun run check` 当作契约校验。写文档时要与代码同步核对。
- **R2b 文档刻意不复制本档案**：`09r2b-selector-execution.md` 只在它的 1.2 保留与选择器
  直接相关的四条（A2/A3、B3、D1、D3），其余指向本文。这是对「同一内容誊抄成第三份副本」
  的正面处理——本仓第一号病史正是这种副本之间的漂移。**不要以「完整」为由把它誊抄回去。**

### 1.7 潜在隐患（已区分为“已排查”与“仍开放”）

| # | 隐患 | 为什么危险 | 本批怎么处理 |
| --- | --- | --- | --- |
| H1 | 旧版 `Frame.last_sub_fault` 是单个 `Option<BlockId>`，而 `active_subroutines` 是栈 | 嵌套 `finally` 内层失败时，外层子程序的来源信息可能被覆盖 → 错误路由到错的退出边 | **已修复并验证**：改为栈化 `subroutine_faults`，保留 `nested_finally_failure_does_not_repeat_outer_finally` 回归；故障来源不再依赖可覆盖单值 |
| H2 | 旧版 `run_blocks`/`run_subroutine` 每个块都 `instructions.clone()` | 每块一次 `Vec` 分配，会污染 09R3 的性能基准 | **已修复并留基准**：移除克隆后同一 27 条栈式向量连续 5 次中位数由 `253.66 ms` 降至 `245.83 ms`，约 `3.1%`；R3 仍需按正式协议重测 |
| H3 | `RuntimeValue::Hash` 对错误对象只哈希判别式 | 契约上允许（不等者可同哈希），但若有人依赖哈希区分会静默错 | 已登记为**已知语义，不要「修」** |
| H4 | 静态说栈值、运行时按堆物化的元组 | 语义可能不等价 | 本批不动，记入风险 |
| H5 | `escape.rs:887-901` 把 `ExitKind::Fatal` 列为可被 catch 吃掉，与 07-B 冲突 | 依赖它会把 Fatal 吞掉 | **09R2C 已按 07-B 实现**：Fatal 绕过 handler/finally/drop；R2D 只需用三种载体复验，不得改生命周期冻结产物 |
| H6 | 容器按**对象身份**相等，结构性 `==` 未实现 | 两个内容相同的数组判不等 | 已知语义，不要「修」 |
| H7 | 合流点 `CategoryMap` 退化为 `Poly` | `Poly` 必须落帧槽，**寄存器机型若把它当普通类别分配会出错** | **已处理**：逐函数类别表已成为正式分配输入，寄存器载体将 `Poly` 固定到帧槽、`None` 视为零宽 |
| H8 | 参数 clone 进被调帧后的引用计数是否平衡，未做峰值内存验证 | 09R3 有「峰值内存恶化 ≤10%」门槛 | 本批不验证，留给 09R3 |

### 1.8 排除指南（症状 → 先查哪里）

**第零条，先查这条**：怀疑某个修复没生效时，**先确认你改的文件真的落盘了**。
上一阶段有三次 shell/Python 替换**静默失配**（不报错也不改动），其中一次让 Agent 基于假输入
误报了一个并不存在的前端缺陷。**改中文源码一律用 `Write`/`Edit` 工具。**

| 你看到 | 先查 |
| --- | --- |
| 「代码看着对，结果不对」 | ① 被测输入真的落盘了吗 ② 同一条规则是否在两层各写一份（A 类） |
| 静态通过、运行时失败 | 同上②；本仓 A5/A6 都是这个形状 |
| 某个值读出来是空的 / 无效句柄 | 绑定寄存器是否登记（C2）、是否被 `Move` 清空（B3） |
| 死循环 | `new_block` 是不是又被当成「分配并切换」（C1） |
| 递归调用自己 / 栈溢出 | `FuncId` 有没有给脚本入口留 0 号（C3） |
| 同名变量串味 | 按名查找是否按作用域消歧（C4） |
| **寄存器类别看起来不对** | **是不是跨函数同编号冲突（C0）** |
| 堆值泄漏 | 临时值是否登记进释放计划（D1） |
| 同一错误报两遍 | 是否有两条检查路径（D5） |
| 新增 `RuntimeValue` 变体后行为诡异 | 下面的三处兜底匹配 |
| 文档承诺的检查不存在 | 该码有没有产生点（D3） |

**三处兜底匹配**（新增 `RuntimeValue` 变体时必须逐一检查，**它们不会编译报错**）：

| 位置 | 兜底行为 | 不加分支的后果 |
| --- | --- | --- |
| `value/mod.rs` 的 `PartialEq::eq` | `_ => false` | `Error(e) != Error(e.clone())`，自己和自己不等 |
| `containers/mod.rs` 的 `is_hashable` | **否定式** `!matches!(...)` | 新变体默认被当作**可哈希** |
| `value/mod.rs` 的 `type_name()` | `_ => "dynamic"` | 类型错误信息显示 `dynamic`，误导排查 |

另两处 `Hash::hash` 与 `scalar_type()` 无兜底分支，会编译报错因而安全。

### 1.9 方法警示：本轮审核实际踩到的易错点

下面三条**都是这一轮真实发生的**，不是假想。它们有一个共同形状：**先下结论、后找依据**。
本仓已经因为「同一规则两处各写一份」吃过 7 次亏（A 类）；这三条是同一个病在**判断层**的形态，
而判断层的错误会直接变成文档里的错误指令，交给下一个 Agent 执行。

#### 第一类：给约束归因前，先验证那个约束存在

- **实例**：R2B 文档曾把「选择计划跨层承载」的难点归给**依赖方向**
  （「如果因为依赖方向不能直接携带 `xiao_types` 的结构」）。这是**可一眼证伪的**：
  `xiao-ir/Cargo.toml` **已经依赖** `xiao-types`，`lower.rs:20` 早就在用
  `Type`/`TypeCheckResult`/`ArrayType`，`lower.rs:821` 的 `runtime_check_kind_name`
  还**直接消费** `xiao_types::RuntimeCheckKind`。真正的障碍是**序列化边界**
  （`IrProgram` 可序列化，而 `Type`/`SelectionPlan` 没有 serde derive）。
- **教训**：写「因为 X 所以必须 Y」时，**X 必须是验证过的**。本仓依赖图很窄，
  `grep` 一次 `Cargo.toml` 就能证实或证伪。**归因错误和拼写漂移同源**——
  都是凭印象写下一件没核实的事，只是它落在散文里而不是代码里。

#### 第二类：判断清单/编号完整性要读实际文件，不要读 diff 的过滤结果

- **实例**：用 `git show --unified=0 | grep '^+'` 看提交，据此**两次**断言编号断裂
  （「提交切分从 2 跳到 5」「验收标准漏了 4」）——**两次都是假阳性**：
  未改动的行是上下文行，**不带 `+` 前缀**。
- **同类**：找一条约束时要**搜全文而不是单节**。我 grep R1-F 一节没找到「按
  `IrValue.declaration_order` 排列」，差点得出「这条约束是编的」，而它就在相邻的
  **R1-E**（`09r-bytecode-machine-research.md:141`）。
- **教训**：**提缺陷前先构造反例**。「撤掉修复 → 用例必须失败」这条规矩**同样适用于文档断言**：
  要断言「某处没有规定 X」，必须**全文搜过**才能下结论。

#### 第三类：转述代理或工具的发现前，自己复核

- **实例**：调研代理报「`{step}` 在降低时被静默丢弃，会产出验证通过但语义错误的 TAC，
  必须先堵」。实测后判定**对当前唯一支持的形态是无害的**——单 `Exact` 项展开后只有 1 条路径，
  `step_by(n)` 在单元素上恒等，所以今天的语义是对的。它是**潜在隐患**，不是活缺陷。
- **教训**：把「静默错误」这类重判写进文档前必须自己复核。**写错和漏报一样有害**：
  接手者会去修一个不存在的问题，而真正该修的隐患反而被这条假警报挤掉注意力。

**判别方法**（比背下这三条更有用）：**这个论断如果我错了，有什么测试或搜索能证伪它？**
答不上来就不要写进交接文档。

---

## 二、硬性约束与开发规定

### 2.1 单一来源原则（最高优先级，A 类已犯 7 次）

跨层规则只能有**一处**定义。已确立的唯一来源（**禁止再开第二份**）：

| 规则 | 唯一来源 |
| --- | --- |
| 退出边拼写 | `xiao_lifetime::ExitKind::{as_name,from_name}` |
| 释放动作类别 | `xiao_lifetime::ReleaseActionKind::{as_name,from_name}` |
| 字符串字面量解码 | `xiao_types::{decode_string_literal,decode_escape}` |
| 负索引规范化 | `xiao_types::normalize_index` |
| 标量名拼写 | `xiao_syntax::ScalarType::{as_str,from_name}`（往返用例钉住） |
| 数值提升/加宽 | `xiao_types::conversion` |
| 赋值兼容判定 | `xiao_types::can_assign` |
| 错误类型名 | `xiao_diagnostics::error_kind_of` 及其权威表 |
| **CFG 后继** | **`verify.rs` 的 `jump_targets`**（本批的活跃分析必须复用，不要另写一份） |

**本批要新增的唯一来源**：

1. **寄存器类别**（逐函数 `CategoryMap`，见 C0）——只能有一处。
2. **`VReg` → 物理位置的分配**——三机型共用一份。
3. **`ScalarType` 与 `ReleaseActionKind` 的字节映射**（编码器需要）——这两个是外部 crate 的枚举，
   映射必须定义一次并配往返用例。

**本批禁止**为寄存器机型另写一份类别判定。**尤其禁止从 `RuntimeValue` 变体反推类别**：
`lint`/`lfloat` 在冻结表里归 `ObjHandle` 但运行时是文本变体；`None` 是零宽不占寄存器；
`Poly` 必须落帧槽——反推给出的分配与冻结口径不同，**直接违背 09R3 的对比口径**。

### 2.1.1 09R2C 兼容门

R2D 的每一个载体实现都必须先通过下面这组兼容门，才能进入三机型对拍：

| 契约 | R2D 的不可变要求 |
| --- | --- |
| 错误身份 | `RuntimeValue::Error` 的 clone 仍指向同一错误身份；catch 绑定、重抛和堆栈不能重新构造等值副本 |
| handler 路由 | 先查当前帧；未匹配才传播到调用方；`TacHandler.protected` 的块区间语义不能被物理寄存器分配改变 |
| finally | `CallSub`/`RetFromSub` 的挂起退出类别栈必须保留；同一动态轮次只执行一次，清理错误按 `suppressed` 规则归并 |
| Fatal | 不查 handler、不执行 finally/drop、不进入 `suppressed`；三种机型都必须保留这一不对称 |
| RuntimeCheck | 只执行降低器实际发出的四类检查；`string_boolean` 当前只预留错误码，选择器/集合/随机/迭代器类别继续留在 `TacProgram.unsupported`，不得静默删掉 |
| 释放计划 | `RunReleasePlan` 的动作顺序、`(scope, exit)` 名称和空寄存器容忍语义保持不变；载体只能改变值的位置 |

这张表是 R2D 的回归清单，不是新的语义来源；每一项的权威实现仍在 09R2C 已登记的
Runtime、TAC 和 VM 路由代码中。

### 2.2 工具规定

- 新建文件用 `Write`，改文件用 `Edit`。**不要用 shell heredoc / Python 替换改含中文或转义的
  源码文件**——本仓每个文件都有中文 Rustdoc，命中率极高，且失败是**静默**的（见 1.8 第零条）。
- **不要用 `git checkout --` 还原临时改动**——它会连带回退你未提交的新增代码
  （上一阶段因此丢过一整块实现）。

### 2.3 区分度验证（每条修复的验收方式）

**不许用「测试通过」结案。** 每条修复都要证明用例**有区分度**：
**撤掉修复 → 用例必须失败 → 还原 → 通过。**

已按此验过：`ExitKind` 拼写锁、临时值释放、`Copy` vs `Move`、一元 `not`、Fatal 非对称、
解耦叶子测试。本批同样适用，见「六、验收标准」。

### 2.4 门禁（每次提交前全跑）

```bash
cargo test  --manifest-path core/rust/Cargo.toml --workspace
cargo clippy --manifest-path core/rust/Cargo.toml --workspace --all-targets -- -D warnings
cargo fmt   --manifest-path core/rust/Cargo.toml --all -- --check
cargo doc   --manifest-path core/rust/Cargo.toml --workspace --no-deps
bun run check && bun run check:coverage && bun test
git diff --check
```

- **不要用 `#[allow]` 掩盖警告或覆盖率缺口。** 新增 `pub` 项须 100% 有 Rustdoc
  （工作区 `missing_docs = "warn"`）。
- 含 `.rs` 的**新目录必须自带 `README.md`**（目录判定**不继承父目录**），且必须出现在
  `docs/module-registry.json` 某条目的 `code`/`tests` 路径中，否则报 `A0-LAYOUT-002`。
- **新建 `tests/` 目录必须带 `tests/README.md`。**
- 提交信息用 `feat:`/`fix:`/`test:`/`docs:`/`chore:` 前缀，**正文说明为什么而不只是做了什么**。

### 2.5 解耦约束（硬性）

1. **语义核不认识载体细节**：`xiao-vm/src/research/semantics/` 对载体的接触面现在是 **6 行**，
   本批结束**不得超过 10 行**；**不得在 `Frame` 上新增任何含机型词汇的字段**
   （`Frame` 的字段全部 `pub`，加进去等于把机型细节泄回语义核）。
2. **不引入 trait object 进热路径**，静态分发。
3. **零新外部依赖**（全仓只有 serde/serde_json/syn，且 `xiao-bytecode` 连 serde 都没有）。
4. `xiao-vm/src/research/` 是叶子；生产 `src/` 不得 `use crate::research`
   （有可执行测试 `research_module_stays_a_leaf` 守着）。
5. **`xiao-bytecode` 不得重新推断类型、重算生命周期、重排释放顺序**，只做 1:1 语义展开。
6. **`VmEventSink` 只在粗粒度边界调用**，不做逐指令回调（会影响 09R3 的性能基准）。
7. **三种机型共用同一组语义向量，禁止为某一型修改期望值**；向量里不得出现机型专属字段。

---

## 三、下一阶段任务

### 任务 0（**先做**）：修掉 C0 的跨函数类别污染

见 1.0。必须**在其他任务之前**完成，因为寄存器机型的分配完全建立在类别表正确的前提上。
改法与验收见该节。这一步是加法，**向量 JSON 零改动**（JSON 里没有 `categories` 字段）。

### 任务 1：`Carrier` 接口演进与子程序故障边界

现状 6 个方法：`empty/read/write/take/depth/peak`，其中 **`depth()` 是死代码**（全仓无调用点）。
接口演进要同时解决四个载体缺口和一个由 R2C 暴露的状态问题：

1. **`C::empty()` 无参数是阻断性的**：寄存器机型必须知道每个 `VReg` 的类别，载体现在完全拿不到
   `CategoryMap`/`TacFunction`/`TacProgram`。构造必须改为接收机型无关的上下文结构体，至少包含当前
   函数、**逐函数**类别映射、整份产物和调用深度。
2. **无溢出通道**：没有 `spill`/`reload`，没有「寄存器文件 + 独立帧槽区」两段存储概念。
   溢出是寄存器/窗口载体的内部决策，不要把 `spill` 指令泄回语义核。
3. **无跨调用保存/恢复**：载体生命周期被钉死在单帧内（`exec.rs` 每帧构造，帧弹出即丢弃），
   R1-L/O 的保存责任无处落地。调用开始/结束可以进 trait，但必须用默认空实现让 `StackCarrier`
   零成本。
4. **指标出口只有 `peak()`**，无法上报 `spill_count`、`stack_map_entries` 和调用保存次数。
5. **H1 已收敛**：旧版 `last_sub_fault` 已替换为与活动子程序配对的
  `subroutine_faults` 栈；`run_subroutine` 的故障来源不会被内层子程序覆盖。后续改动仍不得
  把它退化成单个可覆盖字段。

**设计要求**：

- 构造上下文只描述语义输入，不携带「栈式/寄存器式」字段；新增输入以后仍只改上下文结构体。
- `read/write/take` 的空寄存器错误必须由三机型共用的构造函数产生，保证同一份向量得到同一个
  稳定错误码。
- `depth()` 要么删除，要么改成明确的机型中立语义；不能保留无消费者的模糊名称。每个方法写
  中文 Rustdoc。
- 跨调用保存/恢复、溢出、重载和窗口映射都通过机型中立指标出口上报；不要逐机型新增 getter。

**指标字段必须独立**：`instructions`、`max_call_depth`、兼容保留的 `max_stack_depth`、
`releases`、`spill_count`、`stack_map_entries`、`call_save_count` 分别计数。尤其
`stack_map_entries`（栈式/混合式的可验证映射点）不能与 `spill_count` 合并。

**改动面**：`exec.rs` 的帧创建、调用进出、指标汇总和子程序故障回传必须一起审阅；不能只改
`Carrier::empty` 的签名而让 `Frame` 暗中携带机型细节。

### 任务 2：活跃区间分析（**本批最危险的一块**）

**没有隐式 fallthrough**：`run_blocks` 的 `Flow::Next => return Ok(None)` 意味着
**块不显式跳转就结束函数**。所以 **CFG 后继 = 块内所有指令的显式跳转目标之并集**
（`Jump` / `BranchIf` / `Check.on_failure` / `CallSub.sub`），与 `verify.rs` 的 `jump_targets`
必须是同一口径。

当前 `jump_targets` 是 `verify.rs` 的私有函数，不能让活跃分析再复制一份；第一步应当把它
提为 research 内部共享 helper（或抽到 `cfg.rs`），然后由验证器和活跃分析共同消费。**“逻辑相同”
不等于“各写一份”。**

**两条最容易踩的红线**：

1. **`RunReleasePlan` 会间接读取并销毁寄存器。**
   `run_plan` 按 `plan.actions[].value` 经 `function.value_registers` 查表拿到 `VReg`，然后
   `take` 掉它。所以一条 `RunReleasePlan { scope, exit }` 在活析里**必须被当作读取该计划
   所有动作指向的寄存器**。漏掉这条 → 复用掉的正是释放计划还要用的值 → 释放错值或漏释放，
   而**向量里的 `releases` 序列会直接对不上**。
2. **异常边必须在活析里建模。**
   `TacHandler.protected` 是**块区间**，区间内**任意指令都可能触发异常**，路由会先跑
   `(scope, exit)` 的释放计划再进 handler。所以「handler 入口会读到的寄存器」必须在
   **整个受保护区间内保持活跃**。漏掉这条 → 同样的释放序列错乱。

**要求**：活析结果是**机型中立的产物**，三机型共用；写成**可独立单元测试的纯函数**
（给定 `TacFunction`、释放计划和共享 CFG 后继 → 区间表），**不要把它埋进寄存器载体的实现里**。
数据流必须明确区分：

- `LoadConst`/构造/算术/比较等指令的源寄存器是 use，`dst` 是 def；`Move`、`Release`、
  `Transfer` 的所有权语义不能被普通的“读写集合”替代。
- `RunReleasePlan` 通过 `function.value_registers` 间接读取并销毁计划动作对应的寄存器，
  这些寄存器必须在计划执行点保持活跃，且计划完成后才能结束其区间。
- 每个受保护块区间都要向可能的 catch/finally 入口建立异常边；handler 入口所需的绑定寄存器、
  `finally` 子程序挂起状态和释放动作不能只沿显式正常边传播。
- `CallSub` 的入口/返回、`Return`、`Raise` 和动态检查失败边必须闭合；不能以“没有前驱”删除
  catch 孤岛块，也不能把 `TacBlock.exits` 当作完整可达性描述。

**安全网**：三机型跑同一组 `tests/spec/09-bytecode/*.json`（当前 **27 条**：
`containers` 8 / `errors` 9 / `scalar` 6 / `control` 4），再加上 `r2_stack.rs` 的 40 条
异常回归。期望值含完整 `releases` 事件序列；分配器或活析算错会直接表现为向量失败。
**这是本批的主要保障，不要绕过它，也不要为某一机型改 JSON。**

### 任务 3：分类型寄存器机型

按 R1-H 的类别区间分配物理寄存器。**结构化位与分配表放在载体内部，不要加到 `Frame` 上。**
分配器的输入只能是修复后的逐函数类别表、活跃区间和冻结的 `IrValue.storage`；不能从
`RuntimeValue` 的运行时变体反推类别。

**必须遵守**：

- **`Poly` 与 `None` 有专门路径**（`Poly` 落帧槽，`None` 零宽不占位）——见 H7；
  `Poly` 不是“未知时随便选一个寄存器类”。
- **释放计划中的值必须可被被调用者保存**：凡出现在任意 `IrReleasePlan.actions` 的值，
  必须钉在帧槽或等价的非易失位置，不能只放在易失寄存器里。
- **分配必须确定性**（同一输入同一分配），否则三机型对拍会引入噪声。
- **寄存器复用是本批的明确要求**：按活跃区间复用完即死的物理寄存器；同一结束点的
  tie-break 必须固定（建议按 `VReg` 编号）。
- **调用与异常边同等重要**：跨调用存活值、handler 入口值和 finally 挂起状态不能因寄存器
  复用而改变；不能只在正常 CFG 上做线性扫描。

### 任务 4：混合式机型

按 R1-F 实现，并在载体 README 与测试中写清楚三个分界：具名值进入固定局部窗口，匿名临时值
进入求值栈，参数在栈顶连续区间传递而返回值走返回寄存器。窗口满时进入帧槽的动作计入
`spill_count`，而调用点/帧尾需要的可验证栈映射条目计入 `stack_map_entries`；两者**必须是
独立字段**。

开始实现前必须核对“具名值窗口顺序”的输入。R1-E 的冻结定义要求按 `IrValue.declaration_order` 排列，
而当前 `TacFunction` 只公开 `locals: Vec<VReg>` 和 `value_registers`，没有显式的声明序号。
如果 C0 修复没有顺带携带这项元数据，应在 TAC 中增加机型中立的局部布局表，或明确证明
`locals` 已与生命周期声明顺序一一对应；**不得把 `VReg` 编号、源码 span 或 map 的遍历顺序
当作声明顺序**。信息不足时按交接约束采用不复用的保守窗口，并登记原因。

混合式载体不得通过给 `Frame` 增加 `window`、`spill` 或 `stack` 字段来偷渡机型细节；
这些结构必须留在 `Carrier` 实现内部。其 `CallSub`/异常清理路径仍须与栈式和分类型寄存器式
共享同一 `TacHandler` 与释放计划语义。

### 任务 5：`VmMetrics` 与三入口

`run.rs:65-76` 的 `VmMetrics` 扩展必须**保持 `Copy`**（`metrics()` 是 `pub const fn`，
按值返回）。新增 `spill_count`、`stack_map_entries` 和调用保存次数三个独立字段；
它们不能通过一个“总内存成本”字段合并，否则无法解释三种机型的差异。

`max_stack_depth` 的 doc「载体槽位的历史峰值」是栈式措辞：**保留字段名以免破坏现有调用方，
但把注释改成机型中立的“载体占用峰值”**。栈式报告有效槽位峰值，分类型寄存器式报告
帧槽/非易失位置峰值，混合式报告窗口与求值栈的统一峰值；三者不改变语义结果。

`run.rs` 的 `run()` 可以保留栈式兼容入口，但必须新增机型泛型入口（例如 `run_with<C, S>`），
让 `r2_vectors.rs` 的观察函数按载体参数化。**JSON 零改动**（`Expectation` 只含
`outcome`/`error_code`/`releases`/`max_call_depth`，全部机型无关），同一份期望对三台机型各断言一次。
向量测试要同时核对 `VmEvent` 的释放顺序和 `RunResult`，不能为新机型另造一套观察器。

**顺带清掉 H2**：`run_blocks`（`exec.rs:221`）与 `run_subroutine`（`exec.rs:560`）的
每块 `instructions.clone()`。克隆的是 `TacInstr`，被借的是来自 `self.program: &'p TacProgram`
的 `&'p TacFunction`，与 `&mut self` 正交——**这个 clone 借检器不需要**
（这是静态推理，**需要编译验证**）。先测清理前后的基准，**留下数字**，因为它影响 09R3 的口径；
如果借用关系实际不允许零拷贝，必须登记采用的安全替代方案，而不是偷偷保留每块分配。

### 任务 6：指令编码器

新建 `core/rust/crates/xiao-bytecode/src/research/encode.rs`，在 `research/mod.rs` 重导出。
**与 `verify.rs` 并存，不替换它**（验证器看的是「TAC 是否忠于冻结计划」，与编码是上下层关系，
不是同一件的两种做法）。

按 **R1-Y** 做变长指令 `opcode: u8` + 变长操作数，并让 09R3 能比较三件事：

- **寄存器号与常量池索引用 LEB128 还是定宽 u16** → 编码器要支持**两种操作数宽度变体**；
- 窗口下标访问是否需要独立操作数形式（混合式专属）；
- 调用指令携带 `SigId` 后，参数是否还需逐条编码。

**数据面事实**：`TacOp` 共 **31 个变体**；含 `String` 操作数 **6 处**（`NewDictTable`/
`NewDictColumn` 的 entries 键、`MakeError.type_name`、`Check.kind`、`RunReleasePlan.exit`、
`ExitScope.exit`）；变长 `Vec` **9 处**；`PathStep::Index(i128)`（**128 位有符号**）。
这些都是体积杠杆。31 个变体包含 R2C 新增的 `MakeError`、`CallSub`、`RetFromSub` 和带
`value` 操作数的 `Check`；编码器不能按 R2C 之前的旧指令子集实现后再“补几个 opcode”。

**必须绕开的坑**：

- `ConstPool` 与 `CallSigTable` **只有 `len()` + `get(id)`，没有 `iter()`**——需补只读迭代。
- `TacConstant::Float(f64)` 使 `ConstPool` **只有 `PartialEq` 没有 `Eq`**——字节级往返断言
  需显式决定走 `to_bits()` 还是数值比较（NaN 载荷、±0）。
- `TacHandler.protected` 是**块区间**，编码成 pc 区间需要编码器自建**块号 → pc 表**。
- `TacFunction.value_registers` 的键是 `IrValue.id`（u32），与 `VReg` 空间**互相独立**，不要合并。
- `verify_program` **不校验 `ConstId`/`SigId`/`FuncId` 越界**，编码器需自带边界检查。
- `ScalarType`（来自 `xiao-syntax`）与 `ReleaseActionKind`（来自 `xiao-lifetime`）是外部枚举，
  需给它们定义**稳定且唯一**的字节映射（见 2.1 第 3 条）。

**R1-AB 边界**：这是**研究用编码**。**不得使用 `.xiaoc` 扩展名落盘**，输出应为内存 `Vec<u8>`；
函数名不得叫 `write_xiaoc` 之类。正式分段格式、分区编号与内容寻址归 14/16 阶段。
**R1-AA 版本字段**：「版本不匹配时必须拒绝执行，不得静默降级」——解码侧必须有拒绝路径。

#### 任务 6 的最小编码契约

编码器先定义一个内存结果（例如 `EncodedProgram`），再提供纯函数
`encode(program, options) -> Result<EncodedProgram, EncodeError>` 与对应解码/验证入口。
它不是文件写入器，也不改变 `TacProgram`。结果至少包含：

1. `abi` 的版本、语言和目标字段，且解码入口在读取任何函数体前校验
   `bytecode_abi_version`、`runtime_abi_version` 和 `ir_version`；版本错误必须是结构化
   `EncodeError` 变体（或等价枚举），不能返回“尽量解码”的部分程序。R2D 研究错误先不占用
   语言诊断码；真正进入执行边界的损坏产物才映射到既有 `FATAL_CORRUPT_ARTIFACT_CODE`。
2. 函数目录、基本块起始 pc、指令字节串和 `pc -> IrSpan` 映射。块号到 pc 的表必须在编码
   handler 区间前完成，`TacHandler.protected` 的 `[start, end)` 语义不能改成闭区间。
3. 常量池、调用签名和 handler 元数据的边界检查。`ConstId`、`SigId`、`FuncId`、`BlockId`
   任何越界都在编码阶段拒绝；不能依赖 `verify_program` 替你补做，因为验证器目前不检查前三类
   引用的范围。
4. 可选的操作数宽度策略：`Leb128` 与 `FixedU16` 必须编码同一 TAC 产物、使用同一 opcode
   语义，并只改变操作数字节宽度。超出 `u16` 的编号在定宽模式下要返回错误，而不是截断。
   LEB128 的无符号和 `PathStep::Index(i128)` 的有符号编码规则必须各有往返用例。
5. 稳定 opcode 表和字段顺序。opcode 不按 Rust `enum` 的调试编号隐式生成；新增/调整变体时
   显式更新表和版本字段。`String`、`Vec`、`Option`、`i128` 字段都要有长度/存在位编码，
   解码器遇到未知 opcode 或截断长度必须拒绝。
6. `TacConstant::Float` 采用按位口径（`f64::to_bits()`，保留 NaN 载荷和 `-0.0`），
   不用浮点数值相等替代字节往返；其他常量和 `ScalarType`/`ReleaseActionKind` 的字节映射
   也必须由单一表驱动并配双向测试。

调用指令的第一版建议固定为“`FuncId + SigId + 参数数量 + 参数记录`”，参数记录保留
位置/关键字种类和关键字名称；不要在编码器里重新解析调用签名或重新推断参数类型。混合式
窗口下标若需要独立形式，应当新增显式操作数标签，并证明它与普通 `VReg` 在反解后语义等价。

**往返定义**：`decode(encode(program, mode), mode) == program` 只比较编码器负责的 TAC/ABI
语义字段；不要求 `Vec` 容量、哈希顺序或浮点的数值归一化相等。失败路径也必须测试：坏版本、
坏引用、未知 opcode、截断输入、定宽溢出和非法 `ReleaseActionKind` 均应得到稳定错误。

### 任务 7：`pc -> IrSpan` 源码映射与接线（**最后一个提交**）

R1-Z 要求源码映射是独立于指令流的 `pc -> IrSpan` 表、增量编码。现状是**全仓没有 pc 概念**：
`exec.rs:229`/`:567` 直接把 `instruction.span.start`（**源码字符偏移**）填进
`BackendLocation.bytecode_offset`，**名实不符**（代码注释自承是「尚未冻结物理编码前的权宜」）。

**交付**：编码器产出 ① 每条指令的起始 pc；② 由 pc 序列 + `span` 生成的 delta 表；
③ `pc -> IrSpan` 反解。然后把 `exec.rs` 的两处调用改为**查表取真 pc**。
做成**只读查表入口**，不要在热路径里重建表。映射按函数独立存放，函数入口 pc 从零开始
还是全程序单调递增必须在编码 ABI 中明确；本批建议使用“函数内 pc + 函数目录基址”，避免
把递归帧和不同函数的同一局部 pc 混为一谈。

反解接口至少要定义三种边界：

- 指向一条指令起始 pc 时返回该指令的 `IrSpan`；
- 指向指令中间字节时返回该指令（或明确拒绝，二者只能选一种并固定测试）；
- 超出函数编码范围时返回 `None`/稳定错误，不得返回最后一条源码区间冒充成功。

增量表的 `pc_delta` 与 `span.start/end_delta` 必须使用有界整数解码并检查溢出；源码区间
可以重复或零宽（错误恢复产生的合成区间），但必须满足 `start <= end`；编码器不得用排序或去重
改变 TAC 指令顺序。
“三机型共用同一张映射表语义”指同一 TAC 指令始终映射到同一个 `IrSpan`；不同操作数宽度
或机型可以得到不同的物理 pc，不能因此修改源码区间或把一个机型的 pc 期望硬编码给另一个。
`annotate_fault` 应在进入 `XiaoError`/`FatalError` 堆栈前通过只读映射取 pc；如果映射缺失，
应保留“无后端位置”的结构化状态并记录事件，不能退回 `IrSpan.start`，否则会把本批反例重新
变成假通过。

既有断言 `xiao-vm/tests/r2_stack.rs:843,860` 需一并更新。

### 任务 8：文档同步

- 父文档仍保留「**已登记、尚待交付**」标题，用于标记未完成的 `for`/表声明；R2D
  已交付项单独列在父文档的“已交付”小节，避免把规划文字和完成状态混写。
- 两个 crate 的 research README、`docs/DevDocs/README.md`、`12-tests-and-milestones.md`
  和 `tests/spec/09-bytecode/README.md` 已同步；`docs/module-registry.json` 已登记
  `encode.rs` 的代码与测试路径，但研究模块整体仍保持 `draft`，直到 09R2 阶段退出条件全部满足。
- H1 已改成结构化的栈化子程序故障状态并由嵌套 finally 回归锁定；H2 已移除块级克隆，
  并留下 `253.66 ms -> 245.83 ms` 的可复核基准数字。
- 本批文档门禁已补入实际提交号、27 条共享向量、40 条 R2C 栈式基线（当前栈式回归
  共 42 条，新增两条机型/pc 区分度测试）和 R2D 兼容门；正式全量门禁仍在最终收尾时复跑。

---

## 四、已确认的关键事实（省一轮调查）

- **无隐式 fallthrough**：`Flow::Next => return Ok(None)`。块必须显式跳转，否则函数结束。
- **catch 入口块可能没有前驱**：`escape.rs:913` 在 try 体无 `raise` 时不建边。
  **不能把「没有前驱」当作「不可达」。**
- **块的 `exits` 不是完备描述**：`ExitKind::Normal` 从不被 `record_block_exit` 写，
  它只记异常/非局部转移。不能用它判断可达性。
- **`release_plans` 是不可裁剪的笛卡尔积**（`release.rs:48-59`：每作用域 × 11 退出边，
  无任何裁剪；唯一过滤是值级的）。**绝不能用「某计划是否存在」表达 Fatal/Unmatched 的差别。**
- **`escape.rs:887-901` 与 07-B 冲突**（Fatal 是否可被 catch）：按 07-B 实现，
  **不改 `xiao-lifetime`**（那是 07-B 的冻结产物）。
- **`raise ErrType(code=…, message=…)` 在 IR 里是普通 `Call`**（裸名 callee + `Dynamic` 类型）。
- **`Literal.text` 是含引号的原始源码切片**，必须过 `decode_string_literal`。
- `run_plan` 靠 `function.value_registers` 把 `IrValue.id` 映到 `VReg`。
- **按名引用的实体（函数、表、类型）不物化值**，所以计划里有些 `VReg` 从未被写入——
  `run_plan` 用 `take_if_present` 跳过空槽，**这不是 bug**。
- `VmMetrics` 目前是 `Clone + Copy`，`metrics()` 是 `pub const fn`——扩展时别破坏这两点。
- R2C 的 `TacHandler.protected` 是 `[start, end)` 块区间；finally 子程序本身位于保护区间
  之后，不能因为编码成 pc 区间就把子程序代码重新纳入自己的 handler。
- `Fatal` 与可恢复错误是两个终止通道：编码器可以共享错误元数据格式，但不能把 Fatal 当作
  一个可被 `catch_type` 匹配的普通 `XiaoError`。
- `TacProgram.unsupported` 是有意保留的拒绝信息。编码器收到非空 `unsupported` 时必须在编码前
  拒绝；不能把未降低的检查/语句编码成空操作，也不要在本批引入绕过验证的“宽松模式”。
- `jump_targets` 已提取到 `research::cfg` 共享入口，由验证器和活跃分析共同消费；
  `CallSub`、`Check.on_failure` 等 R2C 边保持同一实现。
- `Frame` 包含 `pending_exits`、`active_subroutines`、栈化的 `subroutine_faults`、
  `active_catches` 和 `completed_finally` 等异常记账字段；这些字段都是语义状态，不能替换成
  带“寄存器/窗口/溢出”名称的机型字段。R2D 已保持 `Frame` 对机型中立。
- `TacFunction.categories` 现在携带函数局部类别表；`parameters`、`locals` 和
  `value_registers` 仍是函数局部，`TacProgram.categories` 只保留脚本入口兼容视图，不能
  用于命名函数的物理分配。
- `verify_program` 当前只检查跳转目标和 `RunReleasePlan` 名称；它不检查 `ConstId`、`SigId`
  或 `FuncId` 的范围，也不提供活跃分析。编码器和活析必须各自复用/补齐明确的校验入口，
  不要把验证器的“通过”误当成编码安全证明。
- `tests/spec/09-bytecode` 的 27 条向量现在由 `r2_vectors.rs` 的同一观察器依次消费栈式、
  分类型寄存器式和混合式三种载体；JSON 没有改动。正式性能/内存门槛仍留给 09R3。

---

## 五、提交切分

按可独立验证的单元分八次，**每次提交后门禁全绿**；每一步都先跑 09R2C 的 40 条栈式回归，
确认语义核没有漂移：

0. **C0**：逐函数类别映射（或全程序唯一编号）+ 跨函数用例。
1. **`Carrier` 接口演进**（含删 `depth()`）、`StackCarrier` 适配、`exec.rs` 构造与指标两处改动，
   并把 H1 的子程序故障来源改成结构化返回或等价的栈化状态。**此提交后栈式行为必须逐条不变。**
2. 活跃区间分析（机型中立的纯函数 + 单元测试），先抽出并复用共享 `jump_targets`，覆盖
   `RunReleasePlan` 间接读取和受保护区间异常边。
3. 分类型寄存器机型 + 单元测试。
4. 混合式机型 + 单元测试。
5. 三机型共用向量对拍（`r2_vectors.rs` 泛型化）+ `VmMetrics` 扩展 + 清 H2 的 clone（附基准数字）；
   此时 27 条 JSON 必须零差异，40 条栈式回归仍全部通过。
6. 指令编码器 + 往返解码 + 两种操作数宽度变体 + 体积对比；先覆盖全部 31 个 `TacOp` 变体，
   再覆盖坏版本/坏引用/截断输入等拒绝路径。
7. `pc -> IrSpan` 映射 + `exec.rs` 接线 + 更新既有断言；删掉源码偏移回退路径。
8. 文档同步（含修掉 `09r-...md:498`）。

**若中途必须停**：0–2 与 6 各自完整可验证，不会留下半成品。

**第 3–4 步若发现 `TacFunction` 提供的信息不足以安全复用**（例如活跃区间跨异常边无法精确闭合），
应当**降到保守方案（按类别全量静态分配、不复用）并明确登记**，
**而不是硬做出一个不确定的分配器**。

---

## 六、验收标准

命令见 2.4。**关键验收不是「测试通过」**：

1. **C0 被真正修掉**：两个函数各自使用同编号但不同类别的寄存器，**前一个函数的类别不被后一个
   改写**；正式分配读取逐函数类别表，而不是程序级合并表。**撤掉修复必须让该用例失败。**
2. **09R2C 栈式行为零漂移**：Carrier 第一步之后，现有 27 条 JSON 向量和 `r2_stack.rs` 的
   40 条回归逐条不变；catch/重抛、嵌套 finally、覆盖性退出、suppressed、Fatal 和四类 Check
   语义都必须保持。**撤掉任何一处适配必须让对应测试失败。**
3. **三机型同一组向量全过**，且**没有为任何一型改过期望值**——用 `git diff` 证明 JSON 未变；
   `r2_vectors.rs` 的观察函数只保留一份，不得为机型复制释放序列解析逻辑。
4. **释放事件序列三机型完全一致**：`releases` 含 `(scope, exit, value, kind)` 序列，
   这是活析正确性的主要证据。**故意破坏活析（例如忽略 `RunReleasePlan` 的间接读取、
   或忽略异常边）必须让向量失败**——**这是本批最重要的一次区分度验证。**
5. **`Poly` 与 `None` 有专门路径**：构造一条合流点退化为 `Poly` 的程序，验证它落帧槽，
   而不是被当作普通类别分配。
6. **分配确定性**：同一输入跑两次，分配表逐位相同。
7. **编码可往返**：全部 31 个 `TacOp` 变体在 `TAC → bytes → TAC` 中等价（`f64` 按位比较）；
   **LEB128 与定宽 `u16` 两种操作数宽度都往返成功**，体积数字可复现。
8. **编码拒绝路径完整**：版本不匹配、`ConstId`/`SigId`/`FuncId`/`BlockId` 越界、未知 opcode、
   截断输入、定宽溢出、非法字符串长度和非法 `ReleaseActionKind` 都返回结构化编码错误，**不得静默降级**；
   只有执行边界把损坏编码转换为既有 `FATAL_CORRUPT_ARTIFACT_CODE`。
9. **`pc -> span` 可反解**：给定 pc 得到正确 `IrSpan`；`bytecode_offset` 里装的是物理 pc 而非源码偏移
   ——**构造一个源码偏移与 pc 数值不同的反例**，并按每种编码宽度分别断言。
10. **H1 有结构化结论**：故障来源不再依赖单个可覆盖字段，并有嵌套 finally 内层失败回归；
    H2 有 clone 前后可复现的分配/指令基准数字。
11. **指标口径独立**：`spill_count`、`stack_map_entries`、`call_save_count` 不合并，
    `max_stack_depth` 文档已机型中立化；同一输入两次分配表逐位相同。
12. **不算假通过**：新增 `pub` 项 100% Rustdoc；不用 `#[allow]` 掩盖。

---

## 七、风险与未决

1. **活析 + 寄存器复用是本批最大的正确性风险。** 若发现信息不足以安全复用，
   **降到保守方案并登记**。三机型向量是安全网，**不是许可证**。
2. **本批跨度大**（8 次提交跨 3 个 crate）。历次教训：**跨层 bug 只在接起来时才显现**，
   所以每批都必须有端到端向量，不能只靠单元测试。
3. **编码器必须零新依赖**——字节写入手写在 `Vec<u8>` 上（`to_le_bytes` / `push` / 手写 LEB128）。
4. **H1 的子程序故障来源**已与 `pending_exits`、`active_subroutines` 通过
   `subroutine_faults` 栈成对维护；后续修改不得退回单值状态。
5. **H2 的 clone 清理**已完成并留下基准数字：移除前/后中位数为 `253.66 ms` /
   `245.83 ms`，R3 仍需按正式协议重测。
6. **编码后的 handler 区间与 pc 映射可能漂移**：块重排、长度前缀或两种操作数宽度任何一处
   不一致，都会让异常路由和错误堆栈指向错误位置；编码器必须先固定块目录，再编码 handler/映射。
7. **不许碰 `.xiaoc`**：研究编码不得冒充公开格式，不得用该扩展名落盘。
8. **`CategoryMap` 的 `Poly` 语义**（H7）与 **C0 的修复**会互相影响：
   修 C0 时要想清楚逐函数表里 `Poly` 是否还承担「合流点退化」的含义——
   **两者不要混为一谈**。
9. **错误对象的身份语义不能被载体复制策略破坏**：参数传递、跨调用保存和 spill/reload
   可能增加句柄引用，但不能把同一 `XiaoError` 变成“内容相等但身份不同”的新对象；
   这项要用 catch 重抛和 `suppressed` 向量锁定。

---

## 八、不要重复做的事

以下已在前面批次修完，**不要重新调查或「再修一遍」**：

- `a = b` 的所有权语义（已有 `TacOp::Copy`；临时值 `Move`、具名绑定间 `Copy`）
- 反引号名与普通名的区分（`name_key(name, backticked)`）
- 一元 `not` 与一元负号（对 `false` 比较 / 生成 `0 - x` 且零常量同宽度）
- 数值提升的桥（降低器按类型层规则插显式 `Cast`）
- `*args`/`**kwargs`（记入 `unsupported`，不静默当普通实参）
- 字符串转义的三份实现（已统一到 `decode_string_literal` / `decode_escape`）
- 字段写入判定（`runtime_value_matches` 复用 `can_assign`）
- 错误类型名单一来源（`xiao_diagnostics::error_kind_of`，`ef5b149` 刚收敛完）
- 4 个无 harness 的快照（已有 `c0c1_snapshots.rs`）
- 跨 crate 重名常量（已加 `CONFIG_` 前缀）
- 3 个死码（`X03-PARSE-003`、`X03-TYPE-007`、`X04-TYPE-008` 已删，文档承诺已清）
- 诊断重复上报（同码同跨度只保留首次）
- 18 个测试文件的诊断码字面值（已全部改为常量引用）
- `ExitKind` 拼写锁、Fatal 非对称、解耦叶子测试（都已有区分度验证）
- 09R2C 已交付的错误类型单一来源、`RuntimeValue::Error` 身份语义、handler 路由、
  `finally` 子程序、`Raise`/`Check` 和 `suppressed` 合并规则；R2D 只做载体适配与编码映射，
  不重新实现这些语义。
- `dynamic_conversion` 的窄语义和其他 RuntimeCheck 的 `unsupported` 边界；不要在寄存器机型
  中偷偷扩展检查类别。
- 09R2C 已覆盖的嵌套 finally 故障回归；只要后续改动影响 `subroutine_faults` 或子程序上下文，
  就重新运行并保留该回归，不要把它当成已完成的 R2D 测试替代品。
