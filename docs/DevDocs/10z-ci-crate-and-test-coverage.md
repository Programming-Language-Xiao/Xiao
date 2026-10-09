# 10Z-CI覆盖2. crate 过滤缺口与全量测试无覆盖

> **背景**：[10Z-CI覆盖](10z-ci-push-coverage-followup.md) 补上了 `tools/`、`cli/`、`docs/` 的 push 覆盖，
> 那批本身做对了。但审核顺着同一根线再查一层，发现两处**更大**的洞：
> **① 路径过滤漏掉了 23 个 workspace 成员里的 10 个**（因为它是枚举式清单）；
> **② 全量默认测试套件在 CI 里从不运行**——唯一的 `--workspace` 带着 `-- --ignored`，只跑被忽略的用例。
>
> **一句话概括本批**：**把「枚举式覆盖范围」换成「目录 glob」从根上消除漏项**，
> 并让**全量默认测试**至少在一个自动触发的地方跑起来。
>
> 状态：**规划稿（2026-10-09）**。待定决策集中在末尾。

## 一、Agent 交接上下文

### 1.1 接手前提

1. [10Z-CI覆盖](10z-ci-push-coverage-followup.md) —— **上一轮**；`tools/gates/run.sh` 的单一来源与 `workspace-gates.yml` 的写法都在那里，本批沿用其口径；
2. [10R](10r-release-accounting-and-native-ci-gate.md) —— 「不另建工作流」的口径出处（本批是**同工作流内新增作业**，不冲突）；
3. `.github/workflows/maintenance-regression.yml`、`.github/workflows/workspace-gates.yml`、`core/rust/Cargo.toml`、`tools/gates/run.sh`、`tools/platform-reproduction/reproduce.sh` —— 现状的全部事实来源。

### 1.2 现状盘点（2026-10-09，审核实测）

**① crate 过滤缺口**

`core/rust/Cargo.toml:3-27` 的 `members` 有 **23 个** crate；`maintenance-regression.yml` 的
`push.paths` / `pull_request.paths` 只**枚举了 13 个**：

```text
已覆盖（13）  xiao-artifacts  xiao-bytecode  xiao-codegen-llvm  xiao-driver  xiao-ir
              xiao-lifetime   xiao-optimizer xiao-runtime       xiao-runtime-abi
              xiao-syntax     xiao-types     xiao-vm            xiao-xar
未覆盖（10）  xiao-source     xiao-diagnostics  xiao-i18n       xiao-config
              xiao-modules    xiao-intrinsics   xiao-package    xiao-lock
              xiao-platform   xiao-doc-coverage-rust
```

而新加的 `workspace-gates.yml` 的 `paths` 里**完全没有 `core/rust/crates/**`**（只有 `core/rust/rust-toolchain.toml`）。

**结论：这 10 个 crate 的改动，push 后两个工作流都不会触发。** 其中 `xiao-diagnostics`（诊断窗口那套）与
`xiao-i18n`（文案目录）正是 10V 动过的 crate，`xiao-package`、`xiao-lock` 也在列。

**② 全量默认测试无覆盖**

把工作流与两个脚本里的 `cargo test` 全部列出（实测）：

```text
maintenance-regression.yml   -p 精选目标（artifact_size_regression / fuzz_xiaoc / fuzz_archive /
                                xiao-artifacts / xiao-xar / cross_platform_release / release_report /
                                n0_a_native_driver / n0_b_dynamic_native / d19a_differential /
                                native_benchmark_probe / xiao-runtime / xiao-runtime-abi …）
tools/gates/run.sh:12        cargo test -p xiao-driver
reproduce.sh:132             cargo test --workspace -- --ignored
```

**唯一的 `--workspace` 带 `-- --ignored`——只跑被忽略的用例。**
即 **CI 里没有任何地方跑全量默认测试套件**，跑的是精选目标。后果：

| crate | `tests/` 文件数 | 默认测试在 CI 里跑过吗 |
| --- | --- | --- |
| `xiao-package` | **15** | **否** |
| `xiao-i18n` | 3 | **否** |
| `xiao-config` | 3 | **否** |
| `xiao-modules` | 3 | **否** |

**本地实测基准**（审核实跑，win32 本机、暖缓存）：

```text
cargo test --workspace   退出码 0    用时 135 s    test result 行 142 个    FAILED 0 个
```

CI 冷缓存会更久，但落在现有作业 20 分钟超时的量级内——**这是本批决策要用到的数字**。

**③ 一条死路径条目**

`workspace-gates.yml` 的 `paths` 里有 `bunfig.toml`，**该文件在仓库中不存在**（实测）。
无害，但它说明这份清单是照抄而非核对过的。

### 审核自身失误（必须记下，因为本批的成因就在这）

**本批的规划文档（10Z-CI覆盖 §1.2）自己写的就是那 13 个 crate 的枚举**——审核读出 `maintenance-regression.yml`
里有什么就写了什么，**没有拿它跟 `Cargo.toml` 的 `members` 对账**。所以规划**漏报了缺口的大头**：
它只报了 `tools/`、`cli/`、`docs/`，没报这 10 个 crate。

这与既有教训是**同一个毛病**：枚举式清单只查了一部分，就把「还剩几个没查」变成了未知数
（此前在 GitHub Action 版本上犯过一次，见审核记忆）。**本批要把它变成规则，而不是再犯一次。**

### 本批边界

| 子任务 | 内容 | 本批 |
| --- | --- | --- |
| 过滤改成 glob | `maintenance-regression.yml` 的 crate 枚举 → `core/rust/**` | **做** |
| 全量默认测试有自动触发点 | 决定放哪并接上 | **做**（放哪见待定决策 1） |
| 死路径条目清理 | 存在性核对 | **做** |
| 枚举式清单的对账规则 | 写进规范 | **做** |
| 上一轮已交付的工作流与共享脚本 | —— | **不改其结构**（只在必要时补 paths） |
| 原生差分矩阵、平台复现脚本本身 | —— | **不做** |

## 二、必须先冻结的 5 条

### 2.1 **R1：过滤范围一律用目录 glob，不再枚举**

**冻结**：

1. 把 `maintenance-regression.yml` 的 `push.paths` 与 `pull_request.paths` 里那 **13 条 crate 枚举**
   替换为 **`core/rust/**`**（`Cargo.toml`、`Cargo.lock`、`rust-toolchain.toml` 自然被覆盖，可一并删掉冗余条目）；
2. **理由不是「少写几行」，而是消除漏项这一类**：枚举 23 个成员正是丢掉 10 个的原因；
   只要还是枚举，下次新增 crate 就会再漏一次。**glob 让「新增 crate」自动被覆盖**；
3. `workspace-gates.yml` 的 paths **保持不含 `core/rust/crates/**`**——它管的是 bun/文档/工具侧；
   Rust 侧由 `maintenance-regression.yml` 负责，两个工作流职责不重叠（`core/rust/rust-toolchain.toml`
   两边都触发无害，保留）；
4. **改动后要验证 glob 真的生效**：至少一次**真实触发证据**——改动一个**此前未覆盖**的 crate
   （如 `xiao-package` 下的注释级改动）→ 推送 → 确认 `maintenance-regression` **被触发**（附运行号）→ 恢复；
5. 同时确认**没有扩得过宽**：`core/rust/**` 只覆盖 Rust 侧，不应把 `tools/`、`cli/`、`docs/` 捎带进来
   （它们归 `workspace-gates.yml`）。

### 2.2 **R2：全量默认测试必须有自动触发点，且与 `--ignored` 那轮是两件事**

**冻结**：

1. **事实要先写进文档**：`cargo test --workspace`（**不带** `--ignored`）在任何工作流里都不存在；
   `reproduce.sh:132` 的 `--workspace -- --ignored` **只跑被忽略的用例**，**不能**拿它充当全量默认测试。
   这两条命令是**两件事**，[10D](10d-environment-gated-test-spec.md) 与 README 的相关措辞要按此校准；
2. **必须在至少一个自动触发的地方跑全量默认测试**。三个方案（见待定决策 1），**选定后在提交说明里写理由**：
   - **P1**：在 `maintenance-regression.yml` 里**新增一个作业**跑 `cargo test --workspace`（push 触发）；
   - **P2**：只加进周定时的 `reproduce.sh`（默认 + 忽略两轮都跑），push 不跑，最长滞后 7 天；
   - **P3**：按变更文件做受影响 crate 的定向测试——**需要变更文件过滤器，本批明确不做**（见 10Z-CI覆盖 §2.2 第 4 条）；
3. **推荐 P1**：本批的主题就是覆盖，而实测 135 s（暖缓存）落在现有作业 20 分钟超时的量级内；
   且 `--ignored` 那轮本来就在同一工作流里跑长任务，「这个工作流只跑快的」这个前提不成立。
   **新增作业而不是塞进现有作业**——`security-maintenance` 跑的是精选安全回归，改变它的性质会让
   它的 20 分钟超时变得难以解释；新作业独立超时更清楚；
4. **不得**为了压时长而缩小范围（`--workspace` 不许改成 `-p` 某几个 crate）；若时长确实超标，
   应当**调超时**或**改方案**，并在提交说明里说明；
5. 全量测试**必须真的会红**：做一次负向取证——在某个此前无覆盖的 crate（如 `xiao-package`）
   临时改坏一个测试 → 推送 → 确认作业**红** → 恢复 → 确认**绿**，运行号写进提交说明。
   这与 [10Z-唯一性](10z-index-uniqueness-followup.md) §2.5、[10Z-CI覆盖](10z-ci-push-coverage-followup.md) §2.4 同一条要求。

### 2.3 **R3：枚举式覆盖范围必须与权威来源对账（本批要立的规则）**

**冻结**：

1. **凡是以枚举方式声明「覆盖了哪些」的配置或文档**（工作流 `paths`、规则清单、crate 列表、
   action 列表、测试目标列表……），**必须写明它的权威来源，并逐项对账**；
2. 对账结果要能复述：**「权威来源共 N 项，本清单 M 项，差集是什么」**。
   只写「已覆盖 X、Y、Z」而不写差集的，**视为未对账**；
3. **能用 glob/模式表达的覆盖范围，一律不用枚举**——本批 R1 就是这条的实例；
4. 这条规则落点：写进 [00A](00a-a0-workspace-and-checkers.md) 或 [10D](10d-environment-gated-test-spec.md)
   的**门禁/清单维护**小节，并在本批的实施记录里引用；
5. **本条不引入自动化检查**（不新增规则号）——先靠人工对账与文档要求；若日后同类错误再犯，
   再考虑加机器检查。

### 2.4 **R4：清单里的每条路径都要存在，或用 glob**

**冻结**：

1. 清掉 `workspace-gates.yml` 里的 `bunfig.toml`（**仓库中不存在**）；
2. **每个字面路径条目都要核对存在性**——`tsconfig.*` 这类 glob 除外；核对结果写进提交说明
   （「N 条字面路径，全部存在」或列出删掉了哪几条）；
3. 新增 `core/rust/**` 后，`maintenance-regression.yml` 里若仍有冗余的字面条目（被 glob 覆盖的），
   **删掉**，避免出现「两处声明同一范围、日后各自漂移」；
4. 这条与 R3 同源：**清单是承诺，不是抄写**。

### 2.5 **R5：不动上一轮已交付的结构与调度**

**冻结**：

1. `tools/gates/run.sh` 的**单一来源地位不变**；`workspace-gates.yml` 与两个平台复现入口继续委托它；
   本批**只在确实需要时**往里加命令（例如若选 P2，`cargo test --workspace` 进这里——但要注意
   `run.sh` 被 push 工作流调用，**加进去就等于 push 也跑**，与 P2 的意图冲突，选 P2 时应加在
   `reproduce.sh` 的门禁段之外）；
2. `platform-reproduction.yml` 的周定时与职责**不改**；
3. 上一轮新建的 `workspace-gates.yml` **结构不改**（只清死路径条目）；
4. 不新增第三方 action；`oven-sh/setup-bun`、`dtolnay/rust-toolchain`、`Swatinem/rust-cache`
   复用既有 pin。

## 三、落点

```text
.github/workflows/maintenance-regression.yml     R1：crate 枚举 → core/rust/**；删冗余字面条目；
                                                 R2/P1：新增全量默认测试作业（若选 P1）
.github/workflows/workspace-gates.yml            R4：删掉不存在的 bunfig.toml
tools/gates/run.sh                               仅按 R5 第 1 条判断是否动（选 P1 则不动）
tools/platform-reproduction/reproduce.sh         R2/P2：默认测试与 --ignored 两轮都要有（若选 P2）
docs/DevDocs/00a-a0-workspace-and-checkers.md    R3：枚举式清单的对账规则
docs/DevDocs/10d-environment-gated-test-spec.md  R2：全量默认测试与 --ignored 是两件事，措辞校准
docs/DevDocs/10z-ci-push-coverage-followup.md    §1.2 的 crate 列表补注「本批只覆盖了三个目录，
                                                 10 个 crate 与全量测试见 10Z-CI覆盖2」
docs/DevDocs/README.md                           主表登记
```

## 四、硬约束

1. **覆盖范围用 glob 不用枚举**（§2.1）——本批的核心；
2. 全量默认测试与 `--ignored` 是**两件事**，都要有（§2.2 第 1 条）；
3. **`--workspace` 不许缩小成 `-p`**；时长超标就调超时或改方案，不许缩范围（§2.2 第 4 条）；
4. **两条覆盖都各自要有真实触发运行号 + 一次真的变红**（§2.1 第 4 条、§2.2 第 5 条）——**本地绿不算**；
5. 清单里每条字面路径都核对过存在性（§2.4）；
6. 枚举式清单要写权威来源与差集（§2.3）；
7. 不新增第三方 action、不新增规则号；不动周定时与上一轮的工作流结构（§2.5）；
8. 每个提交单独跑 `cargo fmt` / clippy；推送前跑 `cargo test --workspace`、`bun test`、
   `bun run check`、`bunx tsc --noEmit`；
9. 提交说明带正文，写明**改了哪个工作流的哪一段**与运行号。

## 五、分步提交

1. **R1 改过滤**：枚举 → `core/rust/**`，删冗余条目；推送到 PR 或按 §2.1 第 4 条做触发取证；
2. **R2 加全量测试**：按待定决策 1 选定方案并实施；做负向取证（改坏一个此前无覆盖的 crate 的测试）；
3. **R4 清死路径**：核对并删除不存在的条目；
4. **R3 写规则**：00A 或 10D 的门禁/清单维护小节；
5. **文档校准**：10D 的措辞、10Z-CI覆盖 §1.2 的补注、README 主表；
6. **全量门禁**：`cargo test --workspace`、clippy、fmt、`bun test`、`bun run check`、`bunx tsc --noEmit`。

## 六、最可能翻车的地方

1. **把 `--ignored` 那轮当成全量默认测试**，认为「CI 已经在跑 workspace 测试了」（§2.2 第 1 条）；
2. **为了压时长把 `--workspace` 缩小成几个 `-p`**——那就又把「枚举」请回来了；
3. **只改过滤不做触发取证**，glob 写错了也不知道（例如误写成 `core/rust/crates/**` 而漏掉 `Cargo.toml`）；
4. **把 `core/rust/**` 也塞进 `workspace-gates.yml`**，让两个工作流职责重叠、同一改动跑两遍；
5. **新增作业塞进现有作业**，把 `security-maintenance` 的性质和超时解释弄乱；
6. **负向取证做成走过场**（改一个不会红的测试、或只看本地）；
7. **清单里仍有不存在的路径**没核对（R4 不是只删 `bunfig.toml` 一条）；
8. **枚举与 glob 并存**，同一范围两处声明，日后各自漂移；
9. 新增第三方 action 或自编 SHA；
10. **提交没写正文**。

## 七、验收

1. `maintenance-regression.yml` 的路径过滤**不含** crate 枚举，`core/rust/**` 一条覆盖全部 23 个成员；
   有**真实触发运行号**证明此前未覆盖的 crate 改动现在能触发（§2.1 第 4 条）；
2. **全量默认测试在自动触发的 CI 里真的跑了**，有运行号；与 `--ignored` 那轮**明确区分**；
   有**一次真的变红**的负向取证（运行号 + 恢复后转绿）（§2.2 第 5 条）；
3. `--workspace` 未被缩小；若时长超限，是调超时或改方案，且提交说明里有解释；
4. 清单里每条字面路径都核对过存在性，`bunfig.toml` 已删，冗余条目不存（§2.4）；
5. 枚举式清单的对账规则已写进规范，且本批实施记录引用了它（§2.3）；
6. `workspace-gates.yml`、`tools/gates/run.sh` 的单一来源地位与 `platform-reproduction.yml` 的调度**均未改变**；
7. 10D 的措辞与 10Z-CI覆盖 §1.2 的补注已校准；
8. `cargo test --workspace`、clippy、fmt、`bun test`、`bun run check`、`bunx tsc --noEmit` 全绿；
9. 没有通过缩小范围、跳过命令或放宽断言换来的绿。

## 八、不负责与不要重复做的事

- **不做** B 系列、19 收口；
- **不做** 10Z-CI覆盖 已完成的那三项目录覆盖（已交付，本批只在其 §1.2 加补注）；
- **不做**变更文件过滤器（P3）；
- **不做** Windows 侧的全量测试作业（成本另议，本批只做 Linux）；
- **不改** `platform-reproduction.yml` 的调度；**不改**上一轮工作流的结构。

## 待定决策

1. **全量默认测试放哪（P1 / P2）**——建议 P1（`maintenance-regression.yml` 内新增作业，push 触发）。
   需要星崽拍板的是**成本**：实测暖缓存 135 s，CI 冷缓存更久，但仍在 20 分钟量级内。
   若星崽认为 push 时长不可接受，选 P2（只进周定时，滞后最长 7 天）。
2. **Windows 侧要不要也覆盖这 10 个 crate**：`core/rust/**` 的 glob 会自动把它们带进 push 过滤，
   但 `maintenance-regression` 只有部分作业是 Windows。本批只保证 Linux 侧真的跑到，Windows 另议。
3. **是否顺带加一条机器检查**（工作流 `paths` 里字面条目的存在性）——本批按 §2.3 第 5 条先不加，
   若这类错误再犯再考虑。

## 相关页面

- [10Z-CI覆盖. push 时的 CI 覆盖缺口](10z-ci-push-coverage-followup.md) —— 上一轮；本批是它没覆盖到的那一层
- [10Z-唯一性. 主索引唯一性门禁收口](10z-index-uniqueness-followup.md) —— 同期立项；验证取证的同一条要求
- [10R. 释放账目收口与原生 CI 门控](10r-release-accounting-and-native-ci-gate.md) —— 「不另建工作流」的口径出处
- [10D. 环境依赖测试专项规范](10d-environment-gated-test-spec.md) —— 门控测试与命令清单
- [00A. 工作区与质量门禁实现方案](00a-a0-workspace-and-checkers.md) —— 规则清单与维护约定
