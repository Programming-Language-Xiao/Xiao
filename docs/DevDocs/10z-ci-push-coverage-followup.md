# 10Z-CI覆盖. push 时的 CI 覆盖缺口（门禁自己不受门禁保护）

> **背景**：`fa9d895` 推送后 `gh run list` 没有任何新运行。查下去发现这不是偶然——
> **push 时唯一可能触发的工作流，其路径过滤只覆盖 `core/rust/**` 与 `tests/**`**，
> 而 `tools/`（**包括门禁自己的代码**）、`cli/`、`docs/` 都不在其中；唯一会跑 `bun test` / `bun run check`
> 的工作流**只在每周一定时跑**。
>
> **一句话概括本批**：**让「改了门禁代码」和「改了文档」在 push 时就有 CI 信号**，
> 并让这段门禁命令保持**单一来源**，不再各抄一份。
>
> 状态：**规划稿（2026-10-09）**。待定决策集中在末尾。

## 一、Agent 交接上下文

### 1.1 接手前提

1. [10R](10r-release-accounting-and-native-ci-gate.md) §（CI 门控那节）—— **「不另建工作流、路径过滤沿用现有」的口径出处**；
   本批要说明为什么它**不适用于**这个纯 bun 作业；
2. [10D](10d-environment-gated-test-spec.md) —— 门控测试与环境准备的既有规范；
3. [00A](00a-a0-workspace-and-checkers.md) —— `A0-*` 规则清单（本批要保护的就是这些规则自己）；
4. `.github/workflows/maintenance-regression.yml`、`.github/workflows/platform-reproduction.yml`、`tools/platform-reproduction/reproduce.sh` —— 现状的三处事实来源。

### 1.2 现状盘点（2026-10-09，审核实测）

仓库只有两个工作流。触发与覆盖如下（**均为读文件与 `gh run list` 实测**）：

| 触发 | 跑哪个工作流 | 跑什么 | 覆盖 `tools/`、`cli/`、`docs/`？ |
| --- | --- | --- | --- |
| **push / PR** | `maintenance-regression.yml` | 三个作业：Rust 尺寸回归、解析器/产物安全、原生差分矩阵 | **否** |
| **每周一 03:17 UTC**（`cron "17 3 * * 1"`） + 手动 | `platform-reproduction.yml` | `reproduce.sh native`，其中含完整门禁段 | 是，但滞后最长 7 天 |

**`maintenance-regression.yml` 的 `push.paths` 一共 18 条**（实测列出）：

```text
.github/workflows/maintenance-regression.yml
core/rust/Cargo.toml        core/rust/Cargo.lock
core/rust/crates/xiao-{artifacts,bytecode,codegen-llvm,driver,ir,lifetime,
                       optimizer,runtime,runtime-abi,syntax,types,vm,xar}/**
tests/benchmarks/**         tests/fuzz/**
```

**不在其中的关键路径**：`tools/**`（repo-check、doc-coverage —— **就是那些替我们抓别人问题的检查器**）、
`cli/**`、`docs/**`、`package.json`、`tsconfig.json`。

**唯一跑 bun 侧门禁的地方**是 `reproduce.sh:146-155` 的「Rust/TypeScript 门禁」段：

```sh
cargo test -p xiao-driver
bun install --frozen-lockfile
bun test
bunx tsc --noEmit -p tsconfig.json
cargo check --manifest-path tests/benchmarks/Cargo.toml
bun run check            # = repo-check all + clippy + lock
bun run check:coverage
cargo fmt --all -- --check
```

它只被 `platform-reproduction.yml` 调用，**而该工作流不响应 push**。

### 这次缺口造成的实际后果（不是假设）

`2c4ec43`、`4570665`、`83582c6` 三个提交都改了 `tools/repo-check/**`——

```
2c4ec43  feat: add DevDocs index uniqueness gate
4570665  fix: close DevDocs uniqueness semantics
```

**在 CI 上是零信号**：push 不触发任何工作流（`gh run list` 最新仍是 `37756208604`）。
而这三个提交里的三处缺陷——`targetPath` 死变量、函数注释与代码语义不一致、规则表缺 `A0-DOCS-003`
——**全部是人工审出来的**。同理，本会话所有 docs-only 提交也都无 CI 兜底。

也就是说：**这批一路在讲的「绿只说明没被覆盖的那部分通过了」，有结构性成因，不只是各提交自己的问题。**

### 本批边界

| 子任务 | 内容 | 本批 |
| --- | --- | --- |
| 让 `tools/`/`cli/`/`docs/` 的改动在 push 时有 CI 信号 | 新增作业 + 对应路径过滤 | **做** |
| 门禁命令的单一来源 | 不再各抄一份 | **做** |
| `platform-reproduction.yml` 的周定时 | —— | **不改**（保留为全量回归） |
| 现有三个 Rust 作业的行为 | —— | **不改** |
| 原生差分的门控矩阵、平台复现脚本本身 | —— | **不做** |
| 引入第三方变更文件过滤器 | —— | **不做**（见 §2.3） |

## 二、必须先冻结的 5 条

### 2.1 **C1：只扩路径过滤没有用，必须同时新增作业**

**冻结**：

1. **只把 `tools/**` 加进 `push.paths` 是无效改动**——现有三个作业全是 Rust 的，
   不跑 `bun test` / `bun run check`，改了门禁代码触发的还是那三个不相干的作业。
   **路径过滤与作业必须一起加**，只做一半等于没做；
2. 新作业要跑的最小集合（与 `reproduce.sh` 的门禁段对齐）：
   `bun install --frozen-lockfile` → `bun test` → `bunx tsc --noEmit -p tsconfig.json` → `bun run check`
   → `bun run check:coverage`。是否并入 `cargo fmt --check` 与 `cargo check --manifest-path tests/benchmarks`
   见 §2.2；
3. **`bun install` 必须带 `--frozen-lockfile`**，与 `reproduce.sh` 一致——否则 CI 会顺手改锁文件，
   把「锁文件变了」这类问题掩盖掉；
4. 作业要设 `timeout-minutes`（现有作业都设了，`security-maintenance` 是 20）。

### 2.2 **C2：`docs/**` 也要能触发，但 GitHub 的 `paths` 是工作流级**

这是本批主要的设计约束，**必须写清楚否则会写出一个跑不动的方案**：

**冻结**：

1. **`on.push.paths` 作用于整个工作流，不是单个作业**——所以「让文档变更只触发文档门禁作业、
   不触发三个 Rust 作业」在同一工作流内**做不到**（除非引入变更文件过滤器，见第 3 条）；
2. 因此二选一：
   - **(a) 新建一个工作流**（如 `workspace-gates.yml`），`paths` 覆盖
     `tools/**`、`cli/**`、`docs/**`、`package.json`、`tsconfig.json`、`tsconfig` 相关文件与该工作流自身；
   - **(b) 留在 `maintenance-regression.yml` 里扩宽 `paths`**——代价是 docs-only 的推送
     也会拉起三个 Rust 作业（`deterministic-size` 在 `windows-2025` 上，最贵）；
3. **建议 (a)**。理由：10R 那句「不另建工作流、路径过滤沿用现有」是针对**原生差分作业**说的——
   它要与既有作业共用 Rust/原生工具链准备，另开工作流会重复那一套。而本作业是**纯 bun**，
   与那三个作业**不共享任何准备步骤**，「沿用」在此处不成立；把它的 `paths` 塞进同一个文件，
   只会让两个不想互相牵动的触发条件绑死；
4. **不引入第三方变更文件过滤器**（`dorny/paths-filter` 之类）：为一个作业级过滤新增一个
   供应链依赖与一份 `permissions` 面，代价大于收益；真要作业级门控，用 (a) 更干净。

### 2.3 **C3：门禁命令必须是单一来源，不许再抄一份**

**冻结**：

1. 现状的命令清单**已经在 `reproduce.sh:146-155`**；新作业若再抄一遍，
   就出现两份副本——**这正是本批一路在治的毛病**（同一定义多处副本，改一处漏一处）；
2. 两条可选做法，**选一条并在提交说明里写清**：
   - **把门禁段抽成脚本**（如 `tools/gates/run.sh`），`reproduce.sh` 与新作业**都调用它**；
   - 或**给 `reproduce.sh` 加一个只跑门禁的 mode**（如 `reproduce.sh gates`），新作业调用该 mode；
3. 无论哪条，**`reproduce.sh` 的既有行为不得改变**——周定时的全量回归要继续跑它原来那些步骤；
4. Windows 侧是否也要跑这组门禁：**本批只做 Linux 作业**（与 `security-maintenance` 同构）。
   若星崽要求 Windows 覆盖，按待定决策 2 另议——不要在实现时顺手加，`windows-2025` 的分钟成本最高。

### 2.4 **C4：必须证明新作业「真的会红」，且推送后由真实运行确认**

本批的主题就是「CI 没覆盖」，所以**本批自己的验证不能只靠本地跑**：

**冻结**：

1. **本地不构成验收**：`bun test` 在本机绿过不等于新作业接线正确。
   **必须在推送后从 `gh run list` / `gh run view` 确认新工作流确实被触发且为 success**；
2. **必须做一次「让它变红」的验证**：临时制造一次门禁违规（例如在 `docs/DevDocs/README.md`
   主表里加一条重复链接，触发 `A0-DOCS-004`），推上去确认**新作业变红**，然后恢复并确认转绿。
   ——这与 [10Y](10y-b-series-triage-rework.md) §2.2 第 3 条、[10Z-唯一性](10z-index-uniqueness-followup.md) §2.5 同一条要求，
   只是这次红的是 CI 而不是本地；
3. **恢复后要能证明干净**：`git diff --quiet` 通过，且**最终状态是绿的**（不要留一个红的运行记录而不解释）；
4. 若因不想在 `main` 上制造红色运行而采用 PR 方式，**要在交接里说明用了哪种方式**，
   并给出对应的运行号——**不接受「我本地试过」**；
5. 提交说明里贴 **运行号**与结论（`gh run view <id>` 的关键行）。

### 2.5 **C5：不动既有作业与调度**

**冻结**：

1. `platform-reproduction.yml` 的周定时**保留**——它是全量回归（四平台、原生、打包回环），
   与「push 时的快速门禁」职责不同，不能因为多了快速门禁就删掉或降频；
2. 现有三个作业的步骤、runner、超时**不改**；
3. **不要**借这次机会把 clippy / `cargo test --workspace` 也塞进新作业——
   `bun run check` 已含 `check:clippy`，重复跑是浪费；`cargo test --workspace` 的覆盖面另议（待定决策 3）；
4. 不新增第三方 action：`oven-sh/setup-bun` 已在 `platform-reproduction.yml` 用过（`@v2`，bun `1.4.1`），
   直接复用同一 pin 即可。

## 三、落点

```text
.github/workflows/workspace-gates.yml（新）        方案 (a)：push/PR 触发，paths 覆盖 tools/**、cli/**、
                                                   docs/**、package.json、tsconfig.json 及其自身
tools/gates/run.sh（新）或 tools/platform-reproduction/reproduce.sh    §2.3：门禁命令的单一来源
tools/platform-reproduction/reproduce.sh            §2.3 第 3 条：既有行为不得改变
docs/DevDocs/10r-release-accounting-and-native-ci-gate.md   §2.2：说明 10R 口径的适用范围边界
docs/DevDocs/10d-environment-gated-test-spec.md      回填「哪些门禁在 push 时跑、哪些只在周定时跑」
docs/DevDocs/README.md                              主表登记
```

## 四、硬约束

1. **路径过滤与作业必须一起加**（§2.1 第 1 条）——只加过滤是无效改动；
2. `bun install` 必须 `--frozen-lockfile`（§2.1 第 3 条）；
3. **命令清单只能有一份**（§2.3）；`reproduce.sh` 既有行为不变；
4. **验收必须包含推送后的真实运行号**与一次**真的变红**（§2.4）——本地绿不算；
5. 不动既有作业、调度与 `platform-reproduction.yml` 的职责（§2.5）；
6. 不新增第三方 action；`oven-sh/setup-bun` 复用既有 pin（`@v2`，bun `1.4.1`）；
7. 每个作业设 `timeout-minutes`；
8. 不新增依赖，`Cargo.lock` 保持同步；每个提交单独跑 `cargo fmt`/clippy；
9. 提交说明带正文，写明**改了哪个工作流的哪个触发条件**与运行号。

## 五、分步提交

1. **定方案**：在本文档或提交说明里写明选 (a) 还是 (b) 与理由（§2.2）；
2. **抽命令单一来源**：§2.3 二选一；确认 `reproduce.sh` 行为不变（本地跑一次 `reproduce.sh native` 的门禁段或至少 `bun run check`）；
3. **加工作流与路径过滤**：按所选方案；复用既有 action pin；设 `timeout-minutes`；
4. **推送并确认真实触发**：`gh run list` 看到新工作流、且为 success；
5. **让它变红**：临时制造一次 `A0-DOCS-004` 违规（README 加重复链接）→ 推送 → 确认新作业红 → 恢复 → 确认转绿；
6. **回填文档**：10R 的口径边界、10D 的「哪些门禁何时跑」、README 主表；
7. **全量门禁**：`cargo test --workspace`、clippy、fmt、`bun test`、`bun run check`、`bunx tsc --noEmit`。

## 六、最可能翻车的地方

1. **只加了 `paths` 没加作业**（或反过来），看起来改了、实际没覆盖（§2.1 第 1 条）；
2. **以为 `on.push.paths` 能按作业过滤**，写出一份自以为只跑轻量作业、实际会把
   `windows-2025` 上的尺寸回归也拉起来的工作流；
3. **把门禁命令又抄了一份**进新工作流，两处从此各自漂移；
4. **`bun install` 漏了 `--frozen-lockfile`**，CI 顺手改锁文件；
5. **验收只报本地结果**——本批的主题就是 CI 没覆盖，却用本地绿来证明 CI 修好了；
6. **「让它变红」做成走过场**（改一个不会触发的路径、或只看本地退出码）；
7. **为了让它变红而把红留在 `main` 上不解释**（要么恢复转绿，要么在提交说明里说明运行号与结局）；
8. **顺手删掉或降频 `platform-reproduction` 的周定时**，把全量回归弄没了；
9. **顺手往新作业里塞 clippy / `cargo test --workspace`**，与 `bun run check` 的内容重复；
10. **新增第三方 action** 或自己编一个 SHA；
11. **提交没写正文**。

## 七、验收

1. **`tools/`、`cli/`、`docs/`、`package.json`、`tsconfig.json` 的改动在 push 后确实会触发 CI**，
   并有对应的运行号为证；
2. 新作业跑的门禁命令与 `reproduce.sh` **来自同一份定义**（§2.3），且 `reproduce.sh` 既有行为未变；
3. **有一次真实的「变红」证据**：临时违规 → 新作业红 → 恢复 → 绿，运行号写在提交说明里（§2.4）；
4. `bun install` 带 `--frozen-lockfile`；作业有 `timeout-minutes`；未新增第三方 action；
5. 现有三个作业、`platform-reproduction.yml` 的调度与职责**均未改变**；
6. 10R 的口径边界与 10D 的「何时跑什么」已回填；
7. `cargo test --workspace`、clippy、fmt、`bun test`、`bun run check`、`bunx tsc --noEmit` 全绿；
8. 没有通过放宽断言、跳过命令或缩小覆盖换来的绿。

## 八、不负责与不要重复做的事

- **不改** `platform-reproduction.yml` 的调度与步骤（§2.5）；
- **不改**现有三个 Rust 作业；
- **不做**原生差分门控矩阵、不做平台复现脚本本身；
- **不引入**变更文件过滤器等第三方 action；
- **不做** Windows 侧的门禁作业（除非按待定决策 2 另行决定）；
- **不做** B 系列、19 收口相关的任何事。

## 待定决策

1. **方案 (a) 新建工作流 还是 (b) 扩宽现有工作流的 `paths`**——建议 (a)（§2.2 第 3 条）。
   这条偏离了 [10R](10r-release-accounting-and-native-ci-gate.md) 的「不另建工作流」口径，
   需要星崽确认该口径的适用范围确实限于「与原生作业共享准备步骤」的情形。
2. **是否也要 Windows 侧覆盖**：`windows-2025` 的分钟成本最高，本批建议只做 Linux。
3. **`cargo test --workspace` 要不要进 push 门禁**：目前 push 时跑的是三个精选作业，
   全量 workspace 测试只在周定时跑。这属于**覆盖面**的取舍，与本批的「路径缺口」是两件事，
   本批不顺手扩，避免把两件事混在一起谈。

## 相关页面

- [10R. 释放账目收口与原生 CI 门控](10r-release-accounting-and-native-ci-gate.md) —— 「不另建工作流 / 路径过滤沿用现有」的口径出处
- [10D. 环境依赖测试专项规范](10d-environment-gated-test-spec.md) —— 门控测试与环境准备
- [00A. 工作区与质量门禁实现方案](00a-a0-workspace-and-checkers.md) —— 本批要保护的 `A0-*` 规则清单
- [10Z-唯一性. 主索引唯一性门禁收口](10z-index-uniqueness-followup.md) —— 同期立项；它的验证之所以只能人工做，正是本批的成因
