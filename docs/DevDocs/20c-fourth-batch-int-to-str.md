# 20C-第四批. 整数族 → `str` 转换

> **背景**：[20C-第三批](20c-third-batch-reprobe-and-status.md) §三 的复探**四行全部仍被既定边界挡住**，
> 按星崽 2026-10-14 的裁定（§八第 1 条「由复探结果决定」）**选择乙**——
> 进入**语言批次**，最小范围**接通 `int`、`sint`、`lint` → `str`**（同裁定 §八第 2 条，
> 经星崽后续裁定三种整数一并接通）。
>
> **一句话概括**：把**已经存在**的整数文本表示**接通**成语言的显式转换
> （`integer as str` / `str(integer)`），两条路径一致、错误身份不变；
> **不扩到浮点格式化**，也**不顺手改常量折叠**。
>
> 状态：**已实施（2026-10-14）**。

## 一、为什么是这一批，为什么是这一个

1. **裁定链**：语言边界批 §六 冻结「复探结果决定下一批」→ 第三批复探四行全挡 → 选乙 →
   乙的最小范围经星崽裁定为 **整数族 `int`/`sint`/`lint` → `str`**；
2. **它直接解除数字格式化的一个前置**：数字格式化需要 `int → str`；大小写转换的夹具
   `str(int(c) + 32)` 还同时需要 `str → int` 和字符分类逻辑，因而本批不能预先宣称大小写转换会解封；
   `and`/`or` 的接通只清掉其中的逻辑阻塞（语言边界批 §2.4 已拆开登记）；
3. **它比另外两个候选面窄**：「容器参数类型」牵动类型系统与标准库 API 形状，
   「非字面量下标」牵动解析与类型两层——量级都不是一批。

## 二、并入的返工项（**先做；以下为实施前审计事实**）

出处：2026-10-14 审核 `b9c82e9`（第三批）。**三条都不是红**，但都属本批一路上在清的那一类
（「某件事到底做没做，文档里读不出唯一答案」；R-c 是同一缺陷的第三次，见该节）。

### 2.1 **R-a（中）：语言边界批 §六 整节没回填**

**事实**：第三批的落点清单列了「`20c-language-boundary-and-operators.md` **§六 复探结论回填**」，
而在本批实施前，该文 §六 **仍是前瞻写法**（「§四 落地后，**重跑**…」「**冻结**：1. 复探必须真跑…」），
**没有任何回填**；它的 §九 验收第 9 条「复探：四条候选逐条给出『现在能不能写』，成表」也**仍未填**。

⇒ 读者在语言边界批里看到的是「**复探待做**」，第三批却说「已做完且有 CI 支撑」——
**同一件事在文档里有两个答案**，与 B-1/B-2/B-3 同族。

**冻结**：二选一，实施记录写明选了哪个——

1. **回填**：把第三批 §三 的表与结论搬到语言边界批 §六（或改为指向），并勾掉 §九 第 9 条；
2. **明写承接**：在 §六 写清「复探由 [20C-第三批](20c-third-batch-reprobe-and-status.md) 执行，
   结果见其 §三/§五」，并把 §九 第 9 条标为「由第三批承接」。

**不许**两处都不动——那会让下一个人以为复探还没跑。

### 2.2 **R-b（低）：A-2 那行的落点写错位**

**事实**：第三批 §2.1 的 A-2 行写「回填语言边界批**状态头**（已完成）」，
实际回填的是该文的 **§1.2 表 B 与 §1.3**；它的**状态头**引的仍是 `38493485483`
（那是修复批的运行号，本身没错，但与 A-2 的措辞对不上）。

**冻结**：把 A-2 的落点改成「该文的表 B/§1.3」，或补一句说明状态头引的是哪一个运行号。
**只改措辞，不改事实。**

### 2.3 **R-c（低，但这是第三次）：`README` 的 `20C-首批` 行状态格 `**` 只开没关**

**事实（审核实测，含归因）**：`docs/DevDocs/README.md` 的 `20C-首批` 行状态格以
`**受控双路径已由维护回归…` 开头，**只有一个粗体标记、没有闭合对**——
渲染会把后面的内容一并吃掉。逐版本对账：

| 版本 | 该行 `**` 计数 |
| --- | --- |
| `8517677`（本批之前） | **2**（平衡） |
| **`b9c82e9`（第三批）** | **1（不平衡）** |

⇒ **这处是 `b9c82e9` 引入的**。

**性质**：**同一个缺陷的第三次**——[20C-语言边界](20c-language-boundary-and-operators.md) §2.4 末
已记过两次（`README` 的 20C 状态格粗体只开没关；`02e41f0` 又犯一次，**两次都在同一类格子里**）。
那时的结论是「**`bun run check` 不查这个**」，并把「要不要防空」留给星崽另定
（最小做法是把「状态格 `**` 奇偶」并进既有文档门禁，但那属**工具侧批次**）。

**冻结**：

1. **本条先修掉实例**：把该行的粗体**闭合**（或去掉这对标记）；
2. **连续第三次了——把它变成一条可复查的规矩**，二选一并在实施记录里写明：
   - **工具侧**：把「表格状态格的 `**` 奇偶」并进 `bun run check`（**这才是根治**，
     但属工具侧，需要星崽先裁定范围）；
   - **人工侧**：在提交前的自检清单里加一条「改过 README 状态格就数一遍 `**`」。
3. **不许**只修这一处、不留规矩——前两次就是这么过去的。

## 三、现状盘点（规划侧实测，2026-10-14；实施前快照）

**这一节的每一条都是跑出来或读出来的，不是推断。**

### 3.1 **能力已经存在——这是「补接通」，不是新语义**

| 事实 | 出处（实测） |
| --- | --- |
| Runtime 已有**每个标量**的稳定文本表示，含 `Int`/`Sint`/`Lint` | `xiao-runtime/src/value/mod.rs:284` `display_text()`——`Self::Int(value) => value.to_string()` 等 |
| Runtime 的转换入口已有 **`bool` → `str`** 这一格 | `xiao-runtime/src/value/ops.rs:219` `(ScalarType::Bool, ScalarType::Str) => { … }` |
| 类型层的转换矩阵已有 **`BoolToStr`** 这个 kind | `xiao-types/src/conversion.rs`（enum 变体与构造处） |

⇒ **整数 → `str` 是把已有的文本表示接通成语言转换**，与 [09B0](09b0-bytecode-closure.md) §3.4
**B0-A 的先例**（`StrToBool` 因「类型层语义已冻结」判为**补接通**，不是新增语言语义）同型。

### 3.2 **拒绝点（一行就够）**

`xiao-types/src/conversion.rs:67` `classify_conversion`：规划时的合法格是
Identity、`Str→Bool`、`Bool→Str`、数值↔数值（加宽/收窄）；
**整数 → `str` 落到末尾的 `Err`** ⇒ `X02-TYPE-006`（`INVALID_CONVERSION_CODE`）。
本批已将 `is_integer(source)` 的三个宽度接入同一 `IntegerToStr` 分类，错误码没有改变。

### 3.3 **一处必须先查清的既有事实**

**`ConversionKind::BoolToStr` 在整个 `core/rust/crates/` 里只出现两次**——
enum 变体与它的构造处，**下游没有任何地方消费它**。
⇒ 降低**不是按 kind 分派**的（否则 `bool as str` 早就走不通）。

**冻结（实施前）**：动手前**先跑一遍 `bool as str` 的既定先例**（两条路径）：
- **若它跑得通** ⇒ 它就是本批的**模板**，照它的分派路径接整数 → `str`；
- **若它跑不通** ⇒ 说明连 `bool as str` 也没接通，**先在本批把这件事记清楚**
  （它是否也是缺口、阻塞谁），**再**决定整数 → `str` 怎么接。

**不许**直接照抄 `BoolToStr` 这个未消费的 kind——那是「看起来有、实际没人用」的形状，
本仓库已因它吃过亏（[09B0](09b0-bytecode-closure.md) §3.5 的「字段存在 ≠ 有人在用」）。

**实施结果**：`bool as str` 的既定 VM/LLVM 路径均已实测通过，输出小写 `true`/`false`；
它确认了 Runtime 通用 `convert_to`/动态 `xiao_runtime_value_cast` 是本批应复用的路径。

## 四、范围冻结

1. **验收目标是整数族 → `str`**：`int`、`sint`、`lint` 均使用十进制、与 `display_text` 一致；
2. **三种宽度一并接通**：统一矩阵项 `IntegerToStr` 不触发运行时检查；夹具分别覆盖每个宽度的
   `as str` 与 `str(value)`，并在 VM/LLVM 原生两条路径验证；
3. **不许扩到浮点**（`float`/`sfloat`/`lfloat` → `str`）——舍入与精度口径**尚未冻结**，
   那是另一个面（星崽裁定的原话就是「不许顺手扩到浮点格式化」）；
4. **不许顺手改常量折叠**（整数字面量 → `str` 字面量）——那是语言边界批 §2.1 另记的缺口；
5. **不许新增 intrinsic**——`display_text` 已在 Runtime 里，本批是**接线**（[20C](20c-official-xiao-library.md) §3.4 第 6 条的规模纪律）；
6. **接口面**：本批只动**语言转换**，**不新增标准库函数**；`text` 模块的公共接口不变。

## 五、落点与实现要求

### 5.1 落点（链路上的每一环都要核，**不许只改类型层**）

```text
docs/DevDocs/20c-fourth-batch-int-to-str.md        本文
core/rust/crates/xiao-types/src/conversion.rs      `is_integer(source)` → `IntegerToStr`，三种宽度共用一格
core/rust/crates/xiao-runtime/src/value/ops.rs     `convert_to`：三种整数复用 `display_text()` 到 `str`
core/rust/crates/xiao-bytecode/src/lower/expr.rs   复用既有 `TacOp::Cast` 降低；无需新增指令
core/rust/crates/xiao-codegen-llvm/src/dynamic/expression.rs  动态 `ScalarCast` intrinsic 复用 Runtime ABI
core/rust/crates/xiao-codegen-llvm/src/ir.rs       静态 `str` 不进入固定宽度模块，本批不改此文件
core/rust/crates/xiao-runtime/src/abi.rs           复用既有 `xiao_runtime_value_cast` 目标分派
tests/fixtures/20c/                                `second-int-to-str.xiao`：三种宽度各测 `as`/`str`，另测动态值
docs/DevDocs/20c-language-boundary-and-operators.md  §2.1 的 R-a（§六 回填或明写承接）
docs/DevDocs/20c-third-batch-reprobe-and-status.md   §2.2 的 R-b（A-2 落点措辞）
docs/DevDocs/02-type-system.md                     转换矩阵的更新（若语义有新冻结）
docs/DevDocs/00-decisions.md                       若与既有转换基线有关，回填
```

### 5.2 实现要求（**冻结**）

1. **两条路径同批**：字节码与 LLVM **同一提交**落地；**不许**一侧先接通、另一侧留拒绝——
   那会让「两边接受边界一致」这条既有性质在本批**中途**被破坏；
2. **错误身份**：整数 → `str` **不新增运行时错误**（整数转文本不会失败）——
   所以这一格**不应**登记运行时检查；**若**实现里出现了 `requires_runtime_check = true`，
   **必须写清为什么**（那是「本可以静态完成却推给运行时」的信号）；
3. **`str(integer)` 与 `integer as str` 同一语义**（[00. 决策基线](00-decisions.md) 已冻结两种写法等价）——
   `int`、`sint`、`lint` 三种宽度的两种写法都要有用例；
4. **动态值边界**：`dynamic` 值转 `str` 走既有的 `RuntimeChecked` 路径，**语义仍取 `display_text`**；
   **不许**给动态值另一种文本表示（那会让静态与动态两条路的结果不一致）；
5. **不动的部分**：`X02-TYPE-006` 这个稳定码**不改**（它仍用于其它非法转换），
   本批只是让三个整数到 `str` 的矩阵格从非法变成合法。

## 六、验收

1. **R-a / R-b / R-c**：语言边界批 §六 已由第三批明写承接；
   §九 第 9 条已标；第三批 §2.1 的 A-2 落点已改正；
   `README` 的 `20C-首批` 行粗体已闭合，**且已按 §2.3 第 2 条留下一条防复发的规矩**；
2. **先例已查清**（§3.3）：`bool as str` 两条路径的实跑结论已写进实施记录；
   **若它跑不通**，缺口已登记（是否阻塞、归谁）；
3. **接通**：`int`、`sint`、`lint` 的 `as str` 与 `str(value)` 在**两条路径**上都能构建、运行，
   结果一致且等于十进制表示；每个宽度均有独立用例；
4. **不新增运行时检查**（或已写明为什么必须有）；
5. **范围**：没有浮点 → `str`；没有常量折叠改动；没有新增 intrinsic；`text` 公共接口未变；
6. **动态值**：动态 → `str` 与静态的文本表示一致，有用例；
7. **通用**：`cargo test --workspace`、clippy、fmt、`bun test`、`bun run check`、`bunx tsc --noEmit` 全绿；
   提交标题带类型前缀、正文非空。

## 七、实施记录与后续

### 7.1 实施记录（2026-10-14）

1. **范围裁定**：星崽选择一并接通 `int`、`sint`、`lint` → `str`；浮点、常量折叠、标准库公共接口和新 intrinsic
   均保持排除。
2. **类型层**：`classify_conversion` 对三个整数源统一返回 `ConversionKind::IntegerToStr`，
   `requires_runtime_check = false`；`X02-TYPE-006` 保留给其它非法转换。
3. **Runtime 与两条路径**：VM 的 `TacOp::Cast` 与动态 LLVM 的 `ScalarCast` 都落到同一
   `RuntimeValue::convert_to`/`xiao_runtime_value_cast`，三种整数均调用 `display_text()` 的十进制表示。
   静态 `ir.rs` 不接受 `str` 固定宽度槽，因此没有新增静态 cast 分支；这不是遗漏。
4. **构造式入口**：动态 LLVM 的 `ScalarCast` 分派补齐 `str(value)`；`as str` 继续沿用既有 Cast 路径。
5. **夹具与测试**：`second-int-to-str.xiao` 分别覆盖三种宽度的 `as str`、`str(value)` 和一个异构数组动态值，
   VM 输出 `42:42:42:42:42:42:42`；类型层、Runtime、动态 LLVM 文本测试均通过。
6. **返工与防复发**：语言边界批 §六 改为承接第三批结果，第三批 A-2 改指向表 B/§1.3，README 粗体已闭合；
   本批采用人工自检规则：改过 README 状态格后，提交前逐行核对 `**` 奇偶。

### 7.2 后续

**本批之后**：回到库批次加「大小写转换」与数字格式化，还是另有先后？
   （`int` → `str` 接通后，大小写转换仍需 `str → int`/字符分类能力；届时按
   [20C-第二/三批](20c-second-batch-probe.md) 的口径**再复探一次**，不拿本批的预期当结果。）

## 相关页面

- [20C-第三批](20c-third-batch-reprobe-and-status.md) —— **本批的直接前置**：§三 复探表、§五 选乙、§八 裁定
- [20C-语言边界批](20c-language-boundary-and-operators.md) §2.4 —— 大小写转换的历史登记与当前剩余阻塞；§六 是本批 §2.1 要回填的对象
- [20C](20c-official-xiao-library.md) §3.4 —— 规模纪律（不许为库批量新增 intrinsic）与双路径验收
- [09B0](09b0-bytecode-closure.md) §3.4/§3.5 —— **B0-A 的「补接通」先例**与「字段存在 ≠ 有人在用」
- [02. 类型系统](02-type-system.md) §显式转换 —— 转换矩阵与 `as` 的既定口径
- [00. 决策基线](00-decisions.md) —— `bool → str` 的文本规则（小写 `true`/`false`）是**同类先例**
