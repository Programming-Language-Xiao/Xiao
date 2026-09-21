# 09-B0-D. 退出码冻结与 Linux 容器实测交接文档

> **这是 B0 的收口补丁批次。** 09-B0-C 完成后，`12-tests:674-679` 的 B0 退出条件第 4 条
> 「内部驱动器能以结构化结果验证成功、错误、**退出码**和诊断事件」**只剩退出码一项没落实**。
> 本文把它拉回 B0 并冻结，另附一次 Linux 容器实测。
>
> 权威依据见第一节；本文**不新增要求**，是**纠正一处误判**。

> **落地状态（2026-09-21）**：退出码契约、公共测试和 Linux 容器门禁均已完成。Linux
> 结果只证明开发环境中的代码可运行，不构成跨平台验收，也不进入 09R3 冻结数字。

## Agent 交接上下文

### 接手前提

1. [09-B0-C. 前端到 VM 内部驱动器](09b0c-frontend-to-vm-driver.md) —— **上一批**，
   本批在它的 `DriverOutcome` 上补退出码，**不重做**它的三段设计。
2. [00A. 工程框架与目录布局](00a-project-layout.md) `:127` 与 `:135` —— **本批的权威依据**，
   退出码是跨语言边界必须携带的字段。
3. [12. 测试与开发里程碑](12-tests-and-milestones.md) `:674-679` —— B0 退出条件原文，
   第 4 条点名退出码。
4. [07. 错误模型与并发安全边界](07-concurrency-and-errors.md) `:175`、`:284` —— locale
   不改变退出码（两处）。
5. [09. 字节码运行模式](09-bytecode-runtime.md) `:175`、[11. CLI、项目配置与平台](11-cli-config-and-platform.md)
   `:239`、`:276` —— 同样要求 locale 不改变退出码。
6. [09R2D. 两种机型与指令编码器交接文档](09r2d-machines-and-encoder.md) —— 开发规定主表。

### 本批交付与不负责

**交付**：`ExitCode` 的语义与取值冻结、`DriverOutcome` 上的稳定派生、locale 中立性断言，
以及一次 Linux 容器实测记录。

**不负责**：TypeScript CLI 的进程退出码映射与 CLI 自身错误（11/X0）、诊断窗口（X0）、
多模块（D/E）、`.xiaoc`（14）。

---

## 一、权威依据：退出码**本来**就该在 B0

### 1.1 两处直接依据

`00a-project-layout.md:135`：

> 进程协议还是库 ABI 由**第 09/11 阶段**另行冻结。无论最终选择哪一种，**边界都必须包含**：
> 请求版本、语言/Runtime 版本、目标条件、优化配置、源码/模块身份、结构化错误、**退出码**、
> 取消和诊断事件。不得以解析人类可读 stdout 代替协议字段。

`00a-project-layout.md:127`：

> `cli/ts/src/diagnostics` | 结构化错误转终端显示、颜色和**退出码**

第二条把它讲得很清楚：**核心提供退出码语义，CLI 负责转成终端显示**。两处合起来是
「核心边界携带退出码字段」+「CLI 把它映射到进程退出码」。

`12-tests` 的 B0 退出条件第 4 条再确认一次：内部驱动器要能验证「成功、错误、**退出码**
和诊断事件」。

### 1.2 这是一处**误判**，要诚实记账

B0-B 的交接文档（[09-B0-B](09b0b-production-vm.md) §4）建议「本批只冻结结构化结果的语义
边界，**具体整数值留给 11/X0**」，B0-B 采纳了这个建议，B0-C 沿用，于是**四处文档都记着
「整数退出码留给 11」**。

**按上面两处权威依据，这个建议是错的**：`00a:135` 把退出码列为第 09 阶段就要冻结的边界
字段，`00a:127` 只把**终端映射**留给 CLI。本批纠正它。

**纠正时要连带更新的四处**（不要只改一处，那是单一来源的反面）：

| 位置 | 现在写的 |
| --- | --- |
| `09b0b-production-vm.md` §4 第 2 条 | 「建议本批只冻结…整数留给 11/X0」 |
| `09b0b-production-vm.md` 现状盘点表 | 「退出码 ❌ 全仓未定义（有意为之）」 |
| `09b0c-frontend-to-vm-driver.md` 现状盘点表 | 同上 |
| `09b0c-frontend-to-vm-driver.md` §3 / 落地记录 | 「整数退出码仍不冻结，继续留给 11/X0」 |

**改法**：保留原文并加一条**修正批注**（本仓既有做法，见 09-B0 §2.1 的 2026-09-21 修正），
而不是把旧文字删掉——删掉会让"为什么曾经这样想"无从追溯。

---

## 二、要冻结的五个值

**已定（2026-09-21）**：五个取值，与 `DriverOutcome` 的三段 + `RunResult` 的三分支组合出的
**五种终局一一对应**。

| 值 | 终局 | 对应现有类型 |
| --- | --- | --- |
| `0` | 成功（**含被 `catch` 消费的可恢复错误**） | `Executed` + `RunResult::Success` |
| `1` | 源码未通过检查 | `Frontend(FrontendError)` |
| `2` | 产物被拒绝执行（**含取消与超时**） | `Rejected(DriverError)` |
| `3` | 运行时未捕获的可恢复错误 | `Executed` + `RunResult::Error` |
| `4` | 致命故障 | `Executed` + `RunResult::Fatal` |

### 2.1 三条必须写进文档注释的判据

1. **被 `catch` 消费的错误是 `0`，不是 `3`**。`RunResult::Error` 指的是**未捕获**、
   一路冒到入口的可恢复错误。这条最容易写反，且写反了会让"程序自己处理好了错误"
   和"程序崩了"在退出码上不可区分。
2. **取消与超时归 `2`**，与其它"产物被拒绝执行"共用。它们已经是 `Rejected` 段的
   `X09-DRIVER-001/002`，退出码层面不必再分——细分信息在结构化诊断里。
3. **`Fatal` 是 `4` 而不是"更严重的 3"**。`Fatal` 不可被普通 `catch` 恢复、也不执行释放
   计划（B0-B 已验证），它是**另一条终止路径**，不是一个错误等级。

### 2.2 取值区间

五个值都在 POSIX 的 `0..=255` 内。**不要**在 B0 冻结任何 `>255` 的取值——
进程退出码在多数平台上会被截断成低 8 位，那会让"定义了一个到不了的值"。

---

## 三、落点与形状

### 3.1 定义在 `xiao-driver`

理由：**只有驱动器能同时看到三段**。

- `xiao-vm` 只看得到 `RunResult` 的三分支，看不到前端失败与验证拒绝；
- `xiao-diagnostics` 看得到 `XiaoError` / `FatalError`，但看不到 `DriverOutcome` 的段划分；
- `xiao-driver` 的 `DriverOutcome` 是**唯一**同时掌握三段的类型。

所以 `ExitCode` 与 `DriverOutcome::exit_code()` 都放 `xiao-driver`。

**建议形状**（细节可调，但下面两条硬要求不能变）：

```rust
pub enum ExitCode { Success, SourceRejected, ArtifactRejected, RuntimeError, Fatal }
impl ExitCode { pub const fn as_process_code(self) -> u8 { /* 0..=4 */ } }
impl DriverOutcome { pub fn exit_code(&self) -> ExitCode { /* 覆盖全部三段 */ } }
```

**硬要求**：

1. **枚举而不是裸 `u8`**。裸数字会让 `as_process_code` 的映射散落到每个消费方，
   而且 `?` 一个 `u8` 没有任何类型保护。
2. **`exit_code()` 对五种终局必须全覆盖**，`match` 不能有兜底 `_`——B0-A/B0-B 的
   穷尽性守卫是同一手法，本批照做。

### 3.2 它是**派生**，不是第二套诊断

退出码从**已有的结构化结果**派生，**不得**另立一套判断逻辑。
具体地说：`exit_code()` 只读 `DriverOutcome` 的段与 `RunResult` 的分支，
**不得**去读 `diagnostic.code()` 的字符串前缀、不得读 `message_id`、不得读任何文本。

这条同时满足 09 文档 `:120` 的「避免 TypeScript 通过解析人类可读文本来判断成功或失败」。

### 3.3 locale 中立性要有断言

四处权威文档要求 locale 不改变退出码（`07:175`、`07:284`、`09:175`、`11:276`）。
B0 还没有 locale 机制，但**断言现在就能写**：退出码只依赖 `DriverOutcome` 的结构，
而结构与诊断文本无关——写一条测试把"同一程序、两种诊断渲染"的退出码钉住。

**不要**因为"反正现在没有 locale"就跳过它：等 11C 接进来时再补，就是给漂移留窗口。

---

## 四、与 11 阶段的分工（**单一来源**）

**已定（2026-09-21）**：**B0 冻结语义与取值；11 只负责进程退出码映射与 CLI 自身错误。**

具体的分工边界：

| 归属 | 内容 |
| --- | --- |
| **B0（本批）** | 五个值的**语义与数字**、`ExitCode` 类型、`DriverOutcome::exit_code()` 的派生规则 |
| **11 / X0** | 把 `as_process_code()` 的结果写进 **进程** 退出码；CLI **自身**错误（参数解析失败、核心启动失败、版本失配）的退出码 |

**11 不得重新定义这五个值。** 若 11 需要新值（例如"CLI 参数错误"），
**新增而不改既有**——与 opcode 表的"只能追加不得重排"同一条理由。

**本批要在 11 阶段文档里留一条注记**（`11-cli-config-and-platform.md:229` 附近，
那里写着"实现命令解析、帮助、版本和错误退出码"），写明取值已由 B0 冻结、11 只做映射。
**否则两处各自定义一份，就是本仓登记过 7 次的 A 类病。**

---

## 五、Linux 容器实测

**已定（2026-09-21）**：构建镜像 + 跑全量门禁（含 benchmarks 独立 crate）+ 落一份 DevDocs 记录。

### 5.1 镜像

`Dockerfile.dev` 已就绪并已按固定版本构建（Rust 1.96.0 + Bun 1.4.0）。它的注释里已有
构建与运行命令，照抄即可，注意两个已知坑；本次构建耗时记录见「落地记录」：

```text
docker build -f Dockerfile.dev -t xiao-dev .

# Git Bash 下必须加 MSYS_NO_PATHCONV=1，否则 -v 的路径会被 MSYS 转换掉
MSYS_NO_PATHCONV=1 docker run --rm -it \
  -v "$PWD":/w -v xiao-target:/w/core/rust/target -w /w xiao-dev bash
```

- **`target/` 用具名卷**（`xiao-target`），不要落在绑定挂载上——仓库在 Windows 盘时
  每次编译都要付跨文件系统开销。
- **首次进容器先 `bun install`**：`node_modules` 在绑定挂载上，不进镜像。

### 5.2 要跑什么

```text
cargo test --manifest-path core/rust/Cargo.toml --workspace
cargo clippy --manifest-path core/rust/Cargo.toml --workspace --all-targets -- -D warnings
cargo fmt --manifest-path core/rust/Cargo.toml --all -- --check
cargo doc --manifest-path core/rust/Cargo.toml --workspace --no-deps
cargo check --manifest-path tests/benchmarks/Cargo.toml      ← 独立 crate，第三次点名
bun test
bun run check
bun run check:coverage
```

### 5.3 边界：**容器不进验收**

09R3 `:135` 写死了：

> **WSL 与容器只当开发环境，不进验收**。…WSL 共享宿主 CPU 与调度、容器多一层文件系统，
> **其数字不能与 Windows 原生并列比较**。

所以本批的容器实测：

- **是**：Linux 上代码是否绿的一次开发环境验证；
- **不是**：跨平台验收，**不得**据此宣称"跨平台已验证"；
- **不得**把任何容器里的数字写进 `tests/benchmarks/reports/`（那是 09R3 的冻结报告）；
- **不得**跑的性能数字与 Windows 原生并列。

### 5.4 落盘

落一份 DevDocs 记录（本文的「落地记录」章节即可，**不新建文件**），内容至少含：

1. 镜像构建结果与耗时；
2. 八项门禁各自的通过/失败与用例数；
3. **与 Windows 原生的差异**（若有用例数或覆盖率差异，必须解释；没有差异也要写"无差异"）；
4. 遇到的坑（环境相关，例如 `bun install` 的网络、卷挂载、PATH）。

**若门禁在容器里失败**：修，但**如实记录失败原因**。**不得**为了让容器绿而放宽门禁、
跳过用例或改语义——那正是 09R2D 记录的假绿形态。

## 落地记录（2026-09-21）

### 退出码实现

`core/rust/crates/xiao-driver/src/run.rs` 新增公开 `ExitCode` 枚举和
`DriverOutcome::exit_code()`。映射只匹配驱动器三段与 VM `RunResult` 三分支：

| 结构化结果 | `ExitCode` | 进程码 |
| --- | --- | ---: |
| `Frontend(...)` | `SourceRejected` | 1 |
| `Rejected(...)`（含取消、超时） | `ArtifactRejected` | 2 |
| `Executed(Success)`（含 `catch` 消费的错误） | `Success` | 0 |
| `Executed(Error(...))` | `RuntimeError` | 3 |
| `Executed(Fatal(...))` | `Fatal` | 4 |

`exit_code()` 不读取诊断 `code`、`message_id`、消息文本或 locale 展示结果；新增的
`b0_d_exit_codes.rs` 用真实 Xiao 源码经过 `FrontendCompiler → lower_program →
verify_for_execution → run_request` 覆盖了五种终局、脚本与 `[main]`、取消/超时、损坏 IR、
`catch` 成功和展示上下文变化。`ExitCode::as_process_code()` 的五个数值也有逐项断言。

### Windows 原生基线

以下是同一工作树在 Windows 原生环境的基线，Rust 1.96.0、Bun 1.4.0；此前 B0-C 门禁已全绿，
B0-D 定向测试和本批最终门禁再次复核。非测试命令的“用例数”列标为不适用，不把耗时当作
性能基准。

| 门禁 | 结果 | 用例/覆盖率 |
| --- | --- | --- |
| `cargo test --manifest-path core/rust/Cargo.toml --workspace` | 通过 | 588 通过，0 失败，0 忽略 |
| `cargo clippy --manifest-path core/rust/Cargo.toml --workspace --all-targets -- -D warnings` | 通过 | 不适用 |
| `cargo fmt --manifest-path core/rust/Cargo.toml --all -- --check` | 通过 | 不适用 |
| `cargo doc --manifest-path core/rust/Cargo.toml --workspace --no-deps` | 通过 | 不适用 |
| `cargo check --manifest-path tests/benchmarks/Cargo.toml` | 通过 | 独立 crate 编译通过 |
| `bun test` | 通过 | 32 通过，0 失败 |
| `bun run check` | 通过 | 仓库检查通过 |
| `bun run check:coverage` | 通过 | 总体 4573/4573（100%），公共 API 2363/2363（100%） |

### Linux 容器实测

镜像命令：`docker build -f Dockerfile.dev -t xiao-dev .`，使用 Docker Desktop
`desktop-linux`，构建成功，耗时约 **2.97 秒**。按文档使用具名卷
`xiao-target:/w/core/rust/target`；镜像内版本为 Rust 1.96.0、Bun 1.4.0。首次进入容器执行
`bun install` 成功，耗时约 **1 秒**。

| 门禁 | Linux 容器结果 | 用例/覆盖率 |
| --- | --- | --- |
| `cargo test --manifest-path core/rust/Cargo.toml --workspace` | 通过（约 17 秒） | 588 通过，0 失败，0 忽略 |
| `cargo clippy --manifest-path core/rust/Cargo.toml --workspace --all-targets -- -D warnings` | 通过（约 4 秒） | 不适用 |
| `cargo fmt --manifest-path core/rust/Cargo.toml --all -- --check` | 通过（约 9 秒） | 不适用 |
| `cargo doc --manifest-path core/rust/Cargo.toml --workspace --no-deps` | 通过（约 8 秒） | 不适用 |
| `cargo check --manifest-path tests/benchmarks/Cargo.toml` | 通过（约 19 秒） | 独立 crate 编译通过 |
| `bun test` | 通过（约 7 秒） | 32 通过，0 失败 |
| `bun run check` | 通过（约 32 秒） | 仓库检查通过 |
| `bun run check:coverage` | 通过（约 7 秒） | 总体 4573/4573（100%），公共 API 2363/2363（100%） |

首次自动化尝试用 `bash -lc` 启动容器，登录 shell 覆盖了 `/usr/local/cargo/bin`，导致五个
Cargo 门禁返回 `127 (cargo: command not found)`；依赖 Rust AST 适配器的 `bun test` 有 4
项失败，`bun run check` 与覆盖率也如实失败。按 `Dockerfile.dev` 的交互式 `bash` 语义改为
非登录 shell 后，PATH 恢复，上表八项门禁全部通过；没有跳过用例、放宽参数或修改门禁。
`node_modules` 位于绑定挂载，故先执行 `bun install`；target 使用具名卷，挂载和 PATH 均在
第二轮实测中确认。

与 Windows 原生结果相比，Rust workspace 和 Bun 用例数、失败数及覆盖率**无差异**；耗时受
宿主调度、文件系统和缓存影响，不能作为 09R3 性能数字。Linux 容器仅作为开发环境验证，
Linux/macOS 仍保留在跨平台待复现清单；没有把容器数字写入 `tests/benchmarks/reports/`，
也没有宣称跨平台已验证。

---

## 六、最可能翻车的地方

1. **顺手改了 11 阶段的东西**（CLI、进程退出码）。本批**只**冻结 B0 侧的值。
2. **退出码另立一套判断逻辑**（§3.2）——读 `diagnostic.code()` 的前缀、读文本，都会让
   它变成第二份语义。
3. **被 `catch` 消费的错误映射成 `3` 而不是 `0`**（§2.1 第 1 条）。这是五个值里最容易
   写反的一个。
4. **改了四处旧文档却漏改一处**（§1.2 的表）——单一来源的反面。
5. **`match` 写了兜底 `_`**（§3.1 硬要求 2），于是新增终局时静默归类。
6. **把容器数字写进冻结报告或宣称跨平台已验证**（§5.3）。
7. **为了让容器绿而放宽门禁**（§5.4）。
8. **忘了 `tests/benchmarks` 独立 crate**——它已经绊倒过一次（B0-B 的 lock 漏提交），
   这是第三次点名。
9. **动了冻结项**：`FORMAT_VERSION = 3`、opcode `0..40`、`tests/spec/` 共享向量、
   `tests/benchmarks/reports/`。

---

## 七、硬性约束

门禁、区分度验证、工具规定、单一来源原则、解耦约束**全部沿用 09R2D 文档第二章**。

### ★ 提交标题必须带规范前缀

`09r2d:417` 要求 `feat:`/`fix:`/`test:`/`docs:`/`chore:` 前缀。B0-A 两个提交违规之后，
B0-B 与 B0-C **都已改正**——保持。本批冻结退出码是新增能力，用 `feat:`；
Docker 记录与文档修正用 `docs:`；建议**分成两笔**（代码与实测记录），便于审核。

### ★ 提交正文必须写「为什么」

本批有两处**特别需要解释**的改动：**纠正 B0-B 的退出码误判**（要说清依据是 `00a:135`
与 `:127`，不是改主意），以及**修四处旧文档的措辞**（要说清是措辞过时而非笔误）。
B0-B/B0-C 的三段"为什么"加"验证："是目前最好的形态，保持。

### 其他

- 新增的每个 `pub` 项都要有文档注释（公共 API 100%、全仓 ≥90% 是硬门槛）。
- 新增公开项要按 B0-A 的既有做法在 `research::` 别名层补一条——**但 `xiao-driver` 没有
  `research` 别名层**（它是 08A 起就在生产路径的 crate），所以本条对它不适用；
  请顺手确认一遍，不要凭空造一个别名层。
- `A0-SIZE-001`：`xiao-driver/src/run.rs` 已 657 行，本批增量不大，但仍要留意 2500 行上限。

---

## 八、验收

沿用 09R2D 的「撤掉实现 → 用例必须失败 → 还原 → 通过」。**关键验收不是「测试通过」**：

1. **五个值各有断言**，且与 `DriverOutcome` 三段的对应关系被钉住（五种终局各一条）。
2. **`catch` 消费的错误是 `0`**——单列一条测试（§2.1 第 1 条）。
3. **`exit_code()` 的 `match` 无兜底**，且有一条守卫测试在新增终局时会失败。
4. **locale 中立**：结构与诊断文本无关，有断言（§3.3）。
5. **§1.2 的四处旧文档都加了修正批注**，一处不落。
6. **11 阶段文档有注记**，写明取值已由 B0 冻结（§4）。
7. **容器实测记录落盘**，含八项门禁结果与 Windows 差异说明（§5.4）。
8. **门禁全绿**，**包含** `tests/benchmarks` 独立 crate。

---

## 九、不负责与不要重复做的事

- **不要做 CLI / 进程退出码映射**（11/X0）、**不要做诊断窗口**（X0）。
- **不要重做 `DriverOutcome` 的三段设计**：B0-C 已交付并有测试，本批只在它上面加派生。
- **不要重做 B0-A 的验证器或 B0-B 的入口 ABI**。
- **不要跑基准并把数字写进冻结报告**（§5.3）。
- **不要移除 `research::` 别名层**（条件见方向稿 §2.1 的修正批注）。
- **不要为多模块做预留设计**。

## 相关页面

- [09-B0-C. 前端到 VM 内部驱动器](09b0c-frontend-to-vm-driver.md) —— 上一批，本批在它上面加派生
- [09-B0-B. 生产 VM 执行闭环](09b0b-production-vm.md) —— 退出码误判的出处（§1.2）
- [09-B0. 字节码最小运行闭环](09b0-bytecode-closure.md) —— 阶段方向稿
- [00A. 工程框架与目录布局](00a-project-layout.md) —— `:127`/`:135` 是本批的权威依据
- [12. 测试与开发里程碑](12-tests-and-milestones.md) —— B0 退出条件第 4 条点名退出码
- [07. 错误模型与并发安全边界](07-concurrency-and-errors.md) —— locale 不改退出码
- [11. CLI、项目配置与平台](11-cli-config-and-platform.md) —— 11 只做映射
- [09R2D. 两种机型与指令编码器交接文档](09r2d-machines-and-encoder.md) —— 开发规定主表
