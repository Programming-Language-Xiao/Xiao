# 10R-收尾. 静态数值路径的收尾、证据接线与 20C 入口准备

> **背景**：[10R-实现](10r-static-overflow-implementation.md) 的 **D1（报告/诊断事件解耦）与 D2（7 处失败块接 Runtime 可通道）**
> 已实现并**经审核实跑复核**：c20 原生探测的 stderr 里 `X11-DIAGNOSTIC-CHANNEL-001` 那行**确已消失**、
> 错误身份/span/退出码**原样不变**；受控环境里 6 个 ignored 测试全过，断言不弱
> （`!text.contains("call void @llvm.trap()")`、依赖面不含 `WS2_32.dll`、13 个边界用例含
> **除零 → `X06-RUNTIME-013`**、真非法指令仍 Fatal）。D6 的 `(0)(1)(2)` 已完成。
>
> **一句话概括**：**把 10R 的出口条件逐条收干净——含两处「证据没有执行点 / 归档证据过期」的修补——
> 再把 20C 的入口条件列成清单**；本批**不**动 20C 库代码。
>
> 状态：**实施中（2026-10-12）**。P-1/P-2 文档与工作流修补、10Z-D 代码接线已落地；Windows 受控原生差分于 2026-10-13 通过；Linux/macOS 复跑、真实 push 运行号、F-3 变红/转绿和产物级重复构建仍待外部执行点。

## 一、Agent 交接上下文

### 1.1 接手前提

1. [10R-实现](10r-static-overflow-implementation.md) —— **本批的前置批次**；§五 的 D1–D7 是已生效的裁定，
   §六 是实施回填；**本批不改 D1–D7 的结论**；
2. [10R-溢出后续](10r-static-overflow-followup.md) —— 原始立项与后续记录；
3. [10Z-D](10z-dynamic-error-constructor-followup.md) —— 本批要实现的动态错误构造参数（D4 已定并批）；
4. [10Z-驱动](10z-performance-driver.md) §2.7 **D7** —— 「证据必须有执行点」；§二 P-2 的判据来源，
   以及**G-1 那一轮已验证可行的修法与纪律**（一次性分支、只碰该路径、变红验证）；
5. [20C-冻结](20c-freeze-and-10-series-closeout.md) §3.6/§3.8 —— 20C 的落点守卫与三处枚举清单；入口骨架已由
   [20C-入口](20c-entry-and-skeleton.md) 落地，本批只记录与库模块有关的边界；
6. [19D](19d-performance-comparison.md)、`tests/benchmarks/baseline.json` —— O-1 判定的对象与**基线变更流程**；
7. [00. 决策基线](00-decisions.md)「10 系列关闭与遗留项」—— F-3、条件 5 等遗留项的卡点登记处。

### 1.2 现状盘点（2026-10-12，审核实测）

**表 A：10R §四 验收 10 项逐条对账**

| # | 验收项 | 现状 | 证据 |
| --- | --- | --- | --- |
| 1 | 链接代价探测 | **满足** | `10r-实现` §2.1.1/§2.1.4；审核复核根因与符号归属 |
| 2 | 7 处逐条身份 + 边界内/外输入 | **满足** | `n0_a` 的 13 例边界测试（含除零 013），受控环境实跑通过 |
| 3 | 身份一致（VM / 原生） | **部分** | 静态数值路径与 D4 动态错误参数已有 Windows 受控原生证据；Linux/macOS 复跑及故意变异红色证据仍待补，见 §七 |
| 4 | 真非法指令仍 Fatal 的分界 | **满足** | `optional_external_illegal_instruction_remains_fatal`；报 `X07-FATAL-005` |
| 5 | 产物：两次构建逐字节相同；体积按基线变更 | **部分** | Windows workspace ignored 门禁已跑过 15E 通用 Runtime 产物两次比较并通过；10R 要求的 D1/D2 后静态数值剖面产物仍未测 |
| 6 | ABI（版本/指纹/旧 Runtime 拒绝） | **满足** | ABI 次版本 1.12；旧 1.10/1.11 接受、1.13 与错误主版本拒绝 |
| 7 | F-3 触发正/负例 | **未做** | 10 系列遗留 |
| 8 | D1 的可复核证据 + 三个边界用例 | **满足** | 依赖面不含 `WS2_32.dll` 的断言；审核实跑复现「那行消失」 |
| 9 | O-1 处置（19D 基线） | **数据不足** | 19D 已登记 D1/D2 影响复核；无受控链接/性能取数，不改基线 |
| 10 | 通用门禁 | **满足** | `bun run check` 退出 0 |

⇒ **本批 = 5、7、9 三项 + 两条修补（P-1、P-2）+ 10Z-D。**

### 1.3 本批边界

| 子任务 | 内容 | 本批 |
| --- | --- | --- |
| **P-2 修补** | 本批核心证据在 push 上**没有执行点** | **做，排第一**（§二 2.1） |
| **P-1 修补** | 归档证据与引用它的文档过期 | **做**（§二 2.2） |
| 10Z-D | 动态错误构造参数（新符号） | **做**（§二 2.3） |
| O-1 判定 | 19D 基线要不要重取 | **做判定**（§二 2.4） |
| 可重复性 | D1/D2 之后新产物的两次构建逐字节相同 | **做**（§二 2.5） |
| F-3 | `tests/benchmarks/**` 的触发正/负例 | **做**（§二 2.6） |
| 三处文档缺陷 | `11ae0`/`15c` 的未闭合粗体与未闭合代码跨度 | **做**（§二 2.7） |
| 20C 库代码与其入口准备 | N-1/N-2/N-3、依赖求解负例 | **库代码不做；入口已由 20C-入口完成**，当前状态见 §三 |
| 静态 `try`/`raise` + 清理链 | —— | **不做**（[10R-实现](10r-static-overflow-implementation.md) **D7** 已另立） |

## 二、必须先冻结的 7 条

### 2.1 **P-2：本批的核心证据在 push 上没有任何执行点**（与 G-1 同型）

**事实（审核逐行核过 `.github/workflows/*.yml`）**：CI 里 `--ignored` 的执行点**只有两处**——
`-p xiao-runtime-abi -- --ignored` 与
`-p xiao-driver --test {c20_capability_probe, n0_a_native_driver, n0_b_dynamic_native, d19a_differential, native_benchmark_probe} -- --ignored`
（Windows 与 Linux 各一份）。**两份工作流里没有 `xiao-codegen-llvm`。**

⇒ 本批的三个核心测试——`optional_static_overflow_uses_recoverable_runtime_report`、
`optional_external_illegal_instruction_remains_fatal`、
`optional_static_numeric_failure_boundaries_match_runtime_contract`——**在 push 时没有人跑**。
周定时的 `tools/platform-reproduction/reproduce.sh:132` 用 `--workspace -- --ignored`，**会**覆盖它们，
但那是**周频、且不是 push 门**。这和 [G-1](10z-closeout-implementation-audit.md) 那次是**同一形状**：
**枚举式清单决定谁受保护**。

**冻结**：

1. **修法沿用 G-1 那一轮已验证的做法**：在**两处原生作业**（Windows 与 Linux）各加
   `-p xiao-codegen-llvm --test n0_a -- --ignored`，与现有 `-p xiao-driver --test …` 并列；
   （若愿改口径，可统一换成 `--workspace -- --ignored` + 排除清单——**能用 glob 就别枚举**，但两处必须同改）
2. **改完按 D7 第 3 条验证**：破坏 `n0_a` 里一个新断言 → 推送确认门禁**变红** → 恢复确认**转绿**，运行号回填；
3. **纪律三条（G-1 那轮踩过）**：验证提交**只碰目标路径**、**不顺手带 `docs/**`/工作流文件**、
   并**确认分支相对 `origin/main` 的整体改动只有该路径**（那次第一次探针就是栽在第三条）；
4. **不得**把「周定时会跑到」或「本机跑过」当作 push 门的证据。

### 2.2 **P-1：归档证据过期，引用它的文档口径也反了**

**事实（审核实跑）**：同一份 c20 原生探测今天产出的 stderr 是

```text
xiao-error class=recoverable code=X06-RUNTIME-009 … span=40..53 exit_code=3
```

而**归档证据仍记录着旧版本**：`docs/DevDocs/assets/20c-capability-20261011/probe.log` 第 7 行、
以及 [20C](20c-official-xiao-library.md) §3.1 表（共 3 处）都含
`X11-DIAGNOSTIC-CHANNEL-001: 诊断事件没有已建立的会话`——**那一行已被 D1 裁定移除**。
更麻烦的是 20C 里还有一句「建议在表格备注里引一句 10E §3.4 的出处，**免得后来人把它当噪音删掉**」，
**方向已经反了**：它不是被当噪音删掉的，是**按裁定移除**的。

**冻结**：

1. 按 [10R-实现](10r-static-overflow-implementation.md) §2.6 的规则，**归档证据变化属基线变更**：
   **显式登记理由（D1）后重出** `probe.log`，**不许**悄悄替换；
2. 20C §3.1 的表与那句建议**同批改**：说明「该行已在 D1 之后不再出现，原因是报告与诊断事件解耦」，
   并保留历史记录（**改口径不改证据**：旧行可留在历史说明里，但不许再当作当前行为）；
3. 这条与 P-2 一样属「改了一处、漏了另一处」——**归档物与引用它的页面必须同批对齐**。

### 2.3 **10Z-D：动态错误构造参数（D4 已定并批，本批实现）**

**冻结**：

1. 按其立项文档 §三 的实现要求做：前端/VM 的错误身份与 `raise` 位置**保持不变**；
   LLVM 路径为**动态文本参数**生成稳定的 `xiao_runtime_error_new_values` 调用；
   非文本值仍返回稳定类型错误，**不许**静默转换；
2. **ABI**：新增符号 ⇒ 按 10T/10U 的版本规则**再升一次次版本**（1.11 → 1.12），
   兼容检查与旧 Runtime 拒绝策略**同批更新**（**这是本批唯一一次 ABI 变更**）；
3. **差分用例**：VM/原生成对、`catch` 与未匹配传播、动态 `message`、非文本参数、失败清理；
   原生侧要有**受控工具链实跑**与**一次故意变异的红色证据**（立项文档原文要求）；
4. 若实现中发现需要**静态**侧的配合（例如 `raise` 的动态参数出现在静态路径），
   **停下来登记**——静态 `try`/`raise` 属 D7 的后续段，**不许**在本批顺手做进去。

### 2.4 **O-1 判定：19D 的受控基线要不要重取**

**事实**：D5 的逐程序补测显示 `after == baseline`（五个样本的产物字节数**没有变化**），
因为**这些产物本来就不含报告路径的引用**；D1 的收益要到**引用报告路径的产物**上才兑现，
而 D2 恰好让静态失败块引用那条路径。

**冻结**：

1. **判据不是「感觉没变」**：要拿 **D1/D2 之后**的实际产物剖面说话——
   哪些产物变了（`uses_runtime` 从 false 变 true 的那些静态标量程序）、变了多少、
   变的是**体积**还是**启动路径**；
2. **若剖面变了** → 按**基线变更流程**重取/重判 19D 的受控取数与阈值（阈值与 sha256 同步、摘要回写）；
   **若没变** → 写明「为何不受影响」的证据（哪几个样本、测了什么），
   **不把「没测」写成「不影响」**；
3. 无论哪种结论，都在 [19D](19d-performance-comparison.md) 的 §2.4/§2.6 与 `baseline.json` 旁**留一句**，
   使人不必翻三个文档才能知道 19D 是否受 10R 影响。

### 2.5 **可重复性：D1/D2 之后的新产物仍要两次构建逐字节相同**

**冻结**：拿至少一个**改了剖面的**静态数值程序（失败块引用 Runtime 的那类）做两次构建，
逐字节比对；时间戳类差异按 15C 的既有归一化处理，**并写明归一化了哪些字段**（沿用 D5 的做法）。
这条是发布门槛（[10R-实现](10r-static-overflow-implementation.md) §2.6 第 3 条），不是可选项。

**证据落点（20C 入口接线后）**：真实产物的逐字节断言位于
`core/rust/crates/xiao-codegen-llvm/tests/15e_ci_gated.rs` 的
`real_artifact_reproducibility_is_byte_comparable`。两平台维护回归的原生作业通过
`cargo test --manifest-path core/rust/Cargo.toml --workspace -- --ignored` 执行它；
`XIAO_STRIP` 已由工作流设置，外部 push 运行号仍待补。原生作业明确跳过的 DNS、平台异常子进程和
交互式诊断窗口用例及其理由登记在 [20C 入口](20c-entry-and-skeleton.md) §2.1；这些排除不覆盖 15E。

**Windows 本机执行记录（2026-10-13）**：在 MSVC + MSYS2 LLVM 环境中完整执行 15E，
`real_artifact_reproducibility_is_byte_comparable` 两次真实构建相同，归一化字段为
`PE.DebugDirectory.TimeDateStamp` 与 `PE.TimeDateStamp`。该夹具是 15E 的通用 Runtime 表程序，
不是 10R 要求的 D1/D2 后静态数值剖面样本；后者仍保持「未验」，不能用本次 15E 结果替代。

### 2.6 **F-3：触发正/负例（与 P-2 同一次做掉）**

**冻结**：做法沿用 G-1 那轮（一次性分支、两个提交各只碰 `tests/benchmarks/**`：
A 破坏 `cpu_model_parser_accepts_linux_field_separators` 的期望值 → 触发并变红；B 还原 → 转绿；做完删分支）。
**纪律同 §2.1 第 3 条**。若本批不做，必须写进 [决策基线](00-decisions.md) 的遗留清单——**不许两边都不提**。

### 2.7 **三处既有文档缺陷（上轮审核查出，本批顺手修）**

**事实（审核用检查器实测，非目测）**：

| 位置 | 缺陷 |
| --- | --- |
| `11ae0-environment-fingerprint-and-activation.md:175` | 引文块 `> 涉及**按键、粘贴、软换行与颜色的用例…` **粗体开了没关**，整段后半被吞进粗体 |
| 同文件 `:156` 的表格行 | 该行**反引号数为奇数**（有代码跨度没闭合），单元格渲染会串 |
| `15c-artifact-reproducibility-and-debug-mapping.md:245` | `…这个说法的**唯一兑现处**——` 是**三个 `**`**，会把后面句子一起吞掉 |

**冻结**：三处各一行修正；修完**在文档里不留别的改动**。
**并记一笔**：这类缺陷 `bun run check` **不查**，同类问题本仓库已出现 **5 次**——
是否立规则由星崽另定（[10Z-收尾](10z-closeout-execution.md) §2.3 的「不开新的工具侧批次」仍有效，本批**不**新立门禁）。

## 三、20C 入口状态（入口已完成，本批不实现库模块）

10 系列关闭与 10R 入口守卫已经完成后，20C-入口批已把四项前置落点同笔落地；本批仍不实现
`text`/`path` 模块。清单及证据见 [20C-入口](20c-entry-and-skeleton.md) §四：

| # | 入口项 | 当前状态 | 复核方式 |
| --- | --- | --- | --- |
| 1 | **N-1 清单同笔落地** | 已在同一提交建立 `stdlib/`、`xiao-standard-library`，并同步 `codeRoots`、00A 顶层目录和 `rust.members` | [20C-入口](20c-entry-and-skeleton.md) §4.1；差集为 4→5、7→8、23→24 |
| 2 | **N-2 `.xiao` 纳入 `sourceExtensions`** | 已按方案甲纳入；既有 9 个 `.xiao` 所在的两个目录通过发现与登记门禁 | [20C-入口](20c-entry-and-skeleton.md) §4.2；`A0-LAYOUT-005` |
| 3 | **N-3 身份字段与 `CompatibilityVersions` 的关系** | 已采用并列 `StandardLibraryIdentity`，既有 `CompatibilityVersions` 与消费者不变 | [20C-入口](20c-entry-and-skeleton.md) §4.3 |
| 4 | **依赖求解负例** | 已覆盖索引直接依赖、传递依赖和本地路径依赖，统一拒绝 `xiao-standard-library` | [20C-入口](20c-entry-and-skeleton.md) §4.4；`xiao-package/tests/e3d_solver.rs` |

四项已经完成；20C-入口只建立最小骨架和隔离证据，仍未写 `.xar` 兼容字段，也未实现
`text`/`path` 模块。后续模块批次必须继续遵守 [20C](20c-official-xiao-library.md) §3.4--§3.6 的同步与兼容守卫。

## 四、落点

```text
.github/workflows/maintenance-regression.yml           §2.1 P-2：两处原生作业加 -p xiao-codegen-llvm --test n0_a -- --ignored
core/rust/crates/xiao-codegen-llvm/tests/n0_a.rs       §2.1 变红验证的变异点（验完恢复）
core/rust/crates/xiao-runtime/src/abi.rs               §2.3 10Z-D：new_values 入口
core/rust/crates/xiao-runtime-abi/src/lib.rs           §2.3 ABI 1.12 与兼容检查
core/rust/crates/xiao-codegen-llvm/src/dynamic/*.rs     §2.3 动态参数发射（不碰静态 try/raise）
tests/benchmarks/src/bin/performance_driver.rs          §2.6 F-3 的变异点（验完恢复）
docs/DevDocs/assets/20c-capability-20261011/probe.log   §2.2 P-1：按基线变更重出
docs/DevDocs/20c-official-xiao-library.md               §2.2 表与那句建议的方向
docs/DevDocs/11ae0-…md、15c-…md                          §2.7 三处一行修正
docs/DevDocs/19d-performance-comparison.md、tests/benchmarks/baseline.json   §2.4 判定留痕
docs/DevDocs/00-decisions.md                            遗留清单：F-3 若不做则登记
```

## 五、验收

1. **P-2**：两处原生作业都跑到 `n0_a` 的 ignored 测试（有运行号）；**变红/转绿验证**各一次、运行号回填；
   验证提交的改动面干净（只碰目标路径）；
2. **P-1**：`probe.log` 按基线变更重出（含理由 = D1）；20C 的表与那句建议方向已正；旧记录**保留在历史说明里**；
3. **10Z-D**：VM/原生成对、动态 `message`、非文本参数、`catch` 与未匹配传播、失败清理各有用例；
   原生侧有受控实跑与**一次变异的红色证据**；ABI 1.12 与兼容检查同批；
4. **O-1**：19D 是否受影响有**明写结论 + 证据**（剖面数据），要么走基线变更流程，要么写明为何不受影响；
5. **可重复性**：改了剖面的一个新静态数值产物两次构建逐字节相同，归一化字段写清；
6. **F-3**：有运行号或已写进遗留清单；
7. **三处文档缺陷**：修正且未夹带其它改动；
8. **20C 入口**：四项清单与最小骨架**已实现**；本批不实现 `text`/`path` 模块，也不写 `.xar` 兼容字段；
9. **通用**：`cargo test --workspace`、clippy、fmt、`bun test`、`bun run check`、`bunx tsc --noEmit` 全绿；
   提交标题带类型前缀、正文非空；**文档粗体标记成对**（本仓库已 5 次未闭合，含 §2.7 那两处）。

## 六、待定决策（给星崽）

1. **P-2 的修法**：已按 P-3 改为两平台 `--workspace -- --ignored` 并设置 `XIAO_STRIP`；真实 push
   红/绿运行号仍待外部执行点，不能用本机结果替代。
2. **N-2**：已按方案甲纳入 `.xiao`，既有 9 个文件的实测结果和 `A0-LAYOUT-005` 见 §三及
   [20C-入口](20c-entry-and-skeleton.md) §4.2。
3. **20C 入口四项**：已在后续 20C-入口批完成；本批边界保留为不实现官方库模块。

## 七、实施记录（2026-10-12）

- **P-2**：初始修补曾在 Windows/Linux 原生作业加入
  `cargo test -p xiao-codegen-llvm --test n0_a -- --ignored --nocapture`；该枚举口径已由 20C P-3
  的 `--workspace -- --ignored` 取代，本机未触发真实 push，因此没有伪造运行号或宣称 push 触发已验证。
- **P-1**：`20c-capability-20261011/probe.log` 已按 D1 基线变更重出，当前 stderr 不再含无会话诊断事件提示；
  20C §3.1 表格改为当前记录，并保留旧提示作为 D1 前历史说明。
- **10Z-D**：动态 `code`/`message` 由 LLVM 调用 `xiao_runtime_error_new_values`；非文本值保留到 Runtime，
  返回稳定类型错误并进入现有错误路由。ABI 次版本为 1.12。Windows 受控原生差分和释放轨迹于
  2026-10-13 通过；变量参数改为借用输入槽，VM 对非文本参数补齐构造表达式 span。Linux/macOS 复跑及故意变异红色证据仍待外部执行点。
- **Windows 全量 ignored 复跑（2026-10-13）**：与 P-3 工作流相同的 workspace 命令通过，
  覆盖 15E、N0-A/B/C、C20、D19A 全量原生差分、O1–O3 表方法矩阵及 D4 成对错误构造用例；
  `real_terminal_session_is_environment_gated` 与 `native_table_methods_debug_execution_o0_o3` 因 Windows runner
  无交互终端而仅在 Windows job 排除，Linux 使用 Xvfb/xterm 执行。该本机运行不产生 GitHub Actions 运行号。
- **O-1/可重复性**：19D 已新增影响复核，结论为**数据不足、不改 baseline.json**；本轮 Windows 受控链接只用于 D4 原生差分与 15E 门禁，
  没有执行 19D 的受控性能取数，因此没有把原生差分或历史体积数字冒充为 O-1 结论。
- **F-3**：尚未有真实 push 的正/负例运行号，继续保留在 00 决策基线的遗留清单。
- **文档缺陷**：11AE0 两处标记和 15C 一处粗体已各修一行；20C 入口四项已由入口批完成，
  当前仅保留 `text`/`path` 模块实现和外部受控证据为后续项。

## 相关页面

- [10R-实现. 静态失败块接入可恢复错误通道](10r-static-overflow-implementation.md) —— **前置批次**；D1–D7 的裁定与实施回填
- [10R-溢出后续. 静态溢出统一错误路径](10r-static-overflow-followup.md) —— 原始立项与后续记录
- [10Z-D. 动态错误构造参数实现记录](10z-dynamic-error-constructor-followup.md) —— §2.3 的实现依据
- [20C-冻结. 10 系列关闭与 20C 六条冻结](20c-freeze-and-10-series-closeout.md) —— §三 入口清单与落点守卫的出处
- [20C. 官方 Xiao 库：能力探测、模块清单与兼容身份](20c-official-xiao-library.md) —— §2.2 要改的表与那句话
- [10Z-驱动. 性能对照的计时驱动器](10z-performance-driver.md) §2.7 —— D7「证据必须有执行点」；§2.1 的判据
- [19D. 性能对照](19d-performance-comparison.md) —— §2.4 O-1 判定的对象与基线变更流程
- [00. 决策基线](00-decisions.md) —— 「10 系列关闭与遗留项」：F-3 等卡点登记处
