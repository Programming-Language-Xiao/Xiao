# 10Z-驱动. 性能对照的计时驱动器（O6 条件 5 的前置）

> **为什么单独立项**：Z-2 的裸机轮次原本要做「C 档截图 + 性能取数」。审核 2026-10-10 实测发现
> **取数那半没有工具**——Java 侧与 LLVM 原生侧**都没有计时驱动器**，`baseline.json` 冻结的协议
> **只被声明、没有被实现**。没有它，把操作者送到机器上也是第三次白跑。
>
> **一句话概括**：**做出那个跑协议、出可复核数字的驱动器**；开发与自测在本机即可，
> 只有**取数**需要受控主机。做完它，Z-2 才具备开轮次的条件。
>
> 状态：**已实施（2026-10-11）**。驱动器已落地并接入 CI（`tools/gates/run.sh` 跑它的单元测试与
> `--self-test`）；**取数尚未开始**，见 [10Z-取数交接](10z-performance-measurement-handoff.md)。
>
> 历史：**可开工（2026-10-10）**。星崽已裁定四项（落点＝独立二进制、VM 侧自测、
> `container-dense` 据实改判、取数后立即冻结阈值），见末尾「已定决策」。

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
| **执行点** | `tests/benchmarks` 是**独立工作区**；全仓库对它的调用**只有 `cargo check`**（`tools/gates/run.sh:16`、`package.json` 的 `check:lock`），**没有任何 `cargo test`**。故驱动器的单元测试、`--self-test`、以及 09R3 设施 `src/main.rs` 的测试**都没有自动执行点** | **缺口**，见 D7 |

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
| **阈值冻结** | 取数完成后**立即**冻结并出判定 | **做**（D5 第 4 条） |
| 调优 | 为达标而改实现 | **不做**——本批只取数与冻结阈值 |

## 二、必须先冻结的 7 条

### 2.1 **D1：两阶段——先语义校验，再计时**

**冻结**：

1. **校验阶段（计时的前置，不可跳过）**：对 `manifest.json` 的每个基准，三侧各跑一次，
   比对输出/错误与 `expected_value` / `expected_error_code`；**不一致就停止**，
   该基准记为 `data-insufficient` 并写明理由——**语义不可比时计时没有意义**；
2. 已知会落在这一步的两个：`scalar-overflow-and-bool-parity`（Xiao 报 `X06-RUNTIME-009`、
   Java 回绕，语义不可比）、`container-dense`（`baseline.json` 的 `data-insufficient` 理由是
   **「Windows native probe rejected numeric_range」**）；
3. **`container-dense` 必须在 Linux 上重新跑一次再判**——那条理由是**Windows 侧的观察**，
   Linux 不能直接继承；跑出来仍是拒绝才维持 `data-insufficient`，
   **若通过则据实改判（星崽 2026-10-10 定）**——**同时改 `baseline.json` 里那条
   「gap belongs to stage 15」的理由**，并写明 **Linux 与 Windows 的差异**（平台差异是事实，
   不是后退；只改平台槽位而留着 Windows 的结论不改，会让清单自相矛盾）；
   （这正是「每一侧都要实跑」的教训，[10Z-Y3](10z-y3-stage-report.md) §一 末尾）
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
6. 三侧用**同一台主机、同一轮、同一输入**；
7. **VM 侧按同口径自测（星崽 2026-10-10 定）**——09R3 的设施测的是**三种字节码机载体**，
   与本任务的三路对照**口径不同**，**不得复用它的数字充当 VM 臂**。VM 臂必须由本驱动器
   按 D2 的同一口径（整进程挂钟、预热 3 / 测量 11、不剔离群）自己测一遍——
   **三路必须同口径，否则「对照」是假的**。
   若实施中发现某侧无法同口径（例如某侧没有独立的进程边界），**写清差异并停下汇报**，
   不要用「差不多」的口径凑合。

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
2. **落点已定（星崽 2026-10-10）：`tests/benchmarks/src/bin/` 下的独立二进制**
   ——同包内即可复用 `manifest.json` 的解析，又不碰 `main.rs` 的行为；
   若需要 `main.rs` 的类型而它们不是 `pub`，**把读取逻辑抽成模块**，**不改 `main.rs` 的行为**；
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
4. **阈值在取数完成后立即冻结（星崽 2026-10-10 定）**：
   - 取数完成前，`threshold.status = "unset"` ⇒ **报告不得写「达到 Java」或任何达标结论**，
     只能给「比值 + 置信区间」与「数据不足」；
   - **取数一完成就冻结阈值**（`status`、`allowed_error`、`regression_limit` 都要填），
     **不要把它拖成下一批的事**——阈值不定，条件 5 就永远只能判「不满足」；
   - 冻结后按阈值出**判定**（通过 / 回归 / 数据不足），并把**冻结依据**（比值、置信区间、
     误差来源）写进报告——**冻结本身要有理由，不是拍一个数**；
   - 冻结发生在**取数之后**，所以冻结时手里有一轮真实数据；`baseline.json` 的
     `baseline_id` 与 sha256 要随之更新（那是基线变更，要在提交说明里写清）；
5. **原始数字要留在产物里**（每轮的样本），不只留统计量——便于第三方复核。

6. **报告必须有分层汇总，顶层状态不得把「一条都没测成」与「全部测成」等同**
   （审核 2026-10-10 实测发现，`ad87d64`/`9ea4cc7` 的驱动器具现这个缺陷）：

   **缺陷形态**：顶层 `status` 的判据是
   `cases.iter().all(|c| c.performance_status == "measured" || c.performance_status == "data-insufficient")`
   → `"development-evidence"`，否则 `"failed"`；而报告里**没有任何汇总计数**。
   于是「5/5 全测成」与「0/5 一条没测成」得到**同一个顶层状态**。

   **它与 `9ea4cc7` 的第一条加固叠加**：那条把「原生输出不是单个整数」从**硬错误**改成逐条
   `Failed` → `data-insufficient`（这个改动本身是对的：一条 workload 坏了不该废掉整轮，
   且该 case 确实记 reason、`performance: None` 不产数字）；但配合上面的判据，
   **一次系统性跑坏会以 `"development-evidence"` 收场而不是 `"failed"`**。

   **冻结**：

   - 报告必须含分层汇总：至少 `measured_cases` / `data_insufficient_cases` / `total_cases`；
   - 顶层 `status` 的判据要改：**`measured_cases == 0` 时不得为 `"development-evidence"`**
     （应为 `"failed"` 或等价的明确状态）——**「一条都没测成」不是证据，是失败**；
   - 理由与本批一路的教训同源：**绿只说明没被覆盖的那部分通过了**。第三方拿到报告必须能
     **一眼**判断这轮到底取没取到数，而不是去数 `cases` 数组。

### 2.6 **D6：开发在本机，取数在受控主机**

**冻结**：

1. **驱动器的开发与自测不需要受控主机**——本机即可跑通全部逻辑（用任意数字验证流程），
   这一条是为了**不把等待机器的成本压进开发**；
2. **但取数必须发生在受控主机上**，且**环境清单在跑之前落盘**
   （CPU、内存、OS/内核、rustc、clang/LLVM、bun、JDK 精确版本串、JVM 参数、构建指纹）；
3. **本机与 CI 的数字一律只作功能证据**，不进结论；
4. 驱动器自身要有**受控主机的自检**：缺 `java`、`javac`、`XIAO_CLANG` 等依赖时**明确失败**，
   不静默跳过（沿用 [10D](10d-environment-gated-test-spec.md) §3.2 的口径）。

### 2.7 **D7：证据必须有执行点——有测试不等于有人在跑它**

**事实（审核 2026-10-10 实测）**：`tests/benchmarks` 是**独立工作区**（有自己的 `Cargo.toml`
与 `Cargo.lock`），而全仓库对它的调用**只有 `cargo check`**
（`tools/gates/run.sh:16` 与 `package.json` 的 `check:lock`），**没有任何 `cargo test`**。
于是下面三样**都没有自动执行点**：

| 有证据能力 | 自动执行点 |
| --- | --- |
| 驱动器的 `#[cfg(test)] mod tests` | **无** |
| 驱动器的 `--self-test`（输出 `bootstrap_byte_identical` 与 `semantic_guard_rejects_wrong_value`） | **无** |
| 09R3 设施 `tests/benchmarks/src/main.rs` 的测试 | **无**（**先前欠账**，非本批引入） |

注：`maintenance-regression.yml` 的 `workspace-default-tests` 跑的是
`cargo test --manifest-path **core/rust**/Cargo.toml --workspace`——**不含** `tests/benchmarks`。

**冻结**：

1. **必须接上执行点**：把 `cargo test --manifest-path tests/benchmarks/Cargo.toml` 接进
   [tools/gates/run.sh](../../tools/gates/run.sh)——它已被 `workspace-gates.yml`（push）
   与 `reproduce.sh`（周定时四平台）**都调用**，是现成的执行点；
2. **`--self-test` 不能留在无人跑的状态**：它输出的两项正是 D1 与 D3 的硬证据。
   把它**并进同一执行点**，或写成单元测试由 `cargo test` 带上——二者选一；
3. **接上之后要证明它真的在跑**：临时破坏一个期望值/断言，确认门禁**变红**，再恢复并确认转绿。
   （这与 [10Z-唯一性](10z-index-uniqueness-followup.md) §2.5、[10Z-CI覆盖](10z-ci-push-coverage-followup.md) §2.4 同一条要求。）
4. **成本可议，但不可以两个都不接**：若整套测试显著拉长 push 门禁，**最低限度接 `--self-test`**
   （最轻，且正好覆盖两项硬证据）；接哪一层由实施方按实测成本定，**在提交说明里写明选了哪层与理由**；
5. **09R3 设施的测试一并受益，但不得因此改动它的行为**（D4 第 1 条仍有效）——
   只是让它从「有人手跑」变成「有执行点」。

**理由**：这条与本批一路的教训同源——**有证据能力不等于有执行点**。
审核在驱动器上逐条核过 D1–D6 都落实了，但**那些证据此前一次都没被自动跑过**；
`15e_ci_gated`（条件 4 的可复现构建比较）就是同一个处境的先例。

## 三、落点

```text
tests/benchmarks/src/bin/…（新）或新 crate         D4：独立驱动器
tests/benchmarks/src/main.rs                       D4：**不改行为**（仅在抽取公共逻辑时动）
tests/benchmarks/baseline.json                     D5：回填槽位
tests/benchmarks/reports/19d-performance.json      D5：platforms.linux 与 statistics
tests/benchmarks/java/Benchmark.java               仅在 Java 侧需要按协议多跑时（否则不动）
docs/DevDocs/19d-performance-comparison.md         D5：口径与结论回填
docs/DevDocs/10z-linux-bare-metal-handoff.md       §〇 与 §七：**已回填**——驱动器就绪，取数由 10Z-取数交接承接
docs/DevDocs/10z-performance-measurement-handoff.md  新：受控主机的取数交接（本批之后交给操作者）
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
6. **冻结阈值**（取数完立即，见 D5 第 4 条）：填 `threshold` 三项 + 写出冻结依据，
   并同步 `baseline_id`/sha256；随后按阈值出判定；
7. **接执行点（D7）**：把 `tests/benchmarks` 的测试与 `--self-test` 接进 `tools/gates/run.sh`，
   并做一次「让它变红」的验证（破坏一个断言 → 门禁红 → 恢复转绿）；
8. **文档**：19D 的口径与结论；把取数交给 [10Z-取数交接](10z-performance-measurement-handoff.md)（原「解除 10Z-Linux §〇 的本轮不做取数」已随该交接的落地完成）；
9. **全量门禁**：`cargo test --workspace`、clippy、fmt、`bun test`、`bun run check`、`bunx tsc --noEmit`。
9. **全量门禁**：`cargo test --workspace`、clippy、fmt、`bun test`、`bun run check`、`bunx tsc --noEmit`。

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
12. **让「一条都没测成」看起来像「全部测成」**——把硬错误降级成逐条 `data-insufficient`
    而不给汇总，顶层状态就分不出这两者（D5 第 6 条，`ad87d64`/`9ea4cc7` 已具现）；
13. **把 `--self-test` 与单元测试当摆设**：两者都能产出硬证据，但**都没接进任何门禁**
    （与 `15e_ci_gated` 同处境）——**有证据能力不等于有执行点**（D7）；
14. **提交没写正文**。

## 七、验收

1. 驱动器能对 `manifest.json` 的每个基准做**三侧语义校验**，且**校验会失败**（有反例证据）；
2. 计时按协议（预热 3 / 测量 11）执行，**构建与编译不计入**，**每次测量后校验输出**；
3. **bootstrap 确定性**：同输入两次运行输出逐字节相同（贴出两次输出）；
4. **不剔除离群样本**，主机负载与后台进程有记录；原始样本随产物落盘；
   **报告含分层汇总**，且 `measured_cases == 0` 时顶层状态**不是** `"development-evidence"`
   （D5 第 6 条）——有反例证据：喂一组全 `data-insufficient` 的输入，确认状态不是「有证据」；
5. `baseline.json` 的槽位与 `reports/19d-performance.json` 的 `platforms.linux` 已按实测回填；
   `windows-native` 标缺、`macos` 不可验证，**没有混口径**；
6. **取数前不写达标结论**（阈值 `unset`）；**取数完成后阈值已冻结**，且冻结有依据
   （比值 / 置信区间 / 误差来源），并按阈值给出了「通过 / 回归 / 数据不足」的判定；
   `baseline_id` 与 sha256 随冻结同步更新；
7. **09R3 的冻结设施与报告未被改动**（`git diff` 可证）；
   **且它的测试现在有执行点**（D7：`tests/benchmarks` 已接进 `tools/gates/run.sh`）；
   **执行点有一次「让它变红」的验证**（D7 第 3 条）；
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

## 已定决策（星崽 2026-10-10）

1. **驱动器落点**：`tests/benchmarks/src/bin/` 下的**独立二进制**（D4 第 2 条）。
2. **VM 侧**：**按同口径自测**，不复用 09R3 三种载体的数字（D2 第 7 条）。
3. **`container-dense` 若在 Linux 通过**：**据实改判**，并同步改 `baseline.json` 里
   「gap belongs to stage 15」的理由、写明 Linux 与 Windows 的差异（D1 第 3 条）。
4. **阈值**：**取数完成后立即冻结**，并写出冻结依据；冻结后按阈值出判定（D5 第 4 条）。

## 待定决策

（无。四项已由星崽裁定，见上。）

## 实施记录（2026-10-11）

- 驱动器落地在 `tests/benchmarks/src/bin/performance_driver.rs`（`ad87d64`），后续 `9ea4cc7` 加固四项、
  `c45f2d0` 补报告汇总与门禁接线。**`tests/benchmarks/src/main.rs`（09R3 设施）一行未动**（D4 第 1 条）。
- **D1–D6 经审核逐条核对属实**（不读声明，直接查代码）：语义校验先行且 `--self-test` 输出
  `semantic_guard_rejects_wrong_value=true`；计时区间由 `Instant::now()` 紧接 `.output()` 界定，
  构建不在内；warmup 与 measurement **两个循环里都校验输出**；无剔除离群样本的逻辑；
  `BOOTSTRAP_RESAMPLES=10_000` / `BOOTSTRAP_SEED=19_015` / `CONFIDENCE_LEVEL=0.95`，
  用 `xiao_types::SeededRandom`，并**校验 `baseline.json` 的协议就是冻结值**；
  `XIAO_CLANG`/`XIAO_RUNTIME_LIBRARY` 缺失时明确失败；报告含**原始样本 `samples_ns`** 与 `version_text_sha256`。
- **D5 第 6 条已完成**：`ReportSummary { measured_cases, data_insufficient_cases, total_cases }`；
  顶层 `status` 仅在 `measured_cases > 0` 且无未知状态且计数自洽时为 `development-evidence`，否则 `failed`。
  审核**做了独立故障注入**——把 `measured_cases > 0` 改回 `true`（即重新引入当初那个缺陷），
  单元测试 `all_data_insufficient_cases_are_a_failed_report` **FAILED**；还原后 6 个测试全绿。
- **D7 已完成**：`tools/gates/run.sh` 加了 `cargo test --manifest-path tests/benchmarks/Cargo.toml`
  与 `--self-test` 两条；审核实跑完整 `run.sh` **退出码 0、171 秒**。推送后 CI 日志证实
  **`performance_driver.rs` 与 `src/main.rs` 的单元测试都跑起来了**、`--self-test` 也执行了
  （两项 `true`）——**09R3 设施那笔「测试无执行点」的先前欠账一并还清**。
- **取数未开始**：需受控主机（Temurin 21 + 固定硬件），交给 [10Z-取数交接](10z-performance-measurement-handoff.md)。
  驱动器本机跑不了全场（本机 `java` 是 1.8，按设计会被版本校验拒绝），**「自检通」不等于「取数能跑通」**。

## 相关页面

- [10Z-Linux. Z-2 裸机轮次交接](10z-linux-bare-metal-handoff.md) §〇 —— 本立项的直接起因（取数无工具）
- [19D. 性能对照](19d-performance-comparison.md) §2.2、§2.4 —— Java 基线裁定与取数口径
- [10Z. 19 收口](10z-19-closeout-and-b-series-implementation.md) §2.4 —— 条件 5 的平台登记口径
- [10Y. 返工与 B 系列分类收口](10y-b-series-triage-rework.md) §2.7 —— 受控取数的七条冻结
- [10D. 环境依赖测试专项规范](10d-environment-gated-test-spec.md) §3.2、§4 —— 缺依赖要明确失败；受控环境准备
