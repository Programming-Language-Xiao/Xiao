# 10Z-收尾. 10 系列收束：B 系列分类、取证与 19 收口

> **背景**：10 系列的字母已经用尽（到 `10Z`），而**主线一步没动**——最近三批
> （[10Z-唯一性](10z-index-uniqueness-followup.md)、[10Z-CI覆盖](10z-ci-push-coverage-followup.md)、
> [10Z-CI覆盖2](10z-ci-crate-and-test-coverage.md)）连续在修门禁与 CI 覆盖，全都是**工具侧的账**。
> [10Y](10y-b-series-triage-rework.md) 的**主体 Y3（B 系列 23 处拒绝面的逐条实跑分类）至今未开始**，
> [10Z](10z-19-closeout-and-b-series-implementation.md) 因此**从未进入实现或收口**。
>
> **一句话概括本批**：**把 10Y/10Z 的未完成项收束成一条可执行的收尾序列**，做完就关 10 系列；
> 并**冻结「不再开新的工具侧批次」**——门禁与 CI 覆盖已经收口，除非它阻塞主线。
>
> 状态：**实施中（2026-10-10）**。Y1、Y5 已完成；**Y3 双侧对照与 Z-1 裁定已完成**，B1 已修复、C 类关闭、D 转独立立项；
> 按 S1 不进入 D 的实现。§1.3 是开工起点，
> §1.0 是本轮的审核补正（含一处跨批遗留的错误证据与一处待修代码）。

## 实施记录（2026-10-09）

- Y1 已完成：原生降低层 `dynamic/container.rs` 的构造参数写入临时改为 `zeroinitializer`，受控 Windows 原生门控真实变红；恢复后同一命令干净通过。红色输出至少包含 `table-constructor-positional/default/keyword` 的结果差异、`X06-RUNTIME-007`、退出码 3 和释放序列差异；详见 10W/10Y 回填。
- Y2 两处 `37756208604` 回填此前已完成并保留；Y5 的 X2/X4 措辞和 macOS `19.14`「不可验证」标定已同步到相关文档。
- Y3 已完成双侧分类与可达性：10P 的 28 处枚举已回写；B1 已实现并完成原生复跑，C 类关闭，D 转独立立项。

## 一、Agent 交接上下文

### 1.0 审核补正（2026-10-09，对 `7d41d98`/`dd92541`/`64df207`/`a322db0`）

审核**独立复跑了 Y3 的 VM 探针**（用其记载的方法 `bun cli/ts/src/main.ts --json run <目录>/main.xiao`），
抽 4 行核对：`container.rs:224`、`expression.rs:371`、`expression.rs:449`、`expression.rs:735`
**逐字吻合**，连其自注的局限（`expression.rs:735`「当前探针是 dict_table」）也复现为
`期望类型 table，实际为 dict_table`。**探针是真跑的，不是编的。** 但顺着其中一行查下去发现：

1. **F1（实打实的缺陷，跨批遗留）**：`container.rs:242`「表构造参数形态」在 10W/10X 里记的
   `X02-TYPE-001` **是错的**——那个码来自探针源码自带的缺陷：`…-> none\n        pass\n…` 里的 `pass`
   **不是 Xiao 的关键字**，实测首条诊断是 `X02-TYPE-001`「未定义名称 pass」，与构造参数形态无关。
   用合法函数体重跑，真实错误是 **`X05-TYPE-004`「init 不存在该关键字参数」**（+「init 缺少必需构造参数」），
   退出码仍为 1。**分类结论（两边都拒绝）不变，错的是证据。** 该缺陷探针同时存在于
   `d19a_differential.rs` 的 `unsupported-kind` 用例，**待修代码见 [10W](10w-a2-closeout-and-a3-boundary.md)**。
2. **F2（措辞）**：Y3 新表把该行写成 `X05-TYPE-004`，**实测证明新值是对的**——但它**静默改正**了
   10W/10X 的旧记录，**没有一个字**说明冲突与理由。已按「加列不抹旧判定」的口径在
   [10P](10p-differential-exemption-and-coverage.md) 补上改判说明，并在 10W/10X 加勘误。
3. **F3（证据强度，且**规划侧有责任**）**：Y1 的提交正文只有摘要，**没有原始输出**。
   而 §1.3(4) **把审核实测的预期输出写进了规格**，照抄即可写出同样的摘要——
   **这条口子是规划时留下的**。已在 §1.3(6) 补上强化的取证要求（带机器特征的原始输出，
   或换一个不在规格里的变异点）。
4. **F4（状态）**：Y3 **已完成**。28 行均有最小源码 / VM 与原生对照 / 分类 / 可达性，
   `dynamic/` 外 4 处明确判为独立项；B/C/D 裁定已回写，B1 已交付，D 转独立立项。

**汇报口径（供参考）**：Y3 候选 **4 组**——动态扩展赋值（`control.rs:206`）、
动态选择计划缺失（`expression.rs:270`）、动态成员读写（`expression.rs:735`/`:752`）、
动态错误构造参数（`expression.rs:670`）；双侧对照与正式选择计划测试均已完成，B/C/D 裁定见 10Z-Z1。
其余条目保持两边一致拒绝或独立边界。

### 1.1 接手前提

1. [10Y](10y-b-series-triage-rework.md) —— **本批的主要来源**：§2.2 的分类方法（最小源码 / VM 实跑输出 / 三分类 / 可达性）、§2.1 的变异验证要求，**全部继续有效，本批不重复定义**；
2. [10Z](10z-19-closeout-and-b-series-implementation.md) —— 收口口径：§2.2 实现范围、§2.3 裸机轮次、§2.4 性能取数、§2.5 三平台拆分、§2.6 收口声明、§2.7 欠格盘点；
3. [10P](10p-differential-exemption-and-coverage.md) 的枚举表 —— Y3 的**回写对象**；B1/B2 的 ABI 顺序来源；
4. [10X](10x-b-series-triage.md) §2.2 —— 旧清单的出处（本批 §1.2 对它做了对账）；
5. [10D](10d-environment-gated-test-spec.md) §4 / §4.5 —— 受控原生环境与本地跑 ignored 的命令；
6. [12](12-tests-and-milestones.md) O6、[19](19-optimization-release.md) `19.14`、[19D](19d-performance-comparison.md) —— 收口的判定对象。

### 1.2 现状盘点（2026-10-09，审核实测）

**未完成项清单**（逐条来自各文档的实施记录，非估计）：

| 编号 | 事项 | 出处 | 状态 |
| --- | --- | --- | --- |
| Y1 | 原生构造参数变异的**红色证据**（原生变异 + 受控原生门控跑红 + 贴输出） | 10Y §2.1 | **已完成**（受控 Windows 原生门控 5 条 `table-constructor-*` 变红，已恢复干净基线） |
| Y3 | **B 系列逐条分类**（最小源码 + VM/原生实跑 + 分类 + 可达性），回写 10P | 10Y §2.4 | **已完成**（28 行、四组双侧对照、B/C/D 裁定与 B1 回填；D 转独立立项） |
| Y5 | `19.14` 的 macOS 一格标「**不可验证**」 | 10Y §2.5 第 3 条 | **已完成**（无 Mac 宿主，CI 显式跳过真实终端） |
| Y6 | 受控性能取数（Temurin 21 + 固定硬件 + 三路对照） | 10Y §2.7 | 未开始 |
| Z-1 | B 系列**实现或关闭**（范围由 Y3 定） | 10Z §2.2 | B1 已完成；C 关闭；D 不纳入，转 10Z-D |
| Z-2 | 裸机 Linux 轮次：`19.14` C 档窗口截图 **+** 性能取数**同一轮** | 10Z §2.3 | 未开始（需裸机） |
| Z-3 | O6 条件 1/3/4 的欠格**逐格拆开** | 10Z §2.7 | 未开始 |
| Z-4 | O6 五条终局对账 + 收口声明（含覆盖边界） | 10Z §2.6 | 未开始（被前面全部阻塞） |

**已收口、本批不碰**：A1（表方法 ABI）、A2/A3（构造参数与一等方法值边界）、诊断窗口保持（10V）、
唯一性门禁（10Z-唯一性）、push 覆盖与 crate/全量测试覆盖（10Z-CI覆盖 / 2）。

**B 系列拒绝面的权威枚举（本批新增，审核实测）**

权威来源：`grep -rn 'feature: *"' core/rust/crates/xiao-codegen-llvm/src/`，共 **28 处**。
按 R3 的口径与 [10X](10x-b-series-triage.md) §2.2 的旧清单对账：

| | 数量 | 说明 |
| --- | --- | --- |
| 权威来源总数 | **28** | `feature:` 字符串的全部出现 |
| 其中 `dynamic/*.rs` | **24** | B 系列的主战场 |
| 其中 `dynamic/` 之外 | **4** | 见下表，**旧清单未覆盖** |
| 10X 旧清单 | 23 名 / **24 行** | 「动态函数调用（2 处）」占 2 行 |

**对账结论**：旧清单与 `dynamic/*.rs` 的 24 处**差集为空**——它在 dynamic 范围内是**完整的**。
但 `dynamic/` 之外另有 4 处，旧清单没有它们：

```text
dynamic.rs:100   "N0-B 当前只接受 64 位 C ABI 目标"
ir.rs:315        "函数值"
ir.rs:319        "动态或容器类型"
ir.rs:493        "*args/**kwargs 形参"
```

**这 4 处是否属于 B 系列，需按 §2.2 第 3 条判定**（见下）。

`dynamic/*.rs` 的 24 处（file:line，供逐条填表；**以代码为准，本表由审核 grep 得出**）：

| # | 文件:行 | 拒绝文案 |
| --- | --- | --- |
| 1 | `dynamic/container.rs:198` | 动态表构造目标 |
| 2 | `dynamic/container.rs:224` | 动态表构造参数缺少 init |
| 3 | `dynamic/container.rs:242` | 表构造参数形态 |
| 4 | `dynamic/container.rs:333` | 动态表字段类型 |
| 5 | `dynamic/control.rs:206` | 动态扩展赋值 |
| 6 | `dynamic/expression.rs:47` | 动态表方法值（A3） |
| 7 | `dynamic/expression.rs:212` | 动态混合多项选择器 |
| 8 | `dynamic/expression.rs:270` | 动态选择器缺少规范选择计划 |
| 9 | `dynamic/expression.rs:371` | print 的关键字或展开实参 |
| 10 | `dynamic/expression.rs:423` | input 的关键字或展开实参 |
| 11 | `dynamic/expression.rs:449` | set 构造器参数 |
| 12 | `dynamic/expression.rs:475` | random.seed 缺少参数 |
| 13 | `dynamic/expression.rs:497` | 动态函数调用 |
| 14 | `dynamic/expression.rs:508` | 动态函数调用 |
| 15 | `dynamic/expression.rs:596` | FatalError 不能构造为可恢复错误值 |
| 16 | `dynamic/expression.rs:631` | 错误构造参数（只支持 code/message） |
| 17 | `dynamic/expression.rs:670` | 动态错误构造参数（当前只支持字符串字面量） |
| 18 | `dynamic/expression.rs:735` | 动态非表成员访问 |
| 19 | `dynamic/expression.rs:752` | 动态非表字段写入 |
| 20 | `dynamic/methods.rs:53` | 动态函数值 ABI（A3） |
| 21 | `dynamic/methods.rs:491` | 动态表方法接收者 |
| 22 | `dynamic/methods.rs:552` | 表方法参数形态 |
| 23 | `dynamic/slot.rs:112` | 动态表声明/初始化 |
| 24 | `dynamic/slot.rs:126` | 动态表字段初始化 |

> 其中 #6 与 #20 是 A3，[10W](10w-a2-closeout-and-a3-boundary.md) 已判**两边一致拒绝**，
> [10X](10x-b-series-triage.md) §2.2 第 4 条已明确**不重复取证**——直接引用结论，不占本批工作量。
> 同理 #2、#3 已由 X2 判为「前端已拒绝、未找到可到达源码」。**即真正待分类的是其余 20 处。**

### 本批边界

| 子任务 | 内容 | 本批 |
| --- | --- | --- |
| Y1 原生变异红色证据 | 补做 | **做** |
| Y3 B 系列分类 | 20 处待分类 + 4 处归属判定，回写 10P | **做（主体）** |
| Y5 macOS 标「不可验证」 | 补做 | **做** |
| Z-1 B 系列实现或关闭 | 按 Y3 结论 | **先汇报后定** |
| Z-2 裸机轮次（C 档 + 取数） | 一次跑完 | **做**（需裸机） |
| Z-3 / Z-4 欠格与收口 | —— | **做**（被前面阻塞） |
| 新的工具侧批次 | 门禁 / CI / 文档规则 | **不做**（见 §2.3） |
| 20C/20D、静态溢出 | —— | **不做**（归 20 系列） |

### 1.3 开工起点（第一步的可执行规格）

**阅读顺序**：本节 → [10Y](10y-b-series-triage-rework.md) §2.1–§2.4（分类与变异的方法，**必须通读**）
→ [10D](10d-environment-gated-test-spec.md) §4 / §4.5（受控原生环境与本地跑 ignored 的命令）
→ [10P](10p-differential-exemption-and-coverage.md) 的枚举表（Y3 的回写对象）。

下面把 **Y1** 与 **Y3 的第一条**写到可以直接照做的粒度。**这些值是审核 2026-10-09 在本机实测过的**，
Agent 不必重新摸索；若与本机实际不符，以实际为准并记录差异。

#### 第一步：Y1 的原生变异红色证据

**(1) 变异点**——`core/rust/crates/xiao-codegen-llvm/src/dynamic/container.rs:280-284`，
表构造参数槽的写入：

```rust
        for (index, value) in argument_values {
            self.emit(format!(
                "  store {VALUE_TYPE} {value}, ptr {}",
                argument_slots[index]
            ));
        }
```

把它改成一律写 `zeroinitializer`（即“忽略全部构造参数”），例如把 `{value}` 换成 `zeroinitializer`
并让 `value` 不再被使用（`let _ = value;`，否则 clippy 会拦）。

**必须落在原生降低层**——改 VM、改前端、或另写 `#[test]` 都不算（[10Y](10y-b-series-triage-rework.md) §2.1 第 1、3 条）。

**(2) 环境**——按 [10D](10d-environment-gated-test-spec.md) §4 准备。本机实测可用的一组值
（**机器相关，仅作参照**）：`vcvars64.bat` 在
`C:\Program Files\Microsoft Visual Studio\18\Community\VC\Auxiliary\Build\`；
MSYS2 在 `D:\msys64`（用 `ucrt64\bin` 下的 `clang/llvm-as/llc/llvm-strip`）；
`XIAO_RUNTIME_LIBRARY=core\rust\target\release\xiao_runtime.lib`（**先 `cargo build --release -p xiao-runtime`**）；
`XIAO_TARGET_TRIPLE=x86_64-pc-windows-msvc`；**并设 `XIAO_DIAGNOSTICS_HOLD_MS=0`**
（否则调试矩阵会堆窗口，见 10D §4.5）。

**(3) 命令**（定向执行，避免整轮 5 分钟）：

```text
cargo test -p xiao-driver --test d19a_differential -- --ignored --exact native_side_matches_the_vm_sides_on_every_case
```

**(4) 期望的红色**（审核实测原文，机器不同数字可能有出入）：

```text
用例 table-constructor-positional：「VM（源码）」与「原生」不一致：输出 "9\n" ≠ ""；
    错误身份 None ≠ Some("X06-RUNTIME-007")；退出码 0 ≠ 3
用例 table-constructor-default：   输出 "4\n" ≠ ""
用例 table-constructor-keyword：   输出 "8\n" ≠ ""
用例 table-constructor-evaluation-order：错误身份 None ≠ Some("X06-RUNTIME-007")
用例 table-constructor-failure：   Runtime 释放序列不一致
test result: FAILED. 0 passed; 1 failed
```

**红信号是 `X06-RUNTIME-007` + 退出码 3，与输出 `"0\n"` 无关**——10X 当年那两条
`assert_ne!(…, "0\n")` 就是照着想象写的（[10Y](10y-b-series-triage-rework.md) §2.1 第 6 条）。
**只要能跑出「至少两条 `table-constructor-*` 不一致」即可算达标**，不必逐字与上面对齐。

**(5) 恢复与证明**：`git checkout -- core/rust/crates/xiao-codegen-llvm/src/dynamic/container.rs`，
然后 `git diff --quiet HEAD -- <该文件>` 必须通过；**再跑一次干净基线确认转绿**
（审核实测：`native_side_matches_the_vm_sides_on_every_case ... ok`，约 73 s）。

**(6) 回填**：把变异点（文件与行）、**红色输出**、恢复证明写进提交说明，并回填
[10W](10w-a2-closeout-and-a3-boundary.md) 与 [10Y](10y-b-series-triage-rework.md) 的实施记录。

> **⚠ 本节 (4) 贴了预期输出，这削弱了「贴输出」的证明力**（审核自省，2026-10-09）。
> 上面那段红色输出是**审核实测的原文**，写进规格是为了让执行方知道「红长什么样」；
> 但代价是——**照抄它就能写出与真跑一致的摘要**。所以只写「出现 `X06-RUNTIME-007`、退出码 3」
> 这类摘要，**不算满足要求**。可接受的两种做法：
>
> 1. **贴带机器特征的原始输出**：完整 harness 输出、耗时、绝对路径、`test result:` 行等
>    （即 `--nocapture` 下的原始文本），照抄规格写不出这些；
> 2. **换一个不在本节规格里的变异点**（例如 `dynamic/expression.rs` 的某条构造路径），
>    其红色形态不在本文档中出现过。
>
> 二选一，**并在提交说明里写明用了哪一种**。这条同样适用于后续任何「规格里给了预期输出」的取证。

#### 第二步（Y3 的第一条）：先跑权威枚举，再逐条填

```bash
grep -rn 'feature: *"' core/rust/crates/xiao-codegen-llvm/src/
```

**这是唯一权威来源**；把结果与 §1.2 的 28 处基线对账，**写出差集**再动手。
之后按 [10Y](10y-b-series-triage-rework.md) §2.4 逐条填
（最小源码 / VM 实跑输出 / 三分类 / 可达性），回写 [10P](10p-differential-exemption-and-coverage.md)（**加列不抹旧判定**）。
**`dynamic/` 之外那 4 处的归属必须单独判定**（§2.2 第 3 条）。

## 二、必须先冻结的 5 条

### 2.1 **S1：执行顺序锁死，不许跳步**

**冻结**：

1. **Y1 → Y3 → 汇报 → Z-1 → Z-2 → Z-3 → Z-4**。每一步是下一步的输入，**不许并行抢跑**；
2. **Y3 做完必须先提交只含文档的分类记录，然后停下来向星崽汇报**（真缺口几条、各需要什么、
   哪些依赖别的阶段），**汇报后才定 Z-1 的范围**——[10Z](10z-19-closeout-and-b-series-implementation.md) §2.1 的入口条件继续有效；
3. **Z-2（裸机轮次）不依赖 Y3**，可以并行准备（改交接文档、装 Temurin 21、落盘环境清单），
   但**产出顺序**仍是先分类、后实现、后收口；
4. **Z-4 不许提前写**：O6 收口声明必须在 Z-1/Z-2/Z-3 都有结论之后才动笔；
   **不允许**先把「满足」写上去再倒推。

### 2.2 **S2：Y3 的清单以代码为准，并按 R3 的口径写出差集**

**冻结**：

1. **权威来源**是 `grep -rn 'feature: *"' core/rust/crates/xiao-codegen-llvm/src/`（**28 处**），
   **不是**任何文档里的清单——包括本节的 24 处表格，它只是**审核实测的基线**，
   若与代码不一致，**以代码为准并在提交说明里写出差集**；
2. 逐条填表，每条给：**最小源码**、**VM 实跑输出**（错误身份 / 输出 / 退出码）、
   **三分类之一**（原生缺口 / 两边都拒绝 / 原生拒绝且 VM 也不该支持）、**可达性**；
   格式与判据沿用 [10Y](10y-b-series-triage-rework.md) §2.4，**本批不重新定义**；
3. **`dynamic/` 之外的 4 处要单独判定归属**，不许默认它们不在范围内：
   - `dynamic.rs:100`「N0-B 当前只接受 64 位 C ABI 目标」——**像平台/目标限制**，不是语言拒绝；判它是「目标平台约束」还是「原生缺口」；
   - `ir.rs:315`「函数值」、`ir.rs:319`「动态或容器类型」、`ir.rs:493`「*args/**kwargs 形参」——**像 IR 层的功能门**，
     要判它们对应**哪些真实源码形态**、VM 是否也拒绝；**若判为不属于 B 系列，要写明理由并登记为独立项**，不许静默略过；
4. **回写 10P 只能加列不能抹旧判定**（[10Y](10y-b-series-triage-rework.md) §2.4 第 2 条继续有效）；
5. **只写「分类中」视为未做**；每条结论必须附**实跑输出**，不能只有读码推断。

### 2.3 **S3：冻结「不再开新的工具侧批次」**

**冻结**：

1. 最近三批连续修门禁与 CI 覆盖（唯一性规则、push 路径、crate 过滤与全量测试），
   **现在都收口了**；本批起**不再开新的工具侧主题**——除非它**阻塞主线或导致主线结论不可信**；
2. 具体地：**不为「更好看的门禁」「更多规则号」「更细的 CI 分层」立项**；
   发现问题就**记进 10Z 的清债盘点**（[10Z](10z-19-closeout-and-b-series-implementation.md) 那张表已有口径），
   **攒到 20 系列或真正阻塞时再动**；
3. **例外**：若主线的某一步（如 Y3 的实跑、Z-2 的裸机）**因为工具缺失而做不了**，
   那算阻塞，按阻塞处理——但要在交接里写明「不做这一步就推不动什么」；
4. 本条的用意：**10 系列的收尾不能再被工具侧吃掉**。上一批的主体被推迟了整整三轮。

### 2.4 **S4：裸机轮次一轮做三件事**

**冻结**：

1. **同一轮、同一台裸机 Linux**上完成：**`19.14` C 档窗口截图** + **O6 条件 5 的性能取数**
   （[10Z](10z-19-closeout-and-b-series-implementation.md) §2.3 第 1 条已定），
   外加**若 Y3 判出需要裸机侧佐证的条目**则一并取；
2. 取证口径沿用：**不设** `XIAO_DIAGNOSTICS_HOLD_MS=0`（让窗口留住）、
   **不设** `XIAO_USE_XVFB`、跑前落盘环境清单、产物用 **results + log 双文件**；
3. **C 档判定**：窗口**确实出现且有截图** → 通过；仍无窗口 → **未通过**并写下观察与假设。
   **不得**把「测试 ok」写成「通过」（[10Z](10z-19-closeout-and-b-series-implementation.md) §2.3 第 8 条）；
4. **Windows 性能格按已定口径标缺**，不用本机数字顶替（[10Z](10z-19-closeout-and-b-series-implementation.md) §2.4 第 3 条）；
5. **一轮跑完就够**，不为不同条目反复开轮次。

### 2.5 **S5：10 系列的关闭条件要写死**

**冻结**：

1. 10 系列**只有在本批全部完成时才算关闭**：Y1/Y3/Y5 有结论、Z-1 有出路（实现或如实关闭）、
   Z-2 的 C 档与取数有结论、Z-3 的欠格逐格写完、Z-4 的 O6 五条终局对账与收口声明落盘；
2. **收口声明必须写明覆盖边界**（不可验证的格算不阻塞，但必须写明——星崽 2026-10-08 已定）；
3. **关闭后才谈 20 系列**（星崽 2026-10-08 已定「B 系列无缺口时先在 10 系列清一遍债再转」，
   本批就是那「一遍」）；
4. **不许**因为「想转 20」而把未完成项改写成已完成，或用「部分」含糊过去；
5. 本批结束时若仍有未完成项，**如实列出并写明各自归属**（留 10 系列 / 转 20 / 单独立项），
   并在 README 把 10 系列的状态写成事实。

## 三、落点

```text
core/rust/crates/xiao-codegen-llvm/src/dynamic/*.rs      Y1 的原生变异点（验完恢复）
core/rust/crates/xiao-driver/tests/d19a_differential.rs   Y1 呈现红色输出；Z-1 若实现则加用例
docs/DevDocs/10p-differential-exemption-and-coverage.md   Y3 的逐条分类与可达性（加列不抹旧判定）
docs/DevDocs/10w-a2-closeout-and-a3-boundary.md           Y1 的红色证据回填
docs/DevDocs/19-optimization-release.md                   Y5：19.14 的 macOS 一格标「不可验证」
docs/DevDocs/19d-performance-comparison.md                Y5/Y6/Z-3/Z-4：O6 五条终局对账
docs/DevDocs/10z-…-results-<日期>.md / -log-<日期>.md      Z-2：裸机 C 档与取数的双文件产物
docs/DevDocs/10t-linux-bare-metal-handoff.md              Z-2：按保持窗口改写取证步骤
docs/DevDocs/10y-b-series-triage-rework.md、10z-…-…md      两项的实施记录
docs/DevDocs/README.md                                    主表登记 + 10 系列状态
```

## 四、硬约束

1. **顺序锁死**（§2.1）；Y3 汇报前不开工实现；
2. **清单以代码为准**（§2.2 第 1 条），差集写进提交说明；
3. **`dynamic/` 之外那 4 处必须单独判定**，不许默认排除（§2.2 第 3 条）；
4. **10P 只加列**；每条结论附实跑输出（§2.2 第 4、5 条）；
5. **不再开新的工具侧批次**（§2.3）——发现问题记进清债盘点，攒着；
6. **裸机一轮三件事**，判据写死（§2.4）；
7. **C 档不许用「测试 ok」充当通过**；性能不用本机数字顶替；
8. 10 系列的关闭条件按 §2.5；**达不到就如实记录**；
9. 版本规则按 10T；不先删拒绝；接不通的继续拒绝并登记；
10. 不新增依赖，`Cargo.lock` 保持同步；每个提交单独跑 clippy 与 fmt；
    推送前跑 `cargo test --workspace`、`bun test`、`bun run check`、`bunx tsc --noEmit`；
11. 本地跑 ignored 用 10D §4.5 的命令（`--skip` 去掉窗口风暴）；
12. 提交说明带正文，写明归属批次与运行号（涉及 CI 时）。

## 五、分步提交

1. **Y1**：原生变异 → 受控原生门控跑红 → 贴输出 → 恢复并证明干净 → 回填 10W（**精确起点见 §1.3**）；
2. **Y3**：按 §2.2 逐条实跑 20 处 + 判定 `dynamic/` 外 4 处 → 回写 10P（加列）→ **提交只含文档**；
3. **停下来汇报**：真缺口条数、各自需要什么、依赖哪些阶段；等星崽定 Z-1 范围；
4. **Z-1**：有缺口按 B1 → B2 实现（每条含差分用例与原生侧变异验证）；无缺口则如实关闭并写明理由；
5. **Y5**：`19.14` 的 macOS 标「不可验证」+ 相关措辞校准；
6. **Z-3**：O6 条件 1/3/4 的「部分」逐格拆开，写明「能补已补 / 需什么环境 / 为何无法补」；
7. **Z-2**：裸机轮次（C 档 + 取数），双文件产物回传；
8. **Z-4**：O6 五条终局对账 + 收口声明（含覆盖边界）；达不到就如实写达不到；
9. **全量门禁**：`cargo test --workspace`、clippy、fmt、`bun test`、`bun run check`、`tsc`；
10. **文档与登记**：10Y/10Z 的实施记录、README 的 10 系列状态。

## 六、最可能翻车的地方

1. **又去修工具**：门禁 / CI / 规则号 / 文档门禁——本批明确冻结（§2.3）；
2. **跳过 Y3 汇报直接实现** B 系列（`10Z` §2.1 的老毛病）；
3. **清单照抄文档而不 grep 代码**，漏掉 `dynamic/` 之外那 4 处，或漏掉未来新增的；
4. **`dynamic/` 之外那 4 处被静默略过**（不说「不属于本批」，也不说「属于」）；
5. **为了有产出而实现判为非缺口的路径**（A3 的前车之鉴）；
6. **Y1 的变异验证做成走过场**（改一个看不出来的地方、或用 VM 测试冒充原生——[10Y](10y-b-series-triage-rework.md) §2.1 已写死）；
7. **C 档又用「测试 ok」充当通过**（10T-Linux 那轮的确切翻车方式）；
8. **收口声明只写「满足」不写覆盖边界**；
9. **为了让 10 系列关掉而把未完成项写成已完成**，或用「部分」含糊过去；
10. **只改一处**：结论改了、README / 10P / 19D 没跟上（本会话已栽过多次）；
11. **提交没写正文**。

## 七、验收

1. **Y1**：提交说明里有原生变异点、受控原生门控的**实际红色输出**、恢复后的干净证明；
2. **Y3**：二十处待分类**每条**都有最小源码、VM 实跑输出、三分类之一与可达性；**以新增列**回写 10P，旧判定未被抹掉；
   **`dynamic/` 之外的 4 处有明确归属判定**（属于 B 系列并分类 / 不属于并写明理由与去向）；
3. **差集**：以 grep 的 28 处为权威，提交说明里写出「权威 28 / 已判 N / 差集是什么」；
4. **Z-1**：判为缺口的已按 B1 → B2 实现（含原生完整比对与原生侧变异验证）；判为非缺口的**未改后端**且有理由；
5. **Z-2**：C 档有截图与原始输出（通过就写通过，没窗口就写未通过 + 观察与假设）；取数有环境清单与三路原始数字；
   Windows 标缺、macOS 不可验证，**没有混口径**；
6. **Z-3**：条件 1/3/4 的每个「部分」已拆成格子，每格写明能补 / 需什么 / 为何无法补；
7. **Z-4**：O6 五条逐条有「满足/部分/不满足」+ 证据 + 限制；**收口声明写出覆盖边界**；达不到如实记录；
8. **10 系列状态**：README 与各文档的状态与事实一致；未完成项如实列出并写明归属；
9. `cargo test --workspace`、clippy、fmt、`bun test`、`bun run check`、`bunx tsc --noEmit` 全绿；
10. 没有通过放宽断言、删用例或缩小范围换来的绿。

## 八、不负责与不要重复做的事

- **不重做**已收口的 A1/A2/A3、诊断窗口保持、唯一性门禁、CI 覆盖（10T–10V、10Z-唯一性、10Z-CI覆盖/2）；
- **不重复定义** [10Y](10y-b-series-triage-rework.md) §2.2/§2.4 的分类方法与 [10Z](10z-19-closeout-and-b-series-implementation.md) §2.3/§2.4 的取证口径——**引用即可**；
- **不开**新的工具侧批次（§2.3）；
- **不做** 20C/20D 与静态溢出（归 20 系列）；
- **不改**窗口机制、保持行为的默认值、周定时的平台复现。

## 待定决策

1. **`dynamic/` 之外那 4 处的归属**：`dynamic.rs:100`（64 位 C ABI 目标）、`ir.rs:315/319/493`（函数值 / 动态或容器类型 / `*args/**kwargs` 形参）
   ——**建议先由 Y3 实跑判定，判为不属于 B 系列的就登记为独立项**，不在本批实现。若星崽希望现在就定归属，可直接指派。
2. **裸机轮次的时机**：Z-2 不依赖 Y3，可以早于 Y3 完成（例如先取 C 档与性能数据，Y3 随后）。
   **建议**：只要裸机可用就尽早跑，避免最后卡在等机器上。
3. **10 系列关闭后的去向**：按星崽已定的「先清一遍再转 20」，本批结束即转 20 系列；
   若本批结束时仍有未完成项，是带走还是留 10 系列，届时按清单定。

## 相关页面

- [10Y. 返工与 B 系列分类收口](10y-b-series-triage-rework.md) —— **本批的主要来源**；分类方法与变异验证要求
- [10Z. 19 收口：B 系列实现与收尾取证](10z-19-closeout-and-b-series-implementation.md) —— 收口口径与裸机轮次
- [10X. B 系列取证与分类](10x-b-series-triage.md) §2.2 —— 旧清单的出处（本批对账为 dynamic 范围内完整）
- [10P. 差分豁免收敛与剩余拒绝面](10p-differential-exemption-and-coverage.md) —— Y3 的回写对象
- [10W. A2 收口与 A3 边界冻结](10w-a2-closeout-and-a3-boundary.md) —— A3 与 A2 剩余形状的既有结论（本批直接引用）
- [19. 优化、兼容性与发布验收](19-optimization-release.md)、[19D. 性能对照](19d-performance-comparison.md) —— 收口的判定对象
