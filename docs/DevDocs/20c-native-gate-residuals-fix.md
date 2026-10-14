# 20C-原生门禁残余项修复：跨步骤遮蔽与释放位置断言

> **背景**：[原生门禁修复专项](20c-native-gates-red-fix.md) 收口时**故意留下两条**——
> 都不是红、都不阻塞 `main`，但**各让一条判据名不副实**。
> 那篇已经很长，且这两条**彼此独立**（一条在 CI 脚本、一条在测试断言），所以单独成文，
> 各自给「最小实施步骤 + 最小验证步骤 + 负例」，直接可执行。
>
> **一句话概括**：**R-1** 让脚本不再"前面一红、后面全不跑"；**R-2** 让短路释放的断言能看见**位置**。
>
> 状态（2026-10-14）：**两条都已实现并取证**——R-1 走一次性分支的红/绿（`38534726488` / `38535786407`），
> R-2 走本地变异；`main` 未引入故意失败，验证分支已删。
> **第三轮审核并入三条**（§七，**没有一条是红**）：绿运行与提交的对应已写准、
> 方案甲的清单宽度（**`tsc` 已一并前移**）、远端 8 条历史一次性分支**已清**（现只剩 `main`）。

## 一、为什么单独成文

1. **同源但不同层**：R-1 在 **CI 脚本**（`tools/gates/run.sh`），R-2 在 **测试断言**（`c20_capability_probe.rs`）；
2. **同一个形状**：两条都是「**绿了 ≠ 判据到位**」——与本批反复出现的那类错同型
   （F-3 → G-1 → 遮蔽 → §5.2.1 → 这里）；
3. **都不能用绿跑取证**：R-1 的判据**只能由一次故意的红**证明（fail-fast 只在有失败时才显出差异）；
   R-2 的判据**只能由一次变异**证明；
4. **都很小**：各自一两处改动 + 一次验证，适合单独派工。

## 二、R-1 **跨步骤遮蔽**（`tools/gates/run.sh`）

### 2.1 事实（修复前，可逐条核）

以下行号和顺序记录修复前的脚本，用来说明跨步骤遮蔽如何发生；当前顺序与执行证据见 §2.7。

`tools/gates/run.sh`：

```bash
3:  set -Eeuo pipefail
12: cargo test --manifest-path "$cargo_manifest" -p xiao-driver --no-fail-fast
13: bun install --frozen-lockfile
14: bun test
15: bunx tsc --noEmit -p tsconfig.json
16: cargo check --manifest-path "$benchmark_manifest"
17: cargo test --manifest-path "$benchmark_manifest" --no-fail-fast
18: cargo run … performance_driver -- --self-test
19: bun run check
20: bun run check:coverage
21: cargo fmt --all … -- --check
```

- `--no-fail-fast` 治的是**同一个 `cargo test` 内的多个测试二进制**：`cargo test -p xiao-driver`
  会拉起 **23 个测试二进制**（本机实测顺序，前几个是：
  `unittests src/lib.rs` → `unittests src/protocol_main.rs` → `tests/20c_first_modules.rs` →
  `tests/artifact_size_regression.rs` → … → `tests/u0_frontend.rs`），
  不关 fail-fast 就会**停在第一个失败者**上；
- **但脚本级仍是失败即停**（`set -e`）：**第 12 行一红，第 13–21 行全部不跑**。
  被遮住的包括 `bun run check`（文档与清单）、`cargo fmt --check`、`check:coverage` 这些**纯静态门禁**，
  **还包括第 17 行 benchmarks 的那次 `cargo test`**；
- **CI 日志里有逐二进制的 `Running <binary>` 行**（workspace 门禁 `38522338905` 的日志可查），
  所以「后面的二进制到底跑没跑」是**逐条可核**的，不需要推断。

### 2.2 冻结（二选一，**必须写明选了哪个**）

- **甲：纯静态步骤前置**——把 `cargo fmt --check`、`bun run check`、`check:coverage`
  移到第 12 行**之前**。它们与 Rust 测试无依赖，前置后不会被任何测试失败遮住；
- **乙：收集全部失败再退出**——去掉 `-e` 的提前退出（逐条执行、各自记结局、最后统一失败）。

**倾向甲**：改动最小，且静态门禁本来就该"早失败"（文档/格式比测试便宜得多）。
**不论选哪个，第 17 行 benchmarks 的 `cargo test` 也必须不再被第 12 行遮住。**

**本次选择：甲。** 静态门禁在 benchmark 与 `xiao-driver` 测试之前执行；保留两个
`cargo test` 的 `--no-fail-fast`，让 benchmark 测试也不会被 `xiao-driver` 测试遮住。

**清单宽度（第三轮审核补充）**：上面「甲」只点了**三项**静态门禁，实施方照做**无误**；
但**同一理由还落在 `bunx tsc --noEmit` 上**——它是**纯静态类型检查**，
当时仍排在 `cargo test -p xiao-driver` **之后** ⇒ 后者一红，它照样被遮住。
**已按 §七 的 R-1b 前置**（现行顺序：`bun install` → fmt → check → coverage → **tsc** →
benchmark 三件 → `cargo test -p xiao-driver` → `bun test`）。
`bun test` 是**动态套件**，要不要一起前移**另论**（比 Rust 测试便宜、但比静态检查贵）。

### 2.3 最小实施步骤

1. 按 2.2 择一改 `tools/gates/run.sh`；
2. **在本机先跑一遍** `bash tools/gates/run.sh`，确认**全绿时行为与改前一致**
   （步骤顺序变了不算破坏，但每一条的输出都要出现）；
3. 本机确认「**全绿的运行证明不了任何事**」——所以真正的验证在 2.4。

### 2.4 最小验证步骤（**这可能是一次性分支，`main` 不许红**）

照 [20C](20c-official-xiao-library.md) §2.4 末三条纪律与第七轮审核的教训：

1. **从 `origin/main` 起一次性分支**——**不许**从一个带未推送提交的 `HEAD` 起
   （第七轮审核就是栽在这里：只看改了哪个文件、没看**分支相对默认分支的整体改动**）；
2. **提交 A**：在 `tests/20c_first_modules.rs`（顺序上的**第 3 个**二进制）里
   **故意弄坏一条断言**，**只碰这一个文件**；
3. 推送，看 **workspace 门禁**日志，核三件事：
   - 该二进制**红**（`test result: FAILED`）——前提成立；
   - **它之后的二进制仍出现 `Running` 行**（从第 4 个 `artifact_size_regression` 起，
     直到最后一个）⇒ **R-1 第 1 条生效**；
   - **静态步骤仍跑了**：
     - 若选**甲**：`cargo fmt`/`bun run check` 的行**出现在**失败的 `cargo test` **之前**；
     - 若选**乙**：它们的行**出现在**失败之后；
     ⇒ **R-1 第 2 条生效**；
4. **提交 B**：还原同一文件；确认**转绿**；
5. **删分支**（本地与远端），**运行号回填本页与专项 §5.2.1**；
6. **不许**让 `main` 变红；**不许**用「本机跑过」代替运行号。

### 2.5 不许

1. **不许**只写「加了参数」或「改了脚本」——这正是 §5.2 在原生作业上踩过的：**加了参数 ≠ 证明跑到了**；
2. **不许**用任意一次**绿跑**作为本条的取证——见 2.6；
3. **不许**在验证提交里顺手带 `core/rust/**`、`docs/**`、工作流文件（否则触发面变混）。

### 2.6 **判据（写死，防止误用）**

> **本条的取证只能来自一次「故意的红」**，且必须同时看到两件事：
> ① 同一 `cargo test` 内、排在失败者之后的二进制**仍跑**；
> ② 静态门禁（按所选的甲/乙）**仍跑**。
>
> **任意一次全绿运行都不构成取证**——fail-fast 只在有失败时才显出差异，
> 全绿时改前改后的行为完全一样。「脚本带上参数仍然正常」与「参数治好了遮蔽」是两件事。
> **不许**用 `38522338905` 或任何绿跑把本条写成已取证。

### 2.7 实施记录

`tools/gates/run.sh` 已按方案甲调整为：`bun install` → `cargo fmt --check` →
`bun run check` → `bun run check:coverage` → benchmark 检查/测试/自测 →
`cargo test -p xiao-driver --no-fail-fast` → Bun 测试与 TypeScript 类型检查。
因此静态门禁不会被任一 Rust 测试失败遮住，benchmark 测试也位于 `xiao-driver` 测试之前。

R-1 的故意红验证使用一次性分支完成：失败的 `20c_first_modules` 二进制之后仍出现
`Running` 行，且静态门禁的输出位于失败测试之前。真实取证如下：

- 红运行 [38534726488](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/38534726488)：
  提交 A `3dc0d82` 只把 `20c_first_modules` 的一条输出断言改坏；日志在
  `test result: FAILED` 后继续出现 `Running tests/artifact_size_regression.rs`，随后还跑到
  `c20_capability_probe` 等二进制；`bun run check` 与覆盖率输出均在失败前出现。
- 绿运行 [38535786407](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/38535786407)：
  **该运行的头是 `f5210a3`**——提交 B `5b693cd` 还原同一断言之后，又叠了一笔
  步骤标签提交（即后来进主线的 `969f06a`），绿跑打在这笔上。标签明确显示
  `cargo fmt --check`、`bun run check`、`check:coverage` 均先于 benchmark 与 `xiao-driver` 测试。

> **对应关系要写准（第三轮审核更正）**：本文上一版把绿运行挂在「提交 B `5b693cd`」上，
> 而该运行的 `headSha` 是 `f5210a3`。**核的人会拿到对不上的 sha**——
> 这正是本批反复强调的「证据要能对着原始输出逐字核」。
> **正确表述：绿运行 `38535786407` 的头是 `f5210a3`（= B 还原 + 步骤标签）。**

提交 A 只改动 Rust 测试文件，未命中 workflow 的 `push.paths`，因此按同一分支手动 dispatch
workspace 门禁；这不改变提交 A 的故意红内容，也保留了从 `origin/main` 起分支的纪律。

**审核复核（2026-10-14，第三轮）**：上面两条取证**由审核拉原始日志与 API 逐条核实**，不是照抄：
红运行 `headSha = 3dc0d82` ✓ 与提交 A 一致；日志时间线 `bun run check` **08:23:26**、
`check:coverage` **08:24:16**、失败 **08:25:03** ⇒ 静态门禁确实在失败之前；
失败之后日志继续出现 `Running tests/artifact_size_regression.rs`、`b0_c_driver`、
`b0_d_exit_codes`、**`c20_capability_probe`**、`compatibility_matrix`、`cross_platform_release`、
`d17c_archive` 等 ✓；benchmarks 的 `Running` 也排在 `xiao_driver` 之前 ✓。
另外核实：验证分支 `codex/20c-gate-residual-verify` **已从远端删除** ✓；
`969f06a` 只加 `printf` 分界、`7aeaec0` 只把断言收成一行（**都没有放松**）。

## 三、R-2 **释放位置断言**（`c20_capability_probe.rs`）

### 3.1 事实

`logical_short_circuit_keeps_shared_release_plans` 现在的断言是
`count(with) > count(without)`（TAC 与 LLVM 各一条）。**它只比总数，对位置不敏感**：

把右侧块（`block[1]`）的 3 条 `Release` 挪到**分支前**（`block[0]`），总数仍是 4，
`4 > 1` **照样绿**——而**短路路径从此会释放从未创建的值**，正是裁定 C 要禁的那一类。

**实现本身是对的**（审核 dump 的 TAC 块结构，见专项 §11.1）：

```text
block[1]  … Release×3 …      ← 右侧块：3 条释放全在这里
block[2]  LoadConst, Copy, Jump   ← 短路块：releases = 0 ✓
block[3]  Copy, RunReleasePlan, … ← 合流块：releases = 0 ✓
```

⇒ **要补的是判据，不是实现。**

### 3.2 冻结

补一条**位置**断言，上面 dump 出的形状是可判定的：

1. **TAC 侧**：**短路块的 `Release` 数必须为 0**（即 `logical.short` 对应的块内不得有 `Release`）；
   ——按现有 dump，`block[2]` 就是那个块，**这条现在就能写**；
2. **LLVM 侧**：右侧的 `xiao_runtime_value_release_` 必须出现在 `logical.right` 标签**之后**
   （不许出现在进入右侧块之前）；
3. 现有两条**总数**断言**保留**（它们覆盖「右侧完全没释放」与「右侧被提前求值」），
   位置断言是**加**，不是替。

### 3.3 最小实施步骤

1. 在 `logical_short_circuit_keeps_shared_release_plans` 里加 §3.2 的两条断言；
2. 跑 `cargo test -p xiao-driver --test c20_capability_probe`，确认**仍绿**（说明当前实现满足位置要求）；
3. 进 3.4 的变异。

### 3.4 最小验证步骤（**必须有一次变异**）

1. **构造一个"释放落到短路块"的变异**——在字节码降低器的短路块里临时插入一次对某个临时值的归还
   （`xiao-bytecode/src/lower/expr.rs::lower_logical`，短路块是 `short_block` 那段）；
2. 跑测试，**位置断言必须变红**（总数断言大概率仍绿——那正是 R-2 要补的差）；
3. **还原**并确认转绿；
4. **若该变异确实构造不出来**，**必须在实施记录里写明原因**，并保留一个能证明位置断言
   「至少会为某件事红」的替代证据——**不许**因为不好做就跳过负例
   （[20AB-FIX](20ab-fix-lifetime-and-removal-verification.md) 的先例：失败证据可以是编译失败，
   但**必须有**，且**加了没有要写明**）。

**实施记录（2026-10-14）**：正常降低与完整 `c20_capability_probe` 测试保持通过。
临时在 `lower_logical` 的 `short_block` 插入一条 `TacOp::Release` 后，测试在
「短路块不能释放未求值右侧的临时值」处失败（`test result: FAILED`）；总数断言尚未先行
失败，证明新增位置断言确实能捕捉该变异。变异已还原，`xiao-bytecode` 工作树无残留改动。

### 3.5 落点（R-1 与 R-2 合并列出）

```text
docs/DevDocs/20c-native-gate-residuals-fix.md            本文
tools/gates/run.sh                                       R-1：按 §2.2 择一改；§2.4 的一次性分支验证
core/rust/crates/xiao-driver/tests/c20_capability_probe.rs  R-2：加位置断言（§3.2）；§3.4 的变异临时点
core/rust/crates/xiao-bytecode/src/lower/expr.rs         R-2：变异只在此处临时进行，**验完还原**
docs/DevDocs/20c-native-gates-red-fix.md                 §5.2.1 与 §11.1.1 的实施结果回填；运行号
```

## 四、顺序与依赖

1. **R-1 先行**：它是 **push 门**上的遮蔽，比 R-2 更常触发；且 R-1 的验证要占一次性分支，
   做完了再动 R-2 更干净；
2. **R-2 次之**：它改的是测试断言，本机可自证（变异 → 红 → 还原 → 绿），**不需要 CI**；
3. **两条互不依赖**，可并行派工；**R-1 的验证与 R-2 的变异不许在同一批提交里**——
   混在一起就没法判断是哪一处让门禁变红的。

## 五、本批已经吃过亏、这次不许再犯的几条

1. **「加了参数」≠「跑到」**（§5.2）；**「脚本正常」≠「遮蔽治好」**（§2.6）；
2. **「断言存在」≠「断言能红」**——每条新判据都要有一次变异；
3. **证据要能对着原始输出逐字核**（审核自己就犯过一次：把 `RunReleasePlan` 写成 `ReleasePlan`）；
4. **比总数 ≠ 比位置**（R-2 的全部内容）；
5. **一次性分支做完就删**，`main` 不变红。

## 六、验收

1. **R-1**：`tools/gates/run.sh` 按所选方案改完；**一次性分支的红的运行号 + 还原后的绿的运行号都有**；
   红的那次日志里能同时看到「失败者之后的二进制仍出现 `Running` 行」与「静态步骤仍跑」；`main` 未变红；
2. **R-2**：位置断言已加（TAC 侧短路块 `Release` 数为 0、LLVM 侧右侧 release 在 `logical.right` 之后）；
   总数断言保留；**变异让位置断言变红**并已还原（或按 §3.4 第 4 条写明原因 + 替代证据）；
3. **回填**：专项 §5.2.1 与 §11.1.1 的实施结果与运行号；`main` 仍绿；
4. **通用**：`cargo test --workspace`、clippy、fmt、`bun run check` 全绿；提交标题带类型前缀、正文非空。

## 七、第三轮审核并入的三条（审 `f8dfc7c`…`43a127f`）

出处：2026-10-14 审核残余项收口那一批。**三条没有一条是红**——
R-1 的取证经审核**拉原始日志与 API 逐条复核**（见 §2.7），R-2 的位置断言经审核
**独立变异复现**（在短路块插一条 `Release` ⇒ `c20_capability_probe.rs:453` 精确变红）。

| # | 项 | 处置 |
| --- | --- | --- |
| **第三轮-1** | **绿运行与提交的对应不精确**：文档把绿运行 `38535786407` 挂在「提交 B `5b693cd`」上，而该运行的 `headSha` 是 `f5210a3`（= B + 步骤标签） | §2.7 **已改正**（含一条口径说明）；专项文档同步 |
| **第三轮-2** | **方案甲的清单宽度**：`bunx tsc --noEmit` 同属纯静态，却仍在 `cargo test -p xiao-driver` 之后 | §2.2 **已补注**；**追加项 R-1b 已做**，见下 |
| **第三轮-3** | **远端还留着 8 条历史一次性分支**：`codex/10j-cross-os-verification`、`codex/10k-runtime-trimming`、`codex/10z-ci-push-coverage-20261009`、`docs/10s-linux-evidence-20261007`、`docs/10t-linux-evidence-20261008`、`docs/10z-linux-window-evidence-20261010`、`docs/10z-performance-results-20261011`、`test/10r-native-mutation` | **已清（删前逐条核过）**，见下；本轮那条验证分支**早已删** ✓ |

**追加项（R-1b，低）——已做（2026-10-14）**：`bunx tsc --noEmit -p tsconfig.json`
已前置到 `bun run check:coverage` 之后、**两个 `cargo test` 之前**，
与 fmt / 文档门禁同组。**未另做取证**——§2.6 那套红/绿已经证明前移有效，
这一步是**把同一个理由用全**；`bash -n` 通过，本地整脚本跑通。
`bun test`（**动态套件**，非静态）**仍在 `cargo test -p xiao-driver` 之后**，按原判断另论。

**清理项（housekeeping，很低）——已做（2026-10-14）**：远端那 8 条历史一次性分支
**已全部删除**，`git ls-remote --heads origin` 现在**只剩 `main`**。
删前逐条核过：6 条未并入提交数为 **0**；`docs/10s-linux-evidence-20261007` 的那 1 笔
（`ce6dc57`）所改的两个文件**与 `main` 上的内容逐字相同**（`git diff --quiet` 退出码 `0`）；
`test/10r-native-mutation` 是「注入 + 移除」的一次性变异对，**本就该删**。

## 相关页面

- [20C-原生门禁修复专项](20c-native-gates-red-fix.md) —— **本文的前置**；§5.2.1（跨步骤遮蔽）、
  §11.1.1（位置断言）、§十二（「绿跑能证明什么、不能证明什么」那张表）
- [20C](20c-official-xiao-library.md) §2.4 —— 一次性分支做红/绿验证的纪律出处
- [10Z-驱动](10z-performance-driver.md) §2.7 D7 —— 「证据必须有执行点」，以及「遮蔽」那条教训
- [20AB-FIX](20ab-fix-lifetime-and-removal-verification.md) —— 失败证据可以是编译失败，但**必须有**
