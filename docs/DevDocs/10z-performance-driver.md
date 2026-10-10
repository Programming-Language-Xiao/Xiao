# 10Z-驱动. 性能对照的计时驱动器（O6 条件 5 的前置）

> **为什么单独立项**：Z-2 的裸机轮次原本要做「C 档截图 + 性能取数」。审核 2026-10-10 实测发现
> **取数那半没有工具**——Java 侧与 LLVM 原生侧**都没有计时驱动器**，`baseline.json` 冻结的协议
> **只被声明、没有被实现**。没有它，把操作者送到机器上也是第三次白跑。
>
> **一句话概括**：**做出那个跑协议、出可复核数字的驱动器**；开发与自测在本机即可，
> 只有**取数**需要受控主机。做完它，Z-2 才具备开轮次的条件。
>
> 状态：**规划稿（2026-10-10）**。待定决策集中在末尾。

## 一、Agent 交接上下文

### 1.1 接手前提

1. [19D](19d-performance-comparison.md) §2.2 —— **Java 基线的裁定**（发行版族、JVM 参数、互校要求、阈值未冻结）；
2. [19D](19d-performance-comparison.md) §2.4/§2.5 与 [10Y](10y-b-series-triage-rework.md) §2.7 —— **取数口径**（受控主机、跑前落盘环境清单、三路对照、原始数字回传、本机与 CI 数字只作功能证据）；
3. [10Z](10z-19-closeout-and-b-series-implementation.md) §2.4 —— 条件 5 的平台登记口径（Linux 实测、Windows 标缺、macOS 不可验证）；
4. [10Z-Linux 交接](10z-linux-bare-metal-handoff.md) §〇 —— **本轮取数被判定为「工具不存在」的出处**；
5. `tests/benchmarks/README.md` —— 09R3 的冻结设施，**它测的不是本任务要测的东西**（见 §1.2）。

### 1.2 现状盘点（2026-10-10，审核实测）

| 环节 | 现状 | 判定 |
| --- | --- | --- |
| 冻结协议 | `tests/benchmarks/baseline.json`：预热 **3**、测量 **11**、`percentile-bootstrap`、**10000** 重采样、seed **19015**、置信 0.95、噪声策略 `record-host-load-and-background-processes; do-not-discard-samples` | **只被声明，未被实现** |
| Java 侧 | `java/Benchmark.java` 收 `main(id, argument)`，输出 `success<TAB><value>`；**不计时** | **缺计时** |
| LLVM 原生侧 | `xiao-driver/tests/native_benchmark_probe.rs` 是**功能探针**：只输出 `{id}:built` / `{id}:native-rejected:{error}`，container-dense 路径自述「**非性能取数**」 | **缺计时** |
| VM 侧 | `tests/benchmarks/src/main.rs`（09R3）**有**计时，但它测的是**三种字节码机载体**，README 明说「没有……LLVM 原生对照」 | 有计时，**但对象不同** |
| 驱动器是否存在 | `grep -rln "javac\|java -Xms\|Benchmark.class"` 在 `*.sh`/`*.ps1`/`*.rs`/`*.ts`/`*.yml` **零命中**；对 `.exe` 重复计时的地方**零命中** | **不存在** |
| 可复用件 | `tests/benchmarks/src/main.rs:816` 的 `percentile()`（**无 bootstrap**）；`xiao-types::SeededRandom`（已由 `d19a_fixed_random.rs` 独立金标测试的确定性 PRNG） | 可复用 |

**关键不对称（决定驱动器的结构）**：

- **Java 侧从命令行取参数**：`java <jvm_args> -cp <classes> Benchmark <id> <argument>`，单次执行输出一个值；
- **原生侧的调用烘焙在源码里**：`native_benchmark_probe` 对 `container-dense` **先往源码追加**
  `print(<entry>(<args>))` 再编译，其余基准的顶层调用已在源码内。

两侧「算同一件事」靠 `manifest.json` 的 `expected_value` / `expected_error_code` **互校**——
这正是 [19D](19d-performance-comparison.md) §2.2 的要求。**所以驱动器必须先校验、再计时。**

### 本批边界

| 子任务 | 内容 | 本批 |
| --- | --- | --- |
| 计时驱动器 | Java / LLVM 原生 / VM 三侧按协议取数 + 统计 + 落盘 | **做** |
| 语义校验 | 三侧输出与 `manifest` 期望值互校 | **做**（计时的前置） |
| **在受控主机上取数** | —— | **不做**（做完本驱动器后才开 Z-2 轮次） |
| 09R3 的冻结设施与报告 | —— | **不改** |
| 阈值与达标判定 | `threshold.status = "unset"` | **不做**，也不许写「达到 Java」 |

## 二、必须先冻结的 6 条

### 2.1 **D1：两阶段——先语义校验，再计时**

**冻结**：

1. **校验阶段（计时的前置，不可跳过）**：对 `manifest.json` 的每个基准，三侧各跑一次，
   比对输出/错误与 `expected_value` / `expected_error_code`；**不一致就停止**，
   该基准记为 `data-insufficient` 并写明理由——**语义不可比时计时没有意义**；
2. 已知会落在这一步的两个：`scalar-overflow-and-bool-parity`（Xiao 报 `X06-RUNTIME-009`、
   Java 回绕，语义不可比）、`container-dense`（`baseline.json` 的 `data-insufficient` 理由是
   **「Windows native probe rejected numeric_range」**）；
3. **`container-dense` 必须在 Linux 上重新跑一次再判**——那条理由是**Windows 侧的观察**，
   Linux 不能直接继承；跑出来仍是拒绝才维持 `data-insufficient`，若通过则**据实改判**
   （这正是「每一侧都要实跑」的教训，[10Z-Y3](10z-y3-stage-report.md) §一 末尾）；
4. 校验通过才算「语义可比」，才进入计时阶段。

### 2.2 **D2：计时口径要写死（协议没写清的部分由本批冻结）**

协议只规定了次数与统计，**没规定测什么**。本批必须把它定死，**两侧必须一致**：

**冻结**：

1. **测量对象＝整进程挂钟时间**（含运行时初始化与退出）——Java 侧是 `java … Benchmark …` 的整进程，
   原生侧是 `.exe` 的整进程；**构建/编译时间绝不计入**；
2. 用**单调时钟**（`std::time::Instant`），不用墙钟；
3. **每次测量后校验输出**——防止「跑错了」或「被优化掉」混进数字；
4. 预热与测量次数**按 `baseline.json` 的 3 / 11**，**不得自行调整**；想改先改 `baseline.json`（那算基线变更）；
5. **噪声策略照抄协议**：`record-host-load-and-background-processes; do-not-discard-samples`——
   **不许剔除离群样本**；主机负载与后台进程要**记录下来**，不是删掉样本；
6. 三侧用**同一台主机、同一轮、同一输入**；VM 侧复用 09R3 已有设施的测量结果**只在口径一致时**才算，
   否则也要自测一遍（口径不一致就写清差异，**不要混算**）。

### 2.3 **D3：统计必须确定性可复现**

**冻结**：

1. 按协议做 **percentile-bootstrap**：**10000** 次重采样、**95%** 置信、**seed 19015**；
2. **用 `xiao-types::SeededRandom`** 做重采样（它已有独立金标测试），**不要引入新依赖**，
   也不要自己写一个未经验证的 PRNG——**determinism 是本条的要点**；
3. **确定性自证**：同一组样本跑两次，bootstrap 输出**逐字节相同**。这是本批的硬证据之一（见 §五）；
4. `tests/benchmarks/src/main.rs` 的 `percentile()` 可复用，但**bootstrap 需要新写**（那里没有）。

### 2.4 **D4：驱动器独立，不改 09R3 的冻结设施**

**冻结**：

1. `tests/benchmarks/src/main.rs` 是 09R3 的**冻结历史验收设施**，它的报告（`reports/windows-native-*.json`）
   与 `09r3-freeze.json` **不得被本批改写**；「任何指令集、ABI 或编码改动都会使本轮数字作废」
   的约束仍适用于它；
2. 新驱动器**另置**（建议 `tests/benchmarks/src/bin/` 下的独立二进制，或新 crate）；
   若它需要 `manifest.json` 的解析类型，**把读取逻辑抽成模块**而不是改 `main.rs` 的行为；
3. 驱动器**不自带**任何 benchmark 实现，只**驱动**：`manifest.json` 给源码与期望值、
   `baseline.json` 给协议与 JVM 参数——**清单是权威来源，不要在驱动器里再抄一份**；
4. 驱动器要能**只跑一个基准**（`--id <benchmark>`）以便定位，也要能跑全套。

### 2.5 **D5：产物落盘——填槽位，不另立格式**

**冻结**：

1. **回填 `tests/benchmarks/baseline.json`**：`status`、`runtime.resolved_version`、
   `runtime.version_text_sha256`（`java -version` 输出的 sha256）、各 workload 的
   `performance_status`（可比的填实测、不可比的填 `data-insufficient` + `reason`）、
   **`platforms.linux`** 由 `not-measured` 改为实测；
2. **更新 `reports/19d-performance.json`** 的 `platforms.linux` 与 `statistics.sample_status`，
   格式沿用该文件既有字段；
3. **`platforms.windows-native` 按已定口径标缺**、`macos` 标**不可验证**——
   **不要**用本机或 CI 数字顶替（[10Z](10z-19-closeout-and-b-series-implementation.md) §2.4）；
4. **`threshold.status = "unset"` 未变 ⇒ 报告不得写「达到 Java」或任何达标结论**，
   只能给「比值 + 置信区间」与「数据不足」；
5. **原始数字要留在产物里**（每轮的样本），不只留统计量——便于第三方复核。

### 2.6 **D6：开发在本机，取数在受控主机**

**冻结**：

1. **驱动器的开发与自测不需要受控主机**——本机即可跑通全部逻辑（用任意数字验证流程），
   这一条是为了**不把等待机器的成本压进开发**；
2. **但取数必须发生在受控主机上**，且**环境清单在跑之前落盘**
   （CPU、内存、OS/内核、rustc、clang/LLVM、bun、JDK 精确版本串、JVM 参数、构建指纹）；
3. **本机与 CI 的数字一律只作功能证据**，不进结论；
4. 驱动器自身要有**受控主机的自检**：缺 `java`、`javac`、`XIAO_CLANG` 等依赖时**明确失败**，
   不静默跳过（沿用 [10D](10d-environment-gated-test-spec.md) §3.2 的口径）。

## 三、落点

```text
tests/benchmarks/src/bin/…（新）或新 crate         D4：独立驱动器
tests/benchmarks/src/main.rs                       D4：**不改行为**（仅在抽取公共逻辑时动）
tests/benchmarks/baseline.json                     D5：回填槽位
tests/benchmarks/reports/19d-performance.json      D5：platforms.linux 与 statistics
tests/benchmarks/java/Benchmark.java               仅在 Java 侧需要按协议多跑时（否则不动）
docs/DevDocs/19d-performance-comparison.md         D5：口径与结论回填
docs/DevDocs/10z-linux-bare-metal-handoff.md       §〇 与 §七：驱动器就绪后解除「本轮不做取数」
docs/DevDocs/README.md                             主表登记
```

## 四、硬约束

1. **先校验、后计时**（D1）；三侧输出必须与 `manifest` 期望值互校；
2. **`container-dense` 在 Linux 上重新跑再判**，不继承 Windows 的观察（D1 第 3 条）；
3. **预热 3 / 测量 11 / seed 19015 / 10000 重采样**照协议，**不得自调**（D2、D3）；
4. **不剔除离群样本**，负载与后台进程要记录（D2 第 5 条）；
5. **bootstrap 用 `SeededRandom`**，且**同输入两次输出逐字节相同**（D3）；
6. **不改 09R3 的冻结设施与报告**（D4）；
7. **清单是权威来源**，驱动器不许再抄一份基准定义（D4 第 3 条）；
8. **阈值未冻结 ⇒ 不写达标**（D5 第 4 条）；
9. **本机/CI 数字只作功能证据**（D6）；
10. 不新增依赖，`Cargo.lock` 保持同步；每个提交单独跑 clippy 与 fmt；
11. 提交说明带正文；涉及 CI 时附运行号。

## 五、分步提交

1. **驱动器骨架 + 语义校验**：读 `manifest.json`，三侧各跑一次并互校；
   **校验要能失败**——故意改坏一个期望值，确认驱动器报错（「能分辨对错」，见 §六 第 2 条）；
2. **计时与落盘原始样本**：按 D2 口径跑预热与测量，样本落盘；
3. **bootstrap 统计**：用 `SeededRandom` 做百分点自助法；**同输入两次输出逐字节相同的自证**；
4. **回填 `baseline.json` 与 `reports/19d-performance.json`**：只填槽位、不另立格式；
5. **只在受控主机上取数**（本步之前的一切都在本机做）：环境清单先落盘，再跑，再回传原始数字；
6. **文档**：19D 的口径与结论；解除 [10Z-Linux 交接](10z-linux-bare-metal-handoff.md) §〇 的「本轮不做取数」；
7. **全量门禁**：`cargo test --workspace`、clippy、fmt、`bun test`、`bun run check`、`bunx tsc --noEmit`。

## 六、最可能翻车的地方

1. **把构建时间混进测量**——Java 的 `javac` 与 `xiao build` 都不在计时区间内（D2 第 1 条）；
2. **校验做成走过场**：不校验就计时，于是「三侧跑的根本不是同一件事」也看不出来（D1）；
   **校验必须能失败**——改坏期望值要报错，否则它没在守门；
3. **跑错基准也能出数字**（没有输出校验）——D2 第 3 条要求每次测量后校验输出；
4. **`container-dense` 直接继承 Windows 的 `data-insufficient`**，没在 Linux 上跑（D1 第 3 条）；
5. **剔除离群样本**「让数字好看」——协议明令不许（D2 第 5 条）；
6. **自己写一个 PRNG 或统计**，导致不可复现（D3）；
7. **改了 09R3 的冻结设施或它的报告**（D4）；
8. **在驱动器里再抄一份基准定义**，于是清单与实现各自漂移（D4 第 3 条）；
9. **阈值没冻结却写「达到 Java」**（D5 第 4 条）；
10. **拿本机/CI 数字充当受控取数**（D6）；
11. **只留统计量不留原始样本**，第三方无法复核（D5 第 5 条）；
12. **提交没写正文**。

## 七、验收

1. 驱动器能对 `manifest.json` 的每个基准做**三侧语义校验**，且**校验会失败**（有反例证据）；
2. 计时按协议（预热 3 / 测量 11）执行，**构建与编译不计入**，**每次测量后校验输出**；
3. **bootstrap 确定性**：同输入两次运行输出逐字节相同（贴出两次输出）；
4. **不剔除离群样本**，主机负载与后台进程有记录；原始样本随产物落盘；
5. `baseline.json` 的槽位与 `reports/19d-performance.json` 的 `platforms.linux` 已按实测回填；
   `windows-native` 标缺、`macos` 不可验证，**没有混口径**；
6. **没有写达标结论**（阈值仍 `unset`）；
7. **09R3 的冻结设施与报告未被改动**（`git diff` 可证）；
8. 取数在受控主机上完成，环境清单在跑之前落盘；
9. `cargo test --workspace`、clippy、fmt、`bun test`、`bun run check`、`bunx tsc --noEmit` 全绿；
10. 没有通过放宽断言、跳过校验或缩小范围换来的绿。

## 八、不负责与不要重复做的事

- **不做**阈值冻结与达标判定（要星崽定，且属发布口径）；
- **不做** Windows 侧取数（按已定口径标缺）；
- **不做** macOS（无宿主、不可验证）；
- **不改** 09R3 的冻结设施与其报告；
- **不做** Java 参考实现的重写（`Benchmark.java` 已冻结；只有在协议要求它按轮次多跑时才动它）；
- **不做** B 系列、19 收口其它项（那是 10Z/10Z-Z1 的事）。

## 待定决策

1. **驱动器的落点**：`tests/benchmarks/src/bin/` 下的独立二进制，还是新 crate？
   审核**倾向独立二进制**（同包内即可复用 `manifest.json` 的解析，又不碰 `main.rs` 的行为）；
   若 `main.rs` 的类型不可复用，再考虑抽模块或新 crate。
2. **VM 侧是否自测**：09R3 的设施测的是三种**载体**，与本任务的三路对照**口径不同**。
   是复用它的数字（并在报告里写明口径差异），还是为 VM 侧也按 D2 口径自测一遍？
   审核**倾向自测一遍**——三路必须同口径，否则「对照」是假的。
3. **`container-dense` 若在 Linux 通过**：是否同时更新 `baseline.json` 里那条
   「gap belongs to stage 15」的结论？（审核倾向：**据实更新并写明 Linux 与 Windows 的差异**，
   那是事实，不是后退。）
4. **阈值**：`threshold.status = "unset"` 要维持到什么时候？取数完成后是否立刻冻结阈值，
   还是先出一次比值再定？这决定条件 5 何时能判「满足」。

## 相关页面

- [10Z-Linux. Z-2 裸机轮次交接](10z-linux-bare-metal-handoff.md) §〇 —— 本立项的直接起因（取数无工具）
- [19D. 性能对照](19d-performance-comparison.md) §2.2、§2.4 —— Java 基线裁定与取数口径
- [10Z. 19 收口](10z-19-closeout-and-b-series-implementation.md) §2.4 —— 条件 5 的平台登记口径
- [10Y. 返工与 B 系列分类收口](10y-b-series-triage-rework.md) §2.7 —— 受控取数的七条冻结
- [10D. 环境依赖测试专项规范](10d-environment-gated-test-spec.md) §3.2、§4 —— 缺依赖要明确失败；受控环境准备
