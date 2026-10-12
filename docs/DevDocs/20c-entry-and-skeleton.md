# 20C-入口. 20C 入口准备：骨架、清单同笔与首批模块的前置

> **背景**：10 系列已关闭，[10R-实现](10r-static-overflow-implementation.md) 的 D1/D2/D4 已落地
> （ABI **1.12**），[10R-收尾](10r-closeout.md) 的实施也已完成大半——**但它的出口条件还没满足**
> （真实 push 运行号、Linux/macOS 受控原生复跑、F-3、产物级重复构建都在等外部执行点）。
> 同时审核在收尾实施里查出 **P-3：执行点仍然是枚举式，差集 5 个**（其中 `15e_ci_gated` 里
> 正住着「两次构建逐字节相同」那条证据）。
>
> **一句话概括**：**先把 P-3 与 F-3 的登记修掉，再按 20C 的守卫把入口四项做成骨架**——
> 本批**不写**库模块；`text`/`path` 的首批实现是下一批。
>
> 状态：**实施中（2026-10-12）**。P-3 接线、N-1/N-2/N-3 与依赖负例已落地；Windows 受控原生差分已于 2026-10-13 通过，Linux/macOS 复跑、真实 push 运行号、F-3 红/绿证据和产物级重复构建仍按 §三 如实登记。

## 一、Agent 交接上下文

### 1.1 接手前提

1. [20C-冻结](20c-freeze-and-10-series-closeout.md) §3.6/§3.8 —— **落点守卫**与**三处枚举清单**（本批主体）；
2. [20C](20c-official-xiao-library.md) §3.2.1 —— **六条冻结**（首批为 `text`/`path`；发布形式=源码；
   身份字段；不进依赖求解要有**可检查**落点）；
3. [10R-收尾](10r-closeout.md) —— **上一批**；其 §五 验收未满足项见本文 §三；
4. [10R-实现](10r-static-overflow-implementation.md) §五 D6 —— 顺序口径（本批处在 `(4)` 之后的入口段）；
5. [10Z-驱动](10z-performance-driver.md) §2.7 **D7** —— 「证据必须有执行点」；§2.1 P-3 的判据；
6. [00A](00a-project-layout.md) §顶层目录与 crate 表、`tools/repo-check/repository.manifest.json`
   —— **三处权威清单**的出处（改动必须同笔）；
7. [11A](11a-environments-and-packages.md) —— 官方库**不进普通依赖求解**的边界；§四 4.4 的负例依据。

### 1.2 现状盘点（2026-10-12，审核实测）

**表 A：10R-收尾 的未满足出口条件**（并列推进，见 §三）

| 项 | 卡在哪里 | 归属 |
| --- | --- | --- |
| P-2 的真实 push 运行号 | **需要一次真推送**（本机无法证明 CI 触发） | 星崽推送 / 一次性分支 |
| F-3 变红/转绿 | 同上 | 星崽推送 |
| 10Z-D 的行为面（受控原生差分、失败清理、**一次故意变异的红色证据**） | Windows 受控原生全量 ignored 门禁已通过，覆盖 VM/LLVM 动态文本、非文本、错误传播和释放轨迹；Linux/macOS 复跑与故意变异红色证据仍需外部执行点 | 10R 后续 |
| 产物级**两次构建逐字节相同**（[10R-收尾](10r-closeout.md) §2.5） | Windows 本机 workspace ignored 已通过 15E 通用 Runtime 产物逐字节比较；10R 指定的 D1/D2 后静态数值剖面产物仍待实测，真实 push 运行号也待补 | 10R-收尾 |
| O-1（19D 是否受影响） | 已判「**数据不足**」并留痕；要判「不受影响」需要**受控链接/性能取数** | 19D |

**表 B：P-3 改造前的权威/清单/差集基线**（审核逐行核，`crates/*/tests/*.rs` 里带 `#[ignore]` 的集成测试目标）

| | 内容 |
| --- | --- |
| **权威（11 个）** | `xiao-codegen-llvm/tests/`：`15e_ci_gated`、`n0_a`、`n0_b_dynamic`、`n0_c_errors`；`xiao-driver/tests/`：`c20_capability_probe`、`d19a_differential`、`n0_a_native_driver`、`n0_b_dynamic_native`、`native_benchmark_probe`；`xiao-package/tests/e3c_remote`；`xiao-runtime/tests/n0_c2_error_boundary` |
| **CI 已覆盖（6 个）** | `c20_capability_probe`、`d19a_differential`、`n0_a_native_driver`、`n0_b_dynamic_native`、`native_benchmark_probe`、**`n0_a`**（本批新加） |
| **差集（5 个）** | `n0_b_dynamic`（**该补**，`XIAO_LLVM_AS` 已有）、`n0_c_errors`（**该补**，同上）、`15e_ci_gated`（**接不上**：需要 `XIAO_STRIP`，两个原生作业**都没设置它**——逐行核过）、`e3c_remote`（理由正当：「CI 不依赖外部解析服务」）、`n0_c2_error_boundary`（理由正当：「由另一测试在子进程内调用」） |

**实施后的当前口径**：Windows/Linux 原生作业已统一执行
`cargo test --manifest-path core/rust/Cargo.toml --workspace -- --ignored`，并设置
`XIAO_STRIP`；`15e_ci_gated` 因而进入 push 门。两个平台都排除
`dns_resolution_failure_is_unavailable`（依赖外部 DNS）与 `platform_failure_child`
（由父测试在子进程中调用）；Windows 另排除终端握手和 O0–O3 调试执行，因为 runner 无交互终端；
Linux 已安装 Xvfb/xterm 并设置诊断程序，实际执行这两项。前两项的测试文件归属分别是
`xiao-package/e3c_remote` 与 `xiao-runtime/n0_c2_error_boundary`。工作流源码已核对，真实 push 运行号待星崽执行。

**表 C：20C 入口四项与其守卫**

| # | 入口项 | 依据 | 现状 |
| --- | --- | --- | --- |
| 1 | N-1 三处清单与目录/crate **同笔落地** | [20C-冻结](20c-freeze-and-10-series-closeout.md) §3.8；**实测**：拆两笔会让中间那笔直接红（`A0-MANIFEST-002` + `A0-LAYOUT-004`） | 已落地：`stdlib/`、crate、`codeRoots`、`rust.members`、00A 同笔 |
| 2 | N-2 `.xiao` 是否纳入 `sourceExtensions` | §3.8；不纳入则官方库源码**不被源目录发现**、README 门禁管不到 | 已裁定甲并通过实测：纳入 `.xiao`，两个既有目录补齐登记信息 |
| 3 | N-3 身份块与 `CompatibilityVersions` 的关系 | §3.4；是**并列新块**还是**扩展公共结构** | 已落地并列 `StandardLibraryIdentity`，现有结构不变 |
| 4 | **依赖求解负例** | [20C](20c-official-xiao-library.md) §3.2.1 第 5 条：**必须有负例**证明普通依赖求解不会解析官方库 | 已落地：`e3d_solver` 负例返回 `X05-PACKAGE-006` |

### 1.3 本批边界

| 子任务 | 内容 | 本批 |
| --- | --- | --- |
| **P-3 修补** | 执行点差集：改用 glob 口径 + 排除清单 | **做，排第一**（§二 2.1） |
| **F-3 措辞修补** | `00-decisions.md` 那行的「本批」指代不明 | **做**（§二 2.2） |
| 10R 欠项 | 表 A 五项 | **并列推进**，不阻塞（§三） |
| N-1 骨架与清单同笔 | `stdlib/` + `xiao-standard-library` + 三处清单 | **做**（§四 4.1） |
| N-2 / N-3 | `.xiao` 发现规则；身份块关系 | **做**（§四 4.2、4.3） |
| 依赖求解负例 | 可检查的隔离证据 | **做**（§四 4.4） |
| `text`/`path` 模块实现 | 首批官方库源码 | **不做**——下一批（§四 4.5 的守卫） |
| 静态 `catch`/`finally` 清理链 | —— | **不做**（[10R-实现](10r-static-overflow-implementation.md) D7 另立） |

## 二、并进来的两条修补（先做）

### 2.1 **P-3：执行点差集——别再枚举，换成 glob + 排除清单**

**事实**（表 B 的改造前基线）：上一批把 `n0_a` 加进两处原生作业，但同 crate 兄弟和
`xiao-package`、`xiao-runtime` 里的目标当时都没有进入 push 执行点；其中

- `n0_b_dynamic`、`n0_c_errors` 的依赖（`XIAO_LLVM_AS` / 真实 clang）**两个原生作业当时都已具备**，
  所以应接入；本批已由 workspace ignored 执行；
- `15e_ci_gated` 需要 **`XIAO_STRIP`**；本批已在 Windows 的 MSYS2 LLVM 与 Linux 的 LLVM
  中探测并设置 `llvm-strip`，所以它不再属于差集；
- `e3c_remote` 与 `n0_c2_error_boundary` 的排除理由已写进工作流注释；交互式诊断窗口仅 Windows job 排除，
  Linux job 已在 Xvfb/xterm 环境执行，避免把平台不具备的输入误写成通用排除。

**为什么不能再枚举**：这一次是「加一个目标」，下一个测试文件一进来又漏一次——
本仓库在同类上已经栽过（crate 过滤漏 10 个、`tests/fixtures/**` 不在任何 paths、G-1）。

**冻结**：

1. **两处原生作业改用 `--workspace -- --ignored` + 排除清单**；
   `tools/platform-reproduction/reproduce.sh:132` **已经在用这个口径** ⇒ 可行性与耗时都有先例；
2. **排除清单每项写理由**：外部 DNS 与平台异常子进程两平台排除，交互式诊断窗口只在无终端的
   Windows 排除；Linux 通过 Xvfb/xterm 执行。`15e_ci_gated` 选择**补环境**，两处原生作业均设置
   `XIAO_STRIP`，并由 `--workspace -- --ignored` 执行；
3. **无论哪条路，都要回答 §2.2 第 2 条**（那条可重复性证据的执行点）；
4. 改完**按 D7 第 3 条验证**：破坏一个断言 → 推送确认门禁**变红** → 恢复确认**转绿**，运行号待外部推送后回填；
5. 验证提交**只碰工作流**、不顺手带 `docs/**`，且**确认分支相对 `origin/main` 的改动只有该路径**。

### 2.2 **F-3 的登记措辞，与「那条可重复性证据住在哪」**

**事实**：`00-decisions.md` 的 F-3 已改为明确归属 `10R-收尾待执行`，卡点是一次真实推送；
而 [10R-收尾](10r-closeout.md) §2.5 要的「两次构建逐字节相同」证据住在
`15e_ci_gated::real_artifact_reproducibility_is_byte_comparable`，本批已把它接入两平台 workspace ignored 门禁。

**冻结**：

1. 把 F-3 那行固定为**`10R-收尾待执行`**，并保留卡点（需要一次真推送）；
2. 在 [10R-收尾](10r-closeout.md) §2.5 **写明证据落在哪个测试/哪条命令上**，
   并指向 §2.1 的处置结论（补环境或显式排除）——**不许**让一条验收的落点查不到；
3. 这两处都是**一行级**改动，**不许**顺手带其它内容。

## 三、10R-收尾 的未完成项（并列推进，不阻塞本批）

**冻结**：

1. 表 A 的五项**如实保留**，并在 [10R-收尾](10r-closeout.md) 的状态行里与事实一致；
   **不许**因为「下一批开工了」就写成已完成；
2. **谁做什么写清**：P-2 与 F-3 的运行号**等星崽推送**（一次性分支，纪律见 [10R-收尾](10r-closeout.md) §2.1 第 3 条）；
   10Z-D 的 Linux/macOS 复跑与变异红色证据仍待外部执行点；O-1 的最终判定**等受控取数**；
3. **O-1 的「数据不足」要留痕**（19D 已登记）：**何时能判**要写清——即「受控链接/性能取数到位之后」，
   否则它会变成一个永远悬着的结论。

## 四、20C 入口四项（本批主体）

### 4.1 **N-1：骨架与三处清单同笔落地**

**事实**：落点是**顶层 `stdlib/`** + **crate `core/rust/crates/xiao-standard-library`**；
三处权威清单分别是 `repository.manifest.json` 的 `codeRoots`（现为 `core/rust,cli/ts,tools,tests`）、
[00A](00a-project-layout.md) 的顶层目录块、以及同文件的 `rust.members`（**23 项**）。
**实测**：只补 `codeRoots` 而不建目录 → `A0-MANIFEST-002` + `A0-LAYOUT-004`，退出码 1。

**实施结果（2026-10-12）**：三处清单与目录已在同一工作树落地：`codeRoots` 从 4 项变为 5 项，
`rust.members` 从 23 项变为 24 项，00A 顶层目录从 7 项变为 8 项。`xiao-standard-library` 只有
带 crate 文档的 `src/lib.rs`，没有 `text`/`path` 模块、普通依赖配置或 `.xar` 字段；
`cargo test -p xiao-standard-library` 是其最小编译执行点。

**冻结**：

1. **同一提交**里完成：`codeRoots` 加 `stdlib/`、00A 顶层目录块加 `stdlib/`（含 README 要求与**工程期**归属）、
   `rust.members` 加 `xiao-standard-library`、建 `stdlib/` 与 crate 骨架（含各自 README）；
   **提交树中清单与目录必须同时存在**；
2. **提交说明里写出清单差集**（权威 23 → 24，`codeRoots` 4 → 5，00A 顶层目录 7 → 8）；
3. crate 骨架**只放最小可编译内容**：**不许**在本批写库模块、不许注册进普通依赖求解、
   不许写 `.xar` 兼容字段；
4. 骨架的 README 要写清：**工程期 20C**、**官方库身份**、**不参与普通依赖求解**、**负责与不负责**。

### 4.2 **N-2：`.xiao` 与 `sourceExtensions`（先实测，再二选一）**

**事实**：`repository.manifest.json` 已将 `.xiao` 纳入 `sourceExtensions`；
`layout.ts` 通过同一套 `codeRoots` 发现规则处理它，目录 README 和 `module-registry.json` 登记仍是硬门槛。

**实测表（方案甲）**：在纳入 `.xiao` 的临时试算和正式清单检查中，9 个文件均通过目录门禁。

| 目录 | `.xiao` 文件数 | README/交接信息 | module-registry 登记 |
| --- | ---: | --- | --- |
| `tests/fixtures/20c/` | 5 | 通过；补充 `工程期：20C` | `tests.xiao-20c-and-benchmarks` |
| `tests/benchmarks/programs/` | 4 | 通过；原有 09R3 信息保留 | `tests.xiao-20c-and-benchmarks` |

**结论：选择甲**。官方库未来的 `.xiao` 源码与现有夹具共用发现、README 和登记门禁；
官方模块的同步要求由标准库模块登记项的 `code`/`tests`/`usedocs` 路径检查兑现。

**冻结**：

1. 方案甲的试算和正式 `bun tools/repo-check/src/cli.ts layout` 均通过；
2. 官方模块登记项必须同时列出源码根、crate、UseDocs、DevDocs 和兼容身份路径；
3. 若后续新增 `.xiao` 目录导致 A0-LAYOUT-001 或 A0-LAYOUT-002，补齐 README/登记，不降低门禁。

这一同步契约由 repo-check 的 `A0-LAYOUT-005` 执行：每个包含 `.xiao` 源码的
`stdlib/**` 目录必须精确出现在 `module-registry.json` 的模块 `code` 中，且同一登记项
必须列出至少一个 UseDocs、DevDocs 和含 `compatibility` 的代码落点；缺一项即失败。

### 4.3 **N-3：标准库身份块与 `CompatibilityVersions` 的关系要写明**

**事实**：现有 `CompatibilityVersions`（`xiao-driver/src/compatibility.rs:167`）是
`language: String`、`ir: u32`、`xiaoc/index_schema/xar`、**`runtime_abi: u64`（单个编码版本）**、`target`；
冻结的身份字段是 `language_version`、`stdlib_interface_version`、`runtime_abi_major`、`runtime_abi_minor`、`target`。

**实施结论**：采用**并列新块**。`xiao-driver` 新增公开的
`StandardLibraryIdentity` 与 `current_standard_library_identity()`；既有
`CompatibilityVersions`、19 的兼容矩阵、发布报告、文档生成和
`current_compatibility_versions()` 均保持原有字段和消费者不变。

**冻结**：

1. `CompatibilityVersions` 不动；新块字段按冻结命名提供，当前接口版本为 `1`；
2. 身份块可序列化、可比较，供构建前检查和后续产物元数据接线；`.xar` 消费仍延后 20E；
3. `runtime_abi_major/minor` 直接读取 `xiao-runtime-abi::ABI_MAJOR_VERSION` /
   `ABI_MINOR_VERSION`，不复制 ABI 字面量。

### 4.4 **依赖求解负例（这条是「可检查落点」的兑现）**

**冻结**：

1. 官方库由**工具链专用加载器/命名空间**提供，**不写入 `PackageRegistry` 的普通依赖图**；
2. **必须有一条负例**（可运行的用例，不是文档声明）证明：把官方库当**普通依赖**解析会**失败**，
   或它**不出现在依赖求解结果里**——二选一，并写明选了哪种与为什么；
3. 该负例要有**执行点**（见 §2.1 的口径）——**新加的测试同样要问「push 时谁跑它」**。

**实施结果**：`xiao-package::solve_dependencies` 保留包名
`xiao-standard-library`，普通依赖请求稳定返回 `X05-PACKAGE-006`；
`core/rust/crates/xiao-package/tests/d1_package.rs` 验证本地路径解析拒绝该身份，
`e3d_solver.rs` 覆盖索引直接和传递依赖；所有负例均验证官方库不进入普通依赖图。
这些测试属于 Rust workspace 默认测试，`workspace-default-tests` 在 push/PR 路径变更时执行。

### 4.5 **守卫：四项完成之前不写库代码**

**冻结**：

1. `stdlib/` 只允许有 **README 与最小骨架**；`text`/`path` 的**模块实现**是下一批；
2. **不许**在本批：把探测夹具注册成普通依赖、把当前探测结果写进 `.xar` 兼容字段、
   为了让骨架编译而放宽任何门禁；
3. 首批模块开工时，必须**逐条引用** [20C](20c-official-xiao-library.md) §3.2.1 的六条冻结与
   §3.1 的探测边界（`X02-TYPE-006`、`X03-TYPE-003` 都是**既定边界**，不是待修缺陷）。

## 五、落点

```text
.github/workflows/maintenance-regression.yml            §2.1 P-3：两处原生作业改 --workspace -- --ignored + 排除清单
tools/repo-check/repository.manifest.json               §4.1 codeRoots/members；§4.2 sourceExtensions（若走甲）
docs/DevDocs/00a-project-layout.md                      §4.1 顶层目录块 + crate 表（含工程期）
stdlib/、core/rust/crates/xiao-standard-library/         §4.1 骨架与 README（最小可编译）
core/rust/crates/xiao-driver/src/compatibility.rs       §4.3 身份块（或并列新块）
core/rust/crates/*/tests/                               §4.4 依赖求解负例（要有执行点）
core/rust/crates/xiao-codegen-llvm/src/dynamic/expression.rs  D4：错误参数直接借用现有槽，避免重复 Runtime 句柄复制
core/rust/crates/xiao-vm/src/semantics/exec.rs           D4：非文本错误参数补齐构造表达式 span，与原生一致
core/rust/crates/xiao-driver/tests/n0_a_native_driver.rs D4：动态参数、错误位置与释放轨迹的受控原生差分
docs/DevDocs/00-decisions.md                            §2.2 F-3 那行的归属与卡点
docs/DevDocs/10r-closeout.md                            §2.2 证据落点；§三 未完成项的如实状态
docs/DevDocs/20c-official-xiao-library.md               §4.2/§4.3 的结论回写（冻结结果留在 §3.2.1）
```

## 六、验收

1. **P-3**：两处原生作业都改成 workspace ignored 口径；**排除清单每项有理由**；
   `15e_ci_gated` 要么被跑到、要么有**显式排除 + 它在哪里被跑到**；
   破坏断言 → 红 → 恢复 → 绿的运行号各一次；改动面干净（只碰工作流）；
2. **F-3 与证据落点**：`00-decisions.md` 那行有明确归属与卡点；
   [10R-收尾](10r-closeout.md) §2.5 写明证据落在哪个测试/命令上；
3. **N-1**：清单与目录**同笔**；提交说明有差集；骨架最小且不含库模块；
   **没有**注册进普通依赖求解、**没有**写 `.xar` 兼容字段；
4. **N-2**：有**实测表**（既有 9 个 `.xiao` 文件的真实结果）+ 明确的甲乙结论；
   同步要求**可被门禁检查**；
5. **N-3**：并列还是扩展**写明**；若扩展则列出受影响消费者；身份里的 ABI 主/次来自常量；
6. **依赖求解负例**：**可运行**且有执行点；
7. **10R 欠项**：五项状态与事实一致（**没有**被写成已完成）；
8. **通用**：`cargo test --workspace`、clippy、fmt、`bun test`、`bun run check`、`bunx tsc --noEmit` 全绿；
   提交标题带类型前缀、正文非空。

## 七、已裁定与待补证据

1. **P-3 的 `15e_ci_gated`**：已按星崽裁定补 `XIAO_STRIP`，两平台工作流采用 workspace ignored；待一次真实 push 回填红/绿运行号。
2. **N-2**：已按星崽裁定选择甲，`.xiao` 纳入通用发现；9 个既有文件的两个目录已实测通过。
3. **N-3**：已按星崽裁定采用并列 `StandardLibraryIdentity`，`CompatibilityVersions` 保持不变。
4. **剩余外部证据**：10R 的 Linux/macOS 受控原生复跑、F-3 红/绿、产物级重复构建和 O-1 受控取数仍按表 A 登记，未冒充完成。

## 相关页面

- [20C-冻结. 10 系列关闭与 20C 六条冻结](20c-freeze-and-10-series-closeout.md) —— §3.6/§3.8 的落点守卫与三处清单
- [20C. 官方 Xiao 库：能力探测、模块清单与兼容身份](20c-official-xiao-library.md) —— §3.2.1 六条冻结与 §3.1 探测边界
- [10R-收尾. 静态数值路径的收尾、证据接线与 20C 入口准备](10r-closeout.md) —— 上一批；其未完成项见本文 §三
- [10R-实现. 静态失败块接入可恢复错误通道](10r-static-overflow-implementation.md) —— D1–D7 的裁定与 ABI 1.12
- [10Z-驱动. 性能对照的计时驱动器](10z-performance-driver.md) §2.7 —— D7「证据必须有执行点」；P-3 的判据
- [00A. 工程框架与目录布局](00a-project-layout.md) —— 顶层目录与 crate 表的权威
- [11A. 虚拟环境与包管理](11a-environments-and-packages.md) —— 官方库不进普通依赖求解的边界
- [00. 决策基线](00-decisions.md) —— F-3 那行的归属登记处
