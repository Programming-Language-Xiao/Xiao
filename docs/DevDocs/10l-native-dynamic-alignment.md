# 10L. 原生动态入口与 VM 对齐（N0-E）

> **10 阶段的续批**。README 把 10 写成「N0-A/B/C/D 全部交付」，但 19A 到 19D 的差分和探测实测出：
> 原生动态入口仍然拒绝一批 VM 能跑的程序，而且 19 的出口条件 2 和 5 都卡在这里。
>
> **一句话概括本批**：先修 19D 审核发现的小问题，并补一道能提前发现 20AB 那类回归的门；
> 然后**先枚举、后动手**，把原生动态入口拒绝的语句和运行时检查逐类对齐 VM。枚举结果先向星崽汇报，再定范围。
>
> 状态：**扩展范围对齐实现中（2026-10-06）**。星崽已确认接入 3 类语句和全部 14 类运行时检查；整数溢出统一错误路径仍按 §4.3 单独跟踪。

## 一、Agent 交接上下文

### 接手前提

1. [19](19-optimization-release.md) O6 出口条件 2、5，以及 [19A](19a-differential-and-fuzz.md) 的 `native_gap` 登记机制；
2. [19D](19d-performance-comparison.md) 的原生可构建性探测（`native_benchmark_probe.rs`）；
3. [10A](10a-n0-native-closure.md) §1.2 的溢出债项（`:162-168`、`:327-328`）；
4. [10D](10d-environment-gated-test-spec.md) 门控规范；本机复现办法见 19A 实施记录（pwsh、vcvars、`XIAO_*` 变量）；
5. [15A](15a-native-pass-mapping-and-runtime-trimming.md) §3.2：任何级别都不能跳过 Runtime 检查；
6. [20](20-builtins-and-standard-library.md) §七：20C 官方 Xiao 库以 10 的原生 Runtime ABI 为前置。

### 现状盘点（2026-10-06，读代码加 Windows 实测；「未核实」表示没查）

| 环节 | 现状 | 判定 |
| --- | --- | --- |
| 动态入口语句 | 原拒绝点已接入 `For`、`Function`、`Import`；旧错误文案漏报 `for` 的问题已随实现移除 | 首轮已接入；嵌套 `finally` 释放差异仍登记 |
| 动态入口运行时检查 | 原 `dynamic.rs:93` 整体拒绝闸门已移除，14 类检查进入 Runtime/VM 对齐路径 | 首轮已接入；高级选择结果仍待原生实测 |
| 整数溢出 | 原生以 `llvm.trap` 终止（19A 实测 Windows 退出码 `0xC000001D`，无 xiao-error 摘要）；VM 报 `X06-RUNTIME-009`。10A 把它登记为 N0-C 债项；我按关键词查了 10E 到 10I，没找到还债记录，但没有逐篇通读 | 疑似未还 |
| 回归门 | 平台复现工作流每周一定时跑 `--ignored`；`maintenance-regression.yml` 的路径过滤含 `xiao-codegen-llvm`，但该文件里没有 ignored 或 native 步骤。20AB 的回归 10-04 引入，要等 10-05 的定时运行才红 | 缺 |

## 二、19 的出口条件还差什么

按 [12](12-tests-and-milestones.md) O6 逐条对账。结论：**19 还不能收口**，本批只能动其中与原生相关的部分。

| O6 条件 | 现状 | 判定 |
| --- | --- | --- |
| 1 兼容矩阵与升级/拒绝策略 | 矩阵已由代码生成；仍有 `Unverified` 格，我没统计个数 | 部分 |
| 2 差分、模糊、损坏恢复、安全测试全部通过 | Windows 受控原生差分已通过；`nested-finally-drops` 的释放序列仍登记为未覆盖；Linux、macOS 没有运行记录 | **不满足**，本批相关 |
| 3 三平台一致 | 只有 Windows 的证据 | 未验证，等 CI 授权 |
| 4 发布报告与重复构建白名单 | 报告已有；19B 记录白名单待实测，我没有重新核对最新状态 | 未核实 |
| 5 相对 Java 的性能对照 | 全部「数据不足」，`container-dense` 原生被拒 | **不满足**，本批相关 |

## 三、先修的小问题 G1–G3

来自对 19D 的审核。每项单独提交，提交说明写明归属 19D。

| 编号 | 问题 | 依据 | 处置 |
| --- | --- | --- | --- |
| G1 | `reports/19d-performance.json` 记录的 `baseline.json` 摘要是 `627457fd7dedddaf…`，实际是 `627457fd7deddadf…`，第 14、15 位对调。仓库里没有任何代码或测试校验这个值 | 从仓库根目录用 `sha256sum` 重算并逐位比对 | 改正数值，并加一个测试重算摘要后与报告比对。摘要用 `xiao_artifacts::Digest256::of_bytes`，不新增依赖。测试要先写明哈希的是哪份字节，并确认 `.gitattributes` 对该文件的行尾处理，避免 Windows 与 Linux 检出结果不同 |
| G2 | `tests/benchmarks/java/README.md` 没写编译要带 `-encoding UTF-8`。本机 JDK 17 默认按 GBK 读源码，直接 `javac` 失败；JDK 18 以后默认 UTF-8，冻结的 21 不受影响 | 本机用 JDK 17 复现，加参数后四个可比程序的输出与 `manifest.json` 一致 | README 补一句 |
| G3 | 19D 把 `container-dense` 被拒写成「`numeric_range` 缺口」。实际是 `runtime_checks` 非空就整体拒绝，`numeric_range` 只是第一个命中的种类 | `dynamic.rs:93` | 更正 19D 的措辞，种类清单等 §四 的枚举结果 |

另有一条不是缺陷，只是来源：19D 把 Java 21 基线记为星崽 2026-10-06 的裁定，我在对话里没看到这条裁定。列在待定决策里，一句话确认即可。

## 四、必须先冻结的 4 条

### 4.1 **先枚举，后动手，枚举结果先向星崽汇报**

在枚举前范围是未知数：拒绝点有两处，各自覆盖多少种类没有人数过。19D 的探测就是这样做的，做完先报，再定后续工作量。

本节的枚举步骤已经完成，结果和证据见第十一节；本提交后暂停，不进入 §4.2 的后端对齐。

**冻结**：

- 第一步只产出一张表，不改后端：对 `manifest.json` 的五个程序、19A 的 `CASES`、`tests/` 里已有的 VM 语义向量，
  列出进入原生动态入口时被拒的**全部语句种类**和**全部 `runtime_checks` 种类**，每项写明触发它的最小源码；
- `runtime_checks` 本身就是 IR 里的列表，可以直接遍历列全，不必靠「第一个命中」逐个试；
- 表里每项标注 VM 侧对应行为（成功、还是哪个错误码）。VM 也不支持的不算原生缺口，单列；
- 做完提交一份只含文档的枚举记录，并**停下来向星崽汇报**。汇报内容包括：有几类、每类大概要动哪几个文件、
  哪些依赖别的阶段。星崽回复范围之后才做 §4.2。

### 4.2 **对齐规则：VM 是基准，原生向它靠**

- 每修一类，就在 `d19a_differential.rs` 的 `CASES` 加一个 VM 对原生的用例，并摘掉对应的 `native_gap` 登记
  （缺口消失时该机制本来就会报错提醒）；
- 不得为了让原生过而改 VM；两边不一致时，先判断谁错，判断依据写进提交说明；
- 不得跳过运行时检查来让构建通过（15A §3.2）。检查暂时接不通，就继续拒绝，并在枚举表里保持登记；
- 原生侧产出的错误必须带与 VM 相同的稳定错误码。只是「能构建」不算对齐。

### 4.3 **整数溢出单独处理，是否并入由星崽决定**

原生溢出现在以 `llvm.trap` 终止，VM 报 `X06-RUNTIME-009`。要对齐就得让溢出进入统一错误路径，
这比前面的种类大：要产生 Runtime 错误对象、经过 `try`/`catch`/`finally` 展开，还要和 10E 把「非法指令」
归为不可被 `catch` 的平台级异常这一条协调。它不是补一个分支的事。

**冻结**：

- 枚举阶段把它量化：涉及哪几个发射点、现有 `llvm.trap` 失败块有多少处；
- 不预设并入。待定决策 2 请星崽选；
- 不论是否并入，19D 的 `scalar-overflow-and-bool-parity` 都按已有裁定保持「数据不足」，不参与通过判定。

### 4.4 **回归门：让下一次 20AB 式回归在推送时就红**

20AB 的回归（`cd08ca0`，10-04）要等 10-05 的定时运行才暴露，中间没有任何门。提交说明里写的是「受控 LLVM 测试按环境门控」，
即没有实跑。

**冻结**：

- 建议新增一个随 push/PR 触发、路径过滤含 `xiao-codegen-llvm` 和 `xiao-driver` 的原生门控作业，跑
  `n0_a_native_driver` 和 `d19a_differential` 的 `--ignored` 用例。它需要 clang、Runtime staticlib 和目标三元组，
  本机已有可用的准备步骤（19A 实施记录），CI 上照 `platform-reproduction.yml` 的准备方式；
- 这会增加 CI 时长和维护面，所以**不自行决定**，列入待定决策 3；
- 没有授权之前，不新增这个作业，也不推送、不触发任何工作流。

## 五、落点

```text
core/rust/crates/xiao-codegen-llvm/src/dynamic.rs、dynamic/control.rs   拒绝点所在；枚举与对齐都从这里入手
core/rust/crates/xiao-driver/tests/d19a_differential.rs                CASES 与 native_gap 登记
core/rust/crates/xiao-driver/tests/native_benchmark_probe.rs           19D 探测，修完后重跑
core/rust/crates/xiao-driver/tests/                                    G1 的摘要校验测试
tests/benchmarks/java/README.md、tests/benchmarks/reports/19d-performance.json   G1、G2
docs/DevDocs/19d-performance-comparison.md                             G3 更正措辞，回填结果
.github/workflows/                                                     仅在待定决策 3 获批后新增作业
docs/DevDocs/README.md                                                 主表登记 10L
```

## 六、硬约束

1. G1–G3 各自单独提交，提交说明写明归属阶段（§三）；
2. 枚举阶段不改后端，枚举记录提交后先向星崽汇报（§4.1）；
3. VM 是基准，不为了原生过而改 VM（§4.2）；
4. 不跳过运行时检查（§4.2，15A §3.2）；
5. 每修一类就加一个 VM 对原生的差分用例并摘掉对应登记（§4.2）；
6. 不用 `cfg` 让某平台悄悄少跑；本机只有 Windows，Linux、macOS 的结论只认 CI 运行记录；
7. 不新增依赖，`Cargo.lock` 保持同步；
8. 未经星崽授权不推送、不触发工作流、不新增 CI 作业；
9. 提交说明要有正文，否则会被提交检查拦下。

## 七、分步提交

1. **G1、G2、G3**：三个小修复，各一个提交；
2. **枚举**：只含文档的枚举记录，提交后停下来向星崽汇报（§4.1）；
3. **按星崽确认的范围逐类对齐**：每类一个提交，含差分用例和登记摘除（§4.2）；
4. **整数溢出**：仅在待定决策 2 选了并入时做（§4.3），单独提交；
5. **重跑 19D 原生探测**：五个程序现在各是什么状态，照实回填 19D 的实施记录；仍被拒的保持「数据不足」；
6. **回归门**：仅在待定决策 3 获批后做（§4.4）；
7. **CI 实跑**：仅在授权后推送并手动触发，读四个平台各自的日志，把运行号与结论回填；
8. **文档与登记**：本文件实施记录、README 主表、19D 回填。

## 八、最可能翻车的地方

1. **只数第一个命中的拒绝**：`dynamic.rs:93` 一遇到检查就返回，试到一个修一个会低估总量；要直接遍历 `runtime_checks`；
2. **改 VM 迁就原生**，把两边都写成「一致」；
3. **把 `llvm.trap` 当成最终的语言错误**，10A 明确说过不能这么宣称；
4. **摘掉 `native_gap` 却没加对应用例**，缺口消失得没有证据；
5. **G1 的哈希受行尾影响**：Windows 检出可能是 CRLF，Linux 是 LF，摘要会不同；
6. **把 Windows 的结果写成三平台一致**；
7. **为了压 CI 时长把 ignored 用例排除出新作业**，门就形同虚设；
8. **汇报前就动手实现**，枚举的意义是让星崽先定范围；
9. **提交说明没写正文**，被提交检查拦下；
10. **忘了 `Cargo.lock` 同步**。

## 九、验收

1. G1 的摘要已改正，且有测试重算比对；G2、G3 已改；三项各自单独提交；
2. 枚举记录已提交，并已向星崽汇报，范围经星崽回复后才开始对齐；
3. 每个对齐过的种类，都有一个 VM 对原生的差分用例，对应 `native_gap` 登记已摘除；
4. 没有通过跳过运行时检查换来的「可构建」；仍接不通的保持拒绝，并在枚举表里登记；
5. 本机 Windows 上 `n0_a_native_driver`、`d19a_differential`、`native_benchmark_probe` 的 `--ignored` 用例按实际结果回填，
   不能把「仍被拒」写成通过；
6. 19D 的实施记录按新的探测结果更新，`container-dense` 等程序的状态照实写；
7. `cargo test --workspace`、`cargo clippy --workspace --all-targets -- -D warnings`、`bun test`、`bun run check` 全绿，
   既有测试未改；
8. Linux、macOS 的所有结论都带 CI 运行号，没有运行号的写「未验证」。

## 十、不负责与不要重复做的事

- 不做性能取数与阈值（19D 的待补项）；
- 不做官方 Xiao 库、`os` 与平台资源（20C、20D）；
- 不改 VM 的语义，不新增优化级别、归档字段或信任规则；
- 不实现 Xiao 自己的 LLVM Pass 注册；
- 不推送、不触发 CI，除非星崽授权。

## 待定决策

1. **范围**：枚举结果出来之前无法估算。建议按 §4.1 先汇报再定，不在本文件里预设。
2. **整数溢出是否并入本批**：建议**不并入**，单独立项。它要让溢出进入统一错误路径，并和 10E 对平台级异常的定义协调，比其他种类大得多。
3. **是否新增随 push/PR 触发的原生门控作业**（§4.4）：建议新增，因为 20AB 的回归已经证明没有这道门会怎样。代价是 CI 时长和一份要维护的 clang 准备步骤。
4. **是否授权推送本地提交并手动触发平台复现工作流**：本地已领先远端 37 个提交。19 的出口条件 3 和 5 的 Linux、macOS 证据都卡在这里。
5. **Java 21 基线的来源确认**：19D 记作星崽 2026-10-06 的裁定，我在对话里没见到。请确认一句，确认后文档保持原样。
6. **20C 是否要等本批**：20 号文档只列了 10 的原生 Runtime ABI 和 11A 包边界为前置，没有写动态入口的函数和导入支持。官方 Xiao 库若用 `def` 和 `import` 写成，就会撞上 §一 的拒绝点，这是我的推断，不是文档原文。建议本批枚举完再定。

## 十一、枚举记录（2026-10-06）

本节是 §4.1 要求的「先枚举、后动手」记录。本提交只改文档；临时探测测试在取数后删除，动态后端、VM、`native_gap` 和 CI 均未修改。

### 11.1 取数范围与方法

- 规定语料为 `tests/benchmarks/manifest.json` 的 5 个程序、19A `CASES` 的 7 个程序，以及 `tests/spec/09-bytecode` 递归发现的 79 个语义向量源码，共 **91 个**。Windows 本机一次性前端编译结果为 **91 成功、0 拒绝**。
- 为补齐语句类别，另用 3 个最小源码观察 `For`、`Function`、`Import`；这 3 个不计入上面的 91 个语料统计。
- 通过 `FrontendCompiler` 直接遍历 `IrProgram.runtime_checks`，没有只看首个检查；IR 递归计数只表示出现次数，不表示每个出现都从动态入口执行。
- VM 行为取自现有 09R2 向量、11B 模块加载向量和一次性 `xiao-driver` VM 运行。没有把 VM 未能验证的用例记成成功或原生缺口。

### 11.2 被动态入口拒绝的语句种类

动态入口仍在 `core/rust/crates/xiao-codegen-llvm/src/dynamic/control.rs` 统一拒绝这三类 `IrStatementKind`。规定语料和最小例证中的 IR 递归计数为：`For` **15**、`Function` **195**、`Import` **1**；函数体内的计数包含在内。

| 种类 | 最小源码 | VM 行为 | 当前原生状态与预计落点 |
| --- | --- | --- | --- |
| `For` | `items = [1, 2]`；`for item in items`；循环体写入 `total` | 成功；09R2G 的 `for` 向量也在三种 VM 载体通过 | 入口拒绝。预计涉及 `dynamic/control.rs` 的迭代控制流、Runtime 迭代 ABI，以及 `d19a_differential.rs` 的差分证据 |
| `Function` | `def identity(int value) -> int`；返回 `value`；`probe = identity(1)` | 成功，调用深度进入 `identity` | 入口拒绝。预计涉及动态函数槽/调用与入口编排（`dynamic/control.rs`、`dynamic/slot.rs`、相关 Runtime ABI）及差分用例 |
| `Import` | 11B `module-loading.json` 的 `from helper import value`，`helper.xiao` 导出 `value = 7` | 成功，结果加载 `project:helper` 一次；`i4a2_module_loading::module_loading_spec_vectors` 通过 | 入口拒绝。预计需要动态入口、模块加载上下文和 Runtime/驱动边界共同接线，不能只改拒绝文案 |

`For`、`Function`、`Import` 是语句种类全集；静态路径中已有相应消费者不等于动态入口已经支持。`control.rs` 当前错误文案只写「函数或导入语句」，因此也漏报了 `for`，这是实现时要一并修正的诊断面。

### 11.3 `runtime_checks` 全量枚举

下面的 8 类在 91 个规定源码中实际出现，都会被 `dynamic.rs:93` 的「列表非空即拒绝」闸门挡住。`R1` 同时触发多个检查，分别用不同实参观察先失败的检查；这不是把一个错误重复计数。

| 检查种类 | 最小源码/来源 | VM 行为 | 当前原生状态 |
| --- | --- | --- | --- |
| `arithmetic` | **R1**：`def result(value) -> int`；在动态 `for item in value` 中执行 `total = total + item`；`probe = result(["x"])` | `X06-RUNTIME-012`（动态算术的操作数类型不满足） | 已观测，入口整体拒绝；预计改动态表达式/Runtime ABI并保留检查 |
| `dynamic_conversion` | **R1** 同一源码与实参 | 同上，`X06-RUNTIME-012`；检查列表同时含 `arithmetic` 和 `dynamic_conversion` | 已观测，入口整体拒绝；不得删除检查来换取构建 |
| `iterable` | **R1** 同一函数，实参改为整数 `1` | `X06-RUNTIME-024` | 已观测，入口整体拒绝；预计接动态迭代检查与错误码 |
| `numeric_range` | 19A `overflow`：`def square(int value) -> int`；返回 `value * value`；`result = square(4000000000)` | `X06-RUNTIME-009` | 已观测；整数溢出仍按 §4.3 单独决策，不在本枚举提交修复 |
| `selector_bounds` | **R2**：`text = "你好"`；`char = text[-1]` | 成功 | 已观测，入口整体拒绝；预计接选择器边界检查并保持 `X06-RUNTIME-017` |
| `set_operation` | 09R2F1 `runtime-operation-boundary-error`：函数内 `computed = value + {1}`，用动态数组实参调用 | `X06-RUNTIME-021` | 已观测，入口整体拒绝；预计接集合操作检查 |
| `set_comparison` | 09R2F1 `runtime-comparison-boundary-error`：函数内 `return value == {1}`，用动态数组实参调用 | `X06-RUNTIME-022` | 已观测，入口整体拒绝；预计接集合比较检查 |
| `set_membership` | 09R2F1 `runtime-membership-boundary-error`：函数内 `return value in {1}`，用动态数组实参调用 | `X06-RUNTIME-023` | 已观测，入口整体拒绝；预计接集合成员检查 |

本次规定语料没有出现以下 6 类；它们不能写成「已从原生缺口消失」。为确认 VM 侧现有契约，另跑了最小补充源码，结果如下：

| 未在规定语料触发的种类 | 补充最小源码 | VM 侧观察 | 枚举结论 |
| --- | --- | --- | --- |
| `string_boolean` | `def parse(str raw) -> bool`；返回 `raw as bool`；`probe = parse("yes")` | `X06-RUNTIME-002` | 规定语料未触发；保留为待枚举原生类别 |
| `selector_step` | 动态 `step = choose(0)` 后执行 `values{step}[=]` | `X06-RUNTIME-018` | 规定语料未触发；保留为待枚举原生类别 |
| `random_count` | 动态 `count = choose(3)` 后执行 `values[?count]`，候选数为 2 | `X06-RUNTIME-019` | 规定语料未触发；保留为待枚举原生类别 |
| `random_seed` | 动态 `seed = choose(-1)` 后执行 `random.seed(seed)` | `X06-RUNTIME-020` | 规定语料未触发；保留为待枚举原生类别 |
| `set_hashability` | `def make(value)`；构造 `{value}`；用 `[1]` 调用 | `X06-RUNTIME-016` | 规定语料未触发；保留为待枚举原生类别 |
| `boolean_condition` | 动态 `for item in value` 后以 `if item` 作条件 | VM 在执行前验证失败：`X09-BYTECODE-002`（动态条件寄存器类别不匹配） | VM 侧当前没有可引用的成功/稳定 Runtime 错误结果，单列为 VM 未支持；不计入原生缺口 |

### 11.4 整数溢出发射点量化

`core/rust/crates/xiao-codegen-llvm/src/ir.rs` 当前没有把溢出转换成 `X06-RUNTIME-009`。源码层可见的 `llvm.trap` 发射模板共 **7 个点**：

1. `llvm.s{add,sub,mul}.with.overflow` 的整数算术溢出分支 1 个共享调用点；
2. 除零检查、最小整数除以 `-1` 检查各 1 个，共 2 个；
3. 整数窄化范围检查 1 个；
4. 浮点有限性检查的非有限分支和范围分支各 1 个，共 2 个。

其中前 5 个经过 `branch_on_trap` 共享辅助，后 2 个在 `check_finite` 直接发射；每次遇到相应操作会按操作复制失败块，所以「7」是发射点模板数，不是任意程序固定生成 7 个块。Windows 19A 早期实测溢出退出为 `0xC000001D`，没有 `xiao-error` 摘要；扩大范围后，带 `numeric_range` 的动态路径已通过 Runtime ABI 对齐为 `X06-RUNTIME-009`，纯静态路径的 trap 模板仍保留。

### 11.5 结论与范围闸门

本次枚举得到 **3 类语句拒绝**、**8 类已在规定语料触发的 Runtime 检查**，以及 **6 类规定语料未触发的 Runtime 检查**。星崽已确认扩大范围，进入第十二节的实现批次；整数溢出仍按 §4.3 单独跟踪。

取数命令（临时文件已删除）为：

```text
cd core/rust
cargo test -p xiao-driver --test 10l-enum-tmp -- --nocapture
cargo test -p xiao-driver --test i4a2_module_loading module_loading_spec_vectors -- --nocapture
```

## 十二、扩大范围实现记录（2026-10-06）

星崽确认接入全部 14 类检查后，已完成首轮后端与 VM 对齐接线：

- `xiao-runtime` / `xiao-runtime-abi` 新增统一二元、一元、显式转换、动态检查和可迭代访问入口；集合运算、比较、成员判断复用 Runtime 唯一算子表。
- LLVM 动态降低器移除「`runtime_checks` 非空即拒绝」闸门，消费 `arithmetic`、`dynamic_conversion`、`numeric_range`、集合检查、字符串布尔、迭代、选择器步长/边界、随机数量/种子、哈希性和动态条件检查。
- 动态 `for` 使用 Runtime 长度/元素入口；顶层动态函数使用独立 `%xiao.value` 指针调用约定；项目文件模块在导入点初始化一次，支持选定导出绑定和模块命名空间字典成员读取。
- VM 字节码降低在动态条件进入 `BranchIf` 前插入布尔转换，`boolean_condition` 现在得到稳定 `X06-RUNTIME-002`，不再在执行前触发 `X09-BYTECODE-002`。
- 定向回归覆盖 Runtime ABI 算子、3 类语句、动态条件和 LLVM 文本降低。10M 在 Windows `x86_64-pc-windows-msvc` 受控工具链上重跑了 `n0_a_native_driver --ignored`、`d19a_differential --ignored` 与 `native_benchmark_probe --ignored`；四个基准恢复可构建，函数体动态算术/布尔条件差分通过。
- 动态路径的整数溢出已接通 Runtime 检查；静态 `ir.rs` 中 7 处 `llvm.trap` 仍未接入 `X06-RUNTIME-009` 统一错误路径，继续按 §4.3 单独跟踪。

当前仍需后续实测确认的行为是高级范围/随机选择在原生路径上的完整结果序列，以及嵌套 `finally` 中动态函数的完整释放序列；19A 原生差分已将该释放差异登记为后续缺口。扩大范围首轮不宣称这两项完成。

## 相关页面

- [10A. LLVM 原生构建闭环](10a-n0-native-closure.md) —— 溢出债项的登记处
- [10E. N0-C 错误路径与源码映射](10e-n0c-error-paths-and-mapping.md) —— 平台级异常与可恢复错误的边界
- [10D. 环境依赖测试专项规范](10d-environment-gated-test-spec.md) —— 门控用例的准备与计数
- [15A](15a-native-pass-mapping-and-runtime-trimming.md) —— 不得跳过 Runtime 检查
- [19A](19a-differential-and-fuzz.md)、[19D](19d-performance-comparison.md) —— `native_gap` 机制与原生探测
- [20. 内置函数与标准库](20-builtins-and-standard-library.md) —— 20C 的前置
