# 09-B0-E. VM 中途取消检查点

> **本批要还的债是 `B0-C-CANCEL-001`**：驱动器目前只做**阶段边界采样**——在开始、
> 前端完成、降低完成、VM 调用前后各采样一次。**VM 指令循环里没有任何检查点**，
> 所以「取消」和「超时」今天都是**事后判定**：VM 会完整跑完，跑完后采样才发现已经
> 超时，于是丢弃已完成的 VM 结果。代价是"取消后程序仍在跑"，收益是"语义不撒谎"。
>
> **本批要把检查点接进热循环。** 这直接触碰 09R3 冻结的性能口径，所以它不是一次
> 普通的接线——**性能对照必须单独记录，不得改变 09R3 固定基准数字**。

## 一、Agent 交接上下文

### 接手前提

1. [09-B0-C. 前端到 VM 内部驱动器](09b0c-frontend-to-vm-driver.md) —— **债项的来源**。
   §五（`:193-218`）写清了方案 A 的选择与依据、三条路的取舍、以及"不允许定义了一个
   没人实现、也没有债项记账的接口"这条纪律。
2. [11X0-E. `xiao build` 与主机工具链发现](11x0e-build-and-toolchain.md) **§5.2**（`:199-206`）
   —— **本批出口条件的权威原文**：可注入取消源、指令循环检查频率、清理/退出码回归、
   关闭检查点后的性能对照。
3. [09R3. 跨平台基准与冻结](09r3-benchmarks-and-freeze.md) §「冻结七项」（`:84-95`）
   与 §「作废规则」 —— **本批最强的外部约束**。尤其第 4 项「异常与清理转移 ABI」。
4. [09-B0-D. 退出码冻结](09b0d-exit-codes-and-linux-verification.md)（尤其 `:100`）
   —— 取消与超时归 `ArtifactRejected = 2`。**本批的"退出码回归"必须对齐这里**。
5. [09R2D. 两种机型与指令编码器交接文档](09r2d-machines-and-encoder.md) **§H5**（`:263`）
   —— `Fatal` 绕过 handler/`finally`/drop 的冻结依据。**本批的核心设计决定要在这条的
   约束下做**。
6. [10D. 环境依赖测试规范](10d-environment-gated-test-spec.md) —— 若新增测试依赖
   `release` 构建或外部环境，按 `#[ignore]` 规格写，**不允许条件 `return`**。

### 现状盘点（2026-09-23 实测）

```text
驱动器侧  xiao-driver/src/run.rs          CancellationToken（:64-90）、RunControl（:96-137）
                                          ControlWindow（:584-630）、八个边界采样点
VM 侧     xiao-vm/src/semantics/exec.rs   run_blocks（:381）、run_subroutine（:823）
                                          step（:450）、unwind（:1305）、route_fault_scoped（:938）
```

**取消链路已经是通的，链路上游是空的**：

- 协议层收取消请求 → `handle_cancel`（`protocol/service.rs:422-458`）→ `token.cancel()`
  → `run_request_response` 的 `.with_cancellation(...)`（`service.rs:175`）。
- **但 TS 侧没有任何东西往里注入取消**：`cli/ts/src/commands/index.ts:86` 调
  `client.runSource(source, { path, module, debug })`——**没传 `signal`**，整个
  `cli/ts/src/` 里除 `client.ts` 内部外没有 `AbortController`。
  **出口条件里的「可注入取消源」在 CLI 层是空的，本批要接线。**

**「超时」今天没有 VM 侧实现**：`ControlWindow::start`（`run.rs:591`）只算一次 deadline，
之后在八个边界比对 `Instant::now()`。VM 侧完全不知情。

**没有预留钩子**：全 `xiao-vm/src` 与 `xiao-driver/src` 搜 `fuel|gas|checkpoint|interval|poll`
只命中寄存器分配器的 `LiveInterval`，与取消无关。

### 本批交付与不负责

**交付**：VM 热循环内的中途取消检查点，含可关闭开关、可注入取消源、清理与退出码回归、
以及**单独记录**的性能对照；CLI 侧把取消源接上。

**不负责**：**别名层清理**（见 §七）。`11x0e §5.4` 结尾曾把别名层清理也写成"由 B0-E
验收"，**本次已裁定拆开**——那个编号冲突的处置见 §七.1。

---

## 二、核心设计决定（**本批最容易做错的地方**）

### 2.1 取消**不能**建模成 `Fault::Error`

用户代码里的 `try { while true { } } catch { ... }` 会把取消**吞掉并继续循环**——
取消永远生效不了。这不是理论风险：`route_fault_scoped`（`exec.rs:948-951`）开头就是

```rust
let Fault::Error(mut error) = fault else { return Err(fault) };
```

任何 `Fault::Error` 都会进 handler 查表。

### 2.2 但也不能简单地复用 `Fault::Fatal`

`Fatal` 是**刻意的不对称通道**（`exec.rs:948-951` 的注释、09R2D §H5）：
**不查表、不清理、不进入普通 `catch`**。它在 `unwind`（`exec.rs:1315-1321`）里

```rust
if matches!(fault, Fault::Error(_)) { /* 执行释放计划 */ }
```

**只有 `Error` 执行释放计划**。

于是"取消"需要的语义是**第三条通道**：

| 通道 | 查 handler 表 | 执行清理/释放 | 进用户 `catch` |
| --- | --- | --- | --- |
| `Fault::Error` | ✅ | ✅ | ✅ |
| `Fault::Fatal` | ❌ | ❌ | ❌ |
| **取消（本批要定义）** | ❌ | **✅（推荐）** | ❌ |

**「不查表 + 要清理 + 不进 catch」这三个条件没有任何现成通道满足。**

### 2.3 必须显式选定的三件事（出口条件里的「清理/退出码回归」就是要你回答这三条）

1. **取消是否绕过用户 `catch`？** —— **必须是"绕过"**。理由是 §2.1 那个死循环：
   若走 `Error`，`catch` 会吞掉取消，本批交付物等于不存在。
2. **取消是否执行释放计划（`run_plan`）与 `finally`？** —— **推荐"执行"**。
   不执行的代价是表/句柄泄漏到宿主；而 `finally` 的存在意义就是清理。
   但**这是本批必须论证、不能默认的点**——因为它直接决定 §2.2 的表往哪边走。
3. **取消映射到哪个 `ExitCode`？** —— **必须是 `ArtifactRejected = 2`**
   （`09b0d:100` 的冻结口径）。**注意 `Fatal` 会映射成 `4`**（`run.rs:59`），
   若取消借用 `Fatal` 通道则必须由驱动器**重新映射**，否则与 B0-D 直接矛盾。

### 2.4 ⚠️ 是否触碰 09R3 冻结第 4 项——**必须专门论证**

09R3 冻结第 4 项「异常与清理转移 ABI」的原文说明是
**「handler 路由、`finally` 子程序、释放序列的转移语义」**（`09r3:92`）。

**判断口径**：只要**不改变**这三者在**字节码层面**的既有语义，就只是**新增一条终止通道**，
而不是改 ABI。**但这必须写成显式论证**，不能靠"我觉得没碰"：

- 如果取消**复用 `finally` 子程序与释放计划的既有执行机制**，只是跳过 handler 查表，
  那它是"对既有转移机制的**一种新调用方式**"——论证空间充分；
- 如果为了取消去**改动** handler 路由规则或释放序列本身，那就是**改 ABI**，
  按 `09r3:27` 的作废规则**本轮全部基准数字作废并需重新冻结**。

**不允许的偷懒**：把 `Fault` 加一个变体就完事、并在文档里写一句"不影响 ABI"。
要么给出上表口径的逐条对照，要么按重新冻结走。

### 2.5 检查点必须**可关闭**，而且"关闭"要说得清

出口条件写的是"**关闭检查点后**的性能对照"，意味着检查点必须能被关掉。
三种关法的**代价不同**，必须写明选了哪种：

- **编译期常量 / feature flag**：热循环里没有运行时分支，测出的"关闭"最干净；
  但"关闭"是构建变体，**不是运行时可选项**。
- **`VmOptions` 字段（运行期布尔）**：可运行期切换，但**该分支本身就在热循环里**，
  "关闭"时仍有一次判断——**对照测的就是这个判断的代价**。
- **环境变量**：同上，且多一层读取。

**推荐 `VmOptions` 字段**（与既有 `max_call_depth` 同处），理由是它可测、可记录、
且"关闭时的残余代价"正是性能对照要诚实报告的数字。**但要在文档里写明这个选择本身
就是被测对象的一部分**。

⚠️ 注意 `VmOptions` 是 `Copy + Eq`（`xiao-vm/src/run.rs:124-127`），**加字段会连锁**到
`VmOptions::validate`（`run.rs:139-155`）、`RunRequest::validate`（`run.rs:229-260`）、
协议层 `RunOptions`（`protocol/request.rs:152-159`）与 `run_options()`（`protocol/run.rs:47-63`）、
以及 TS 侧 `messages.ts:62` 与 `client.ts:151`。**这是一条完整的协议字段链路，不是加一个字段。**

---

## 三、硬约束

### 3.1 ⚠️ 循环有**两处**，不是一处

| # | 函数 | 位置 | 内部循环 |
| --- | --- | --- | --- |
| 1 | `Vm::run_blocks` | `xiao-vm/src/semantics/exec.rs:381` | `loop` 在 `:384`，`for` 在 `:389` |
| 2 | `Vm::run_subroutine` | `xiao-vm/src/semantics/exec.rs:823` | `loop` 在 `:836`，`for` 在 `:841` |

两处各自独立地做同一件事（`:390-391` 与 `:842-843`）。**只改 `run_blocks` 会让
`try { } finally { while true { } }` 这类程序的中途取消完全失效**——`finally` 走的是
`run_subroutine` 的独立循环。**「清理回归」这个出口条件必须专门覆盖这条。**

自然的插入点是那两行的指令计数自增。`fn step`（`exec.rs:450`）是 `TacOp` 巨型 match，
**不要把检查点塞进 `step` 内部**——它是每条指令的公共路径，且 `step` 不知道自己在第几条。

### 3.2 计数器要**跨递归帧单调**

`execute`（`exec.rs:233`）对每次 Xiao 函数调用**递归一次**（调用点在 `exec.rs:629`）。
**循环里的局部计数器会在每层调用重置**，导致递归深度一变检查频率就变。
用 `self.metrics.instructions`（`xiao-vm/src/run.rs:276`）取模才是跨帧单调的。

⚠️ **但 `metrics` 会被表回调整体合并替换**（`semantics/tables.rs:183-186` 的
`sync_table_events`）。**不要依赖"取模后的余数"**——用单调绝对计数，或独立计数器。

⚠️ `metrics.instructions` **已经是对外可观测指标**（`RunOutcome.metrics.instructions`，
并由 `sink.rs:263` 复制进 `Metrics` 事件）。**检查点不得改变它的取值**。

### 3.3 检查点应放在 `finish_table_effects` **之后**

`finish_table_effects`（`semantics/tables.rs:190-216`）在每次 `step` 后调用
（`exec.rs:392` 与 `:844`），会合并表生命周期回调产生的挂起故障。
**放在它之前，表析构错误可能盖过取消信号。**

### 3.4 依赖方向：VM 不能复用驱动器的 `CancellationToken`

`xiao-driver` 依赖 `xiao-vm`（`Cargo.toml` 的 `xiao-vm = { path = "../xiao-vm" }`），
所以 `CancellationToken`（`xiao-driver/src/run.rs:64`）**不能**被 `xiao-vm` 直接使用。

⚠️ **dev-dependency 反噬**：`xiao-vm/Cargo.toml` 的 `[dev-dependencies]` 里有
`xiao-driver`。**为了让 VM 测试能用 `CancellationToken` 而把它提成正常依赖，就是真实循环依赖。**

VM 侧要自带等价类型（或收 `&dyn Fn() -> bool` / 泛型轮询参数）。**两种做法都行，
但选哪种要写进文档**——它决定了 VM 的公开面。

### 3.5 `research::` 别名层要同步

`09b0c:285` 规定「新增公开项要按 B0-A 的既有做法在 `research::` 别名层补一条」。
若本批往 `xiao-vm` 加公开类型/函数，`xiao-vm/src/research/mod.rs` 的重导出列表要跟着补，
否则 `tests/benchmarks` 那份回归夹具会与生产面脱节。

**注意**：`research/` 现在**只是兼容重导出层，没有实体**
（`src/research/mod.rs:1-37`；`research/machine/` 与 `research/semantics/` 下**只有 README**）。

### 3.6 09R3 基准：**只读，不得产生 diff**

1. **不修改** `tests/benchmarks/reports/` 下任何既有 JSON（尤其 `09r3-freeze.json`
   与四个 `windows-native-*.json`）。`09b0c:260-261` 明确要求「不应产生
   `tests/benchmarks/reports/` 的 diff」。
2. **不修改** `tests/benchmarks/manifest.json` 的协议字段（预热 3、测量 11、
   `format_version`、opcode 范围、输入规模）。
3. 新数字**另落一份文件**（如 `09b0e-checkpoint-*.json`），并在文件与文档里显式标注
   「**这是 09R3 冻结数字之外的附加对照，不构成重新冻结**」。
4. 对照必须是**同机、同协议、同构建配置**的 A/B（检查点开/关），
   **不是与冻结报告里的旧数字跨次比较**——`09r3:136-138` 要求"同一次构建产出"才可比。
5. 读数只能在 `release` 下取，且确认 `core/rust/rust-toolchain.toml` 仍是 `1.96.0`。

### 3.7 `tests/benchmarks` 是**独立 crate**

`cargo test --workspace` 覆盖不到它。验证清单**固定包含**
`cargo check --manifest-path tests/benchmarks/Cargo.toml`；
若给任何 Rust crate 增删依赖，**`tests/benchmarks/Cargo.lock` 必须同步**
（历史上已漏交一次）。`bun run check` 里的 `check:lock` 会抓这个。

### 3.8 `A0-SIZE-001`

`exec.rs` 已 **1609 行（64%）**、VM `run.rs` 539 行、driver `run.rs` 707 行。
在 `run_blocks`/`run_subroutine` 里加代码前先核
[00E](00e-file-size-gate.md) 的阈值与豁免方式——**本批的目标应当是不需要豁免**。

---

## 四、分步提交（**不要一笔做完**）

| 步 | 内容 | 为什么这个顺序 |
| --- | --- | --- |
| **1** | 在 VM 侧定义**可注入的取消源**与关闭开关，**不接循环** | 先把公开面与 `VmOptions` 链路定下来，此时无行为变化 |
| **2** | 接 `run_blocks` 的检查点 | 主路径；做完全部回归 |
| **3** | 接 `run_subroutine` 的检查点 | **单独一笔**，因为它是 §3.1 那个最容易漏的地方；配 `finally` 死循环的区分度用例 |
| **4** | CLI 侧接取消源（`commands/index.ts` 传 `signal`） | 与 VM 侧解耦，可独立验收 |
| **5** | 性能对照 | 前四步都稳定后再测；**另落文件** |
| **6** | 文档与登记收尾 | UseDocs、`module-registry.json`、README |

**每一步的验收**：既有测试不改、退出码不变、门禁全绿。

---

## 五、最可能翻车的地方

1. **只改一处循环**（§3.1）——`finally` 里的死循环不可取消。**本批头号翻车点。**
2. **把取消建模成 `Fault::Error`**（§2.1）——被用户 `catch` 吞掉，交付物等于不存在。
3. **借用 `Fatal` 却忘了重新映射退出码**——`Fatal` 是 `4`，B0-D 冻结的是 `2`（§2.3）。
4. **没有论证就宣称"没碰冻结第 4 项"**（§2.4）——要么按口径逐条对照，要么走重新冻结。
5. **动了 `tests/benchmarks/reports/` 里的文件**（§3.6）——冻结数字是输入，不是待办。
6. **用局部计数器取模**（§3.2）——递归一深检查频率就变了。
7. **依赖方向踩环**（§3.4）——`xiao-vm` 正常依赖 `xiao-driver` 是真环。
8. **加了公开项却漏了 `research::` 别名层**（§3.5）。
9. **忘了 `tests/benchmarks/Cargo.lock`**（§3.7）——`check:lock` 会拦，但别让它第一次就拦到。
10. **顺手改语义**。本批只加检查点；发现可以顺手修的地方**另开提交**。

---

## 六、验收

沿用 09R2D 的「撤掉实现 → 用例必须失败 → 还原 → 通过」。**关键验收不是「测试通过」**：

1. **区分度用例**：一条**取消时仍在 `finally` 里死循环**的程序必须能被打断
   （撤掉 `run_subroutine` 的检查点即失败）——这条是 §3.1 的证明；
2. **区分度用例**：`try { while true { } } catch { 继续循环 }` 必须**不能吞掉取消**
   （§2.1 的证明）；
3. **清理回归**：取消时 `finally` 与释放计划的执行情况**与 §2.3 的选定一致**，并配断言；
4. **退出码回归**：取消/超时仍映射 `ArtifactRejected = 2`，且**既有的三条**
   驱动器边界测试（`run.rs:694`、`tests/b0_c_driver.rs:96`、`protocol_tests.rs:146`）未改仍通过；
5. **`metrics.instructions` 取值不变**（§3.2）——检查点不得污染这个对外指标；
6. **可关闭开关的对照**：关闭时的性能数字与开启时**同机同协议**，落**新文件**，
   且 `git status` 里 **`tests/benchmarks/reports/` 无 diff**；
7. **CLI 侧取消源已接线**（`commands/index.ts` 传 `signal`），并有 TS 侧测试；
8. **门禁全绿**，含 `check:lock`、`bunx tsc`、`cargo check --manifest-path tests/benchmarks/Cargo.toml`。

---

## 七、不负责与不要重复做的事

### 7.1 别名层清理**不在本批**（编号冲突已裁定）

`11x0e §5.4`（`:221-233`）结尾写「待迁移完成后由 **B0-E（别名层清理专项）** 单独验收」，
而同文 §5.2 又把 `09-B0-E` 定义为「VM 中途取消检查点」。**同一个编号被赋予了两个职责。**

**本次裁定：拆开，`09-B0-E` 只留取消检查点。** 理由：

- 两者**验收方式完全不同**——取消检查点是**性能敏感**的（要动热循环 + 单独的性能对照），
  别名层清理是**纯重构**（删兼容重导出 + 迁基准工具）。共用一份验收，
  哪一项拖慢都会堵住另一项。
- 别名层清理的**触发条件仍未满足**（2026-09-23 实测）：`tests/benchmarks/src/main.rs`
  仍有 2 处 `research::`，`r2_*.rs` 共 **7 个**（`xiao-vm` 下 6 个 +
  `xiao-bytecode` 下的 `r2_tac.rs`）全部仍引用别名层。
  ⚠️ `09b0 §2.1` 的措辞是「6 个 `r2_*.rs`」，那是**只数了 `xiao-vm` 目录**——
  跨两个 crate 实际是 7 个，写新计划时按 7 个算。

**别名层清理应另登记一个编号**（`09-B0-F` 或独立债项），由后续批次做。

### 7.2 其余不负责

- **不改 `Fault`/`RunResult` 的既有变体语义**——加变体要按 §2.4 论证。
- **不重跑或改动 09R3 的基准**（`09b0:445`：冻结数字是输入，不是待办）。
- **不做 `print` / 内置函数**——`11x0 §1.3` 已把它归 20 阶段。
- **不做 `xiao test` 的项目测试语义**——归 **X0-T**（见
  [11X0-T](11x0t-project-test-semantics.md)），**本批不碰**。
- **不改协议语义**——只加取消所需的字段，且要遵守 §2.5 那条完整的字段链路。
- **不要顺手修 `protocol/service.rs:229` 的 `cancel_response` 死路径**——
  它硬编码 `accepted: false`，实际走的是 `handle_cancel`（`service.rs:360`）。
  这是既有现象，**另开提交**。

---

## 八、落地结果（2026-09-23）

本批已经按 §四分阶段落地，最终选择如下：

1. `xiao-vm` 提供独立的 `CancellationToken`/`CancellationSource`。驱动器只依赖 VM 的
   抽象，不形成反向依赖；`CancellationSource` 同时承载跨线程取消标记和绝对截止时间。
2. `VmOptions` 增加 `checkpoints_enabled` 与 `checkpoint_interval`。检查点关闭是运行期
   布尔开关，关闭时仍保留该分支的真实成本；计数器使用 VM 共享的单调计数，不改变
   `metrics.instructions`。
3. `run_blocks` 与 `run_subroutine` 都在 `finish_table_effects` 之后轮询。取消使用独立的
   `Fault::Cancelled`/`RunResult::Cancelled` 通道：不查用户 `catch`，但按活动作用域先尝试
   既有 `finally`，再复用 `unmatched_error` 释放计划；`finally` 内部也有检查点，因此死循环
   清理可以被打断。既有 `Error` 与 `Fatal` 的路由、清理和退出语义未改写。
4. 驱动器把同一个取消源和 deadline 注入生产 `RunRequest`；VM 终止后仍由驱动器边界映射
   为 `DriverOutcome::Rejected`，因此取消/超时保持 `ArtifactRejected` 进程码 `2`。
5. TypeScript CLI 的 `run`/`build` 命令沿 `CommandContext.signal` 传入 `ProtocolClient`；
   真实入口用 `AbortController` 接收 `SIGINT`，客户端继续通过既有 `cancel` 帧绑定请求 ID。

### 8.1 验证结果

- `cargo test -p xiao-vm --test b0_b_production cancellation_`：取消绕过 `catch`、释放计划执行、
  `finally` 死循环中断和关闭开关回归通过。
- `bun test cli/ts/src/commands/index.test.ts cli/ts/src/protocol/client.test.ts` 与
  `bunx tsc --noEmit -p tsconfig.json` 通过。
- `cargo check --manifest-path tests/benchmarks/Cargo.toml` 通过；基准 crate 仅同步新增
  `VmOptions` 字段和 `RunResult::Cancelled` 分支，未改其冻结协议或既有报告。
- 附加 release A/B 结果见
  [`09b0e-checkpoint-performance.json`](09b0e-checkpoint-performance.json)。它使用同一份 TAC、
  同一构建、交替样本测量，**不构成 09R3 重新冻结**。
- `tests/benchmarks/reports/` 无 diff；既有 09R3 冻结数字保持只读输入。

### 8.2 分阶段提交

- `7ec5cb6`：新增 VM 取消源、检查点配置和协议字段链路。
- `f5043c2`：接入主解释循环、deadline 注入和关闭开关回归。
- `d9a8efc`：接入 `finally`/释放计划清理与独立取消通道。
- `47d1b90`：CLI `AbortController`/`AbortSignal` 接线及 TypeScript 回归。
- `b6bed61`：附加 release A/B 性能对照夹具与独立基准兼容修复。

---

## 九、接手前先做：本批的收尾

本批的核心设计**已经做对**（§八 的落地结果与 §二/§三 逐条吻合：三条通道语义正确、
两处循环对称接入、计数器跨帧单调、退出码保持 `2`、性能对照方法学扎实）。
**但留下三件必须先处理的收尾**，处理完再动别的。

### 9.1 `Fault` 的文档注释**漂移**了（真缺陷）

`xiao-vm/src/semantics/exec.rs:35-43` 现在的注释是：

```rust
/// 可恢复错误与致命故障是两条通道：致命故障不执行释放计划，也不能被普通
/// 处理器捕获，因此不能与 [`XiaoError`] 共用一个变体。
pub enum Fault {
    Error(XiaoError),
    Fatal(FatalError),
    Cancelled,          // ← 本批新增，注释没跟上
}
```

**两处不对**：

1. 现在有**三个**变体，注释还说"两条通道"；
2. 更要紧的是——`Cancelled` **执行**释放计划（§2.3 第 2 条的推荐），
   而注释用来分类的那条线正是"是否执行释放计划"，**它完全没覆盖 `Cancelled`**。

⚠️ **这是本仓头号病（单一来源漂移）的活标本**：同一份语义在**两处**各写一份——
`Fault` 枚举的注释，与 `unwind` 的 doc 注释（`exec.rs:1344-1348`）。
**后者已经更新了**（「取消先执行 `finally`，再复用同一释放计划；`Fatal` 则完全跳过释放计划」），
**前者没有**。这类漂移不会让测试变红，只会让下一个人读错。

**修法**：把 `Fault` 的注释改成三条通道，并写明 `Cancelled` 的完整语义——
**不查 handler 表、执行 `finally` 与既有释放计划、不进用户 `catch`**；
同时说明它与另两条的区别（对照 §2.2 那张表）。

### 9.2 09R3 冻结第 4 项的论证**没到 §2.4 的要求**（部分缺陷）

§2.4 写的是「要么给出上表口径的**逐条对照**，要么按重新冻结走」。
文档 §八.3 给了结论（「既有 `Error` 与 `Fatal` 的路由、清理和退出语义未改写」），
方向正确，但**没有按 `09r3:92` 的三条口径逐条对照**，也没明写
「这是**新增终止通道**，不是改 ABI」。

**修法**：补一段逐条对照，三条都要点到：

| 冻结第 4 项的三条口径（`09r3:92`） | 本批是否改动 |
| --- | --- |
| handler 路由 | ❌ 未改——`Cancelled` 走 `route_fault_scoped` 的 `else` 分支，**根本不进入查表逻辑** |
| `finally` 子程序 | ❌ 未改——`run_cancelled_finalies` 是**对既有 `finally` 机制的新调用**，子程序本身的执行语义没变 |
| 释放序列的转移语义 | ❌ 未改——`Cancelled` **复用** `unmatched_error` 计划，计划本身与 `Error` 走的是同一份 |

**结论应当是**：这是「新增一条终止通道，并**新调用**既有转移机制」，
不是改 ABI，因此**不触发 `09r3:27` 的作废规则**。**但这句话必须写出来**，
不能靠读者从代码反推——这正是 §2.4 禁止的那种偷懒。

### 9.3 六笔提交**正文全空**（违规·复发）

`7ec5cb6`/`f5043c2`/`d9a8efc`/`47d1b90`/`b6bed61`/`8028cac` 六笔的正文都是 **0 字节**，
而 `11X0-H §9.5` 明确定「**空正文是违规**」，§9.5 还把「解耦批次尤其重要」写成了通则。

**这是复发**——`checker.rs` 那批 8 个空正文的账刚记完，紧接着就又是 6 个。
说明「**在交接文档里补说明**」这个补救**没有传导到提交环节**：文档写好了，
下一次提交时照旧不带正文。

**处置（与上次一致，不改写历史）**：本批的分阶段说明已由文档 §八.2 补齐，那 6 笔保持原样。
**但要补一条机制**：把「提交前核对正文非空」提升为**提交前的固定动作**，
与门禁同等对待——**写在文档里已经两次失效了**，下次应当由检查器或提交模板兜住，
而不是再写一句提醒。

### 9.4 一处小出入

文档 §八 的标题写「落地结果（2026-09-23）」，实际提交日期是 **2026-09-24**。改成实际日期。

---

## 相关页面

- [09-B0-C. 前端到 VM 内部驱动器](09b0c-frontend-to-vm-driver.md) —— **债项来源与方案 A 的选择**
- [11X0-E. `xiao build` 与主机工具链发现](11x0e-build-and-toolchain.md) §5.2 —— **出口条件权威原文**
- [09R3. 跨平台基准与冻结](09r3-benchmarks-and-freeze.md) —— 冻结七项与作废规则
- [09-B0-D. 退出码冻结与 Linux 容器实测](09b0d-exit-codes-and-linux-verification.md) —— 取消/超时归 2
- [09R2D. 两种机型与指令编码器交接文档](09r2d-machines-and-encoder.md) §H5 —— `Fatal` 的不对称通道
- [10D. 环境依赖测试规范](10d-environment-gated-test-spec.md) —— `#[ignore]` 写法规格
- [00E. 单文件行数门禁交接](00e-file-size-gate.md) —— `A0-SIZE-001` 与豁免机制
- [11X0-T. 项目测试语义与结果协议](11x0t-project-test-semantics.md) —— 与本批同期的另一笔 X0 收口
