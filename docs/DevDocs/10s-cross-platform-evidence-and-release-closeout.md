# 10S. 跨平台证据刷新与释放账目收口（N0-K）

> **10 阶段的续批**，接 [10R](10r-release-accounting-and-native-ci-gate.md)。10R 补上了原生 CI 门控
> 并用一次真实的失败验证了它；审核确认 I6 已修、账目扩展到位。
>
> **一句话概括本批**：**把「三平台」从只有 Windows 变成「Windows 门禁 + Linux 裸机 + macOS CI」，
> 并把剩下四处释放差异逐调用收掉**，让 19 的 O6 条件 2 和 3 有可判定的依据。
>
> 状态：**实施中（2026-10-07）；释放差异与 A1 准备已完成，CI 修复复跑中，裸机新版与可见窗口证据待补**。星崽本轮补充了可用宿主（见 §1.2），据此调整跨平台安排。

## 一、Agent 交接上下文

### 1.1 接手前提

1. [10R](10r-release-accounting-and-native-ci-gate.md) 第九至十三节 —— 账目数据、I6 修复记录、CI 证据；
2. [10Q](10q-selector-case-strength-and-release-audit.md) 第九节 —— 释放审计口径与选择器差异的定位；
3. [19C](19c-maintenance-and-debug.md) §2.5 —— `19.14` 的 A/B/C 三档证据口径；
4. [10D](10d-environment-gated-test-spec.md) §4 —— 门控环境准备与三个坑；
5. `tools/platform-reproduction/` 与 `.github/workflows/platform-reproduction.yml` —— 跨平台复现入口与其最近状态；
6. [12](12-tests-and-milestones.md) O6 —— 收口对账的权威五条。

### 1.2 星崽补充的宿主条件（2026-10-07）

| 宿主 | 可用性 | 能拿到的证据 |
| --- | --- | --- |
| Windows 本机 | 已有 | 原生门控本机实跑（此前几批一直在用） |
| **Linux 裸机（带图形化）** | **星崽提供** | 裸机 `reproduce.sh native`；**Linux 桌面开窗**（Xvfb 之外的真实图形会话） |
| macOS | **只有 CI** | `platform-reproduction.yml` 的 macos arm64 矩阵任务 |

**这条直接影响三件事**：

1. 19 O6 条件 3 的「三平台一致」第一次有了**裸机 Linux** 的可能。`tools/platform-reproduction/README.md`
   明确写「Docker/WSL 结果是功能证据，不能冒充裸机验收」——星崽的机器正好补上这一格；
2. 19C 的 `19.14` C 档里「Linux 桌面开窗」从「没有宿主」变成**可验证**；
3. macOS 仍然只有 CI，而 **CI 会显式跳过真实终端用例**：`platform-reproduction.yml` 对 macos-arm64 设
   `XIAO_SKIP_REAL_TERMINAL_TEST=1`，`reproduce.sh` 据此 `--skip real_terminal_session_is_environment_gated`
   并打印「macOS CI 无 GUI 会话，显式跳过」。所以 **macOS 的终端/诊断窗口这一类证据在 CI 上拿不到**，
   不要把它算作已覆盖（§2.4）。

### 1.3 现状盘点（2026-10-07，审核实测）

| 环节 | 现状 | 判定 |
| --- | --- | --- |
| 原生 CI 门控 | 新作业已上线并通过「故意让它红」验证（`37608705803` 失败、`37609550782` 恢复、`37614663496` 全绿）；**只覆盖 Windows** | 复核有效，本批扩（§2.2） |
| `platform-reproduction` | 最近一次是 **10-05 的定时红灯**（`57d49476`），那之后的修复（D1 等）从未在该工作流上跑过，**状态已过期** | 本批刷新（§2.2） |
| 剩余释放差异 | 四处登记：`selector-range` / `selector-random` / `selector-open-range`（范围两条定位到 `selector_bounds` 的借用转换，随机走 `value_select` 未定位）、`held-table-instance`（VM 6 事件 / 原生 4，构造与初始化临时引用待逐调用核对） | 本批收（§2.3） |
| 19 O6 条件 2 | 未满足：上述四处仍是 `Drops` 豁免 | 本批处理 |
| 19 O6 条件 3 | 未验证：只有 Windows | 本批部分（§2.2） |
| `table-user-drop` | 原生因表方法 ABI 拒绝，已有独立测试**钉住**（既断言 VM 基线，又断言降级必须以「动态表方法」失败） | 保留，A1 处理（§2.6） |

### 本批边界

| 子任务 | 内容 | 本批 |
| --- | --- | --- |
| M1–M4 | 本批审核返工项（§2.1） | **做**，每项单独提交 |
| 跨平台证据 | Linux 裸机 + macOS CI + Windows 门禁的分工（§2.2） | **做** |
| 剩余释放差异 | 四处 `Drops` 豁免的逐调用定位与收口（§2.3） | **做** |
| `19.14` C 档 | Linux 桌面开窗（星崽机器）；macOS 如实标不可得（§2.4） | **做 Linux 那一格** |
| 引用计数核对常态化 | 10R 待定决策 3 | **由星崽定**（待定决策 2） |
| A1 表方法 ABI | 剩余拒绝面的第一批 | **只做准备**，实现留下一批（§2.6） |
| 静态溢出 | `ir.rs` 的 7 处 `llvm.trap` | **不做**（已有立项文档） |

## 二、必须先冻结的 6 条

### 2.1 **M1–M4：本批审核发现的返工项**

| 编号 | 问题 | 依据 | 处置 |
| --- | --- | --- | --- |
| M1 | **原生门控只覆盖 Windows**，而 Windows 恰好是已由本机反复验证过的宿主；真正的跨平台盲区（Linux、macOS）没有门 | 读工作流；10R 待定决策 1 | 见 §2.2 |
| M2 | **`platform-reproduction` 的状态已过期**：最近一次是 10-05 的红灯，当时包含此后已修的 D1 回归；它又是 macOS **唯一**的证据来源 | `gh run list`；`57d49476` 之后的修复记录 | 手动触发一次刷新，把四平台结果与运行号回填（§2.2） |
| M3 | **四处 `Drops` 豁免仍未收口**，其中 `held-table-instance` 与随机选择两项**连定位都没有**（前者「构造/初始化临时引用待逐调用核对」，后者「未逐调用定位」） | 读 `native_gap` 的 reason 与 10R 第九节 | 见 §2.3 |
| M4 | **不要把 macOS 的 CI 绿当成终端/窗口已覆盖**：CI 对 macos-arm64 显式 `--skip real_terminal_session_is_environment_gated` | 读工作流 env 与 `reproduce.sh` 的跳过分支 | 见 §2.4 |

### 2.2 **跨平台证据：三种宿主，三种分工，写清各自能证明什么**

**冻结**：

1. **Windows**：本机 + 10R 新增的 CI 门禁（已通过故意失败验证），维持不变；
2. **Linux 裸机（星崽的机器）**：**按 [10S-Linux 裸机交接](10s-linux-bare-metal-handoff.md) 执行**——
   那份档写明了前置包、工具链版本、要跑的命令、要记录的环境与回传模板；
   要点是：不设 `XIAO_USE_XVFB`（真实图形会话）、不改源码、失败也照实回传。
   记录必须**明确标注这是裸机证据**——只有它能填进 19 O6 条件 3，Docker/WSL 不行；
   落点写进本批实施记录，并与 10R 的 CI 证据并列（不要混为一谈）；
3. **macOS**：只有 CI。手动触发 `platform-reproduction.yml`（与 M2 的刷新合并成一次运行），
   取 macos-arm64 的日志；**必须同时记录「真实终端用例被显式跳过」这一事实**，
   不能只写「macOS 绿」；
4. Linux CI 的 `XIAO_USE_XVFB=1` 用的是虚拟显示——**它不能替代星崽机器上的桌面证据**，
   两者在记录里分开；
5. 是否把 Linux 加进 10R 那个随 push 的原生门控作业：**由星崽定**（待定决策 1）。
   注意 GitHub 的 `ubuntu-24.04` 与星崽机器的发行版不同，加了也只是多一档 CI 证据，不等于裸机验收。

### 2.3 **剩余释放差异：逐调用定位，然后收口**

10Q/10R 已经证明这条路可行——把范围差异定位到 `selector_bounds` 的借用转换就是靠对照实验 + 逐调用数事件。
剩下四处按同一方法做。

**冻结**：

1. **口径沿用 10Q**：事件数 / 对象数 / 销毁位置；轨迹只记释放与销毁、不记 retain，
   **不得**仅凭总数相等就宣布引用平衡；
2. **对照实验是主要手段**：对可疑调用做「打开/关闭」的成对测量，看事件数的变化是否与预期一致，
   像 10Q 对 `selector_bounds` 那样。读数写进数据表，不能只写结论；
3. **判定分两类**并分别处置：真失衡（重复/漏释放、提前销毁）要修；追踪口径差异要写明
   **为什么 VM 那侧是基准**，并保留豁免 + 具体原因。此前 10P 对选择器的判断属于后者，
   本批要把「为什么」补到机制层面；
4. 收口后摘掉对应的 `native_gap` —— 过期豁免守卫会强制这一步，别绕过它；
5. `held-table-instance` 是四处里唯一**连定位都没有**的一项，优先做；
6. **不得**为了让测试变绿而删除用例、放宽断言或新增豁免字段；也不得改 VM 语义。

### 2.4 **`19.14` C 档：Linux 能做，macOS 要如实标注不可得**

**冻结**：

- **Linux 桌面开窗**：在星崽的图形化 Ubuntu 上验证，**步骤与判据见
  [10S-Linux 裸机交接](10s-linux-bare-metal-handoff.md) §五**；要的是**行为证据**（用例通过、窗口肉眼可见、
  截图、关窗后进程正常结束），不是只断言字符串——这与 19C §2.5 对 B 档的要求同一条；
- **macOS LaunchServices / 真实终端**：**本批无法取得证据**。CI 显式跳过真实终端用例，且没有 Mac 宿主。
  文档与主表必须继续写「未验证」，并写明**原因**（宿主缺失 + CI 跳过），
  不得用 macOS 矩阵任务的绿来暗示已覆盖；
- 两条结论分别回填 [19C](19c-maintenance-and-debug.md) 的 `19.14` 行与 18C 的 `18.13` 行。

### 2.5 **19 的 O6 条件 2、3 的收口口径**

**冻结**：

- 条件 2 要等 §2.3 的四处差异有了结论（修掉或判定为口径并写明依据）之后才可能满足；
  **本批不得在结论未出之前宣布条件 2 满足**；
- 条件 3 需要三平台证据：Windows（已有）、Linux 裸机（本批取得）、macOS（只有 CI 且存在跳过项）。
  因此条件 3 在本批之后**大概率仍是「部分」**——按 [12](12-tests-and-milestones.md) O6 五条原文逐条写，
  写清每一格缺的是什么；
- 对账表落在 [19D](19d-performance-comparison.md)（10P 已把表放那里），本批只更新行内容；
- **禁止**用「三平台都跑过了」这类笼统说法掩盖 macOS 的跳过项。

### 2.6 **A1 表方法 ABI：只做准备，不动手**

按 10P/10R 的 ABI 顺序，A1（函数表与生命周期）是第一顺位，它同时卡住 `container-dense` 的性能对照、
`table-user-drop` 的原生路径和 19.16。

**冻结**：

- 本批只产出**动手前的清单**：现有 `IrTableSignature` / 表定义段与 Runtime 侧缺什么、
  函数表在 ABI 里怎么表示、`drop` 的调用时机、影响的 crate、验收用例（至少含 `table-user-drop` 与
  `container-dense` 的构建与比对）；
- **本批不实现**，不改 `dynamic/container.rs` 的表方法拒绝点；
- 清单要能让下一个 Agent 直接开工，不需要再读一遍 10L/10P 的枚举历史。

## 三、落点

```text
core/rust/crates/xiao-codegen-llvm/src/dynamic/     §2.3 定位与修复（expression/container/release）
core/rust/crates/xiao-driver/tests/d19a_differential.rs   §2.3 摘除登记；§2.2 记录所用用例
core/rust/crates/xiao-runtime/                      §2.3 若定位到 Runtime 侧的重复引用
docs/DevDocs/10s-*.md                               数据表、Linux 裸机记录、A1 准备清单
docs/DevDocs/19c-*.md、18c-*.md                     §2.4 C 档回填（Linux 有证据、macOS 不可得）
docs/DevDocs/19d-*.md                               §2.5 O6 对账按五条更新
tools/platform-reproduction/README.md               仅在需要写清 Linux 裸机记录格式时
docs/DevDocs/README.md                              主表登记
```

## 四、硬约束

1. M 系列、跨平台证据、差异收口、A1 准备各自单独提交，提交说明写明归属 10R，并带正文；
2. 宿主分工**不得混淆**：裸机 Linux、Docker/WSL、CI 虚拟显示、macOS-CI-跳过项，四者在记录里分开写（§2.2）；
3. macOS 的跳过项**不得**被算作已覆盖（§2.4）；
4. 释放差异先定位、先出数据，再判定；不得只凭总数相等宣布平衡（§2.3）；
5. 不得删用例、放宽断言或新增豁免字段；不得改 VM 语义（§2.3）；
6. 条件 2/3 未达成就照实写「部分」，并逐条说明缺什么（§2.5）；
7. A1 本批**不实现**（§2.6）；
8. 不新增依赖，`Cargo.lock` 保持同步；
9. 判定原生行为必须实跑受控环境（含 `XIAO_DIAGNOSTICS_PATH`）；
10. 每个提交单独跑 clippy 与 fmt；推送前跑全量门禁含 `bun run check`。

## 五、分步提交

1. **§2.2 刷新**：手动触发 `platform-reproduction`，取四个平台日志并回填（含 macOS 的跳过说明）；
2. **§2.2 Linux 裸机**：在星崽的机器上跑 `reproduce.sh native`，记录发行版/架构/工具链版本与完整命令；
3. **§2.3 定位**：先做 `held-table-instance`，再做随机选择，最后把范围两条的「为什么 VM 是基准」补到机制层面；
   每项一个提交，附数据表；
4. **§2.3 收口**：能摘的登记摘掉；判定为口径的保留豁免并写明依据；
5. **§2.4 C 档**：Linux 桌面开窗的行为证据；回填 19C/18C，macOS 写「不可得 + 原因」；
6. **§2.5 对账**：按 O6 五条更新 19D 的表；
7. **§2.6 A1 清单**：只出文档；
8. **全量门禁与实跑**：`cargo test --workspace`、clippy、fmt、`bun test`、`bun run check`，
   以及受控环境下的原生 `--ignored`；
9. **推送与观测**：推送后确认原生门控作业仍绿，CI 运行号回填；
10. **文档与登记**：本文件实施记录、README 主表。

## 六、最可能翻车的地方

1. **把 Docker/WSL 或 CI 虚拟显示的结果写成裸机 Linux 验收**——README 明令禁止；
2. **把 macOS 矩阵任务的绿当成终端/窗口已覆盖**，忽略它的显式跳过；
3. **只看事件总数就宣布引用平衡**，不看对象数与销毁位置，也不看是否提前销毁；
4. **为了摘登记而删用例或放宽断言**；
5. **条件 2/3 还没到就宣布满足**，或反过来一直不更新对账表；
6. **A1 顺手开工**，把本批拖成两个批次；
7. **星崽机器上的记录缺版本信息**（发行版、glibc、架构、工具链），导致证据不可复现；
8. **C 档只给文本断言**，没给行为证据；
9. **只看 `cargo test` 就宣布门禁全绿**（`bun run check` 不在其中）；
10. **提交没写正文 / 没单独跑 clippy**。

## 七、验收

1. M1–M4 逐项有结论；
2. §2.2：`platform-reproduction` 有新运行号且四平台结论逐条写明（含 macOS 跳过项）；
   Linux 裸机记录含发行版、glibc、架构、工具链版本与完整命令，并明确标注为裸机证据；
   Linux CI（Xvfb）与裸机证据分开陈述；
3. §2.3：四处差异各有数据表与判定；判定为真失衡的已修并摘除登记，判定为口径的写明机制层面依据；
   `held-table-instance` 与随机选择不再停留在「未定位」；
4. §2.4：Linux 桌面开窗有行为证据；macOS 写「不可得」并给原因；19C/18C 已回填；
5. §2.5：19D 的 O6 表按五条原文逐条更新，条件 2、3 的状态与缺口写清楚；
6. §2.6：A1 准备清单可直接开工，且本批未动实现；
7. 受控环境下三组原生 `--ignored` 按实际结果回填，写明跑了哪几条、哪个提交、哪个平台；
8. `cargo test --workspace`、clippy、fmt、`bun test`、`bun run check` 全绿，且每个提交单独绿；
9. 没有通过放宽断言、删用例、缩小范围或新增豁免字段换来的绿。

## 八、不负责与不要重复做的事

- **不实现** A1 表方法 ABI 与其余剩余拒绝面（只出准备清单，§2.6）；
- **不实现**静态路径 `llvm.trap`（已有立项文档）；
- **不做**受控 Java 基线取数与性能阈值；
- **不做** 20C 官方 Xiao 库、20D `os` 与平台资源；
- **不改** VM 的选择器与释放语义；
- **不新增**豁免字段、不新增工作流文件。

## 待定决策

1. **是否把 Linux 加进随 push 的原生门控作业**（10R 待定决策 1 的延续）。现在多了「星崽有裸机 Linux」
   这一条件：加了只是多一档 CI 证据，**不等于**裸机验收；不加则 Linux 仍只在定时/手动时覆盖。
   建议：加，但记录里与裸机证据分开写。
2. **引用计数核对是否常态化**（10R 待定决策 3）：本批收口之后，是否让原生差分每次都输出事件数摘要
   作为常规门禁，还是保持一次性审计。
3. **`llvm.trap` 立项的排期**（10R 待定决策 4）：排在 A1 之前还是之后。
4. **macOS 的长期安排**：星崽没有 Mac 宿主，CI 又跳过真实终端——若 19.14 的 macOS 那一格长期无法验证，
   是否在 19 的收口标准里把它明确标注为「不可验证」并说明理由，而不是一直挂「未验证」。

## 相关页面

- [10R. 释放账目收口与原生 CI 门控](10r-release-accounting-and-native-ci-gate.md) —— 上一批；数据表与 CI 证据
- [10Q. 用例区分度与释放账目核对](10q-selector-case-strength-and-release-audit.md) —— 审计口径与对照实验
- [19C. 维护策略与调试三平台](19c-maintenance-and-debug.md) §2.5 —— `19.14` 的 A/B/C 三档口径
- [10D. 环境依赖测试专项规范](10d-environment-gated-test-spec.md) §4 —— 门控环境准备
- [12. 测试与开发里程碑](12-tests-and-milestones.md) O6 —— 收口对账的权威条目
- `tools/platform-reproduction/README.md` —— 各平台证据的定位与边界

## 九、首轮跨平台与裸机证据（2026-10-07）

手动运行 [37631381063](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/37631381063) 使用0ae4850。Windows失败于旧数组产物符号断言；Linux amd64/arm64及macOS arm64均失败于通用值复制导致weak组件可达但未登记。macOS日志明确跳过真实终端用例；Linux CI设置XIAO_USE_XVFB=1，只是虚拟显示功能证据。不得把汇总作业成功当作四平台成功。

星崽通过 [PR #3](https://github.com/Programming-Language-Xiao/Xiao/pull/3) 回传Ubuntu裸机结果；本批原样接入ce6dc578的两份证据文件，不修改原始日志、不将PR视为已合并：
[环境与结果](10s-linux-bare-metal-results-20261007.md)、[完整日志](10s-linux-bare-metal-log-20261007.md)。Ubuntu26.04.1/x86_64/glibc2.43，Rust1.96.0、Bun1.4.0、LLVM21.1.8，Wayland桌面、DISPLAY=:0，未设Xvfb；直接执行脚本退出101，13条原生差分构建因weak未登记失败，未到打包。真实终端测试返回ok，但操作者未看到窗口、无截图，Linux C档仍未验证通过。它是真实裸机失败证据，不是裸机验收通过。

交接命令已修正：管道执行通过PIPESTATUS[0]保存脚本退出码；rustc -vV保留完整host信息。PR实际命令使用直接重定向，退出101未被tee掩盖。

### 第二轮 CI 与不可达调用登记修复

[37634885776](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/37634885776)（77d1a24）：Windows 通过；Linux amd64/arm64、macOS arm64 均在 function-heap-return-error 与 function-finally-raises 失败，错误为「产物未观察到 IR 层登记的 Runtime 组件：weak」。上一轮的 13 项漏登记已消除，但正常返回死块中的 value_copy 被错误计入依赖；finally 必定抛错使这些块无入口路径，LLVM 删除后产物不含 weak。

修复将复制 ABI 的传递依赖登记移到完整模块生成后，按各函数入口和 br 边遍历可达块；显式弱引用计划仍独立登记。不强制链接无用组件，不放宽产物校验。回归覆盖两条死返回路径，以及抛错前参数复制仍需 weak 的反例。Windows 受控原生差分、Rust workspace、fmt、Clippy 通过；跨平台结果以后续运行记录为准。macOS 真实终端仍显式跳过，Linux CI 仍为 Xvfb。

第三轮 [37638188213](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/37638188213)（f42f2c7）的 Linux 已通过差分，随后四条原生基准构建探针失败：这些源码只定义函数、未调用，函数内部复制被计入依赖，而链接器移除了未调用函数。最终实现统一从 main 的 entry 出发，同时遍历直接调用和基本块跳转；访问键为函数名与标签，覆盖循环/递归而不串用同名标签。新增未调用函数、被调用函数和递归的正反例；不以强制保留符号或降低构建探针断言消除红灯。

## 十、表实例逐调用核对

以下成对实验在 Windows x86_64-pc-windows-msvc、77d1a24 基础上进行，使用本机 clang/llvm-as、release Runtime 和诊断渲染器，运行受控 d19a_differential。每次仅修改表中所列一处，采数后恢复；实验变更未并入生产代码。事件序号从 0 开始，包含 strong_release 与 destroy，不包含 retain。

| 对照 | VM 事件/对象/destroy 位置 | 原生事件/对象/destroy 位置 |
| --- | --- | --- |
| 原用例：Item.value = 7 | 6 / 1 / 5 | 4 / 1 / 3 |
| 仅跳过原生初值 setter | 6 / 1 / 5 | 3 / 1 / 2 |
| 仅跳过 VM 字段函数 execute（保留 receiver） | 4 / 1 / 3 | 4 / 1 / 3 |
| 双方增加第二个标量字段 other = 8 | 7 / 1 / 6 | 5 / 1 / 4 |

Runtime tables::initialize 的 callback_view 克隆与归还是双方共有的一次临时释放。VM semantics/tables.rs 构造 receiver 时克隆实例，再把参数复制进字段函数帧；字段 set 读取 receiver 产生逐字段临时引用。原生 table_set 则通过 clone_strong 临时持有实例并在写入结束归还。双方每增加一个字段都增加一次释放；固定差二来自 VM 的 receiver 与字段函数参数帧。跳过字段函数减少参数帧和字段读取两次，跳过原生 setter 减少一次，与逐调用机制一致。

判定：当前用例是适配层临时持有的追踪口径差异，保留仅 Drops 豁免并更新原因。VM 字段函数执行及其生命周期是既有语言语义基准，不能为了事件数一致改 VM 或给原生人造持有。双方对象仅销毁一次且发生在最后释放后；该结论结合所有权调用链与成对实验，不从总数相等推导一般性无泄漏，也不宣称表用户 drop 已覆盖（仍受 A1 拒绝）。

## 十一、随机选择逐调用核对

环境、临时实验恢复和序号口径同 §十；对象 1 是源数组，对象 2 是选择结果。

| 对照 | VM 事件/对象/destroy（序号:对象） | 原生事件/对象/destroy（序号:对象） |
| --- | --- | --- |
| 原用例 ?2 | 10 / 2 / 7:2、9:1 | 11 / 2 / 8:2、10:1 |
| 抽样改 ?1，并同步预期和为 3 | 8 / 2 / 5:2、7:1 | 10 / 2 / 7:2、9:1 |
| ?2，仅关闭原生 iterable 检查 | 10 / 2 / 7:2、9:1 | 10 / 2 / 7:2、9:1 |

VM SelectorApply 读取源一次，再对每条选中路径通过 read_ir_path/index_get 克隆源一次；?2 的源前缀有三次释放，?1 有两次。原生 value_select 内部 value_to_runtime 只克隆源一次并批量选取，加上调用方 load_slot 的一次持有，总是两次源释放。抽样数量实验确认这一差别；随机路径不经过 selector_bounds。

循环结果侧，VM Len 一次、两轮 IndexGetDynamic 各一次、绑定退出一次，共四次释放；原生多持有一份循环源 SSA（循环结束归还），并经 iterable 动态检查做一次借用转换，共六次。关闭 iterable 检查恰好减少一次结果对象释放。因此原生源侧少一、结果侧多二，净多一。最后一行总数和销毁位置相同，但源前缀仍是 VM 三次、原生两次，完整轨迹并不相同，不能据此摘除豁免。

判定为当前适配层的临时引用口径差异，保留仅 Drops 豁免；输出、错误、退出码等仍严格比较，过期豁免守卫保留。VM 逐路径选择与循环读取是既有语义基准，不以原生批量 ABI 的实现反改 VM。两对象各销毁一次、结果先于源销毁；这不是对未测路径或 A1 生命周期的普遍保证。

## 十二、范围与开区间逐调用核对

环境同 §十。范围 1~2 与开区间 <2 都选中两个元素，下表读数对两例分别成立；仍分别校验各自聚合结果（5 与 3）。

| 对照 | VM 事件/对象/destroy（序号:对象） | 原生事件/对象/destroy（序号:对象） |
| --- | --- | --- |
| 原用例 | 10 / 2 / 7:2、9:1 | 13 / 2 / 10:2、12:1 |
| 仅关闭原生 selector_bounds 检查 | 10 / 2 / 7:2、9:1 | 12 / 2 / 9:2、11:1 |
| 仅关闭原生 iterable 检查 | 10 / 2 / 7:2、9:1 | 12 / 2 / 9:2、11:1 |

原生 bounds 检查借用转换克隆源并归还，使源对象前缀从三次变为四次；关闭后恢复三次。循环结果侧机制与 §十一相同：额外的 SSA 持有和 iterable 检查各归还一次临时引用，关闭后者减少一次结果释放。源侧多一、结果侧多二，解释全部三次差值；两种关闭实验虽然总数相同，所减少的是不同对象的释放。

VM 使用既有选择器读取与循环指令语义，因此继续作为可观察语义基准；原生适配 ABI 的额外借用转换不能倒逼 VM 改写生命周期。判定当前两例为追踪口径差异，保留仅 Drops 豁免并写入具体机制，四例均不再登记「未定位」。不添加豁免字段，不削弱输出或完整轨迹比较，不宣称 O6 条件 2 全部满足。

## 十三、本机验证与剩余输入

26bc75b 实现的 Windows x86_64-pc-windows-msvc 验证：Rust workspace 全量、workspace all-targets Clippy（-D warnings）、fmt、bun run check 通过；本轮 Bun 测试 287 通过、5 跳过。受控环境同时指定 XIAO_TARGET_TRIPLE、XIAO_CLANG、XIAO_LLVM_AS、XIAO_LLC、XIAO_STRIP、XIAO_RUNTIME_LIBRARY 与 XIAO_DIAGNOSTICS_PATH，运行 n0_a_native_driver 三项、n0_b_dynamic_native 一项、d19a_differential 一项、native_benchmark_probe 一项，共六项 ignored 测试通过。构建探针仍为四例可构建、container-dense 因表方法 ABI 拒绝，不是性能通过。

M1 保持既有 Windows push 门控，未获新增 Linux push 门控的决定；Linux/macOS 通过现有定时/手动矩阵取证。M2 刷新暴露的组件登记问题已修，运行结果逐次留存；M3 四处差异的机制与对照数据见 §十至十二，仍仅豁免 Drops；M4 继续明确 macOS 无桌面宿主且 CI 跳过真实终端。A1 [开工清单](10s-a1-table-method-abi-preparation.md)已完成，本批未实现。

Ubuntu 裸机修复版复跑待星崽回传；先前 PR #3 的失败原文保留。Linux 可见窗口、截图与关窗后退出证据仍缺，macOS 真实窗口本批不可得；19C/18C 和 19D O6 已同步，不宣布 10S 全部验收或 19 收口。引用计数常态化、静态溢出排期与长期 macOS 标准仍按待定决策保留。
