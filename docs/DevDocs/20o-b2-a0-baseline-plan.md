# 20O-B2-A0. 工作快照与基线执行规划

> **这是 B2-A 的 A0 基线执行规划，不写产品函数、不执行 A1 依赖变更、不改 Cargo/Cargo.lock、intrinsic、ABI、诊断表或测试实现。**
> 规划入口为`dc0f05f0`后的当前提交；B2-A规划已由A-P1/A-P2/A-P3实际批准，A0已获准另批执行。
> 当前30 confirmed / 0 pending，F-D已签字、联合冻结已激活；产品仍未开工。
> A0只建立真实功能基线，不交付B2-A能力；A1的依赖/值身份和后续产品实现不在本页。

> **执行状态（2026-11-05）：A0 Windows 本地基线已通过，源码为 `d0e4a5fb9043aa6c93dce7892dbe1ef333d3b276`。**
> B1契约9/9、资源17/17、VM/Windows原生精确点各1项、workspace门禁与28/28守卫均有本轮实际记录。
> 文档门禁的两次依赖环境失败及实体复制后的成功均保留；有效收口只引用`closeout.json`，原始`result.json`仍为失败。Linux/macOS本地未验证。A1入口成立，但A1～A7及B2-A产品尚未开始，见§十与[完整证据](assets/20o-b2-a0/entry-20261105/README.md)。

## 一、入口与目的

| 项 | 当前状态 |
| --- | --- |
| 签字基线 | F-D三项实际批准；已签草案正文摘要`8bca4801…`；台账30/0 |
| B2-A规划 | A-P1/A-P2/A-P3三项实际批准；A0～A7获准另批实施 |
| 版本 | Runtime ABI 1.18、`CODEGEN_VERSION` 20、活动intrinsic最大32、`cmath` planned |
| 依赖 | F-7六包已确认；尚未解析、锁定或构建；A0不添加依赖 |
| B1 | 七库20/20功能与教程/registry verified；旧721条F10只作历史 |
| 产品 | 未开工；A0不写产品代码 |
| 历史债务 | 22D/22C、19/11/22保持独立，不由A0核销 |

A0的目的不是“证明B2-A已经可用”，而是确认在添加任意精度依赖和修改数值内核之前，当前源码、Runtime、工具链和B1功能基线可以被唯一识别、复跑和对账。

## 二、A0范围

### 2.1 要做

1. 记录源码提交、工作区状态、签名/台账/答复摘要、工具链和现有锁文件；
2. 只读确认ABI 1.18、生成器20、intrinsic最大32、`cmath`仍planned；
3. 在依赖变更前执行B1契约、资源、VM/原生精确点和workspace门禁；
4. 运行B1精确入口守卫，确认工作流仍要求恰好1 passed、28/28；
5. 使用唯一任务目录隔离Runtime/缓存/输出，明确`XIAO_STDLIB_ROOT`来自仓库；
6. 保存实际命令、平台、退出码、原始日志、失败样本和摘要；
7. 将A0失败与22D/22C、19/11/22历史债务分开归属。

### 2.2 明确不做

- 不添加`num-bigint`/`bigdecimal`或修改Cargo文件；
- 不运行A1依赖解析、构建、锁更新或A1～A7产品；
- 不启用`b1_f10`，不重跑或改写旧721条F10；
- 不修改B1产品、错误表、预算、工作流阈值或22D/22C断言；
- 不把B2-A规划批准写成产品已开工；
- 不用只读CI代替本地真实复跑。

## 三、A0开工闸门

执行前必须确认：

1. 当前HEAD及工作区干净；签名正文、30项台账、F-D/A-P回执摘要与已批准基线一致；
2. A0的源码/证据目录和唯一任务目录已创建，不覆盖历史证据；
3. `XIAO_STDLIB_ROOT`指向本仓库stdlib，原生缓存/输出不复用B1旧目录；
4. 当前平台工具链可运行，Rust/Bun/PowerShell版本被记录；不能运行的平台明确标未验证；
5. B1历史721条F10仍按历史身份读取，不进入A0测量；
6. 文档门禁和产品边界检查通过；若需要产品/工作流修改，先停止并回A0范围外。

## 四、执行步骤

### A0-1：快照与身份

1. 记录`git rev-parse HEAD`、分支、`git status --porcelain`、源码树摘要；
2. 记录已签草案正文、30项台账投影、F-D/A-P回执的SHA-256；
3. 记录`core/rust/Cargo.toml`、`core/rust/Cargo.lock`、`rust-toolchain.toml`摘要；
4. 只读核对ABI 1.18、生成器20、活动intrinsic最大32、ID100墓碑、`cmath` planned；
5. 记录Rust、Cargo、Bun、PowerShell、clang/linker和目标平台版本；
6. 保存到A0证据目录，不写产品文件。

退出：源码、签名、工具链和锁文件身份可唯一复算。

### A0-2：B1契约与资源基线

在依赖变更前运行B1现有功能基线，至少覆盖：

1. 七库20入口公开`std`/`stdlib`双路径、导出、移除/归档、普通包隔离和教程一致性；
2. 17项资源边界、上限+1、UTF-8首错、Base64长度/尾位、CSV字段边界；
3. 错误身份、输入不变和失败原子性；
4. 实际命令和完整日志按平台保存；不以旧报告替代本次执行。

建议实际入口（以仓库当前测试名为准，执行时必须记录实际命令）：

```text
cargo test --manifest-path core/rust/Cargo.toml -p xiao-driver --test 20o_b1_imp_contract
cargo test --manifest-path core/rust/Cargo.toml -p xiao-driver --test 20o_b1_imp_resources
```

退出：B1契约/资源基线在A0任务目录内实际通过，或失败被定位并保留原始输出；不修改产品来“修绿”。

### A0-3：VM与原生精确点

1. VM精确入口：

```text
cargo test --manifest-path core/rust/Cargo.toml -p xiao-driver --test 20o_b1_probe -- b1_capabilities_have_vm_golden_outputs --exact --nocapture
```

2. 原生精确入口按平台执行，断言恰好1 passed、0 failed、0 ignored；Windows/Linux/macOS各自记录实际目标、工具链、Runtime和输出根；
3. 原生入口：

```text
cargo test --manifest-path core/rust/Cargo.toml -p xiao-driver --test 20o_b1_probe -- --ignored b1_capabilities_match_native_golden_outputs --exact --nocapture
```

4. 不能运行的平台明确标“未验证”，不沿用B1旧原生可执行文件或旧结果；
5. 记录唯一任务目录、`XIAO_STDLIB_ROOT`、Runtime静态库摘要和产物路径。

退出：VM与已执行平台的B1精确点通过；未执行平台有明确缺口，不用旧结果补洞。

### A0-4：工作区与守卫

1. 运行现有workspace门禁：`bun run check`（docs/all、clippy、lock）；
2. 运行`docs/usedocs/layout`和`git diff --check`；
3. 运行B1精确入口守卫：

```text
pwsh -NoProfile -File tools/gates/b1-probe-guards.ps1 -BashPath <MSYS bash>
```

4. 守卫必须保持28/28，四条路径的精确目标不能被默认/ignored/重复输出绕过；
5. 记录实际平台与退出码；CI只作外部参考，不能替代本地复跑。

退出：workspace与守卫通过；失败不通过修改工作流阈值或删减场景处理。

### A0-5：A0收口

1. 汇总源码、签名、工具链、锁文件、B1功能、VM/原生、workspace和守卫证据；
2. 生成A0证据清单和SHA-256，保存实际命令、平台、退出码和原始日志；
3. 明确哪些平台已执行、哪些未验证；
4. 确认没有产品、Cargo、ABI、intrinsic、诊断表、测试或工作流改动；
5. 给出A1入口结论：基线成立则允许另批A1；否则停止并列出缺口。

退出：A0基线可对账，A1尚未开始，B2-A产品仍未开工。

## 五、证据矩阵

| 项 | 必存证据 |
| --- | --- |
| 源码/签名 | HEAD、工作区状态、草案正文、台账、F-D/A-P回执摘要 |
| 工具链/锁 | Rust/Bun/Pwsh/目标平台、Cargo.toml、Cargo.lock、rust-toolchain摘要 |
| 版本边界 | ABI 1.18、生成器20、intrinsic最大32、ID100、cmath planned |
| B1契约 | 20入口双路径、导出、移除/归档、普通包、教程一致性；实际命令和日志 |
| B1资源 | 17项资源、上限+1、UTF-8/Base64/CSV边界；失败原子性 |
| VM精确点 | 恰好1 passed、0 failed、0 ignored；实际目标与日志 |
| 原生精确点 | 各平台实际执行；未执行平台明确标注 |
| workspace | docs/usedocs/layout、clippy、lock、diff检查 |
| 守卫 | 28/28，四条路径、精确目标、防重复/零/多输出 |
| 隔离 | 唯一任务目录、XIAO_STDLIB_ROOT、Runtime/缓存/产物摘要，无旧可执行文件复用 |
| 边界 | 无产品/Cargo/ABI/intrinsic/诊断/测试/工作流改动 |

## 六、退出条件

1. A0源码、签名、工具链、锁文件身份唯一；
2. ABI 1.18、生成器20、intrinsic最大32、cmath planned保持不变；
3. B1契约、资源、VM/原生精确点和workspace门禁有实际执行记录；
4. B1精确入口守卫28/28；
5. 失败与22D/22C、19/11/22债务分开归属；
6. 未添加依赖、未写产品、未运行F10、未改工作流；
7. A0证据目录完整，摘要可复算；
8. 只有A1入口被打开，A1～A7仍未开始。

## 七、风险与禁止

- 用CI绿色或旧报告替代本地B1复跑；
- 因cmath planned导致默认审计拒绝，就误判B1基线失败；
- 为通过基线修改产品、测试、工作流或预算；
- 复用B1旧target/缓存/原生可执行文件；
- 运行`b1_f10`或旧721条并写成A0/B2-A当前成绩；
- 把“规划已批准”写成“A0已执行”或“产品已开工”；
- 把未执行平台写成通过。

## 八、交接

A0通过后下一入口是A1：依赖、适配层与值身份；A1才允许按已批准规划添加F-7六包、锁文件和033/034诊断，仍不进入B2-B/C。

新Agent先读：

1. [B2-A实施规划](20o-b2-a-implementation-plan.md)；
2. [B2数值基础规划](20o-b2-numeric-foundation-plan.md)；
3. [冻结草案](20o-b2-freeze-draft.md)；
4. [B1收口记录](20o-b1-closeout.md)与[B1 REM证据](20o-b1-rem-evidence.md)；
5. [B2-A审批与来源证据](assets/20o-b2-a/entry-20261105/README.md)。

## 九、落点

```text
docs/DevDocs/20o-b2-a0-baseline-plan.md                本规划
docs/DevDocs/20o-b2-a-implementation-plan.md           A0～A7总规划
docs/DevDocs/assets/20o-b2-a0/entry-20261105/          A0实际证据
core/rust/crates/xiao-driver/tests/20o_b1_imp_contract.rs B1契约
core/rust/crates/xiao-driver/tests/20o_b1_imp_resources.rs B1资源
core/rust/crates/xiao-driver/tests/20o_b1_probe.rs     VM/原生精确点
tools/gates/b1-probe-guards.ps1                        28/28守卫
```

A0只建立基线；A1依赖与值身份、A2～A7仍须另批执行和验收。

## 十、A0实际执行与交接（2026-11-05）

源码入口为`d0e4a5fb`，Git树为`593da1c97780120fc9c8aa980e6f7e803168d1c1`；开工时主工作区和独立工作树均干净。
唯一任务根为`D:\project\Xiao\.a0-baseline\20261105-2bc5a0f4c6c0`，源码、Cargo target、Xiao缓存和临时产物分别隔离。
`XIAO_STDLIB_ROOT`指向该工作树的`stdlib`，Runtime静态库从空target重新构建，未复用B1旧原生文件。
Rust/Cargo固定1.96.0，Bun1.4.0、PowerShell7.6.6、LLVM22.1.2；目标为`x86_64-pc-windows-msvc`。

| 入口 | 本轮实际结果 |
| --- | --- |
| B1契约 | 9 passed、0 failed、0 ignored；双导入路径、真实归档、导出/移除、普通包隔离与教程一致性 |
| B1资源 | 17 passed、0 failed、0 ignored；含最大合法输入、预算首错、UTF-8/Base64/CSV边界 |
| VM精确点 | `--exact --nocapture`，唯一汇总1 passed、0 failed、0 ignored |
| Windows原生精确点 | `--ignored --exact --nocapture`，唯一汇总1 passed、0 failed、0 ignored；七库O0/O3及FIX-B清理用例通过 |
| workspace | Clippy全workspace/all-targets、workspace check、benchmark check、锁文件差异检查均通过；Cargo全程`--locked` |
| docs/usedocs/layout/覆盖率 | 相同源码在工作树内实体复制TypeScript5.9.3后，`repo-check all`退出0；原有覆盖率warning保留 |
| B1计数守卫 | 28/28；是四条路径的模拟守卫，不是Linux/macOS产品执行证据 |
| 平台缺口 | Linux/macOS本地未验证；不以B1旧CI替代 |

首次文档门禁因依赖链接不能解析TypeScript退出2；补根依赖路径后的第一次重试又被目录门禁以`A0-LAYOUT-002`拒绝。
随后只把同一TypeScript5.9.3的132个文件逐字复制到工作树内部，移除两处临时junction，第三次门禁通过；没有修改产品、测试或门禁规则。
最初执行器的`result.json`如实保留`passed=false`，最终有效结论在`closeout.json`；失败日志没有覆盖或删减。
下游只能以[closeout.json](assets/20o-b2-a0/entry-20261105/closeout.json)引用有效收口，不能把初轮`result.json`写成通过或原始执行一次全绿。
源工作树实际文档门禁为[repo-all-local-copy.json](assets/20o-b2-a0/entry-20261105/repo-all-local-copy.json)，主仓库的[workspace-doc-gate.json](assets/20o-b2-a0/entry-20261105/workspace-doc-gate.json)仅为当时收口文档的补充检查，两者cwd与证据用途分别保留。

已签草案正文摘要仍为`8bca48018be3bf2cf38f615d7cf35d55a0ff9745f73e3c196161abd854d286ff`，30项台账投影、F-D/A-P回执均核对通过。
Windows检出草案为CRLF，签字核对使用已定的UTF-8/LF口径，并同时保存原始检出摘要。
ABI1.18、生成器20、intrinsic最大32及ID100墓碑未变，cmath仍planned；本批没有产品、Cargo/锁、诊断、测试、工作流或UseDocs状态改动。

**退出结论：A0本地基线成立，打开已批准范围内的另批A1入口；本轮止于A0，不启动A1。**
52个原生可执行文件的运行期间稳定摘要已记录，临时二进制由既有测试清理；这些摘要不是另一次产品执行。
B1旧721条F10未运行、仍为历史证据；22D/22C及19/11/22债务保持独立，A7仍需B2-A自身三平台证据。

2026-11-06补充复核见[引用与规范文本摘要](assets/20o-b2-a0/review-fix-20261106/README.md)：23个身份文件同时记录原始与UTF-8/LF规范摘要，源工作树、固定Git内容与主工作区规范内容一致，原归档保持原样。
该次重新查询确认`d88f68a7`的workspace CI `42048065769`成功；仅作为该提交的外部CI记录，不代替A0本地证据，也不代表后续补注文档提交的CI结论。
