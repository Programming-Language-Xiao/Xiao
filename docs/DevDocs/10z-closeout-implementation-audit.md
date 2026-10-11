# 10Z-收口审核. 收口实施审核：三处返工与两处登记

> **背景**：`b55f59f`（T4 修复）、`33b1ed0`（阈值冻结与回填）、`b3eddb1`（状态对账）三个提交兑现了
> [10Z-收口](10z-19-closeout-declaration.md) 的 T1–T4。**数字是对得上的**——审核逐项重算过四个
> 原生/Java 比值、四个保守区间和两个 sha256，全部逐位一致；`bun run check` 退出码 0。
>
> **一句话概括**：**数值全对，但三件事没落地**——「这份冻结证据说自己是对着哪个基线测的」、
> 「把达标线从 1.0 放宽到 1.10 有没有依据」、「给 T4 加的那条回归测试在 push 时到底有没有人跑」。
> 其中第三处**是规划侧规范自己留下的洞**。
>
> 状态：**部分核销（2026-10-11）**。`24c0a5c` 已落地 F-1/F-2/F-4/F-5，
> 并把 benchmark 测试与自检接入维护回归；CI 运行 `38117244885` 已证明该执行点实际运行。
> 只改 `tests/benchmarks/**` 的 push 正例、负例与恢复运行号仍未补，不能把这笔触发验证写成完成。

## 一、Agent 交接上下文

### 1.1 接手前提

1. [10Z-收口](10z-19-closeout-declaration.md) —— **被审对象**；T1–T4 的定义与其实施记录都在那里；
2. [10Z-驱动](10z-performance-driver.md) D5 第 4 条与 §2.7 D7 —— 「取数完立即冻结阈值」与
   「证据必须有执行点」两条原始条款；F-2、F-3 直接改它们；
3. [10Z-取数](10z-performance-measurement-handoff.md) §八 —— 阈值冻结要含**冻结依据**（比值、置信区间、误差来源）；
4. `tests/benchmarks/baseline.json`、`tests/benchmarks/reports/19d-performance.json` —— 被冻结的两份事实；
5. `docs/DevDocs/assets/10z-performance-20261011/` —— 本轮唯一的原始证据（报告 JSON、日志、人工清单）；
6. [00. 决策基线](00-decisions.md) 第 346–347 行、[09R](09r-bytecode-machine-research.md) §R1-AC
   —— **F-2 要对话的那条「10% 政策」的原文**；
7. [19. 优化、兼容性与发布验收](19-optimization-release.md) 第 34、43、69 行 —— 「至少达到 Java 以上」的原话。

### 1.2 审核实测（2026-10-11，审核本机）

| 核什么 | 怎么核的 | 结论 |
| --- | --- | --- |
| benchmarks 单元测试 | `cargo test --manifest-path tests/benchmarks/Cargo.toml` | **7 passed**，含新增的 `cpu_model_parser_accepts_linux_field_separators` |
| **T4 的根因与效果** | **独立复刻修前/修后两版逻辑**，喂真实 `/proc/cpuinfo` 文本 | 修前 `None`（⇒ `unavailable`）、修后 `12th Gen Intel(R) Core(TM) i5-12450H`；ARM 的 `Hardware` 行同理；值里含冒号不误切 |
| `baseline.json` 摘要 | `sha256sum` | `5df18ed7…`，与收口声明所写**一致** |
| `version_text_sha256` | 对报告里的 `java_version_text` 重算 | `090529cc…`，**一致**（口径 = 含尾换行） |
| 四个比值与四个保守区间 | 从原始 `samples_ns` 与 `bootstrap` 逐项重算 | **逐位一致**；并确认区间构造是 `[native_low/java_high, native_high/java_low]`，所以「保守」名副其实 |
| 仓库门禁 | `bun run check` | 退出码 0 |
| benchmarks 侧格式 | `cargo fmt`/`clippy --manifest-path tests/benchmarks/Cargo.toml` | 干净（但**没有门禁在跑它**，见 F-3） |

**核过、判为没问题的四处**（本批不动）：

- T1/T2/T3 的**字面要求**都满足：阈值三项 + 依据 + `baseline_id`/sha256 同步；按实测回填；
  `container-dense` 改判并保留 Windows 观察为平台差异；
- T4 **如实登记**为「代码已修、受控复跑未完成」，**没有把单元测试冒充真机证据**——这正是规范要的态度；
- 归档报告的 `cpu_model=unavailable` 与人工清单的 `i5-12450H` **不一致**，这一点被显式写进
  [10Z-收口](10z-19-closeout-declaration.md) 的 T4 段，没有掩盖；
- `tests/benchmarks/reports/19d-performance.json:4` 顶层 `status=data-insufficient` 与
  「平台覆盖不足、不作跨平台通过或回归结论」的口径**一致**，不是漏改。

### 1.3 本批边界

| 事项 | 本批 |
| --- | --- |
| F-1 19D 状态块自相矛盾 | **做**——纯文档 |
| F-2 阈值依据与判据语义 | **做**，但判据取哪一条**要星崽先定**（§五） |
| F-3 D7 执行点的触发路径 | **做**——这是**返工**，不是新工具侧批次（见下） |
| F-4 归档证据的基线身份 | **做**——**只写注记，不改归档 JSON** |
| F-5 两处口径登记 | **做**，半句到一句 |
| T4 的受控复跑 | **不做**——归下一轮取数，本批只登记其缺口 |
| B1 释放性质的成对实验 | **不做**——归 [10Z-Z1](10z-z1-scope-and-gap-verdict.md) 的后续判定 |
| 归档 JSON 重写 | **不做**——`tests/benchmarks/reports/README.md` 写明「旧报告不得重写」 |

> **关于「不开新的工具侧批次」**：[10Z-收尾](10z-closeout-execution.md) §2.3 的冻结仍然有效。
> F-3 改的是**既有工作流的路径过滤一行**，属于审核发现的返工，不构成新批次。

## 二、必须先冻结的 5 条

### 2.1 **F-1：19D 顶部状态块自己和自己矛盾**

**事实**（`19d-performance-comparison.md` 文件开头）：

- 第 10 行：`状态：**收口中（2026-10-11）**。F1–F8 已有结论；**Linux 受控性能取数与阈值已落地**，跨平台边界仍未满足 O6 条件 5。`
- 第 11–12 行：`…阈值和 CI 证据策略已经按星崽裁定冻结；**受控环境性能数字与 CI 实跑记录仍缺，19.17 继续只能给出「数据不足」**。`

**同一状态块里前后相反**：上面说性能数字已落地，下面说性能数字仍缺。而且与本提交自己改过的
§2.4、§落点、README 主表（都已写「已完成 Linux 判定流程」「阈值已冻结」）也对不上——
`b3eddb1` 改了第 10 行，**漏了第 11–12 行**。

**冻结**：

1. 第 11–12 行必须改成与本轮实施结果一致的口径，例如「受控**Windows 性能主机**与 **macOS 宿主**仍缺，
   因此跨平台判定仍为数据不足」——**保留「还缺什么」，删掉「性能数字仍缺」**；
2. 不得顺手把 O6 条件 5 改写成「满足」（见 2.2 与收口声明的现行判定）。

### 2.2 **F-2：阈值依据引错政策，且把达标线从 1.0 放宽到 1.10**

**事实一：依据指向的那条政策，说的不是这件事。**

收口声明与报告都把冻结依据写成「现有性能政策的 10% 误差/回归预算」。核到原文有两处：

- [00. 决策基线](00-decisions.md) 第 346–347 行：`性能门槛：最终方案相对同机 Rust 栈式基线稳态吞吐至少提升 10%；冷启动、峰值内存和编码体积不得恶化超过 10%；语义结果必须完全一致。`
- [09R](09r-bytecode-machine-research.md) §R1-AC 的门槛表：`稳态吞吐 | 相对同机 Rust 栈式基线 ≥ +10%`。

这条政策管的是 **09R 的机型选择（候选机型 vs 同机 Rust 栈式基线）**，它：

1. **比较对象不同**——不是「LLVM 原生 vs Java」；
2. **方向相反**——它要求**快 10%**（`≥ +10% 提升`），被借来当成了**允许慢 10%**。

**事实二：报告里实际冻的是「低于 1.10」。**

`tests/benchmarks/reports/19d-performance.json:158` 的 `rule`：
`原生/Java 中位数比值及保守 percentile-bootstrap 比值区间均须低于 1.10；VM 比值仅作诊断`。

把「达到 Java」的判据从 `1.0` 放宽到 `1.10`，等于**允许比 Java 慢 10% 仍记为通过**，
与 [19 规范](19-optimization-release.md) 第 34 行「至少达到 Java 以上」、
[00. 决策基线](00-decisions.md) 第 446 行冲突。若真要沿用那条政策的**方向**做类比，
对应判据应是 `≤ 1/1.10 ≈ 0.9091`（**Xiao 至少快 10%**），也不是 `1.10`。

**事实三：`baseline.json` 不自描述，而歧义有实质后果。**

`tests/benchmarks/baseline.json:66-70` 的 `threshold` 只有三个裸数字：

```json
"threshold": { "status": "frozen", "allowed_error": 0.1, "regression_limit": 0.1 }
```

`rule` 只写在报告 JSON 与收口声明里。**被冻结的那份文件说不清这两个数各是「什么的上限」**，
而 19.13 的 CI 回归门禁要吃的正是它。`regression_limit` 更彻底——**两处都没写它的适用规则**
（`rule` 只说了 1.10 那条），「10% 的什么」没有定义。

后果不是纸面的：`named-local-loop` 的**保守区间上界是 `0.114699`**。若有人把 `allowed_error = 0.10`
读成「比值须低于 0.10」，这一条会被判不通过/回归；按报告写的 `1.10` 则通过。
**同一份冻结数据，两种读法结论不同。**

**但要说清**：**这不改变今天的判定**。四个比值最大 `0.0932`、四个区间上界最大 `0.1147`，
在 `0.9091`、`1.0`、`1.10` 三种判据下**都是通过**。本条要修的是**依据与语义**，
不是数字，也**不允许为了让判据「看起来有据」而改数字**。

**冻结**：

1. **把判据写进被冻结的那份文件**：`baseline.json` 的 `threshold` 补 `rule`（与报告同口径）
   与 `regression_limit` 的判法（相对冻结比值的漂移，还是绝对上限），两处文字保持同一口径；
2. **改正依据的出处**：要么引一条真正管「原生 vs Java」的政策，要么**明说这是我们新定的判据**；
   不得继续把 09R 的机型选择政策写成它的来源；
3. 达标线的取值见 §五 第 1 条，**由星崽定**；
4. 改完之后 `baseline.json` 的 sha256 会变，**[10Z-收口](10z-19-closeout-declaration.md)
   与 [19D](19d-performance-comparison.md) 里引用的摘要必须同步**——上一轮刚因为「同步摘要」立过规矩。

**实施记录（`24c0a5c`）**：星崽已裁定采用审核倾向的 A 方案，达标线回到 `≤ 1.0`；
`baseline.json` 已补齐 `rule` 与 `regression_rule`，报告和收口声明同步为同一口径。
冻结基线摘要为 `6017605e…`，Java 版本摘要仍为 `090529cc…`。
本节引用的 `reports/19d-performance.json:158` 是审核 `b3eddb1` 时的原文；
后续提交已改写该字段，不能把行号处的当前文字当成审核当时看到的内容。

### 2.3 **F-3：D7 的执行点触发不到它要守护的路径（规划侧的洞）**

**事实链**（全部可逐行核对）：

1. `tools/gates/run.sh:16-18` 确实加了 benchmarks 的测试与自检：
   `cargo check` → `cargo test --manifest-path tests/benchmarks/Cargo.toml` → `--self-test`；
2. 在 push 上调用 `run.sh` 的是 `workspace-gates.yml`，而它的 `paths`（第 5–14 行）是
   `tools/**`、`cli/**`、`docs/**`、`package.json`、`bun.lock`、`tsconfig*`、`core/rust/rust-toolchain.toml`
   ——**不含 `tests/benchmarks/**`**；
3. 含 `tests/benchmarks/**` 的是 `maintenance-regression.yml`（第 7–12 行），
   而它的四个作业**全部只针对 `core/rust/Cargo.toml`**（`:43`、`:64`、`:83`、`:88`、`:131`、`:179` 的 `run:` 无一例外）。

**⇒ 只改 `tests/benchmarks/**` 的 push，那条回归测试没有自动执行点。**
周定时能跑到（`tools/platform-reproduction/reproduce.sh:143 → run.sh:17`），**push 时跑不到**。

**`b55f59f` 正是这种提交**——它只改了 `tests/benchmarks/src/bin/performance_driver.rs`，
也就是那条新测试**在被加进来的那次推送里没人跑**。

**这笔算规划侧的**：D7 第 1 条（[10Z-驱动](10z-performance-driver.md) 第 197–199 行）
是我写的，写了「`run.sh` 已被 `workspace-gates.yml` 调用，是现成的执行点」，
**却没核这个执行点的触发路径是否覆盖被守护的路径**。

**更值得记的是 D7 的验收为什么没抓到**：D7 的实施记录（同文件第 332–336 行）写「推送后 CI 日志
证实 `performance_driver.rs` 与 `src/main.rs` 的单元测试都跑起来了」——**那次推送同时改了 `tools/**`**
（就是 `run.sh` 本身），所以它**必然**触发 `workspace-gates.yml`。
**「跑了」的证据恰好来自一个把缺口遮住的场合。**

**冻结**：

1. 修法二选一（按成本选，在提交说明里写明选了哪个与理由）：
   - **甲**：`workspace-gates.yml` 的 `paths`（push 与 pull_request 两处）加 `tests/benchmarks/**`
     （或直接 `tests/**`，与 `core/rust/**` 同级的 glob 口径一致）；
   - **乙**：把 `cargo test --manifest-path tests/benchmarks/Cargo.toml` 与 `--self-test` 两条
     加进 `maintenance-regression.yml`——它的过滤**已经**含该路径，且不与 `docs/**` 绑定；
2. **接上之后必须证明它真的在跑**：做一次**只改 `tests/benchmarks/**`** 的推送，
   按 D7 第 3 条破坏一个断言确认**变红**，恢复后确认转绿，并把运行号回填。
   **这次的验证提交不许顺手改 `tools/**`**——否则又是同一个遮蔽；
3. `paths` 改了两处（`push` 与 `pull_request`）要**同时改**，只改一处等于半个门禁。

**实施记录（`24c0a5c`）**：按乙方案把
`cargo test --manifest-path tests/benchmarks/Cargo.toml` 与 `--self-test` 接入
`.github/workflows/maintenance-regression.yml`；该工作流的 `push`/`pull_request`
过滤已经包含 `tests/benchmarks/**`。维护回归 `38117244885` 的日志含 benchmark
单测和 `bootstrap_byte_identical=true` 自检输出，证明执行点已运行。
只碰 `tests/benchmarks/**` 的正例触发、负例变红和恢复转绿仍待外部 push 运行号，
因此 F-3 仍是「执行点已跑通，触发验证待补」。

### 2.4 **F-4：冻结阈值引用的那份证据，自我标识为「对着待定基线测的」**

**事实**：`docs/DevDocs/assets/10z-performance-20261011/performance-report.json` 里

```text
baseline_id     = "java-temurin-openjdk-21"
baseline_status = "pending-controlled-run"
```

而 `tests/benchmarks/baseline.json:3-4` 现在是 `java-temurin-openjdk-21-linux-20261011` /
`controlled-linux-measured-threshold-frozen`。**同一个仓库里，报告页说新、原始证据说旧**：
`tests/benchmarks/reports/19d-performance.json` 的 `baseline.sha256` 已经更新成新摘要
`5df18ed7…`，而它指向的那份原始报告仍写着旧 id。**冻结阈值所用的那组数字，出处自称是「待定基线」。**

**冻结**：

1. **不许改归档 JSON**（`tests/benchmarks/reports/README.md` 第 9 行「旧报告不得重写」，
   以及 [19D](19d-performance-comparison.md) §2.6「旧报告保持冻结」）；
2. 修法二选一：
   - **甲**：在[结果文档](10z-performance-measurement-results-20261011.md) 加一段注记，写明
     **该报告早于阈值冻结，其 `baseline_id` / `baseline_status` 是冻结前的取值**，
     冻结后的基线身份以 `baseline.json` 为准；
   - **乙**：等 T4 的受控复跑重出报告——驱动器读 `baseline.json`，会自动带上新 id——
     届时把旧报告在这条链上的位置一并说明；
3. 无论选哪条，**[10Z-收口](10z-19-closeout-declaration.md) 的实施记录里要指出这个不一致曾经存在**，
   不要让它只活在本文里。

### 2.5 **F-5：两处口径登记（都只需半句到一句）**

1. **`unverifiable` 口径的扩用没有写授权**：`platforms.macos` 由 `not-measured` 改成 `unverifiable`。
   「不可验证」这个词在 [19D](19d-performance-comparison.md) §2.5 的定义位置是**专为 `19.14` 的
   macOS 真实终端那一格**设的独立口径；扩用到**性能**平台栏在实质上是对的
   （与「macOS 没有可补路径、Windows 只是暂无专用主机」的区分一致），但扩用处**没写一句理由**，
   读起来像 `19.14` 的口径漏了过来。**冻结**：在 19D §2.5 补一句，说明该口径同样适用于性能平台栏，
   并点明它与 Windows「标缺」的**性质差别**。
2. **标签与措辞不齐**：`platforms.windows-native` 由 `pending-controlled-run` 改成 `data-insufficient`。
   标签本身合规（§2.4 允许「受控基线未取 ⇒ 数据不足」），但
   `tests/benchmarks/reports/19d-performance.json:22` 给的理由文字是
   「无专用 Windows 受控性能主机」——**那描述的是「未测」，不是「测了但不足」**。
   **冻结**：把理由文字改成与标签同口径的表述。此项**不影响 O6 条件 5 的判定**。

### 2.6 **明确不属于本批的两处（登记，不动）**

1. **T4 的「受控复跑」仍缺**——实施方**自己如实登记**了（`10z-19-closeout-declaration.md` 的 T4 段），
   并且没有把单元测试冒充受控证据。**这是对的**：归档报告里的 `cpu_model=unavailable`
   是修前那一次的输出，修后侧要等复跑，本批**不补**、也不许用单元测试顶替；
2. **T4 的修复本身正确**——审核独立复刻两版逻辑喂真实 `/proc/cpuinfo` 文本，
   修前 `None`、修后拿到型号，根因（冒号前是制表符）与效果都成立。**不需要返工**。

## 三、实施收据（2026-10-11）

| 审核项 | 当前状态 | 证据与限制 |
| --- | --- | --- |
| F-1 状态块 | 已核销 | `19d-performance-comparison.md` 已删除互相矛盾的「性能数字仍缺」表述；Windows/macOS/CI 的缺口仍保留 |
| F-2 阈值语义 | 已核销 | `baseline.json`、19D 报告和收口声明统一为原生/Java 比值及保守区间 `≤ 1.0`；`allowed_error` 仅为测量容忍，回归规则单独写明 |
| F-3 执行点 | 部分核销 | `24c0a5c` + CI `38117244885` 已证明维护回归会跑 benchmark 测试和自检；只改 benchmark 路径的正例、负例、恢复运行号待补 |
| F-4 基线身份 | 已核销 | 结果文档注明归档报告早于阈值冻结；旧 JSON 未重写，最终身份以 `baseline.json` 为准 |
| F-5 口径登记 | 已核销 | 19D §2.5 解释 macOS `unverifiable` 与 Windows `data-insufficient` 的性质差别，Windows 理由文字已对齐 |

上述「已核销」只表示本审核项的文档或执行点已落地，不改变 10Z 收口声明中
T4 受控复跑、B1 成对实验、Z-4 和 O6 条件缺口的原判定。

## 三、落点

```text
docs/DevDocs/19d-performance-comparison.md               F-1 状态块第 11–12 行；F-5 §2.5 补口径扩用一句
tests/benchmarks/baseline.json                           F-2 threshold 补 rule 与 regression_limit 判法
tests/benchmarks/reports/19d-performance.json            F-2 rule 与 baseline.json 同口径；F-5 第 2 条理由措辞
docs/DevDocs/10z-19-closeout-declaration.md              F-2 依据出处与摘要同步；F-4 第 3 条登记
docs/DevDocs/10z-performance-measurement-results-*.md     F-4 注记（选甲时）
.github/workflows/workspace-gates.yml                    F-3 甲：paths 两处
.github/workflows/maintenance-regression.yml             F-3 乙：新增两条命令
docs/DevDocs/10z-performance-driver.md                   F-3 D7 实施记录补「触发路径」一句，并改正原「已完成」的表述
```

## 四、验收

1. **F-1**：19D 顶部状态块**内部无矛盾**，且与 §2.4、§落点、README 主表三处同口径；
2. **F-2**：`baseline.json` 单独打开就能说清两个数各是什么的上限；依据的出处要么正确、要么标明是新定判据；
   报告与声明里的依据文字与它一致；**摘要随文件更新同步**；判据取值经星崽裁定；
3. **F-3**：做一次**只碰 `tests/benchmarks/**`** 的推送，**该工作流被触发**且 benchmarks 的测试真的执行；
   按 D7 第 3 条破坏断言确认**变红**、恢复后**转绿**，运行号回填到 10Z-驱动；
4. **F-4**：结果文档（或收口声明）里能读到这个不一致及其处置；
5. **F-5**：19D §2.5 有一句授权性说明；`windows-native` 的理由文字与标签同口径；
6. **通用**：`bun run check` 退出码 0；提交标题带类型前缀、正文非空。

## 五、已定决策与仍待补证

1. **F-2 的达标线**：星崽已裁定采用 A，判据为 `≤ 1.0`；`allowed_error` 只表示测量容忍预算，
   不放宽达到 Java 的发布判据。
2. **F-3 的修法**：星崽已裁定采用乙，把 benchmark 测试和自检放进维护回归；
   只改 `tests/benchmarks/**` 的正例、负例、恢复运行号仍是外部验证项。
3. **本批的提交形态**：并入 10Z-收口 的现有提交序列，还是自成一段返工提交。
   审核倾向**并入**——它们本来就是同一次实施的审核返工。

## 相关页面

- [10Z-收口. 19 收口：O6 终局对账与 10 系列关闭](10z-19-closeout-declaration.md) —— **被审批次**；T1–T4 的定义与实施记录
- [10Z-驱动. 性能对照的计时驱动器](10z-performance-driver.md) —— D5 第 4 条（阈值冻结）与 §2.7 D7（执行点）
- [19D. 性能对照](19d-performance-comparison.md) —— F-1 与 F-5 的落点；O6 条件 5 的判定处
- [10Z 性能取数结果](10z-performance-measurement-results-20261011.md) —— F-4 注记的落点
- [10. 决策基线](00-decisions.md) 第 346–347、446 行 —— F-2 要对话的那条「10% 政策」的原文
- [09R. 字节码寄存器机型特别研究](09r-bytecode-machine-research.md) §R1-AC —— 同上
- [19. 优化、兼容性与发布验收](19-optimization-release.md) 第 34、43、69 行 —— 「至少达到 Java 以上」的原话
