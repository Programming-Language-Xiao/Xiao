# 20C-入口. 20C 入口准备：骨架、清单同笔与首批模块的前置

> **背景**：10 系列已关闭，[10R-实现](10r-static-overflow-implementation.md) 的 D1/D2/D4 已落地
> （ABI **1.12**），[10R-收尾](10r-closeout.md) 的实施也已完成大半——**但它的出口条件还没满足**
> （真实 push 运行号、受控原生差分、F-3、产物级重复构建都在等外部执行点）。
> 同时审核在收尾实施里查出 **P-3：执行点仍然是枚举式，差集 5 个**（其中 `15e_ci_gated` 里
> 正住着「两次构建逐字节相同」那条证据）。
>
> **一句话概括**：**先把 P-3 与 F-3 的登记修掉，再按 20C 的守卫把入口四项做成骨架**——
> 本批**不写**库模块；`text`/`path` 的首批实现是下一批。
>
> 状态：**规划稿（2026-10-12）**。入口是 §二 的两条修补；§三 的 10R 欠项**并列推进**，不阻塞本批开工。

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
| 10Z-D 的行为面（受控原生差分、失败清理、**一次故意变异的红色证据**） | **需要受控原生机**（结构面已有非 ignored 测试，进 `workspace-default-tests`） | 受控环境 |
| 产物级**两次构建逐字节相同**（[10R-收尾](10r-closeout.md) §2.5） | 证据住在 `15e_ci_gated::real_artifact_reproducibility_is_byte_comparable`，**该目标没有执行点**（见 P-3） | 10R-收尾 |
| O-1（19D 是否受影响） | 已判「**数据不足**」并留痕；要判「不受影响」需要**受控链接/性能取数** | 19D |

**表 B：P-3 的权威/清单/差集**（审核逐行核，`crates/*/tests/*.rs` 里带 `#[ignore]` 的集成测试目标）

| | 内容 |
| --- | --- |
| **权威（11 个）** | `xiao-codegen-llvm/tests/`：`15e_ci_gated`、`n0_a`、`n0_b_dynamic`、`n0_c_errors`；`xiao-driver/tests/`：`c20_capability_probe`、`d19a_differential`、`n0_a_native_driver`、`n0_b_dynamic_native`、`native_benchmark_probe`；`xiao-package/tests/e3c_remote`；`xiao-runtime/tests/n0_c2_error_boundary` |
| **CI 已覆盖（6 个）** | `c20_capability_probe`、`d19a_differential`、`n0_a_native_driver`、`n0_b_dynamic_native`、`native_benchmark_probe`、**`n0_a`**（本批新加） |
| **差集（5 个）** | `n0_b_dynamic`（**该补**，`XIAO_LLVM_AS` 已有）、`n0_c_errors`（**该补**，同上）、`15e_ci_gated`（**接不上**：需要 `XIAO_STRIP`，两个原生作业**都没设置它**——逐行核过）、`e3c_remote`（理由正当：「CI 不依赖外部解析服务」）、`n0_c2_error_boundary`（理由正当：「由另一测试在子进程内调用」） |

**表 C：20C 入口四项与其守卫**

| # | 入口项 | 依据 | 现状 |
| --- | --- | --- | --- |
| 1 | N-1 三处清单与目录/crate **同笔落地** | [20C-冻结](20c-freeze-and-10-series-closeout.md) §3.8；**实测**：拆两笔会让中间那笔直接红（`A0-MANIFEST-002` + `A0-LAYOUT-004`） | 未做 |
| 2 | N-2 `.xiao` 是否纳入 `sourceExtensions` | §3.8；不纳入则官方库源码**不被源目录发现**、README 门禁管不到 | 未定（甲乙待实测） |
| 3 | N-3 身份块与 `CompatibilityVersions` 的关系 | §3.4；是**并列新块**还是**扩展公共结构** | 未写明 |
| 4 | **依赖求解负例** | [20C](20c-official-xiao-library.md) §3.2.1 第 5 条：**必须有负例**证明普通依赖求解不会解析官方库 | 未做 |

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

**事实**（表 B）：上一批把 `n0_a` 加进两处原生作业，但**同 crate 的三个兄弟**、
以及 `xiao-package`、`xiao-runtime` 各一个**仍然没有执行点**；其中

- `n0_b_dynamic`、`n0_c_errors` 的依赖（`XIAO_LLVM_AS` / 真实 clang）**两个原生作业都已具备** ⇒ **该补**；
- `15e_ci_gated` 需要 **`XIAO_STRIP`**，而两个原生作业**都没有设置该变量** ⇒ 它**接不上**，
  **而文档里没有这一句**——**这正是「漏」与「有理由的排除」的区别所在**；
- `e3c_remote`、`n0_c2_error_boundary` 的排除**理由正当**，但它们目前**没有写在任何地方**。

**为什么不能再枚举**：这一次是「加一个目标」，下一个测试文件一进来又漏一次——
本仓库在同类上已经栽过（crate 过滤漏 10 个、`tests/fixtures/**` 不在任何 paths、G-1）。

**冻结**：

1. **两处原生作业改用 `--workspace -- --ignored` + 排除清单**；
   `tools/platform-reproduction/reproduce.sh:132` **已经在用这个口径** ⇒ 可行性与耗时都有先例；
2. **排除清单每项写理由**（至少含 `e3c_remote`「CI 不依赖外部解析服务」与
   `n0_c2_error_boundary`「由另一测试在子进程内调用」两项），**并说明 `15e_ci_gated` 取哪条路**：
   - **补环境**：给两处原生作业设 `XIAO_STRIP`（msys2 的 `mingw-w64-ucrt-x86_64-llvm` 与 Linux 的
     `llvm` 都应提供 `llvm-strip`，**先实测确认**）；
   - **或显式排除**：写清「为什么不在 push 上跑」，并**同时指明它在哪里会被跑到**；
3. **无论哪条路，都要回答 §2.2 第 2 条**（那条可重复性证据的执行点）；
4. 改完**按 D7 第 3 条验证**：破坏一个断言 → 推送确认门禁**变红** → 恢复确认**转绿**，运行号回填；
5. 验证提交**只碰工作流**、不顺手带 `docs/**`，且**确认分支相对 `origin/main` 的改动只有该路径**。

### 2.2 **F-3 的登记措辞，与「那条可重复性证据住在哪」**

**事实**：`00-decisions.md` 里 F-3 那行写的是「**本批**收掉或 10 系列遗留」——
那是**关闭批**的口径，**「本批」现在指哪批不明**；而 [10R-收尾](10r-closeout.md) §2.5 要的
「两次构建逐字节相同」证据住在 `15e_ci_gated`（且该目标没有执行点）。

**冻结**：

1. 把 F-3 那行改成**明确归属**（`10R-收尾` 待执行 / 留 10 系列，二选一后写死），
   并补上**卡点**（需要一次真推送）；
2. 在 [10R-收尾](10r-closeout.md) §2.5 **写明证据落在哪个测试/哪条命令上**，
   并指向 §2.1 的处置结论（补环境或显式排除）——**不许**让一条验收的落点查不到；
3. 这两处都是**一行级**改动，**不许**顺手带其它内容。

## 三、10R-收尾 的未完成项（并列推进，不阻塞本批）

**冻结**：

1. 表 A 的五项**如实保留**，并在 [10R-收尾](10r-closeout.md) 的状态行里与事实一致；
   **不许**因为「下一批开工了」就写成已完成；
2. **谁做什么写清**：P-2 与 F-3 的运行号**等星崽推送**（一次性分支，纪律见 [10R-收尾](10r-closeout.md) §2.1 第 3 条）；
   10Z-D 的行为面**等受控原生机**；O-1 的最终判定**等受控取数**；
3. **O-1 的「数据不足」要留痕**（19D 已登记）：**何时能判**要写清——即「受控链接/性能取数到位之后」，
   否则它会变成一个永远悬着的结论。

## 四、20C 入口四项（本批主体）

### 4.1 **N-1：骨架与三处清单同笔落地**

**事实**：落点是**顶层 `stdlib/`** + **crate `core/rust/crates/xiao-standard-library`**；
三处权威清单分别是 `repository.manifest.json` 的 `codeRoots`（现为 `core/rust,cli/ts,tools,tests`）、
[00A](00a-project-layout.md) 的顶层目录块、以及同文件的 `rust.members`（**23 项**）。
**实测**：只补 `codeRoots` 而不建目录 → `A0-MANIFEST-002` + `A0-LAYOUT-004`，退出码 1。

**冻结**：

1. **同一提交**里完成：`codeRoots` 加 `stdlib/`、00A 顶层目录块加 `stdlib/`（含 README 要求与**工程期**归属）、
   `rust.members` 加 `xiao-standard-library`、建 `stdlib/` 与 crate 骨架（含各自 README）；
   **提交树中清单与目录必须同时存在**；
2. **提交说明里写出清单差集**（权威 23 → 24，`codeRoots` 4 → 5，00A 顶层目录 7 → 8）；
3. crate 骨架**只放最小可编译内容**：**不许**在本批写库模块、不许注册进普通依赖求解、
   不许写 `.xar` 兼容字段；
4. 骨架的 README 要写清：**工程期 20C**、**官方库身份**、**不参与普通依赖求解**、**负责与不负责**。

### 4.2 **N-2：`.xiao` 与 `sourceExtensions`（先实测，再二选一）**

**事实**：`repository.manifest.json` 的 `sourceExtensions = ['.rs','.ts','.tsx']`；
`tools/repo-check` 里检索 `.xiao` **一处都没有**；`layout.ts:58` 只遍历 `codeRoots` 去发现源目录。

**冻结**：

1. **先实测**：把 `.xiao` 加进 `sourceExtensions`（**只读试算或临时改动后还原**），
   看既有 **9 个** `.xiao` 所在目录（`tests/fixtures/20c/`、`tests/benchmarks/programs/`）
   是否满足 `A0-LAYOUT-001` 的 README 与**交接信息**要求；
2. **按实测二选一**：
   - **甲**：纳入 `.xiao`（官方库源码进通用发现范围）；
   - **乙**：为官方库源码目录**另立一条目录规则**（例如 `stdlib/**` 必须有模块 README）；
3. 无论甲乙，**官方库模块的 README/UseDocs/兼容元数据同步要求都要可被门禁检查**——
   **没有门禁兜底的同步要求等于没有**（[20C](20c-official-xiao-library.md) §3.2.1 第 6 条）；
4. 若选甲导致既有目录变红，**不许**把要求降级去迁就，而是**补齐那些目录**或改选乙。

### 4.3 **N-3：标准库身份块与 `CompatibilityVersions` 的关系要写明**

**事实**：现有 `CompatibilityVersions`（`xiao-driver/src/compatibility.rs:167`）是
`language: String`、`ir: u32`、`xiaoc/index_schema/xar`、**`runtime_abi: u64`（单个编码版本）**、`target`；
冻结的身份字段是 `language_version`、`stdlib_interface_version`、`runtime_abi_major`、`runtime_abi_minor`、`target`。

**冻结**：

1. **实现前**先写明是**新增一套并列的标准库元数据块**（`CompatibilityVersions` 不动），
   还是**扩展既有公共结构**；
2. 若选扩展，**先列出受影响的消费者**（19 的兼容矩阵、发布报告 `release_report`、文档生成、
   `xiao-driver` 的 `current_compatibility_versions`）；
   **不许**顺手改公共类型；
3. 身份要**可读、可比较**，并能进入构建产物元数据（`.xar` 消费延后 20E）；
4. 身份里的 `runtime_abi_major/minor` **必须来自 `xiao-runtime-abi` 的常量**（现为 **1.12**），
   **不许**在别处再写一份字面量（单一来源）。

### 4.4 **依赖求解负例（这条是「可检查落点」的兑现）**

**冻结**：

1. 官方库由**工具链专用加载器/命名空间**提供，**不写入 `PackageRegistry` 的普通依赖图**；
2. **必须有一条负例**（可运行的用例，不是文档声明）证明：把官方库当**普通依赖**解析会**失败**，
   或它**不出现在依赖求解结果里**——二选一，并写明选了哪种与为什么；
3. 该负例要有**执行点**（见 §2.1 的口径）——**新加的测试同样要问「push 时谁跑它」**。

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
docs/DevDocs/00-decisions.md                            §2.2 F-3 那行的归属与卡点
docs/DevDocs/10r-closeout.md                            §2.2 证据落点；§三 未完成项的如实状态
docs/DevDocs/20c-official-xiao-library.md               §4.2/§4.3 的结论回写（冻结结果留在 §3.2.1）
```

## 六、验收

1. **P-3**：两处原生作业都改成 glob 口径；**排除清单每项有理由**；
   `15e_ci_gated` 要么被跑到、要么有**显式排除 + 它在哪里被跑到**；
   破坏断言 → 红 → 恢复 → 绿的运行号各一次；改动面干净（只碰工作流）；
2. **F-3 与证据落点**：`00-decisions.md` 那行有明确归属与卡点；
   [10R-收尾](10r-closeout.md) §2.5 写明证据落在哪个测试/命令上；
3. **N-1**：清单与目录**同笔**；提交说明有差集；骨架最小且不含库模块；
   **没有**注册进普通依赖求解、**没有**写 `.xar` 兼容字段；
4. **N-2**：有**实测表**（既有 9 个 `.xiao` 目录的真实结果）+ 明确的甲乙结论；
   同步要求**可被门禁检查**；
5. **N-3**：并列还是扩展**写明**；若扩展则列出受影响消费者；身份里的 ABI 主/次来自常量；
6. **依赖求解负例**：**可运行**且有执行点；
7. **10R 欠项**：五项状态与事实一致（**没有**被写成已完成）；
8. **通用**：`cargo test --workspace`、clippy、fmt、`bun test`、`bun run check`、`bunx tsc --noEmit` 全绿；
   提交标题带类型前缀、正文非空。

## 七、待定决策（给星崽）

1. **P-3 的 `15e_ci_gated`**：**补 `XIAO_STRIP`** 让它进 push 门（审核倾向，先实测 msys2/Linux 是否都有
   `llvm-strip`），还是**显式排除并说明在哪里跑**？
2. **N-2 走甲还是乙**：审核倾向**甲**（官方库源码就是 `.xiao`，进通用发现范围才与「每模块同步 README」
   的验收对齐），但**先看实测**——若既有目录补不齐，就转乙；
3. **N-3 取并列还是扩展**：审核倾向**并列新块**（不碰 19 的兼容矩阵与发布报告的既有消费者），
   代价是同一份身份有两个来源，需在文档里说清各自用途；
4. **是否需要在本批就把 `XIAO_STRIP` 那类环境变量统一登记**（两处原生作业与 10D §4 的清单对账）——
   若有第三处露头，说明该清单也该换成单一来源。

## 相关页面

- [20C-冻结. 10 系列关闭与 20C 六条冻结](20c-freeze-and-10-series-closeout.md) —— §3.6/§3.8 的落点守卫与三处清单
- [20C. 官方 Xiao 库：能力探测、模块清单与兼容身份](20c-official-xiao-library.md) —— §3.2.1 六条冻结与 §3.1 探测边界
- [10R-收尾. 静态数值路径的收尾、证据接线与 20C 入口准备](10r-closeout.md) —— 上一批；其未完成项见本文 §三
- [10R-实现. 静态失败块接入可恢复错误通道](10r-static-overflow-implementation.md) —— D1–D7 的裁定与 ABI 1.12
- [10Z-驱动. 性能对照的计时驱动器](10z-performance-driver.md) §2.7 —— D7「证据必须有执行点」；P-3 的判据
- [00A. 工程框架与目录布局](00a-project-layout.md) —— 顶层目录与 crate 表的权威
- [11A. 虚拟环境与包管理](11a-environments-and-packages.md) —— 官方库不进普通依赖求解的边界
- [00. 决策基线](00-decisions.md) —— F-3 那行的归属登记处
