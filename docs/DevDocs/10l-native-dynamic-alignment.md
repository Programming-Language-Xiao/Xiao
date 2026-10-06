# 10L. 原生动态入口与 VM 对齐（N0-E）

> **10 阶段的续批**。README 把 10 写成「N0-A/B/C/D 全部交付」，但 19A 到 19D 的差分和探测实测出：
> 原生动态入口仍然拒绝一批 VM 能跑的程序，而且 19 的出口条件 2 和 5 都卡在这里。
>
> **一句话概括本批**：先修 19D 审核发现的小问题，并补一道能提前发现 20AB 那类回归的门；
> 然后**先枚举、后动手**，把原生动态入口拒绝的语句和运行时检查逐类对齐 VM。枚举结果先向星崽汇报，再定范围。
>
> 状态：**规划稿（2026-10-06）**。文中「建议」未经星崽确认，确认项集中在末尾「待定决策」。

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
| 动态入口语句 | `dynamic/control.rs:115` 把 `For`、`Function`、`Import` 三类语句统一报 `Unsupported`，文案只写「函数或导入语句」，漏了 `for` | 待枚举 |
| 动态入口运行时检查 | `dynamic.rs:93`：只要 `runtime_checks` 非空就整体拒绝，不区分种类。19D 把 `container-dense` 被拒写成 `numeric_range`，那只是第一个命中的种类，不是唯一种类 | 待枚举 |
| 整数溢出 | 原生以 `llvm.trap` 终止（19A 实测 Windows 退出码 `0xC000001D`，无 xiao-error 摘要）；VM 报 `X06-RUNTIME-009`。10A 把它登记为 N0-C 债项；我按关键词查了 10E 到 10I，没找到还债记录，但没有逐篇通读 | 疑似未还 |
| 回归门 | 平台复现工作流每周一定时跑 `--ignored`；`maintenance-regression.yml` 的路径过滤含 `xiao-codegen-llvm`，但该文件里没有 ignored 或 native 步骤。20AB 的回归 10-04 引入，要等 10-05 的定时运行才红 | 缺 |

## 二、19 的出口条件还差什么

按 [12](12-tests-and-milestones.md) O6 逐条对账。结论：**19 还不能收口**，本批只能动其中与原生相关的部分。

| O6 条件 | 现状 | 判定 |
| --- | --- | --- |
| 1 兼容矩阵与升级/拒绝策略 | 矩阵已由代码生成；仍有 `Unverified` 格，我没统计个数 | 部分 |
| 2 差分、模糊、损坏恢复、安全测试全部通过 | Windows 本机通过。原生差分 7 例里 `overflow`、`nested-finally-drops` 登记为未覆盖；Linux、macOS 没有运行记录 | **不满足**，本批相关 |
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

范围现在是未知数：拒绝点有两处，各自覆盖多少种类没人数过。19D 的探测就是这样做的，做完先报，再定后续工作量。

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

## 相关页面

- [10A. LLVM 原生构建闭环](10a-n0-native-closure.md) —— 溢出债项的登记处
- [10E. N0-C 错误路径与源码映射](10e-n0c-error-paths-and-mapping.md) —— 平台级异常与可恢复错误的边界
- [10D. 环境依赖测试专项规范](10d-environment-gated-test-spec.md) —— 门控用例的准备与计数
- [15A](15a-native-pass-mapping-and-runtime-trimming.md) —— 不得跳过 Runtime 检查
- [19A](19a-differential-and-fuzz.md)、[19D](19d-performance-comparison.md) —— `native_gap` 机制与原生探测
- [20. 内置函数与标准库](20-builtins-and-standard-library.md) —— 20C 的前置
