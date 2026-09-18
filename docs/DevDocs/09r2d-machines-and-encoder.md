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

`main` 上 HEAD 为 `ef5b149`，门禁全绿。09R2 已交付四批：

| 批次 | 内容 | 提交 |
| --- | --- | --- |
| 批次 1 | TAC 模型与降低、对账验证器、栈式解释器、语义向量 | `51ceaac` `efe9789` `f035279` `a9b68ab` |
| 批次 2 | 容器运行时对象、精确索引、临时值释放 | `1f8d560` `a93fd3f` `885a278` `7dc998a` |
| 跨层审计 | 12 项缺陷 + 契约清理 | `7a67af3` `3f6aa94` `85a11d8` `3d1956f` `59a0317` `2bb5dc8` `ccba90a` `e900f3b` |
| 09R2C | 错误对象、handler 路由、finally 子程序、Raise/Check | `d733522` `76e3533` `ef5b149` |

**现状**：语义核 `xiao-vm/src/research/semantics/exec.rs`（约 1200 行）只支持栈式载体；
`Carrier` trait 有 6 个方法；`run.rs` 的 `run()` 硬编码 `Vm::<StackCarrier, _>`；
全仓**没有任何编码器代码**，也**没有 pc 概念**。

### 本批交付与不负责

**交付**：`C0` 逐函数类别映射修复、`Carrier` 接口演进、活跃区间分析、分类型寄存器机型、
混合式机型、`VmMetrics` 机型中立化、三机型共用向量对拍、指令编码器（含两种操作数宽度变体）、
`pc -> IrSpan` 源码映射与接线、文档同步。

**不负责**：R2b 选择器全量（多选/区间/步长/随机）、`for` 与表声明、正式 `.xiaoc` 格式
（分段、分区编号、内容寻址、Protobuf 索引归 14/16 阶段）、JIT/PGO/LTO、
**09R3 的基准测量本身**（本批只负责让三种机型可比）。

---

## 一、上一阶段缺陷档案

**这一节是本批最重要的部分。** 同类缺陷在上一阶段反复出现，其中一类在「专门修它的那一批」里
**又犯了一次**。以下每条都是已发生的事实，不是推测。

### 1.0 C0：本批必须先修的阻断性缺陷（新发现，未修）

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

### 1.5 流程性事实（不是代码缺陷，但会影响你）

- **`76e3533` 的提交正文为空**（800 行执行核心），违反「正文说明为什么」的规定。
  **不重写历史无法修复**，接手时不要试图回填。
- 近期有**两处文档描述与代码不符**：一处声称某表的消费者存在（实际没有），
  一处把未交付项列在「已交付」下（本批要修的就是这处，见任务 8）。
- **教训**：这个仓库的文档承诺会被 `bun run check` 当作契约校验。写文档时要与代码同步核对。

### 1.6 潜在隐患（**未证实也未证伪**）

| # | 隐患 | 为什么危险 | 本批怎么处理 |
| --- | --- | --- | --- |
| H1 | `Frame.last_sub_fault` 是单个 `Option<BlockId>`，而 `active_subroutines` 是栈 | 嵌套 `finally` 内层失败时，外层子程序的来源信息可能被覆盖 → 错误路由到错的退出边 | **本批必查**：构造「`finally` 里再套 `try/finally` 且内层抛错」的向量 |
| H2 | `run_blocks`（`exec.rs:221`）与 `run_subroutine`（`exec.rs:560`）**每执行一个块都 `instructions.clone()`** | 每块一次 `Vec` 分配，会**污染 09R3 的性能基准** | 本批清理，清理前后各测一次并留下数字 |
| H3 | `RuntimeValue::Hash` 对错误对象只哈希判别式 | 契约上允许（不等者可同哈希），但若有人依赖哈希区分会静默错 | 已登记为**已知语义，不要「修」** |
| H4 | 静态说栈值、运行时按堆物化的元组 | 语义可能不等价 | 本批不动，记入风险 |
| H5 | `escape.rs:887-901` 把 `ExitKind::Fatal` 列为可被 catch 吃掉，与 07-B 冲突 | 依赖它会把 Fatal 吞掉 | **按 07-B 实现，不改 `xiao-lifetime`**（已登记） |
| H6 | 容器按**对象身份**相等，结构性 `==` 未实现 | 两个内容相同的数组判不等 | 已知语义，不要「修」 |
| H7 | 合流点 `CategoryMap` 退化为 `Poly` | `Poly` 必须落帧槽，**寄存器机型若把它当普通类别分配会出错** | 本批硬性要求：`Poly`/`None` 有专门路径 |
| H8 | 参数 clone 进被调帧后的引用计数是否平衡，未做峰值内存验证 | 09R3 有「峰值内存恶化 ≤10%」门槛 | 本批不验证，留给 09R3 |

### 1.7 排除指南（症状 → 先查哪里）

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

### 2.2 工具规定

- 新建文件用 `Write`，改文件用 `Edit`。**不要用 shell heredoc / Python 替换改含中文或转义的
  源码文件**——本仓每个文件都有中文 Rustdoc，命中率极高，且失败是**静默**的（见 1.7 第零条）。
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

### 任务 1：`Carrier` 接口演进

现状 6 个方法：`empty/read/write/take/depth/peak`，其中 **`depth()` 是死代码**（全仓无调用点）。
四个已确认的缺口：

1. **`C::empty()` 无参数是阻断性的**：寄存器机型必须知道每个 `VReg` 的类别，载体现在完全拿不到
   `CategoryMap`/`TacFunction`/`TacProgram`。构造必须改为能拿到这些。
2. **无溢出通道**：没有 `spill`/`reload`，没有「寄存器文件 + 独立帧槽区」两段存储概念。
3. **无跨调用保存/恢复**：载体生命周期被钉死在单帧内（`exec.rs:163` 每帧构造，
   `exec.rs:185` 随帧 pop 丢弃），R1-L/O 的保存规则无处落地。
4. **指标出口只有 `peak()`**，无法上报 `spill_count` 与「栈映射条目数」。

**设计要求**：

- 构造改为接收一个**机型无关的上下文结构体**（含当前函数、**逐函数**类别映射、整份产物、调用深度），
  而不是加一串长参数——后续增加输入不会波及所有实现。
- 溢出与重载是**寄存器机型内部的分配决策**（语义核没有对应指令），**不要放进 trait**；
  跨调用保存/恢复只在语义核知道「一次调用开始/结束」时发生，**这个要进 trait**，用默认空实现
  让 `StackCarrier` 零成本。
- 指标经一个**机型中立的统计结构**上报，不要逐机型加 getter。其中「峰值内存」「溢出次数」
  「栈映射条目数」「调用保存次数」必须是**独立字段**——R1-F 明确后两者的语义不同
  （栈式必须在每个跳转目标都能验证栈深，寄存器式完全不需要），**不可合并**。
- 空寄存器的错误必须由**三机型共用的构造函数**产生，保证同一份向量在三台机型上得到
  **同一个稳定错误码**，否则同一份 JSON 无法对三台各断言一次。
- 删掉或改造 `depth()`。每个方法写中文 Rustdoc。

**改动面**：`exec.rs` 只有 `:163`（构造）与 `:185-187`（指标）必须动。

### 任务 2：活跃区间分析（**本批最危险的一块**）

**没有隐式 fallthrough**：`run_blocks` 的 `Flow::Next => return Ok(None)` 意味着
**块不显式跳转就结束函数**。所以 **CFG 后继 = 块内所有指令的显式跳转目标之并集**
（`Jump` / `BranchIf` / `Check.on_failure` / `CallSub.sub`），与 `verify.rs` 的 `jump_targets`
是同口径——**必须复用同一份函数，不要另写一份**（否则就是又一次 A 类缺陷）。

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
（给定函数与产物 → 区间表），**不要把它埋进寄存器载体的实现里**。

**安全网**：三机型跑同一组 `tests/spec/09-bytecode/*.json`（当前 **27 条**：
`containers` 8 / `errors` 9 / `scalar` 6 / `control` 4），期望值含完整 `releases` 事件序列
——分配器算错会直接表现为向量失败。**这是本批的主要保障，不要绕过它。**

### 任务 3：分类型寄存器机型

按 R1-H 的类别区间分配物理寄存器。**结构化位与分配表放在载体内部，不要加到 `Frame` 上。**

**必须遵守**：

- **`Poly` 与 `None` 有专门路径**（`Poly` 落帧槽，`None` 零宽不占位）——见 H7。
- **分配必须确定性**（同一输入同一分配），否则三机型对拍会引入噪声。
- **寄存器复用是本批的明确要求**：按活跃区间复用完即死的物理寄存器。

### 任务 4：混合式机型

按 R1-F 实现。在代码文档里明确它与另外两型的**具体差别**，并实现「栈映射条目数」的上报。
注意它与 `spill_count` **必须是独立字段**。

### 任务 5：`VmMetrics` 与三入口

`run.rs:65-76` 的 `VmMetrics` 扩展必须**保持 `Copy`**（`metrics()` 是 `pub const fn`，
按值返回）。新增溢出次数与栈映射条目数两个独立字段。

`max_stack_depth` 的 doc「载体槽位的历史峰值」是栈式措辞：**保留字段名以免破坏现有调用方，
但把注释改成机型中立措辞**，并让新机型上报其自身有意义的峰值。

`run.rs:91-92` 的 `run()` 硬编码 `Vm::<StackCarrier, RecordingSink>`。三机型要复用同一组向量，
`r2_vectors.rs:77` 必须**泛型化**——**JSON 零改动**（`Expectation` 只含
`outcome`/`error_code`/`releases`/`max_call_depth`，全部机型无关），让同一份期望对三台机型各断言一次。

**顺带清掉 H2**：`run_blocks`（`exec.rs:221`）与 `run_subroutine`（`exec.rs:560`）的
每块 `instructions.clone()`。克隆的是 `TacInstr`，被借的是来自 `self.program: &'p TacProgram`
的 `&'p TacFunction`，与 `&mut self` 正交——**这个 clone 借检器不需要**
（这是静态推理，**需要编译验证**）。先测清理前后的基准，**留下数字**，因为它影响 09R3 的口径。

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
这些都是体积杠杆。

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

### 任务 7：`pc -> IrSpan` 源码映射与接线（**最后一个提交**）

R1-Z 要求源码映射是独立于指令流的 `pc -> IrSpan` 表、增量编码。现状是**全仓没有 pc 概念**：
`exec.rs:229`/`:567` 直接把 `instruction.span.start`（**源码字符偏移**）填进
`BackendLocation.bytecode_offset`，**名实不符**（代码注释自承是「尚未冻结物理编码前的权宜」）。

**交付**：编码器产出 ① 每条指令的起始 pc；② 由 pc 序列 + `span` 生成的 delta 表；
③ `pc -> IrSpan` 反解。然后把 `exec.rs` 的两处调用改为**查表取真 pc**。
做成**只读查表入口**，不要在热路径里重建表。
既有断言 `xiao-vm/tests/r2_stack.rs:843,860` 需一并更新。

### 任务 8：文档同步

- **修掉 `09r-bytecode-machine-research.md:498` 的「已交付并登记：」小节**：它把
  `for` 与表声明、两种机型、指令编码器三项列在「已交付」下，但**三者都没有交付**
  （`lower/stmt.rs:129` 仍把它们记入 `unsupported`；全仓无任何编码器代码）。
  标题应改为「**已登记、尚待交付**」并逐条注明真实现状。
- 本批交付记录、两个 crate 的 README、`research/README.md`、DevDocs 主表、`12-tests-and-milestones.md`。
- `docs/module-registry.json`：编码器与两个新载体的 `code`/`tests` 登记。
- **登记 H1/H2 的结论**：查出是缺陷就修，不是就记为「已排查」——**不要留悬案**。

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

---

## 五、提交切分

按可独立验证的单元分八次，**每次提交后门禁全绿**：

0. **C0**：逐函数类别映射（或全程序唯一编号）+ 跨函数用例。
1. **`Carrier` 接口演进**（含删 `depth()`）、`StackCarrier` 适配、`exec.rs` 构造与指标两处改动。
   **此提交后栈式行为必须逐条不变。**
2. 活跃区间分析（机型中立的纯函数 + 单元测试），复用 `jump_targets`。
3. 分类型寄存器机型 + 单元测试。
4. 混合式机型 + 单元测试。
5. 三机型共用向量对拍（`r2_vectors.rs` 泛型化）+ `VmMetrics` 扩展 + 清 H2 的 clone（附基准数字）。
6. 指令编码器 + 往返解码 + 两种操作数宽度变体 + 体积对比。
7. `pc -> IrSpan` 映射 + `exec.rs` 接线 + 更新既有断言。
8. 文档同步（含修掉 `09r-...md:498`）。

**若中途必须停**：0–2 与 6 各自完整可验证，不会留下半成品。

**第 3–4 步若发现 `TacFunction` 提供的信息不足以安全复用**（例如活跃区间跨异常边无法精确闭合），
应当**降到保守方案（按类别全量静态分配、不复用）并明确登记**，
**而不是硬做出一个不确定的分配器**。

---

## 六、验收标准

命令见 2.4。**关键验收不是「测试通过」**：

1. **C0 被真正修掉**：两个函数各自使用同编号但不同类别的寄存器，**前一个函数的类别不被后一个
   改写**。**撤掉修复必须让该用例失败。**
2. **栈式行为零漂移**：第 1 次提交后，现有 27 条向量与 `r2_stack.rs` 逐条不变。
   **撤掉任何一处适配必须让用例失败。**
3. **三机型同一组向量全过**，且**没有为任何一型改过期望值**——用 `git diff` 证明 JSON 未变。
4. **释放事件序列三机型完全一致**：`releases` 含 `(scope, exit, value, kind)` 序列，
   这是活析正确性的主要证据。**故意破坏活析（例如忽略 `RunReleasePlan` 的间接读取、
   或忽略异常边）必须让向量失败**——**这是本批最重要的一次区分度验证。**
5. **`Poly` 与 `None` 有专门路径**：构造一条合流点退化为 `Poly` 的程序，验证它落帧槽，
   而不是被当作普通类别分配。
6. **分配确定性**：同一输入跑两次，分配表逐位相同。
7. **编码可往返**：`TAC → bytes → TAC` 与原产物等价（注意 `f64` 的比较口径）；
   **两种操作数宽度变体都往返成功**，体积数字可复现。
8. **版本不匹配被拒绝**：改 `bytecode_abi_version` 后解码必须报错，**不得静默降级**。
9. **`pc -> span` 可反解**：给定 pc 得到正确 `IrSpan`；`bytecode_offset` 里装的是 pc 而非源码偏移
   ——**构造一个源码偏移与 pc 数值不同的反例**，断言取到的是 pc。
10. **H1 有结论**：嵌套 `finally` 内层失败的向量跑通，或明确指出缺陷并修复。
11. **不算假通过**：新增 `pub` 项 100% Rustdoc；不用 `#[allow]` 掩盖。

---

## 七、风险与未决

1. **活析 + 寄存器复用是本批最大的正确性风险。** 若发现信息不足以安全复用，
   **降到保守方案并登记**。三机型向量是安全网，**不是许可证**。
2. **本批跨度大**（8 次提交跨 3 个 crate）。历次教训：**跨层 bug 只在接起来时才显现**，
   所以每批都必须有端到端向量，不能只靠单元测试。
3. **编码器必须零新依赖**——字节写入手写在 `Vec<u8>` 上（`to_le_bytes` / `push` / 手写 LEB128）。
4. **H2 的 clone 清理**会改变基准数字：**先测后改**并留下数字，因为它影响 09R3 的口径。
5. **不许碰 `.xiaoc`**：研究编码不得冒充公开格式，不得用该扩展名落盘。
6. **`CategoryMap` 的 `Poly` 语义**（H7）与 **C0 的修复**会互相影响：
   修 C0 时要想清楚逐函数表里 `Poly` 是否还承担「合流点退化」的含义——
   **两者不要混为一谈**。

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
- 09R2C 已交付的 handler 路由、`finally` 子程序、`Raise`/`Check`
