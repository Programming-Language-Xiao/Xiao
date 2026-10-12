# 10R-实现. 静态失败块接入可恢复错误通道

> **背景**：[10R-溢出后续](10r-static-overflow-followup.md) 只立项未实现（26 行），它把范围写成
> 「把 `ir.rs` 的 7 处 `llvm.trap` 接到 Runtime 可恢复错误通道」。**审核实测发现那条通道在静态路径上没有下游**：
> 静态降低器（`ir.rs`）既**不支持 `try`/`raise`**（`ir.rs:871` 直接 `unsupported("异常控制流")`），
> 也**没有任何 drop/清理发射**，而且**不链接 Runtime**（`dynamic.rs:5-7` 的原话：「静态程序仍由
> `ir::lower_static_program` 生成……**不会因为本模块存在而链接 Runtime**」）。
>
> **一句话概括**：**批的不是「把 trap 换成 call」，是「静态产物要不要从此链接 Runtime」**——
> 这决定了产物体积、可重复构建白名单和精确字节基线的走向，而立项文档把它列成了一行影响面。
>
> 状态：**规划稿 + 静态夹具链接探测已完成 + 两条跨阶段后果已登记（2026-10-12）**。
> 探测本身做对了（只读不改、明写「不能证明没有隐式链接诊断窗口通道」）；
> 审核补了三条（§2.1.2、§2.1.3）：表 C 的「静态从不链接 Runtime」要加 debug 限定；
> **四个基准程序里三个走静态路径** ⇒ **O-1 会动 19D 的性能基线**；
> **O-2 要避开 `WS2_32` 就得拆「报告/诊断事件」，而那正是上一轮判过「不要删」的那一行**。
> 逐程序补测与 §五 第 1 条的范围裁定仍待办。

## 一、Agent 交接上下文

### 1.1 接手前提

1. [10R-溢出后续](10r-static-overflow-followup.md) —— **立项文档**（目标、与 10E 的协调、影响面、验收）；
   本批是它的实现，**但范围要按 §2.1 的探测结果重写**；
2. [10E](10e-n0c-error-paths-and-mapping.md) §3.4 与其他节 —— **平台 Fatal 与语言级错误的边界**；
3. [09R2c](09r2c-exception-control-flow.md) —— **异常路由与清理链的既有设计与口径**（动态/VM 侧已实现）；
4. [09R2g](09r2g-for-and-iteration.md)、[10L §11.4](10l-native-dynamic-alignment.md) —— 7 处 `llvm.trap` 的**量化出处**；
5. [10Z-D](10z-dynamic-error-constructor-followup.md) —— **动态错误构造参数**立项；与本批同触 ABI 与原生错误发射面（见 §2.7）；
6. `xiao-diagnostics/src/lib.rs` 与 `xiao-runtime-abi/src/lib.rs` —— **错误身份与 ABI 符号的权威出处**；
7. [19D](19d-performance-comparison.md)、[19C](19c-maintenance-and-debug.md) —— 体积回归的**精确字节基线**与可重复构建白名单（链接面一变它们就相关）。

### 1.2 现状盘点（2026-10-12，审核实测）

**表 A：7 个发射点（以代码为准，`core/rust/crates/xiao-codegen-llvm/src/ir.rs`）**

| # | 位置 | 触发条件 | 今天的失败方式 |
| --- | --- | --- | --- |
| 1 | `ir.rs:1350` → `branch_on_trap`（`ir.rs:1716`） | 整数算术溢出（`llvm.*.with.overflow` 位） | `llvm.trap` |
| 2 | `ir.rs:1515` → 同上 | 整数除/模的**除数为 0** | `llvm.trap` |
| 3 | `ir.rs:1527` → 同上 | 最小值除/模 **-1** | `llvm.trap` |
| 4 | `ir.rs:1622` → 同上 | 整数**窄化越界** | `llvm.trap` |
| 5 | `ir.rs:1692` → 同上 | 浮点有限性/范围（`hi \|\| lo \|\| nan`） | `llvm.trap` |
| 6 | `ir.rs:1395`（`finite.trap`） | `check_finite` 的 **NaN 分支** | `llvm.trap` |
| 7 | `ir.rs:1406`（`finite.range_trap`） | `check_finite` 的**越界分支** | `llvm.trap` |

⇒ 「7 个发射点模板」= **5 处走 `branch_on_trap` + 2 处直接发 trap**（`check_finite` 的两条）。

**表 B：好消息——span 的管道已经铺好了**

`branch_on_trap(&mut self, condition: String, _span: IrSpan)` 与
`check_finite(&mut self, value, ty, span: IrSpan)` **都收 span**，但函数体里 `_span` 未用、`check_finite` 末尾写着 `let _ = span;`。
⇒ **7 处全部已有 span 在手**，本批要做的是**用掉它**，不是新铺管道。

**表 C：坏消息——那条通道没有下游（这是本批真正的范围问题）**

| 事实 | 出处 |
| --- | --- |
| 静态降低器**不支持 `try`/`raise`**：`return Err(unsupported("异常控制流", …))` | `ir.rs:871` |
| 静态降低器**没有任何 drop / 清理发射** | `ir.rs` 全文无 `drop`/`release`/`cleanup`/`unwind` 命中 |
| 静态降低器**非 debug 构建不链接 Runtime**（静态程序 = `i64/f64/i1`）；**debug 构建会链接** `xiao_native_debug_start`，且 `artifact.rs:391` 要求产物含该符号 | `dynamic.rs:5-7` 的模块文档原话；`ir.rs:400`、`artifact.rs:391`（**审核 2026-10-12 补的限定**，见 §2.1.2） |
| 静态降低器只接受标量类型（`str`、容器、函数值均 `Unsupported`） | `ir.rs:305/309/314/318` |
| **动态**降低器已有完整的可恢复错误通道与符号：`xiao_runtime_error_class()`、`xiao_runtime_error_attach_span(i64,i64)`、`xiao_runtime_error_raise_value(ptr,ptr)`、`xiao_runtime_error_raise_type(...)` | `xiao-runtime-abi/src/lib.rs:667-692`；`dynamic/control.rs:270`、`dynamic.rs:378`、`dynamic/runtime_abi.rs:133/155` |

⇒ 立项文档验收写的「携带源码 span，**沿所属函数/调用方 `catch`/`finally` 清理链传播**」，
在静态路径上**今天没有任何链可沿**。所以：**本批必须先决定范围切在哪**（§五 第 1 条），
否则会做出「错误构造出来了但无处可去」的半成品。

**表 D：错误身份的权威（`xiao-diagnostics/src/lib.rs`）**

| 模板 | 应使用的身份 | 依据 |
| --- | --- | --- |
| 整数算术溢出、最小值除 -1、浮点非有限（表 A 的 1/3/5/6/7 中语义属溢出与非有限的） | `NUMERIC_OVERFLOW_CODE = X06-RUNTIME-009`（**文档字符串写着「溢出或产生非有限结果」**） | `xiao-diagnostics/src/lib.rs:35` |
| 除数为 0（表 A 的 2） | **`DIVISION_BY_ZERO_CODE = X06-RUNTIME-013`**（「整数除法或取模的除数为零」） | `xiao-diagnostics/src/lib.rs:43` |
| 整数窄化越界（表 A 的 4） | **待定**：009 的语义是溢出/非有限，窄化是否归它、还是 `INVALID_VALUE_CODE = X06-RUNTIME-012`（`lib.rs:41`），**必须逐条定**（立项文档明写「不能将所有七处都改为溢出码」） | 同上 |

### 1.3 本批边界

| 子任务 | 内容 | 本批 |
| --- | --- | --- |
| §2.1 通道与缺口探测 | 静态/动态发射器差异、现有符号、**链接 Runtime 的代价实测** | **做，且排第一** |
| 7 处失败块接通道 | 构造带 code + span 的可恢复错误 → 标准报告与退出码 | **做**（范围按 §五 第 1 条定） |
| 静态 `try`/`raise` + 清理链 | 静态异常控制流与 drop 发射 | **待裁定**（见 §五 第 1 条；倾向拆到后续批次） |
| 平台 Fatal 与语言错误的分界 | 真非法指令仍 Fatal；不许把平台 trap 一律翻译 | **做**（§2.4） |
| ABI 版本与符号 | 是否需要新符号、主/次版本怎么动 | **做**（§2.5） |
| 纯静态产物的体积/白名单 | 链接面变化后的精确基线处置 | **做**（§2.6） |
| F-3 | `tests/benchmarks/**` 的触发正反例（10 系列遗留） | **顺手收掉**（§2.8） |
| 10Z-D 动态错误构造参数 | —— | **待裁定是否并批**（§2.7） |
| 20C 的落点准备（N-1/N-2/N-3） | `codeRoots`/00A/`sourceExtensions`/身份字段关系 | **不做**，归 20C 库代码之前那一批 |

## 二、必须先冻结的 8 条

### 2.1 **第一步是探测：静态产物链接 Runtime 到底要付什么**

**为什么第一**：立项文档把「纯静态产物是否需链接 Runtime 的裁剪/产物验证」列成一行影响面，
但它其实是**本批最大的那个决定**——静态程序今天**不链接 Runtime**，接通道就意味着要链。

**冻结**：

1. 探测要**成表**，不许一句「影响可控」：
   - 静态程序链接 Runtime 后，**产物字节数**变化多少（拿 `tests/benchmarks/programs/` 与 19C 的体积基线程序各测一次）；
   - **是否会拉入诊断窗口**（立项文档明写「**不隐式链接完整诊断窗口**」）——用符号面/依赖面证明，不是读注释；
   - 是否影响**可重复构建白名单**（19 的 `19.18`/条件 4 与 10S 的白名单口径）；
2. 探测**只读不改**：本步骤不修改 `ir.rs`，先出结论再动手；
3. 结论**先汇报再继续**——它决定本批范围是「甲/乙」哪条（§五 第 1 条）。

### 2.1.1 **Windows 受控静态夹具探测记录（2026-10-12）**

本次探测只生成临时夹具，没有修改 `ir.rs` 或正式测试。固定目标为
`x86_64-pc-windows-msvc`，工具为 clang 22.1.2，Runtime 为当前 release
`xiao_runtime.lib`（20,792,154 字节）。三份样本来自同一份静态标量 LLVM IR：

这三份样本只覆盖“最小静态标量夹具”。§2.1 要求的
`tests/benchmarks/programs/` 四族源码和 19C `SIZE_REGRESSION_SOURCE` 基线尚未在本提交中测量，
因此下表用于判断链接机制和上限量级，**不构成完整样本矩阵的结论**。

| 样本 | 链接输入 | 产物字节数 | PE 运行时依赖 | 相对纯静态 |
| --- | --- | ---: | --- | ---: |
| `static` | 不引用 Runtime | 107,520 | `KERNEL32.dll` | 基线 |
| `error-class` | 仅引用 `xiao_runtime_error_class` | 218,112 | `api-ms-win-core-synch-l1-2-0.dll`、`KERNEL32.dll`、`ntdll.dll` | +110,592（+102.9%） |
| `error-report` | 再引用 `xiao_runtime_error_attach_span`、`xiao_runtime_error_exit_code`、`xiao_runtime_error_report` | 513,024 | 上述三项 + `WS2_32.dll` | +405,504（+377.1%） |

`error-report` 样本没有调用 `xiao_runtime_diagnostic_prepare`、
`xiao_runtime_diagnostic_ready` 或 `xiao_runtime_diagnostic_finish`，但仍出现
`WS2_32.dll`。原因已逐行核对：`xiao_runtime_error_report` 的
`write_runtime_report` 无条件调用 `xiao_runtime_diagnostic_event`（`xiao-runtime/src/abi.rs:1282-1322`），
因此普通错误报告会把诊断事件的网络通道一并拉入。当前实现可以证明“没有自动启动握手”，
却**不能**证明“没有隐式链接诊断窗口通道”；这项验收在拆分报告与诊断事件之前保持未通过。

重复链接的原始 PE 字节在每个样本中只差两个链接时间戳（文件头和调试目录）；按 15C
已有的 `PE.TimeDateStamp` 与 `PE.DebugDirectory.TimeDateStamp` 归一化后，三份样本均逐字节相同，
没有新增可重复构建白名单项。这个结果只说明现有归一化仍有效，不把原始时间戳差异伪装成字节相同。

**夹具探测结论**：静态路径接入 Runtime 的代价不是零；只读错误身份入口已经使产物约增至 2.03 倍，
调用现有完整报告入口约增至 4.77 倍并引入 `WS2_32.dll`。在确定是否接受该链接面、以及是否先把
普通错误报告与诊断事件拆开之前，本批不修改 7 处失败块。

### 2.1.2 **审核复核与两条补测要求（2026-10-12）**

**复核过了什么**（审核独立核，不是读结论）：

1. **根因成立**：`xiao_runtime_error_report` 走的 `write_runtime_report`（`xiao-runtime/src/abi.rs:1283`）
   **无条件**调用 `let _ = xiao_runtime_diagnostic_event(&event);`（同文件 **:1320**，无会话判断、无守卫）
   ⇒ 报告路径本身就是诊断事件的生产者，`WS2_32.dll` 必然被拉进 ✓；
2. **符号归属也核了**：`llvm-nm --print-file-name xiao_runtime.lib` 显示
   `xiao_runtime_error_class` / `xiao_runtime_error_report` 在 `cgu.02`、
   `xiao_runtime_diagnostic_event` 在 **另一个** `cgu.15` —— 所以 `WS2_32` 是靠**调用引用**拉进来的，
   这也解释了为什么 `error-class` 样本没有 `WS2_32`（它只是一行 `pending_class().raw()`，不碰诊断）✓；
3. **本次两个提交零代码改动**（`git show --stat` 无 `.rs`）⇒ §2.1 第 2 条「只读不改」满足 ✓；
4. **没有把「没证明」写成「证明了」**：明写「可以证明没有自动启动握手，却**不能**证明没有隐式链接
   诊断窗口通道」，并把该验收标为**未通过**——这正是要的态度。

**⚠ 我的表 C 有一句写绝对了，要改**：静态降低器**不是**「从不链接 Runtime」——
`ir.rs:400` 在 `debug_startup.is_some()` 时声明 `xiao_native_debug_start()`，
而 `artifact.rs:391` 会**要求产物里有这个符号**。所以准确说法是：
**非 debug 的静态构建不链接 Runtime；debug 的静态构建已经链接一个 Runtime 侧符号**。
这条不只是纠错——它说明「静态产物链接 Runtime 符号」**在本仓库已有先例**，路线的可行性因此更高。

**补测要求（比 43ac124 登记的更要紧，见 O-1）**：

1. **四个基准程序里三个走静态路径**（审核读源码确认，判据见 `dynamic/predicate.rs:103`
   `program_uses_runtime`）：
   `deep-call-recursion.xiao`、`deep-expression-arithmetic.xiao`、`named-local-loop.xiao` 是**纯标量** ⇒ **静态路径**；
   `container-dense.xiao` 用容器/表/`try-finally`/`text` ⇒ 动态路径。
   ⇒ **4.77× 有真实载体**（不是只有合成夹具），且它们**正是 19D 的性能基准**；
2. **19C 的体积基线源码是 `print("19C size baseline")`** ⇒ `print` 带 `str` ⇒ **动态路径** ⇒
   大概不在射程内——但**要实测确认**，不许推断；
3. 所以补测要**逐个基准程序**测：产物字节数（前/后）、PE 依赖面、以及**是否本来就走动态路径**。

### 2.1.3 **两条新增的跨阶段后果（审核实测，本批必须一起登记）**

**O-1：10R 会动 19D 的性能基线。** 19.16 测的是**整进程挂钟**（冷启动含在内），
而上面三个走静态路径的基准正是 19D 的被测对象——静态产物体积从 107KB 量级变到 218KB/513KB，
**冷启动必然被影响**。⇒ 10R 一旦落地，19D 的受控取数与**已冻结的 10% 阈值**可能都要按
**基线变更流程**重取/重判（`baseline.json` 的 `threshold` 与 sha256 同步、摘要回写）。
**这不是「顺手改一个数」**，是那次冻结明文规定的基线变更。
**审核补记**：本条是我 10R 规划里**漏掉**的跨阶段后果（规划只看代码，没接性能面）。

**O-2：要避开 `WS2_32` 就得拆「报告 / 诊断事件」，而那正好是上一轮判过的那一行。**
审核在收口轮明确判过：无会话时那行
`X11-DIAGNOSTIC-CHANNEL-001: 诊断事件没有已建立的会话` **不是缺陷、不要删**
（依据 [10E](10e-n0c-error-paths-and-mapping.md) §3.4「失败时要有确定的行为——**但『静默』不是可选项**」）。
而它**正是** `abi.rs:1320` 这条无条件调用的产物。⇒
**「保留那行」与「不拉进 `WS2_32`」是同一个开关的两端**：

- **保留**：吃下 4.77× + `WS2_32.dll`，10E §3.4 的现状不动；
- **拆开**（只在有会话时发事件）：`WS2_32` 掉出去，但那行从普通运行里消失——
  等于把「无会话」从**失败状态**重新分类为**正常**，是**用户可见的契约变更**。

**冻结**：这条**不许**当实现细节顺手处理——要星崽裁定，并在裁定后同步
[10E](10e-n0c-error-paths-and-mapping.md)、[10V](10v-diagnostic-window-hold.md)、
[19D](19d-performance-comparison.md) 与 [19C](19c-maintenance-and-debug.md) 的调试路径说明
（若用户可见行为有变，还要动 `docs/UseDocs/`）；**不许**只改代码不改文档。

### 2.2 **七处模板逐条定身份，不许一律改成溢出码**

**冻结**：

1. 每一处都要写清：**触发条件 / 使用的错误身份 / span 从哪来 / 判定输入是什么**（表 A 的 7 行扩成实现表）；
2. **除零必须用 `X06-RUNTIME-013`**，不是 009（立项文档原文：除零应复用 Runtime 的专用除零身份）；
3. 窄化越界用哪个码**必须显式裁定**（009 还是 012），并写明理由；**不许**默认沿用 009；
4. 浮点的「NaN」与「范围」是否同码要写清（`NUMERIC_OVERFLOW_CODE` 的文档字符串含「非有限结果」，倾向同码）；
5. 每一条都要有**边界内/边界外**两个输入（立项文档的验收原话）。

### 2.3 **不许用「删检查换性能」，成功路径结果不变**

**冻结**：checked intrinsic 的成功路径**保留现有计算结果**；不得为了少一个失败块而放宽检查、
也不得把检查挪到只有动态路径才有的地方。**性能收益不是本批目标**（立项文档：不使用开发机计时宣称性能达标）。

### 2.4 **平台 Fatal 与语言级错误的分界（与 10E 协调）**

**冻结**：

1. **真正的外部硬件非法指令仍为平台 Fatal**；
2. **编译器主动发射的语言级数值失败不再借非法指令模拟**——这正是本批要消除的形态；
3. **不许**把平台非法指令捕获后一律翻译成可恢复 `ArithmeticError`（10R 立项文档的原话）；
4. 两条边界都要有**用例证明**：一个真 Fatal（外部非法指令）与一个语言级失败（数值越界），
   在原生侧走**不同的**通道。

### 2.5 **ABI：先定「要不要新符号」，再定版本**

**冻结**：

1. 先查清现有符号够不够（表 C 已列出四个 `xiao_runtime_error_*`）；**够用就不动 ABI**，
   不够才扩展——**不许**先把版本号升了再找理由；
2. 若扩展：主/次版本、**工具链指纹**与**旧 Runtime 拒绝策略**必须一致（10T/10U 的版本规则）；
3. **不得**隐式链接完整诊断窗口（同 §2.1 第 1 条）。

### 2.6 **体积/白名单的基线变更要走流程，不许顺手改数字**

**冻结**：

1. 若链接面变化导致体积变化，**按基线变更单独登记**（改了什么、为什么、影响哪份基线），
   **不许**为了让门禁变绿而直接改 `assert_eq!` 里的字节数；
2. 19C 的体积基线（`artifact_size_regression.rs` 的**精确相等断言**）与 19D 的 `baseline.json`
   都在射程内，**先测再判**；
3. 可重复构建（两次构建逐字节相同）**必须仍然成立**——这是发布门槛，不是可选项。

### 2.7 **与 10Z-D 的关系：并批还是相邻，要一次裁定**

**事实**：[10Z-D](10z-dynamic-error-constructor-followup.md)（动态错误构造参数）与本批**同时**要动
「原生可恢复错误的发射面 + Runtime ABI + 三路差分验收」。10Z-D 的实现要求里明写需要 `xiao_runtime_error_new_values`。

**冻结**：

1. **一条 ABI 变更只做一次**：若两批都动 ABI 主/次版本，就会**连着升两次**、
   并让 19 的兼容矩阵与差分矩阵**连改两次**——所以默认**并批**，除非有明确的反理由；
2. 若星崽裁定**不并批**，则要写明**先后顺序与各自的 ABI 版本落点**，并说明为什么不并；
3. 无论并批与否，**错误身份的单一来源**都是 `xiao-diagnostics`，**不许**在 LLVM 文本里写死码字面量。

### 2.8 **F-3 顺手收掉（10 系列遗留，与本批同型）**

**冻结**：

1. 做法沿用 G-1 那轮（已验证可行）：从 `origin/main` 起**一次性分支**，两个提交各只碰 `tests/benchmarks/**`——
   A 破坏 `cpu_model_parser_accepts_linux_field_separators` 的期望值 → 推送确认维护回归**被触发并变红**；
   B 还原 → **转绿**；做完删分支；
2. **纪律三条**：只碰该路径、不顺手带 `docs/**` 或工作流文件、**并确认分支相对 `origin/main` 的整体改动只有该路径**；
3. 不收就写进遗留清单——**不许两边都不提**。

## 三、落点

```text
core/rust/crates/xiao-codegen-llvm/src/ir.rs             7 处失败块；branch_on_trap/check_finite 用掉 span
core/rust/crates/xiao-codegen-llvm/src/dynamic/runtime_abi.rs   ABI 声明面（若需新符号）
core/rust/crates/xiao-runtime/src/abi.rs                  Runtime 侧实现（若需新符号）
core/rust/crates/xiao-runtime-abi/src/lib.rs              ABI 符号与版本常量
core/rust/crates/xiao-driver/tests/d19a_differential.rs    原生/VM 差分用例（错误身份、span、退出码）
core/rust/crates/xiao-driver/tests/artifact_size_regression.rs  体积基线（若链接面变化，按基线变更走）
docs/DevDocs/10r-static-overflow-followup.md              实施记录回填（立项 → 已实现）
docs/DevDocs/10z-dynamic-error-constructor-followup.md     §2.7 并批时一并收口
docs/DevDocs/README.md                                    本批登记与状态
```

## 四、验收

1. **探测**（§2.1）：链接代价表已出（字节数、符号面/依赖面、白名单影响），**先汇报后动手**；
2. **7 处逐条**：每处有触发条件、错误身份、span 来源、判定输入，且有**边界内/边界外**两输入；
   **除零为 `X06-RUNTIME-013`**，窄化身份有显式裁定记录；
3. **身份一致**：同一源码在 VM 与原生上的**错误码/span/退出码**一致（差异要逐条解释，不许含糊）；
4. **分界**：真非法指令仍是 Fatal，语言级失败走可恢复通道，两者各有用例；
5. **产物**：两次构建逐字节相同；体积与白名单若变化，**按基线变更登记**；
6. **ABI**：要么没动（并写明「现有符号够用」的证据），要么动了且工具链指纹与旧 Runtime 拒绝策略一致；
7. **F-3**：有运行号或明确写进遗留清单；
8. **O-1/O-2**（§2.1.3）：两条跨阶段后果都有处置记录——O-1 要么重取 19D 基线并按基线变更登记，
   要么有**逐程序补测**证明静态剖面不受影响；O-2 要么保留现状并写明接受 `WS2_32`，
   要么拆开后**文档与代码同批改**（10E/10V/19C，必要时 UseDocs）；**不许**只改代码；
9. **通用**：`cargo test --workspace`、clippy、fmt、`bun test`、`bun run check`、`bunx tsc --noEmit` 全绿；
   提交标题带类型前缀、正文非空；**文档粗体标记成对**（本仓库已出现 5 次未闭合）。

## 五、待定决策（给星崽）

1. **范围切在哪**——这是本批最大的一个：
   - **甲（完整，按立项文档字面）**：先实现静态 `try`/`raise` + 清理链，再把 7 处接上去。
     代价：等于把 09R2c 的异常路由在 LLVM 静态路径上做一遍，**远超「接通道」**；
   - **乙（缩小，审核倾向）**：本批只做「**失败块构造可恢复错误（code + span）→ 标准报告与退出码**」，
     **不做 catch/finally 传播**；把 `try`/`raise` 与清理链**明确列为后续批次**，
     并在立项文档里把验收的「沿清理链传播」改成后续段的那一条。理由：乙能**立刻兑现**
     「不再借非法指令模拟」与「错误身份双路径一致」，且**不依赖**静态异常控制流；
     而甲会让本批滑成一个大工程，期间静态路径的错误仍然是 trap（半成品状态更久）。
2. **10Z-D 是否并批**（§2.7）：审核倾向**并批**（一条 ABI 变更只做一次）；
3. **静态产物链接 Runtime 是否接受**（§2.1）：若探测显示体积/白名单代价过大，
   要不要退成「只为错误路径链接一个最小符号集」——**由探测结论决定，现在不预设**；
4. **窄化越界的身份**（§2.2 第 3 条）：009 还是 012；
5. **O-2：要不要把「普通错误报告」与「诊断事件」拆开**（§2.1.3）——这是本轮探测带出来的新裁定点，
   **两端的代价都已写清**：保留则吃下 4.77× + `WS2_32.dll`；拆开则那行
   `X11-DIAGNOSTIC-CHANNEL-001` 从普通运行里消失（**用户可见的契约变更**，要同步 10E/10V/19C，
   必要时动 UseDocs）。审核**不倾向任何一边**，因为这两端都是既有冻结的直接推论；
   但要求：**裁定后文档与代码同批改**。
6. **O-1：10R 落地后 19D 的受控取数与阈值要不要重取**（§2.1.3）——三个走静态路径的基准正是
   19D 的被测对象，而 19.16 测的是整进程挂钟。审核倾向：**先把逐程序补测做完再判**，
   不要现在就承诺「阈值不变」。

## 相关页面

- [10R-溢出后续. 静态溢出统一错误路径](10r-static-overflow-followup.md) —— **立项文档**；本批实现它，但范围按本文 §五 第 1 条裁剪
- [10E. N0-C 错误路径与映射](10e-n0c-error-paths-and-mapping.md) —— 平台 Fatal 与语言级错误的边界出处
- [09R2c. 异常控制流实现交接文档](09r2c-exception-control-flow.md) —— 异常路由与清理链的既有设计（静态侧若走甲，参照它）
- [10Z-D. 动态错误构造参数后续立项](10z-dynamic-error-constructor-followup.md) —— §2.7 的并批对象
- [10L. 原生动态对齐](10l-native-dynamic-alignment.md) §11.4 —— 7 处 trap 的量化出处
- [19C. 维护策略与调试三平台](19c-maintenance-and-debug.md)、[19D. 性能对照](19d-performance-comparison.md) —— 体积基线与白名单口径
- [20C-冻结. 10 系列关闭与 20C 六条冻结](20c-freeze-and-10-series-closeout.md) —— 本批是它 §3.7 那道门禁的兑现
