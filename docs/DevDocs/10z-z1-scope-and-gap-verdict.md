# 10Z-Z1. Y3 收口结论与 Z-1 范围

> **这是什么**：[10Z-Y3](10z-y3-stage-report.md) 已完成四组对照，本文是**收口结论**与
> **Z-1（B 系列实现或关闭）范围**的决策输入。写它的直接原因：四组的判定其实**落在三种不同形态上**，
> 而 [10Z-Y3](10z-y3-stage-report.md) §2.2 的三行判定表只有一行叫「缺口」，
> 需要先把「谁落在哪一行」摆清楚，才能定 Z-1 要不要做、做什么。
>
> **一句话概括**：①②③ 的表形态与计划形态关闭；C 按“两边都拒绝”关闭；B 的错误身份差异已在
> Z-1 修复；④ 是唯一「合法程序、VM 执行、原生拒绝构建」的形态，已登记为独立后续项。
>
> 状态：**口径已裁定，B1 已完成；D 转独立立项**。C 不进入实现，Z-1 不处理 D。

## 一、结论总览

四组候选按**可观察行为**归类（全部为已实跑，原始行见 [10Z-Y3](10z-y3-stage-report.md) §2.3.1 与其日志）：

| 形态 | 成员 | VM | 原生 | 判定 |
| --- | --- | --- | --- | --- |
| **A. 完全一致** | ①表读 / ①表写 / ②表写 / ③五形态 | 输出 `7` / `42` / `42` / `5·7·4·3·4` | 产物同值同退出码 | **不是缺口** |
| **B. 同在运行期拒绝，身份不同** | ①字典接收者 | 修复前 `X06-RUNTIME-002`，退出 3 | 修复前 `X06-RUNTIME-012`；B1 后同为 `X06-RUNTIME-002`，退出 3 | **低优先级错误身份缺口已修复** |
| **C. 阶段不同** | ①标量接收者 / ②字典写入 | 运行期 `X06-RUNTIME-002`，退出 3 | **编译期拒绝** `X11-PROTOCOL-007`，构建退出 2 | **两边都拒绝，关闭** |
| **D. VM 执行、原生拒绝构建** | **④ 动态错误构造参数** | **运行到用户自己的 `raise`**（`report.code=DYNAMIC`，退出 3） | **编译期拒绝** `X11-PROTOCOL-007 动态错误构造参数`，构建退出 2 | **独立立项，不纳入 Z-1** |
| **E. 防御性不可达** | ③ 的守卫本身 | —— | —— | 合法源码触发不到；已由正式测试守住（§二） |

**没有一组落在「VM 成功产出结果而原生拒绝构建」的字面意义上**——但这句总结**会误导**，因为
④ 的 VM 行为不是「产出结果」，也不是「拒绝」，**而是把程序跑到底**。下一节专门讲这个。

## 二、必须纠正的一个框架错误（审核自记）

审核在上一轮汇报里写过一句「**没有一组是『VM 成功产出结果而原生拒绝』，按现有口径 Z-1 可能一条都不需要实现**」。
**这句话按字面成立，按实质是错的**——它把 ④ 归错了行。

④ 的最小源码是一个**合法程序**：

```xiao
code = "DYNAMIC"
raise ArithmeticError(code = code)
```

它在 VM 上的行为是：**接受动态参数、构造错误、执行到用户自己写的 `raise`**，退出 3、错误码是用户给的 `DYNAMIC`。
**这不是 VM 在拒绝这个程序**——这是 VM **按语言的语义把它跑完了**。而原生**在产物之前就拒绝构建**。

也就是说：**存在一个语言允许、前端接受、VM 能执行的程序，原生后端造不出产物。**
这正是「原生缺口」的定义，只是它没落在 [10Z-Y3](10z-y3-stage-report.md) §2.2 第三行
「VM 成功产出**结果**」的**措辞**内——**那是措辞的漏洞，不是结论**。

> 教训与 §一 末尾那条同源：**判定表是按措辞用的，但结论要按实质下**。
> 三行表是为了省事，不是为了让边界情形自动落格。

## 三、口径需分三种情形作答（不是一个）

**这是本次要给星崽的核心问题。** 请分别作答，不要合成一条：

### A. 完全一致（①表 / ②表 / ③）

**没有分歧，不需要任何口径**。登记为「不是缺口」即可，只改文档。

### B. 同在运行期拒绝、但错误身份不同（①字典接收者）——**低优先级缺口，已由 B1 修复**

VM `X06-RUNTIME-002`「期望类型 table，实际为 dict_table」vs 原生产物 `X06-RUNTIME-012`「Runtime ABI 调用失败」。

星崽裁定按“错误身份也要比”的口径算缺口。B1 已把原生动态成员读取改为统一的
`xiao_runtime_dynamic_member_get` 路径：非表值建立 `RuntimeError::type_mismatch("table", actual)`，
原生调试产物现在与 VM 都报告 `X06-RUNTIME-002`、退出码 3。B1 属低优先级错误报告一致性修复，
不改变两边都拒绝该程序的语义结论；新增兼容入口使 Runtime ABI 次版本由 1.9 升至 1.10。

### C. 阶段不同（①标量接收者 / ②字典写入）——**按两边都拒绝关闭**

VM 运行期 `X06-RUNTIME-002` vs 原生**编译期**拒绝（`X11-PROTOCOL-007`，构建退出 2）。

星崽裁定算一致：`value.member` 用在标量上、`value.member = 42` 用在字典上，**都是写错的程序**；
原生「编译期就拒绝」比 VM「跑起来才报错」**更严**，不是更差。把「更早拒绝」当缺口，
会引出「原生不该比 VM 更早发现问题」这种反直觉的结论。

### D. VM 执行、原生拒绝构建（④ 动态错误构造参数）

见 §二。**星崽裁定：登记为原生缺口。**

理由：这一组与 B/C **性质不同**——B/C 的程序本身是错的，VM 只是「晚一点才报错」；
而 ④ 的程序是**合法且正确的**，VM 把它跑完了，原生却造不出产物。
把它和 B/C 混成一条口径，会掩盖「有一类合法程序原生做不了」这个事实。

星崽裁定单独立项、不在 Z-1 实现：给错误构造参数加动态值支持，涉及错误构造 ABI/运行时路径
（[10U](10u-table-construction-and-function-value-abi.md) 的版本规则、[10T](10t-table-method-abi-implementation.md)
的证据要求都要适用），明显超出「收尾」的量级。按 [10Z](10z-19-closeout-and-b-series-implementation.md) §2.2 第 5 条，
**工作量超出就单独立项**。落点见[动态错误构造参数后续立项](10z-dynamic-error-constructor-followup.md)。

### B1 附带新增的释放豁免：登记描述与轨迹不符（审核补正，2026-10-10）

B1 在 `d19a_differential.rs` 的 `native_gap` 里**新增了一条豁免**（只豁免 Drops）。原登记文案是
（现已按本节补正）：

> 「B1：错误身份已与 VM 对齐；动态成员错误路径的**原生临时值释放仍比 VM 少一次**，仅豁免 Drops」

审核在受控原生环境跑 `native_side_matches_the_vm_sides_on_every_case`（整体 **passed，39.84 s**），
取该用例两侧轨迹：

```text
VM     = ["0:1:strong_release", "1:1:strong_release", "2:1:strong_release", "3:1:strong_release", "4:1:destroy"]
NATIVE = ["0:1:strong_release", "1:1:strong_release", "2:1:strong_release", "3:1:strong_release"]
```

**与登记不符**：两侧 `strong_release` **都是 4 条，一次不少**。差的不是释放，是 **`destroy`**——
**原生侧完全没有 destroy 事件，即对象 `1` 从未被销毁**。所以这不是「少一次释放」，
而是「**少一次销毁**」，性质比登记的重。

**按 [10Z-收尾](10z-closeout-execution.md) §2.2 第 6 条补全四要素**（原登记一项未给）：

| 要素 | 实测 |
| --- | --- |
| **事件数** | VM **5** / 原生 **4** |
| **对象数** | **1**（只涉及对象 id `1`） |
| **销毁位置** | VM 在事件索引 **4** 为 `1:destroy`；**原生无** |
| **引用持有机制** | **待判**——需成对实验区分「原生错误路径少一次 release」与「新 ABI 经 `*const XiaoValue` 多持一次引用」；两种都指向对象未归零 |

**待判项（交实现方）**：这是**真泄漏**还是**追踪口径差异**。

- 判为**泄漏**（对象在本应释放的路径上没有归零）→ **应当修**，不能用 Drops 豁免一笔带过。
  Drops 豁免的用途是「已知且有明确依据的**追踪口径**差异」（[10S](10s-cross-platform-evidence-and-release-closeout.md)/[10T](10t-table-method-abi-implementation.md) 那几条），
  不是给「对象没销毁」用的；
- 判为**口径** → 按上表四要素把依据登记全，再保留豁免。

**在判定之前，不要把这条豁免当作「已解释的差异」**——[10Q](10q-selector-case-strength-and-release-audit.md) 的 K4 与
[10Z-收尾](10z-closeout-execution.md) §六 第 6 条（「只比释放总数，不看对象数、销毁位置与持有机制」）说的就是它这个形态。

**代码文案已补正**：`d19a_differential.rs` 的 `native_gap` 中，
`dynamic-dict-member-error-identity` 的 reason 现已采用与轨迹一致的写法：

> 「B1：错误身份已与 VM 对齐。原生侧**无 destroy 事件**（对象未归零），VM 在事件 4 销毁对象 1；
> 事件数 VM 5 / 原生 4，对象数 1。**性质待判**：泄漏还是追踪口径，见 10Z-Z1。仅豁免 Drops。」

（文案改了要重跑一次受控差分确认仍通过；reason 字符串本身不影响判定逻辑。）

**审核已排除的一条路（2026-10-11，供接手的人省一轮排查）**：
**缺口不在 codegen 的释放**——B1 改完之后，`emit_table_get` 的动态分支仍然调用
`self.release_value(object_value);`（`xiao-codegen-llvm/src/dynamic/expression.rs`，
在 `@xiao_runtime_dynamic_member_get` 调用之后）。**所以不是「codegen 漏了一次 release」。**

**该查哪**：新 Runtime 入口里的

```rust
let value = match unsafe { value_to_runtime(&*value) } { … };
```

——**`value_to_runtime` 有没有多持一次引用**。判据是我手里那条轨迹：两侧 `strong_release`
**都是 4 条**，而**轨迹只记 release/destroy、不记 retain**（[10Q](10q-selector-case-strength-and-release-audit.md) 定的口径）。
因此「原生多持一次、且那次从未释放」正好产出这个形态——**引用计数停在 1，永远到不了 0，所以没有 destroy**。
这与**泄漏**一致。

**成对实验**：只改一处（补上那次释放，或直接审 `value_to_runtime` 的返回是否拥有强引用）→ 采数 →
看 `destroy` 是否出现、轨迹是否与 VM 逐项一致 → 恢复。

本批已补 Runtime ABI 回归夹具 `abi::tests::dynamic_member_type_error_releases_input_value`：
直接把字典值传入 `xiao_runtime_dynamic_member_get` 时，轨迹为两次 `strong_release` 后出现
`destroy`，因此单独的 `value_to_runtime` 借用转换路径可以归零；原生函数实参/临时槽组合的
受控差异仍未由这条夹具覆盖，不能据此提前撤销豁免或认定泄漏已修复。

## 四、按三种口径，Z-1 分别要做什么

| 裁定 | Z-1 的产出 | 对 19 的 O6 条件 2 |
| --- | --- | --- |
| B 算缺口；C 算一致；D 单独立项 | **Z-1 已完成 B1**：统一字典接收者错误身份；C 只改文档；D 转后续立项 | 缺口清单保留 D 一条；B 已修复，C 不新增 |

**裁定已在 19D 的 O6 条件 2 里如实体现**：B 的差分已修复，C 作为两边都拒绝关闭，D 保留为已立项缺口。

## 五、落点（口径定了之后）

```text
docs/DevDocs/10p-differential-exemption-and-coverage.md   四组判定与理由；旧判定保留、加列不改
docs/DevDocs/19d-performance-comparison.md                O6 条件 2 的缺口清单按已裁定口径更新
docs/DevDocs/10z-y3-stage-report.md                       把 §三 的结论指向本文
docs/DevDocs/README.md                                    主表登记
docs/DevDocs/10z-dynamic-error-constructor-followup.md    D 的错误构造 ABI 后续立项
core/rust/crates/xiao-codegen-llvm/…                       B1 已实现，含运行时 ABI、差分用例与原生复跑
```

## 六、不要做的事

- **不要为了少做事而把 ④ 压成「两边都拒绝」**——它拒绝的是**合法程序**，与 B/C 性质不同（§二）；
- **不要为了让缺口清单看起来干净而选「全部算一致」**——口径要按可观察行为定，不是按工作量定；
- **不要把 ④ 硬塞进 Z-1**——它涉及错误构造 ABI，超出收尾量级（§三 D）；
- **不要重新取证**：四组的实跑证据已齐（[10Z-Y3](10z-y3-stage-report.md) + 其日志 + `y3_selector_plan.rs` 正式测试），
  引用即可。

## 七、验收

1. B / C / D **分别**有裁定记录；
2. 10P 的四组判定与裁定一致，旧判定未被抹掉；
3. 19D 的 O6 条件 2 缺口清单已更新，④ 的独立立项去向已写明；
4. B1 含差分用例、Runtime ABI 测试与原生侧复跑（[10Y](10y-b-series-triage-rework.md) §2.1 的要求不变）；
5. 没有把 D 纳入 Z-1 实现；
6. `cargo test --workspace`、clippy、fmt、`bun test`、`bun run check`、`bunx tsc --noEmit` 全绿。

## 已定裁定

0. **B1 新增的释放豁免**：登记描述与实测轨迹不符（不是「少一次释放」而是「**少 destroy**」，
   即对象从未销毁），四要素已按实测补全；**「泄漏还是追踪口径」为待判项**，
   由实现方做成对实验判定后再定修或继续豁免（见 §三 末尾）。

1. **B**：算低优先级错误身份缺口；B1 已统一为 `X06-RUNTIME-002`。
2. **C**：算两边都拒绝；原生更早拒绝错误程序，不进入 B 实现。
3. **D**：登记为原生缺口并单独立项，不纳入 Z-1 实现。
4. **③ 守卫注释**：正式测试已固定计划生产契约；是否加代码注释留作后续整理，不阻塞 Z-1。

## 相关页面

- [10Z-Y3. Y3 阶段汇报与收尾对照](10z-y3-stage-report.md) —— 四组对照的完整证据与判定表
- [10Z-收尾. 10 系列收束](10z-closeout-execution.md) —— 本批的序列与 S1 入口条件
- [10P. 差分豁免收敛与剩余拒绝面](10p-differential-exemption-and-coverage.md) —— 28 行分类表与回写对象
- [10Z. 19 收口](10z-19-closeout-and-b-series-implementation.md) §2.2 —— 判为缺口后的实现口径
- [D 后续立项：动态错误构造参数](10z-dynamic-error-constructor-followup.md) —— 错误构造 ABI 的后续范围
- [19D. 性能对照](19d-performance-comparison.md) —— O6 条件 2 的缺口清单落点
