# 20D-资源承载. `ExternalResource` 的 Runtime 变体

> **背景**：[20D-冻结](20d-freeze-resource-contract.md) 已把契约冻死——
> 首批种类 `file`/`directory`、**共享强引用**、关闭幂等、容器/产物边界、
> 跨模块用 `XiaoValueTag::ExternalResource = 17`。
> 本批是 **20D 的第三步**（裁定顺序：冻结 → 能力上下文 → **资源承载** → 首批 API），
> 要做的是**新增 `RuntimeValue` 变体**。
>
> **而方向稿 §4.2 明写**：「当前 `xiao-runtime` 的 `is_hashable` 使用**排除式匹配**，
> 新变体若漏加分支反而可能被误判为可哈希」——**这一批的全部风险就在这里**。

状态：**已实施（2026-10-17）并审核通过**：§六 三道**全部裁定**（均取甲——都选失败安全的一侧），
§三 九处检查点均已接线并有回归；本批仍未接任何 `os.*` intrinsic。
审核证据与三条尾账见 §十一。

## 一、为什么是这一批

1. **顺序**：[20D-规划](20d-platform-resources-stage-plan.md) §4.2 的裁定
   **不可倒**——承载必须在**第一个资源 intrinsic** 之前（方向稿 §4.1 末句、§4.3）；
2. **它是「新变体」最容易静默出错的一批**：§三 已把**九处检查点**逐处核过，
   其中**四处编译器不会提醒**；
3. **不先承载就写 API，等于把变体的每个检查点补两遍**。

## 二、冻结已定死的（**输入，不许重新论证**）

| 项 | 冻结内容 | 出处 |
| --- | --- | --- |
| 承载形态 | **一个统一的 `ExternalResource` Runtime 承载**，内部带稳定种类标签 | [20](20-builtins-and-standard-library.md) §4.1 |
| 首批种类 | **`file` + `directory`**（`process` 不在首批） | [冻结](20d-freeze-resource-contract.md) §2.1 |
| 复制语义 | 赋值/传参/返回/模块导出**复制同一个逻辑资源的强引用**，不复制底层对象 | 同上 §2.3 |
| 关闭 | 幂等；关闭后任何资源操作统一报 **`X06-RUNTIME-025`** | 同上 §2.2 |
| 容器边界 | **数组/元组允许**；**集合元素/字典键拒绝** | 同上 §2.3 |
| 产物边界 | 禁止进 `const`/字节码常量池/`.xiaoc`/`.xar`；源级 `X02-TYPE-008`，产物级 `X07-FATAL-002` | 同上 §2.3 |
| ABI | `XiaoValueTag::ExternalResource = 17`（只追加），载荷 `XiaoHandle`，**不暴露宿主裸指针** | 同上 §2.4 |
| 可见性 | 能力上下文**不作为用户值**；资源**是**用户可见的种类，但句柄内部不可见 | 同上 §2.7、§4.1 |

**本批不接任何 intrinsic**。`os.open_file` / `os.close` / `os.read_text` 是**下一批**的事。

## 三、**新增变体的九处检查点**（审核侧逐处核过）

方向稿 §4.2 要求覆盖「相等 / 哈希性 / `type_name` / `scalar_type` / 编码器 / 常量池 / 两条后端」。
我把 `RuntimeValue` 的**实际匹配点**逐个找出来，分成两类——四处静默点加五处编译器强制点，
合计**九处**；这个分类就是本批的检查清单：

### 3.1 **静默的四处**——编译器**不会**提醒，漏了就是错结果

| # | 位置 | 为什么静默 | 漏改的后果 |
| --- | --- | --- | --- |
| 1 | `xiao-runtime/src/containers/mod.rs:36` `is_hashable` | **排除式** `!matches!(…)` | 资源**被判为可哈希**，可进集合/字典键——**违反冻结的容器边界** |
| 2 | `xiao-runtime/src/value/mod.rs:337` `type_name` | 末尾 `_ => self.scalar_type().map_or_else(\|\| "dynamic", …)` | 资源类型名**静默变成 `"dynamic"`**——而冻结明写「不能落入 `dynamic`」 |
| 3 | `xiao-runtime/src/value/mod.rs:205` `PartialEq` | 末尾 `_ => false` | **同一个值和自己比也不相等**（`x == x` 为假），静默 |
| 4 | `xiao-runtime/src/abi.rs` `value_to_runtime` | 末尾 `_ => Err(InvalidArgument)` | 新标签**过不了 ABI 反向转换**，资源无法从 C 侧回来 |

**这四处必须显式加臂，而且每一处都要有专门的用例**——`cargo` 不会因为它们漏改而报错。

### 3.2 **编译器会强制**的五处

| # | 位置 | 机制 |
| --- | --- | --- |
| 5 | `value/mod.rs:244` `Hash` | 显式列出全部变体，无 `_` |
| 6 | `value/mod.rs:308` `scalar_type` | 显式列出全部变体，无 `_` |
| 7 | `value/mod.rs:284` `display_text` | 显式列出全部变体，无 `_` |
| 8 | `xiao-runtime/src/abi.rs` `runtime_to_value` | 显式列出全部变体，无 `_` |
| 9 | `xiao-driver/src/protocol/mapping.rs:239` | 显式列出全部变体，无 `_` |

**这五处不需要「记得改」——不改就编译不过。** 本批的要求是：**改的时候顺手确认它们的语义**，
而不是靠编译器提醒然后随手填一个占位。

## 四、本批要做的事

1. **新增 `RuntimeValue::ExternalResource(...)`**：内部承载 = 强引用计数 + **稳定种类标签**
   + **关闭状态**；**单线程**（方向稿 §4.1 第一条）；
2. **逐处处理 §三 的九个检查点**：四处静默的要显式加臂（§六 的两道裁定决定其中两处的语义），
   五处编译器强制的按语义填写；
3. **关闭语义**：显式关闭与最后一个强引用释放**幂等**；关闭后任何资源操作报
   `X06-RUNTIME-025`（`XiaoError::capability_unavailable` 是**另一个码**，别混）；
4. **ABI 标签**：`value_to_runtime` 接受 `ExternalResource = 17`，
   `runtime_to_value` 产出它；**不暴露宿主裸指针**；
5. **产物边界**：资源**不得**进常量池 / `.xiaoc` / `.xar`——
   源级走 `X02-TYPE-008`、产物级走 `X07-FATAL-002`（冻结已定），**要写可执行的拒绝**；
6. **回归**：释放后 / 关闭后 / 失效后的错误路径各有用例；
   **容器边界**（数组/元组允许、集合/字典键拒绝）各有用例。

## 五、不许

1. **不许**在 `is_hashable` 里「顺手加一个排除分支就交差」而不做 §六 的裁定——
   方向的取舍决定**将来每个新变体**的行为；
2. **不许**让 `type_name` 落进 `_ => "dynamic"`：**必须显式加臂**；
3. **不许**让 `PartialEq` 对同一资源返回 `false`（`x == x` 必须为真）——
   这是 `Eq` 契约的最低要求，而 `_ => false` 恰好会违反它；
4. **不许**把资源做成 `Send`/`Sync` 或跨线程（方向稿 §4.1：**单线程 Runtime 对象**）；
5. **不许**在本批接任何 `os.*` intrinsic——**那是下一批**；
6. **不许**让资源进入常量池 / `.xiaoc` / `.xar` 而只靠文档声明——冻结要求**可执行的拒绝**。

## 六、三道裁决（已落定）

### 6.1 裁决一：`is_hashable` 改成**正向允许**，还是**加专门拒绝分支**

方向稿 §4.2 原文给了两条路：「实现时应**改为正向允许标量/`none`**，或**至少为资源加入专门拒绝分支**」。
现有 Runtime 还把模块命名空间和函数身份作为内部可哈希值保留；正向表必须同时保留这两个既有分支，
否则会把资源批误改成模块身份语义变更。

| | 做法 | 代价 |
| --- | --- | --- |
| **甲** | **改成正向允许**：把语言层标量、`none` 和既有内部模块身份列为可哈希，其余一律拒绝 | 一次改对，**将来每个新变体默认不可哈希**（失败安全）；但要逐个确认既有变体的判定不变 |
| **乙** | **只给资源加拒绝分支** | 改动最小；但**下一个新变体还会再踩一次**——这正是方向稿警告的复发形状 |

**审核倾向：甲。** 理由：这是**失败安全**的方向——忘了处理新变体时，它**不可哈希**（安全侧），
而不是可哈希（危险侧）。乙 只是把这次的坑填了，坑还在。

**裁定（星崽，2026-10-17）：甲——改正向允许。** 连带一条：
**既有 19 个变体的判定必须逐个确认不变**——`is_hashable` 从「排除谁」改成「允许谁」，
是一次**语义反转**，任何一个既有变体判错都会**改变既有程序的行为**。
这与「新增一个变体」不是同一个风险等级，**验收要单独一条**（§八 第 4 条）。

### 6.2 裁决二：**同一资源的两个别名**算不算相等

`PartialEq` 的 `_ => false` 会静默生效，所以这条必须显式定。

| | 做法 | 说明 |
| --- | --- | --- |
| **甲** | **算相等**：同一逻辑资源的任意别名 `a == b` 为真 | 与冻结的「共享同一逻辑资源」一致；但要求 `Hash` 也按逻辑资源身份（而 §4.2 说资源不可哈希，所以不冲突） |
| **乙** | **算不相等**：资源相等只对同一 `RuntimeValue` 句柄成立 | 更保守；但用户把同一个文件的别名传进两个函数后比较会得到「不等」，**反直觉** |

**审核倾向：甲。** 理由：冻结把「克隆句柄的语义是**共享同一逻辑资源**」写进了最低契约；
相等判定应当与它一致，否则用户看到的语义会分裂。

**裁定（星崽，2026-10-17）：甲——两个别名算相等。** 连带两条：

1. **相等按「逻辑资源身份」判**，不是按 `RuntimeValue` 的包装地址；
2. **`Hash` 不受影响**：资源本就不可哈希（裁决一之后是**正向不允许**），
   所以 `Hash`/`Eq` 的自洽约束在这里**不产生**额外要求——这一句要写下来，
   免得实现侧有人为了「自洽」去给资源补一个哈希。

### 6.3 裁决三：`display_text` 印什么

资源不是标量，`display_text` 用于 `print` 与整数/文本转换路径。三种写法：

| | 印什么 |
| --- | --- |
| **甲** | `resource<file>` / `resource<directory>` —— 与 §4.2 要求的 `type_name` 同形，**不泄露句柄** |
| **乙** | `"<file>"` —— 更短，但和 `protocol/mapping.rs` 里容器用的 `<{kind}>` 风格混在一起，看不出是资源 |
| **丙** | 走 `Debug` —— **不行**：会打印内部结构，而 `Table`/`Array` 现在就是 `format!("{self:?}")`，资源**不能照抄** |

**审核倾向：甲。** 理由：冻结要求 `type_name` 是 `resource<kind>`；显示用同一形状，
用户就不会在「类型名」和「打印值」之间看到两套说法。**丙 明确排除**。

**裁定（星崽，2026-10-17）：甲——印 `resource<file>` / `resource<directory>`。**
连带一条：**`display_text` 与 `type_name` 必须用同一份 `kind` 文本**——
两处各拼一次字符串就是「同一规则两处各写一份」（这一轮已经吃过两次：判零规则、
`lower_ascii` 的内联）。

## 七、落点

```text
docs/DevDocs/20d-resource-carrier.md                  本文
core/rust/crates/xiao-runtime/src/value/mod.rs        RuntimeValue 变体 + PartialEq/Hash/scalar_type/type_name/display_text
core/rust/crates/xiao-runtime/src/containers/mod.rs   is_hashable
core/rust/crates/xiao-runtime/src/abi.rs              value_to_runtime / runtime_to_value + XiaoValueTag::ExternalResource
core/rust/crates/xiao-runtime-abi/src/lib.rs          标签常量（= 17，只追加）
core/rust/crates/xiao-driver/src/protocol/mapping.rs  显示映射（编译器强制）
02-type-system.md / 03-collections.md                 若裁定的语义需要写回规格
docs/DevDocs/README.md                                阶段索引加本文一行
```

## 八、验收

1. **九处检查点逐处有处理**：四处静默的**各有一条专门用例**（不能只靠「实现了」），
   五处编译器强制的**语义被确认**（不是随手填占位）；
2. **`type_name` 不落 `dynamic`**：资源返回 `resource<file>` / `resource<directory>`；
3. **`x == x` 为真**：同一句柄自比必须相等；
4. **不可哈希**：资源进集合 / 字典键被拒——**可执行的拒绝**，不是文档声明；
   **并且**：`is_hashable` 改成正向允许之后，**既有 19 个变体的判定逐个确认不变**
   ——这是一次**语义反转**，不是单纯加臂；要有一条**覆盖既有变体**的用例集，
   **不许**只测「资源被拒了」；
5. **关闭语义**：幂等、关闭后报 `X06-RUNTIME-025`、最后一个强引用释放同样幂等；
6. **不暴露宿主裸指针**：ABI 侧只用 `XiaoHandle`；
7. **没有接任何 `os.*` intrinsic**；
8. **通用**：`cargo test --workspace`、clippy、fmt、`bun test`、`bun run check`、`bunx tsc --noEmit` 全绿；
   提交标题带类型前缀、正文非空；凡改动 `README.md` 的 20D 行或本文，逐行跑一次粗体配对检查。

## 九、裁定结果（已执行）

| # | 问题 | 裁定 | 落在哪 |
| --- | --- | --- | --- |
| 1 | **`is_hashable` 的改法** | **甲：改正向允许**（标量、`none` 和既有内部模块身份可哈希，其余默认拒绝） | §6.1 |
| 2 | **同一资源的两个别名算不算相等** | **甲：算相等** | §6.2 |
| 3 | **`display_text` 印什么** | **甲：`resource<file>` / `resource<directory>`** | §6.3 |

**来源**：星崽直接裁定（AskUserQuestion，2026-10-17）；三条与审核倾向一致——
但按 [20C](20c-official-xiao-library.md) §2.4 **G-2**，一致**不等于**推定，本文以**直接裁定**记。

**三条的共同理由（写在纸面上）**：它们都在选**失败安全**的那一侧——
哈希改正向 ⇒ 忘处理的新变体默认**不可哈希**；别名相等 ⇒ 与「共享同一逻辑资源」一致，
不会让用户看到两套语义；显示用 `resource<kind>` ⇒ 与 `type_name` 同形，
用户不会在「类型名」和「打印值」之间看到两套说法。

## 十、实施记录与验收证据（2026-10-17）

- `RuntimeValue::ExternalResource` 使用单线程 `ExternalResourceHandle`，内部保存首批种类
  `file`/`directory` 和幂等关闭状态；不保存宿主裸句柄，复制只增加同一对象的强引用。
- `is_hashable` 已改为正向允许表，既有模块身份、标量、字符串和 `none` 的结果保持不变，
  资源、表、容器和错误值均不可哈希。
- `PartialEq` 按逻辑资源身份比较别名；`type_name` 与 `display_text` 共用
  `resource<kind>` 规则；关闭后 `ensure_open` 返回 `X06-RUNTIME-025`。
- `XiaoValueTag::ExternalResource = 17` 已加入 ABI 标签；`value_to_runtime`、`runtime_to_value`、
  值复制和释放路径均显式支持它，协议映射不泄露句柄地址。
- 回归覆盖资源别名相等、幂等关闭、关闭后错误、ABI 双向回转和哈希白名单（含既有 19 个变体）；没有新增或接通
  `os.open_file`、`os.close`、`os.read_text` intrinsic。

**验收状态：已满足。** 资源承载已就位，下一批才可以接入首批 `os` API。

## 十一、审核附录（2026-10-17）

审核对象：`9412d2b`（实现）与 `ce0a86d`（自查补丁）。方式是**自己跑**——读 diff、逐处核代码，
对四处静默点做**变异测试**（改坏 → 用例必须红 → 还原 → `git status --porcelain` 为空），
再重跑全量门禁。

### 11.1 通过的项（证据）

| 项 | 证据 |
| --- | --- |
| 九处检查点 | 逐处读到显式臂；`display_text` 与 `type_name` 共用 `ExternalResourceKind::display_name()`（`value/mod.rs:184`），没有两处各拼一份 |
| 既有 19 变体的判定 | 旧排除式排除的恰是 `Table`/`TableDropView`/`Array`/`Tuple`/`DictTable`/`DictColumn`/`Set`/`Error`；新正向表允许的恰是其余 11 个，**逐一相等**。用例 18 个在 `b06_runtime.rs:257`，`TableDropView` 由同文件 `:224` 覆盖，合计 19/19 |
| 四处静默点的用例是真的 | 变异：资源加进允许表 → 单元与容器边界**同时红**；删 `type_name` 资源臂 → 实际值 `"dynamic"`；`PartialEq` 改 `false` → 别名用例红；`value_to_runtime` 改 `Err` → ABI 回转用例红；从允许表删 `Module` → 19 变体用例红。还原后五条用例全绿 |
| 关闭 | 两次 `close` 幂等、`ensure_open` 报 `X06-RUNTIME-025`（i18n 已登记 `runtime.resource_closed`）、别名逐个释放只触发一次 `Destroy` |
| 容器边界 | 数组/元组接受、集合**可执行地**拒绝（`CONTAINER_HASHABILITY_CODE`）；资源作**字典键**在当前 Runtime 不可达——`DictHandle` 的键类型是 `String`，属结构性拒绝而非漏测 |
| ABI | 标签 17 只追加；`owns_strong_handle` 含资源；copy/release 路径齐全；载荷只有种类与关闭位，**无宿主裸指针**；句柄继承单线程标记，未实现 `Send`/`Sync` |
| 没有 `os.*` | `xiao-intrinsics` 两次提交均未改动，全仓无 `os.open_file`/`os.close`/`os.read_text` |
| 门禁 | `bash tools/gates/run.sh` exit 0；`cargo test --workspace --no-fail-fast` **1161 passed / 0 failed / 34 ignored**；`bun test` 295 pass / 5 skip / 0 fail |
| CI | `9412d2b`：维护回归 `39010194102`、workspace 门禁 `39010194082`；`ce0a86d`：`39011432023`、`39011431951`——四次全成功 |

### 11.2 F-1（中）：产物边界的**可执行拒绝**没有落地

§四.5 与 §五.6 要求资源不得进入常量池 / `.xiaoc` / `.xar`，源级 `X02-TYPE-008`、
产物级 `X07-FATAL-002`，且**不许只靠文档声明**。两次提交都没有改动
`xiao-bytecode` / `xiao-xar` / `xiao-artifacts`；`FatalError::corrupt_artifact`
只有构造器与单测，没有生产调用点。

**实情是本批没有插入点**，这不是漏看：

- `TacConstant`（`xiao-bytecode/src/tac.rs:232`）是 8 个标量变体的**封闭枚举**，资源进不去；
- `resource` 类型还没有进入类型层，源级 `const` 根本写不出资源；
- VM 当前只消费**本进程刚生成的 TAC**，外部 `.xiaoc` 校验没有接入
  （`xiao-vm/src/run.rs:476-478` 的原注释）。

⇒ **处置**：在「20D-首批 API」让 `resource<kind>` 进入类型层时**一并实现**两条拒绝，
并把本条挂到该批的出口——已写进 [20D-规划](20d-platform-resources-stage-plan.md) §4.2。
§十 的「验收状态：已满足」只覆盖 §八 那八条，**不覆盖 §四.5**。

### 11.3 F-2（低）：裁定记录被事后改写，没记来源

- `9412d2b` 把 §三 的「八处」改成「九处」。**改得对**——原稿自己列的是
  #1–#4 + #5–#9 = 九处，标题却写八处；这是**审核侧自己的数错**（本会话第二次，
  上一次是「六件事」对应 A–G 七项）。
- `ce0a86d` 把 §6.1 的甲选项与 §九 裁定表由「只把标量与 `none`」改成
  「标量、`none` 和既有内部模块身份」。**实质也对**——旧排除式本来就允许
  `Module`/`ModuleFunction`，照原文字面实现反而会改掉既有语义。
- **问题在记录**：两处都是裁定/规划**之后**的补正，却没有一行说明来源，
  读起来像星崽当时裁的就是改后文本。按 [20C](20c-official-xiao-library.md) §2.4
  **G-2** 的同一标准，补正也要留痕——本节即留痕，**裁定方向未变**。

### 11.4 F-3（低）：ABI 次版本没有动，也没写是否故意

`XiaoValueTag::ExternalResource = 17` 已加入（`xiao-runtime-abi/src/lib.rs:326`），
`ABI_MINOR_VERSION` 仍是 13（同文件 `:18`），由能力上下文批的 `789d2a9` 升上来。

功能上无风险：标签是**只追加**，兼容检查是
`required_minor <= ABI_MINOR_VERSION`（`xiao-runtime/src/abi.rs:871`），
原生产物静态装入 Runtime，不存在「旧 Runtime 收到新标签」的路径。
⇒ **不必**单独升 1.14；但应补一句「标签随 1.13 发布，不再单独升次版本」，
免得后来的人以为漏升。本节即记录。

## 相关页面

- [20D-冻结](20d-freeze-resource-contract.md) §2.1–§2.4 —— **本批的直接前置**：种类、复制、关闭、ABI 都已冻
- [20. 内置函数与标准库](20-builtins-and-standard-library.md) §4.1/§4.2 —— 承载契约与**排除式匹配的警告原文**
- [20D-能力上下文](20d-capability-context.md) —— **上一批**：能力的载体与校验
- [20D-规划](20d-platform-resources-stage-plan.md) §4.2 —— 四批顺序的裁定出处
