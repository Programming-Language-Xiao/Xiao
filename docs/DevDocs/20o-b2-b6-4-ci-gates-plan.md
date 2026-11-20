# 20O-B2-B6-4. 三平台与 CI 工程门禁实施规划

> **2026-11-19，B6-4 受限交接，正式结果 `b6_4_passed=false`。** `original_passed=false`，`followup_passed=false`，`windows_coff_verified=false`，`22d_open=true`。修复快照的三平台功能和 Linux 工程门禁功能验证通过，Windows COFF 导出完整性未验证；严格口径下 `followup_passed=false`。Windows 22D 的三处间歇性释放差分待归因。B6-5 规划完成，未开始；[规划](20o-b2-b6-5-closeout-plan.md)。[交接状态](assets/20o-b2-b/entry-20261119-b6-4-followup/status.json) · [修复及复跑证据](assets/20o-b2-b/entry-20261119-b6-4-followup/README.md)
> 入口：B6-3 固定 `58176ff5` 的三平台边界证据（765 次语言执行、C ABI 11 项、B1 契约 9/9 与资源 18/18、B4 回归、4 条真实输出加 24 条派生负例）；B6-2 性能绑定 `79ea0375` 及其 source-boundary 证明；B6-1 仍为 `restricted/stage_passed=false`。
> B6-4 只做三平台执行、CI 与工程门禁证据；不重跑 B6-2 性能、不重跑 B6-3 全边界、不跑 721、不改产品/工作流阈值/ABI/registry。
> B6-1 的 Windows COFF 导出完整性未验证，Linux Clippy 当时未执行。B6-4 的 Linux workspace Clippy 已通过。

## 一、入口与范围

| 项 | 当前状态 |
| --- | --- |
| B6-3 | 固定 `58176ff5`，`boundary_passed=true`；三平台 VM/O0/O3 各 85 条、C ABI 11 项、B1 契约 9/9 与资源 18/18、B4 两别名；守卫回放为 4 条真实输出 + 24 条派生负例 |
| B6-2 | 固定 `79ea0375`，`perf_pass=true`；source-boundary 证明后续仅有 Rustdoc/加载器移位，新增性能样本 0 |
| B6-1 | `restricted/stage_passed=false`；Windows COFF `UnverifiedCoffExports`、Linux Clippy 未执行 |
| 产品基线 | `58176ff5`，ABI1.19/生成器22/接口1；到当前 HEAD 的产品、测试、工具和锁文件树无差异 |
| B6-4 执行身份 | 使用实际 CI `head_sha`；它与产品基线分开登记，不能把 `58176ff5` 写成 CI 运行提交 |
| 本批目标 | Linux 完整仓库门禁、三平台维护回归及其 CI run/job/attempt、B1 28/28 与平台身份 |
| 未做 | B6-2 性能重跑、B6-3 全边界重跑、B6-5 收口/B7 交接、registry/教程、旧721 |

B6-4 是 **工程门禁与 CI 证据批**，不是性能或边界验收批。B6-2 的 `perf_pass` 与 B6-3 的 `boundary_passed` 只按各自快照引用；B6-4 不重测性能、不把工程门禁绿色写成性能或三平台完整性通过。

### 1.1 要做

1. 固定产品基线 `58176ff5`，并记录每次 CI 的实际 `head_sha`、锁、工具链与目标；B6-2 `79ea0375` 和 B6-3 `58176ff5` 只作为各自阶段的独立证据引用；
2. Linux 执行 `workspace-gates.yml` 的完整仓库门禁，并通过 `maintenance-regression.yml` 记录 Rust workspace 默认测试；不要把 `workspace-gates` 单独称为完整 Rust workspace 测试；记录实际执行的 controlled-cargo、`check:clippy`、`check:lock` 命令与两份锁文件摘要，不以工作流名称代替；
3. 通过维护工作流的 Windows/Linux/macOS 原生回归取得三平台执行与 CI 身份；保存 run/job/attempt、工作流/脚本摘要、源码/Runtime/tool/target、日志与平台身份；Linux 默认测试的 `--skip b1_capabilities_have_vm_golden_outputs` 与随后的精确 B1 单跑必须分成两段记录，不得写成无跳过完整运行；
4. 采集 Linux VM 和 Windows/Linux/macOS 原生精确入口的四份实际 Cargo 输出。维护作业在精确入口前失败时，可从同一产品树的独立补验取证。将 24 项派生负例、本次维护工作流和脚本摘要一并保存；
5. push 路径过滤：按各工作流实际过滤条件和事件记录；未触发时不把其他 CI 记录当成覆盖证明；
6. 逐平台/后端/优化级别记录结果；失败与不可验证项保留，不得汇总为通过；
7. 证据落在 `assets/20o-b2-b/entry-20261119-b6-4/`，按 SHA-256 存储日志原件。定稿后执行一次文档门禁并记录本批 DevDocs 字节摘要；
8. B6-5 交接：汇总 CI/工程门禁/平台证据，列出失败/不可验证项，不启动 B6-5 或 721。

### 1.2 明确不做

- 不改产品/测试/工作流阈值/默认超时/预算/输入；只允许为证据补充不改变行为的身份、摘要和日志记录；不把 A6 的 40 分钟时限自动继承到 B6；
- 不改 ABI1.19、生成器22、接口1、intrinsic 1～48/ID100、registry/教程；
- 不重跑 B6-2 性能、不重跑 B6-3 全边界、不跑旧721、不启动 B6-5；
- 不覆盖 B6-1/B6-2/B6-3 历史证据，不把 B6-1 `restricted` 写成全通过；
- 不把工程门禁绿色写成三平台完整性、Clippy 全通过或性能通过；
- 不在 B6-4 内修产品；发现缺陷记录失败并退回所属阶段或另立修复批。

### 1.3 CI 入口与覆盖边界

| 工作流 | 实际覆盖 | 本批用途与限制 |
| --- | --- | --- |
| `workspace-gates.yml` | 仅 Linux `linux-workspace-gates`，执行 `bash tools/gates/run.sh`；包含 Rust 格式、research 测试/Clippy、`bun run check`、类型检查、benchmark 与 `xiao-driver` 测试 | 用于 Linux 仓库门禁；`bun run check` 才包含 `repo-check all`、workspace Clippy 与 `check:lock`。该工作流本身不执行完整 Rust workspace 测试，也不是三平台矩阵 |
| `maintenance-regression.yml` | Linux 默认 workspace 测试、Windows/Linux/macOS `native-regression`，另有 Windows 确定性体积与 Linux 安全作业 | 手动派发时 `b1_f10=false`、`b2_a_perf=false`；Linux 默认测试将 B1 精确用例单独运行，三平台原生回归分别提供真实输出。该工作流里的 `b1-probe-guards.ps1` 另运行 28 个模拟输入用例，不等于 4 真实 + 24 派生回放。用于本批三平台维护回归，不代表三平台 Clippy/fmt/repo-check |
| `platform-reproduction.yml` | 定时或手动的通用平台复现；矩阵含 Windows/Linux/macOS，Clippy 只在 Linux amd64 作业执行 | 可作补充环境记录，不替代上述 B6-4 门禁或维护回归；若引用，单列该工作流及其覆盖范围 |

`workspace-gates.yml` 的 push 路径包含 `docs/**`，本规划提交可触发该 Linux 门禁；`maintenance-regression.yml` 的 push 路径不包含 `docs/DevDocs/**`，应手动派发并保存 `event`、`ref`、`head_sha`。`platform-reproduction.yml` 没有 push 触发器。每个结论都绑定实际运行的 workflow、run、job、attempt 和 head SHA。

产品身份与执行身份分开记录：`58176ff5` 是 B6-3 固定产品基线，CI 的 `head_sha` 是 B6-4 实际源码/工作流提交。对比两者的产品、测试、工具和锁文件树；工作流文件及回执工具按 CI `head_sha` 摘要。保存 Rust/Bun/LLVM 版本、runner OS/架构、target triple、Cargo 锁摘要、`CARGO_TARGET_DIR`、实际 Runtime 文件 SHA-256。维护工作流使用 `rust-cache`，必须记录缓存动作和命中状态，不得把缓存构建称为冷构建；如需本机复跑，另用新任务根与独立 target。CI 的受控 Cargo 输出还需能证明 `--locked` 生效。

维护工作流的三平台原生矩阵是常规回归，不调用 B6-3 的专用 `prepare.py`/`run.py --mode all`，也不据此重建 B6-3 的 85 条矩阵结果。CI 用 run/job/attempt 标识执行；只有本机直接启动的命令要求记录进程 PID。

## 二、开工闸门

1. B6-3 审计与 B6-2 source-boundary 证明通过；B6-1 restricted 口径可读；
2. 当前产品 `58176ff5` 到 HEAD 的 `core/`、`stdlib/` 无产品差异；ABI1.19/生成器22/接口1；
3. Windows/Linux/macOS 维护 Runner、CI 权限与 run/job/attempt 可取得；macOS 无法调度时按 `unverifiable`；
4. `workspace-gates.yml` 的 Linux fmt/Clippy/`check:lock`/`repo-check` 入口可复现；B6-1 的 Linux Clippy `not_run` 作为历史事实保留，不替代 B6-4 本批门禁；
5. CI 每个 job 使用独立 Runner；维护工作流若恢复 Cargo 缓存，记录其状态与 target 路径，不标为冷构建；本机复跑另建唯一任务根；历史证据只读；
6. 不改工作流阈值/默认超时；若需要调整，先请星崽另行裁定。

### 2.1 开工单（2026-11-19）

1. 入口：B6-3 `58176ff5` 边界功能通过（765 次语言执行、每平台 C ABI 11 项、B1 契约 9/9 与资源 18/18、B4 两别名、4 条真实输出与 24 条派生负例）。引用 B6-2 `79ea0375` 性能和源码差异证明，以及 B6-1 `restricted/stage_passed=false`。产品基线 `58176ff5`，ABI 1.19、生成器 22、接口 1。开工前核对产品、测试、工具和锁文件。
2. 第一批：Linux `workspace-gates.yml`。保存 event/ref/head_sha/run/job/attempt、缓存动作与命中状态、实际 controlled-cargo/`check:clippy`/`check:lock` 命令、两份锁文件摘要；不得用工作流名称或 YAML 文本代替命令回执。
3. 第二批：手动派发 `maintenance-regression.yml`，`b1_f10=false`、`b2_a_perf=false`。分别记录 Linux `workspace-default-tests` 的默认 workspace 测试（含 `--skip b1_capabilities_have_vm_golden_outputs`）与随后的精确 B1 单跑；以及 Windows/Linux/macOS `native-regression` 三作业的 run/job/attempt、head_sha、缓存/环境/Runtime/tool/target。
4. 守卫：单独记录 `b1-probe-guards.ps1` 的 28 个模拟输入；另以 4 份真实 Cargo 输出执行 24 项派生负例回放。两类回执分开保存，均不代表产品矩阵。
5. 平台验证等级按 B6-3 原产物引用。Windows COFF 为 `UnverifiedCoffExports`，Linux ELF 与 macOS Mach-O 为 `Complete`。B6-4 另记录维护用例的实际产物校验结果。
6. 证据：新建 `assets/20o-b2-b/entry-20261119-b6-4/`；日志按 SHA-256 内容寻址；历史 B6-1/B6-2/B6-3 目录不改写；文档定稿后单次 docs 门禁。
7. 退出：工程门禁、三平台 CI、守卫回执齐全，失败/缺项明确，B6-5 入口明确；本批不启动 B6-5、不重跑 B6-2 性能或 B6-3 全边界、不跑721。

### 2.2 执行记录（2026-11-19）

首轮将 `5758aa51` 推送到独立 CI 分支，触发两条工作流并暴露出测试环境缺项。复跑快照 `97d5f288` 在两个 Linux 工程入口准备组件诊断所需的 LLVM 和 Runtime，在三个原生作业提供浮点比较的独占输出目录。手动复跑沿用原测试、门禁选择和时限。Linux 仓库门禁与默认 workspace 分别通过，B1 VM 精确用例随后单跑一次。

Linux 和 macOS 在浮点比较 `original-pow-eq-static-o0` 上遇到相同的 `rc` 组件拒绝，交 B4 LLVM Runtime 组件登记范围调查。Windows 浮点比较通过 62 例，随后在 22D `native-lock-wait-027` 释放顺序断言失败，交 22D 单列追踪。三个原生维护作业均为失败。

三平台临时测试可执行文件的字节身份登记为 `unverifiable`。原生日志记录用例结果和部分产物路径，Runtime 与工具摘要另有实测回执。[证据范围](assets/20o-b2-b/entry-20261119-b6-4/README.md#只读复核)

Linux 和 macOS 的 B1 原生精确入口在维护作业中未到达，由同一产品树单独补验。首次补验的工具链偏差已保留，最终四份实际输出均来自 Rust 1.96.0。28 项模拟守卫与 4 份真实输出、24 项派生负例回放均通过。[完整作业编号、原始日志与只读复核](assets/20o-b2-b/entry-20261119-b6-4/README.md)

### 2.3 后续修复与复跑（2026-11-19）

B4 在 `9d75ab1a` 按手工维护的 ABI 依赖表展开可达入口的组件。CI 执行提交 `e1191445` 的三平台原生维护、Linux 默认 workspace 和工程门禁各通过一轮。Windows COFF 内部符号完整性未验证。Windows 原作业失败于 `native-lock-wait-027`，本机完整入口另在 `xcon-bounded-string-success` 和 `spawn-join-str-multiargument` 失败。三个释放差分均归 22D 追踪。[修复与复跑证据](assets/20o-b2-b/entry-20261119-b6-4-followup/README.md)

## 三、分批执行

### B6-4A：固定源码与工程门禁

1. 固定产品基线 `58176ff5` 和实际 CI `head_sha`，复核产品、测试、工具及锁文件树无漂移；
2. 在 Linux 运行 `workspace-gates.yml`，由 `maintenance-regression.yml` 的 `workspace-default-tests` 提供 Rust workspace 默认测试回执；分别登记工作流实际执行的命令、跳过项与精确补跑项；
3. 记录 run/job/attempt、每项命令、运行前后源码/锁摘要、工具/target/Runtime 身份、缓存状态与原始日志；CI 日志按 SHA-256 内容寻址；
4. 保存失败与重试；若 Linux CI 的 Clippy 未执行或门禁缺失，标记本批缺项，不得用 B6-1 历史 `not_run` 代替本批结果。

退出：工程门禁逐项有回执，失败/缺项明确。

### B6-4B：三平台执行与 CI

1. 手动派发 `maintenance-regression.yml`，确认两个性能输入均为 false；保存 Linux VM B1 精确测试和 Windows/Linux/macOS 原生 B1 精确测试的四份实际输出，以及三平台 `native-regression` 的 run/job/attempt；缺少任一平台时标记 `unverifiable`；
2. 保存工作流/脚本/实际 CI head/Runtime/tool/target 摘要、缓存状态与平台身份；如现有日志没有 Runtime SHA，只追加不改变执行行为的摘要记录；
3. 分平台记录原生回归结果与既有 skip 项；该矩阵不声称在 Windows/macOS 执行了完整仓库 Clippy、fmt 或 repo-check；
4. 对照两个工作流各自的 push 过滤规则，按实际事件记录是否触发。

退出：三平台身份与结果可复核，缺平台/失败保留。

### B6-4C：B1 守卫与平台完整性

1. 复核维护工作流的 28 个模拟输入守卫，并另以 B6-4 自身脚本绑定当前 CI `head_sha` 的 workflow blob，对四份真实 B1 输出执行 24 项派生负例回放；保存两类回执，不将模拟输入描述为真实输出；
2. 按 B6-3 原产物引用组件验证等级，并记录 B6-4 维护用例实际触发的组件校验；
3. 保存本批平台/工具/Runtime 摘要与日志，并将 B6-3 的组件等级明确标为历史快照事实。

退出：守卫与平台完整性口径准确，未验证项不写成通过。

### B6-4D：证据与 B6-5 交接

1. 生成 B6-4 证据清单，内容寻址；
2. 汇总工程门禁、CI、平台结果、失败/不可验证项；
3. 形成 B6-5 入口：B6 整批收口；本批不启动 B6-5；
4. 历史证据不改写；B7/B2-C 仍后置。

退出：B6-4 证据完整；B6-5 入口明确。

## 四、证据矩阵

| 层 | 必测 |
| --- | --- |
| 工程门禁 | Linux `workspace-gates` 的 workspace Clippy、fmt、`check:lock`、`repo-check`，以及维护工作流中独立的 workspace 默认测试 |
| CI | Linux 仓库门禁与 Windows/Linux/macOS 维护回归各自的 run/job/attempt、工作流摘要及 push 过滤事实 |
| 平台 | Windows/Linux/macOS 原生维护回归实际执行或 `unverifiable`；COFF/ELF/Mach-O 等级只按 B6-3 快照引用 |
| 守卫 | `b1-probe-guards.ps1` 的 28 个模拟输入用例；另有 4 份实际 CI 输出 + 24 项派生负例回放；工作流自检不代替产品测试 |
| 身份 | 产品基线、CI head、锁/Runtime/tool/target/缓存/执行文件/日志摘要；本机直接执行另记 PID |
| 边界 | 无产品/工作流阈值/ABI/registry/721 变更 |

## 五、退出条件

1. B6-4A 工程门禁逐项有回执，失败/缺项明确；
2. 三平台维护回归实际执行或 `unverifiable` 明确，Linux 仓库门禁与各 CI 身份可核对；
3. 28/28 守卫与平台验证等级记录准确；
4. 无产品/测试/工作流阈值/ABI/registry/721 变更；
5. 历史 B6-1/B6-2/B6-3 证据未改写；
6. 未把 Linux 专属仓库门禁写成三平台门禁；B6-1 的 Windows COFF 未验证与 Linux Clippy 未执行继续按历史事实表述，不混入 B6-4 本批结果；
7. B6-5 入口明确；B6-5 规划完成、执行未开始；B7/B2-C 未开始。

`b6_4_passed` 要求原批通过、后续复验通过且 22D 已关闭；`followup_passed` 要求 Linux 工程门禁和三平台完整验证通过。按 §2.1 的平台口径，Windows 回归可登记功能通过及 `UnverifiedCoffExports`，但该等级不满足 `followup_passed` 或 B6-4 通过条件；COFF 完整性以 `windows_coff_verified` 单列，三平台产物完整通过要求各平台实际达到 `Complete`。受限交接的平台缺项纳入 B2-C 补验。[B2-C 开工闸门](20o-b2-c-final-closeout-plan.md#二开工闸门)

## 六、风险与禁止

- 把工程门禁绿色写成三平台完整性/性能通过；
- 修改工作流阈值/默认超时或继承 A6 时限补绿；
- 把 B6-1/B6-2/B6-3 快照混成同一测量身份；
- 忽略 Linux Clippy/Windows COFF 缺项；
- 覆盖历史证据或启动 B6-5/721；
- 在 B6-4 内修产品。

## 七、交接

B6-5 按 `b6_4_passed=false`、`original_passed=false`、`followup_passed=false`、`windows_coff_verified=false`、`22d_open=true` 接收 B6-4，并将 22D 间歇性释放差分未关闭的状态交接 B7。B6-1 按 `restricted` 引用，B6-2 性能绑定 `79ea0375`，B6-3 边界绑定 `58176ff5`。原 B6-4 门禁绑定 `97d5f288`，B1 补验绑定 `907c4cb3`，组件修复后 CI 复验绑定 `e1191445`。Linux 164 个导出的对账与 Windows 定向 Clippy 补验绑定 `780398e5`。[交接状态](assets/20o-b2-b/entry-20261119-b6-4-followup/status.json) · [本机补验归档](assets/20o-b2-b/entry-20261119-b6-4-followup/local-checks.zip) · [B6-5 收口规划](20o-b2-b6-5-closeout-plan.md)

新 Agent 先读：

1. [B6 资源性能规划](20o-b2-b6-resource-performance-plan.md)；
2. [B6-3 实测](assets/20o-b2-b/entry-20261118-b6-3/README.md)；
3. [B6-2 实测与自审](assets/20o-b2-b/entry-20261118-b6-2/README.md)；
4. [B6-1 macOS 专项](assets/20o-b2-b/entry-20261118-b6-1-macos/README.md)；
5. [官方库总纲](20o-official-library-program.md)。

## 八、落点

```text
core/rust/crates/xiao-*/                               工程门禁
.github/workflows/                                     CI run/job/attempt
tools/gates/                                            维护工作流/守卫脚本
docs/DevDocs/assets/20o-b2-b/entry-20261119-b6-4/      B6-4 证据
```

本页只做 B6-4；B6-5 规划完成、执行未开始；B7 与 B2-C 仍后置。
