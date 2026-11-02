# 20O-B1-IMP-REM. 剩余四库、F10 与收口规划

> **本页保留 REM 的实施规划并登记当前进度，不替代冻结契约或 B1-CLOSE。**
> 精确契约仍以 [B1 冻结草案 §三～§七](20o-b1-freeze-draft.md#三七库已确认签名与调用边界f2f8f12) 为唯一正文；
> 本页只规划 math/statistics/fmt 之后的四库、F9/F10 和收口前置。
> 不新增 intrinsic、ABI、私有 helper、公开 `dynamic` 或语言语法。
>
> 状态：**REM-A/B/C退出对账完成，CLOSE已同步七库教程/registry为verified，状态提交门禁与证据已收齐（2026-11-03）**。
> 本页保留REM实施规划；下文planned是REM当时的出口要求，当前进度见[CLOSE收口](20o-b1-closeout.md)。
> 基线为已推送的 `90ea5a5a`；三模块/12 入口的 VM/三平台证据见
> [41385483703](assets/20o-b1-imp/self-review-41385483703/README.md)。
> 七库、生成器20释放前置及固定源码身份后的完整F10已由41464439400第二次尝试 / 41464421187取证；旧721条为历史快照，见 [最终对账](20o-b1-rem-evidence.md)。B1-CLOSE已独立收口。
> REM交付时七库为planned；CLOSE当前已同步教程与registry为verified，状态提交门禁与退出对账现已完成。

hex 已通过本机九项契约与原生 O0/O3；Base64 已通过本机九项契约、两模式最大合法输入、
实际解码长度超限及尾位优先级；原生 O0/O3 精确目标已通过。两库同名入口的别名和函数内导入已加入共享案例。
以上保留逐模块本机记录；整批三平台功能、生成器20释放与完整F10已取证，见 [最终证据](20o-b1-rem-evidence.md)。
URL 已通过六模块九项契约、全部 UTF-8 临界/非法向量和 ASCII/四字节字符最大资源边界，
原生 O0/O3 精确目标已通过。
CSV 已通过七模块 20/20 的九项契约、最大 ASCII/Unicode/全引号/多换行/字段数、E 等号及超限
资源用例，原生 O0/O3 精确目标已通过。F10 和显式释放诊断在 REM-C 独立执行。

## 一、当前事实与入口

| 项 | 当前状态 | 依据 |
| --- | --- | --- |
| 已落地产品 | 七模块20/20已收口；生成器20功能与释放三平台已取证，七库verified | [当前证据](20o-b1-rem-evidence.md) |
| 已修复前置 | std 别名、固定命名调用、模块绑定、字符串码点索引 | [返工记录](20o-b1-imp-blockers.md) |
| 三平台证据 | 41464439400第二次尝试全绿，同提交门禁41464421187成功；历史22D失败与债务保留 | [最终证据](20o-b1-rem-evidence.md) |
| 产品函数体 | 20项已落地，IMP/REM资源/性能/生命周期已取证，CLOSE只复核和升级状态 | [B1 契约 §三](20o-b1-freeze-draft.md#三七库已确认签名与调用边界f2f8f12) |
| 性能/生命周期 | 固定源码身份工具重跑31负载，四报告721条完整通过；生成器20六组释放向量三平台均匹配 | [完整报告](assets/20o-b1-imp/rem-identity-final-41464439400/README.md) / [释放诊断](20o-b1-rem-release-prerequisite.md) |
| 教程/registry | CLOSE已同步七库为verified | `docs/module-registry.json` |

本阶段只处理这些未完成项，不重问 F1～F13/E-2，不重做 FIX-B，也不认领 19/11/22 发布债务。

## 二、阶段拆分

### REM-A：hex 与 base64（编码对与同名碰撞）

目标文件：`stdlib/hex.xiao`、`stdlib/base64.xiao`，四个入口：

```text
encode([int] bytes) -> str
decode(str text) -> [int]
encode([int] bytes, bool url_safe) -> str
decode(str text, bool url_safe) -> [int]
```

必须：

- 按 B1 §3.2/§3.3 实现严格字节、大小写、补位/尾位和两种 Base64 字母表；
- 使用 B1 §四限额，恰好上限合法、上限+1 失败；错误码按 §六；
- 不修改输入数组；结果数组新建；失败无部分结果；
- 补 `hex`/`base64` 的 `encode`/`decode` 别名导入碰撞测试，不能因同名静态绑定互相覆盖；
- 产品夹具与独立黄金放 `tests/fixtures/20o-b1/imp/{hex,base64}/`，教程保持 planned。

### REM-B：url 与 csv（UTF-8 与状态机）

目标文件：`stdlib/url.xiao`、`stdlib/csv.xiao`，四个入口：

```text
encode_component(str text) -> str
decode_component(str text) -> str
parse_record(str record) -> [str]
format_record([str] fields) -> str
```

必须：

- URL 严格实现 `%HH` 语法、UTF-8 非最短/代理/截断/超范围拒绝、`%00`/BOM/组合字符保留；
- CSV 严格实现记录子集状态机、空字段/零字段、引号四状态、CR/LF、E 预算和写入前预算检查；
- 使用 B1 §四限额，特别是 CSV 的字段/单字段/S 预检和精确
  `E = S + Q + 2*W + (k-1) ≤ 65,536`；失败统一 `CSV_LIMIT_EXCEEDED`；
- 不剥 BOM、不裁剪空白、不自动解析数字/日期；输入只读，失败无部分输出；
- 补 URL 全部 UTF-8 临界向量、CSV 全引号/多换行/字段边界/往返夹具；
- CSV 最大合法记录必须做性能测量，不能只测短记录或预算算术。

### REM-C：20/20 契约、F9/F10 与生命周期前置

完成四个模块后，本阶段做**整批收口准备**，不是直接升级 verified：

1. 用 `core/rust/crates/xiao-driver/tests/support/b1_imp.rs` 的活跃产品清单扩展四处：
   - `20o_b1_probe.rs` 经 `case_names()` 自动执行全部产品，不恢复独立 `B1_IMP_CASES`；
   - `20o_b1_imp_contract.rs` 共用 `PRODUCTS`，与独立 `CALL_CONTRACTS` 自动矩阵对账；
   - `docs/module-registry.json` 四模块的 code/tests；
   - 产品夹具与教程示例。
2. 20 个入口逐项完成：真实导出、公开 `std.`/`stdlib.` 双路径、逐形参类型/缺参/重复/未知/过多、
   未导入拒绝、逐函数移除、真实归档重放、普通包隔离、教程逐字一致。
3. 同名碰撞：hex/base64 `encode`/`decode`、math/statistics `min`/`max`；不能只测单模块。
4. 保持现有两条精确目标，不新增 workflow 路径或守卫路径；现有四路径 28/28 不变。
5. 三平台功能证据另跑一次；B1-CLOSE 再升级教程/registry 为 verified。

### B1-CLOSE（独立收口）

执行细则见 [七库收口规划](20o-b1-closeout-plan.md)。只有在 20/20、F9/F10、释放差异和
本阶段证据全部满足后才进入：

- 三平台维护回归、完整 workspace/clippy/fmt/docs/usedocs/layout/覆盖率；
- 用实际 run/job 更新 B1 文档；
- 七库教程与七库 registry 条目 `planned → verified`；
- 记录未覆盖边界，不把本地结果写成三平台结论。

## 三、实现约束

1. **不新增顶层 helper 或顶层 import**：产品模块顶层只保留冻结的公开 `def`；需要复用
   `stdlib.text` 时只能函数内导入或内联，最终导出集合必须由真实模块接口验证。
2. **沿用现有代码风格**：纯 Xiao、`array_push`、`for/while`、`chr`/`int` 码点能力；
   不新增 intrinsic、泛型约束、私有可见性、默认参数或公开 `dynamic`。
3. **错误与限额直接以 B1 §四/§六为准**，不另立错误身份；资源错误用 `ResourceError`，
   算术不可表示用 `ArithmeticError / X06-RUNTIME-009`，其他内容错误用 `Error`。
4. **输入只读、结果新建、失败无部分输出**；不能通过修改调用方数组或复用其可变存储“省内存”。
5. 结果构造优先块构造/平衡拼接；不要逐字符反复 `result = result + character` 造成二次复制。
6. 不修改已冻结的 F1～F13/E-2、ABI 1.18、CODEGEN_VERSION 或 22D/19/11 发布债务；
   若实现需要新增语言能力，停止并另开批次。

星崽在2026-11-02明确授权例外：普通for/selector清理缺口按独立前置修复，生成器19→20；
ABI仍为1.18，不改22D排序规则、断言或IR释放语义。该前置与纯库产品分开提交。

## 四、测试与证据矩阵

| 层次 | 必测 |
| --- | --- |
| 每模块成功 | 空输入、代表性输入、边界值、完整产品样例与独立黄金 |
| 每模块错误 | B1 §六每个 code 的触发、类别、优先级、`error_code` 实际读取 |
| 每模块资源 | §四每项上限、恰好上限、上限+1、失败无部分结果、输入不变 |
| 每模块差分 | VM 与原生 O0/O3 逐字 stdout/退出码/stderr/清理 |
| 模块接口 | 真实导出集合、`std.`/`stdlib.`、别名/同名碰撞、未导入拒绝 |
| 模块级 | 逐函数移除、真实 `.xar` 重放、普通包隔离、教程 fixture 逐字一致 |
| 全批 | 20/20 入口矩阵、shared support 清单、registry 与源码一致 |
| F9/F10 | 每模块最大合法输入、翻倍比≤3、VM≤10s、原生≤2s、增量内存≤64MiB |
| 生命周期 | 已知原生/VM 字符串释放差异的复现、归因/修复或明确裁定 |

产品夹具与黄金放在 `tests/fixtures/20o-b1/imp/<module>/`；错误用例必须通过
`error_code(err)` 打印稳定 code，不能只打印分支标签。结果黄金不能用同一产品算法生成。

### 4.1 F10 必测项

- hex：65,536 字节 encode、131,072 字节 decode；坏字符/奇数长度优先顺序；
- base64：两种模式 encode/decode、最大输入、真实解码长度超限、补位/尾位；
- url：最大 UTF-8 输入、全 ASCII 与 4 字节字符、多百分号、输出上限；
- csv：65,536 字节记录、全引号/多换行最坏膨胀、E 恰好上限/上限+1、字段数上限；
- format_int/pad 的已有测试保留，不能因扩展到四库而删除；
- 性能测量按固定批次、预热、中位数、O0/O3 记录；未达标就登记并按规则停止，不缩小限额。

### 4.2 释放差异门槛

`20o-b1-imp-blockers.md` 已记录字符串样例的 `native 14 / VM 16`（去掉越界后 `native 12 / VM 13`）
销毁差异：部分输入/末次循环字符只有 strong_release、没有 destroy。下一阶段必须：

1. 用同一 fixture/trace 复现，保存完整日志；
2. 判定是产品调用、字符串索引缓存、原生循环还是生命周期计划的问题；
3. 修复后补 VM/native destroy 数与释放顺序回归；若需改动 22 释放语义或更大范围，
   先单独提交/取得裁定，不能在本批静默放宽断言；
4. B1-CLOSE 之前若仍未修复，只能登记为未收口限制，不能写生命周期验收通过。

## 五、文档与登记落点

释放差异已有显式复现和具体归因，独立前置草案见 [释放诊断](20o-b1-rem-release-prerequisite.md)。
星崽已授权生成器19→20的独立前置；只补原生清理接线，IR释放语义不变，六组同源向量已在三平台匹配。
F10 发现的原生 str 桥接整串临时复制已按内部性能修复独立处理，见
[桥接诊断与重测](20o-b1-rem-string-bridge.md)；它不改变 ABI 或生成器版本。

已提交的产品与内部前置：`08074f6e`（hex）、`8f9f9047`（base64）、`381a1a2e`（url）、
`dab4f289`（csv）、`b585eeb0`（str 桥接）。四库均先过 VM/契约/资源，再过本机原生 O0/O3；
七模块九项契约回归实际执行 20/20 参数矩阵，真实导出、函数移除、归档、普通包与教程一致性共用活跃清单。

显式性能工具覆盖 20 入口的 31 负载，增加 1,024 字段 CSV；原生 O0/O3 分记。
自审发现初版短调用的单次校准未确保 50ms，已改为迭代校准到 100ms，并对正式五批逐项断言≥50ms；
旧报告只保留为探索记录，完整 F10 以修正工具的新报告为准，不按旧 JSON 的 passed 字段直接收口。
报告每完成一条保存检查点，`complete=false` 表示队列未完成。维护回归的手动 `b1_f10=true`
沿用原有作业运行测量并保存 JSON，默认 fmt 性能测试与两条精确目标不变。

```text
stdlib/hex.xiao, base64.xiao, url.xiao, csv.xiao
stdlib/<module>.md                         四模块 README
stdlib/README.md                           模块清单与状态
tests/fixtures/20o-b1/imp/<module>/        产品源码与独立黄金
core/rust/crates/xiao-driver/tests/support/b1_imp.rs 活跃产品清单（四模块加入）
core/rust/crates/xiao-driver/tests/20o_b1_probe.rs    精确目标扩展
core/rust/crates/xiao-driver/tests/20o_b1_imp_contract.rs 契约矩阵自动扩展
docs/UseDocs/library/<module>.md            保留 planned，增加产品示例
docs/module-registry.json                   四模块 code/tests；status 仍 planned
docs/DevDocs/20o-b1-imp-remainder.md        本规划
docs/DevDocs/README.md / 20-builtins...     索引与阶段表
```

共用的 `case_names()`/`PRODUCTS` 必须与 `CALL_CONTRACTS` 的 20 项对账；新增模块后
不能让精确目标仍只跑旧案例而静默通过。`tools/gates/b1-probe-guards.ps1` 保持
四路径 28/28；若确实新增精确入口，必须同步守卫和场景总数。

## 六、退出条件

1. 七模块、20/20 入口全部实现，导出集合恰好等于冻结表；无 helper/顶层 import 泄漏；
2. F9 每个上限和 F10 每个测量项有结果；未达标清楚登记，不伪造通过；
3. 每模块 VM/native O0/O3 + 错误/边界/资源/输入不变/失败清理；
4. 20 项公开双路径、逐形参负例、同名碰撞、移除、归档、普通包和教程一致性通过；
5. 释放差异已修复并有三平台/VM 对账，或有星崽明确的未收口裁定；
6. 默认 workspace、VM 精确、三平台原生精确和完整门禁通过，证据带 run/job；
7. 七库教程/registry 仍为 planned；B1-CLOSE 才升级。

## 七、交接顺序

1. 读 [B1 §三～§七](20o-b1-freeze-draft.md#三七库已确认签名与调用边界f2f8f12)；
2. 读 [B1-IMP 规划](20o-b1-imp-seven-libraries.md) 的四库算法/限额细节；
3. 读本页与 [剩余阻塞](20o-b1-imp-blockers.md)；
4. 读 `core/rust/crates/xiao-driver/tests/support/b1_imp.rs`、`20o_b1_imp_contract.rs`、`20o_b1_probe.rs` 的共享清单模式；
5. 按 REM-A → REM-B → REM-C 提交，每模块先 VM/契约再 native；遇到契约未写明的语义先停下来问星崽。
