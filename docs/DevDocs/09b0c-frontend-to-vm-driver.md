# 09-B0-C. 前端到 VM 内部驱动器交接文档

> **本批的可执行交接。** 上游方向稿见 [09-B0. 字节码最小运行闭环](09b0-bytecode-closure.md) §5；
> 本文把它展开成清单，并补上方向稿写作时尚未实测的事实。
>
> B0-C 的**权威定义**是 [09R](09r-bytecode-machine-research.md) `:648-649`：
> 「`FrontendArtifact` → 字节码 → VM 执行，提供结构化成功、错误、退出码、堆栈和事件结果；
> `xiao run` 与 TypeScript CLI 接线继续留到 11/X0」。本文**不新增要求**。

### 本批落地记录（2026-09-21）

09-B0-C 已在 `xiao-driver` 落地。`DriverRequest` 把前端请求、B0-B 运行参数、模块/源码
身份、事件容量和取消/超时控制收在一个库 ABI 中；`FrontendVmDriver` 只编排
`FrontendCompiler → lower_program → xiao_vm::run_request`，不重新解析源码、推断类型/生命周期，
也不搬迁基准工具的命名函数到函数零。入口函数仍由生产 ABI 固定为函数零，脚本与 `[main]`
都沿用 B0-B 的同一入口。

结果采用单一 `DriverOutcome` 枚举，分为 `Frontend`、`Rejected` 和 `Executed` 三段。这样前端
失败可以保留完整 `FrontendError.diagnostics`，执行后的 `Success/Error/Fatal` 又可以保留
B0-B 的事件、指标和 `ReportRecord`；若使用 `Result<RunOutcome, DriverError>`，前端诊断和已执行
故障的结构都会被压扁或另造一层包装。整数退出码仍不冻结，继续留给 11/X0。

取消/超时选择方案 A：驱动器在开始、前端完成、降低完成和 VM 调用前后采样；VM 指令循环
检查点记为 `B0-C-CANCEL-001`，出口批次为 `11/X0`，不会把边界采样伪装成中途中断能力。
稳定驱动器编号为 `X09-DRIVER-001`（取消）、`X09-DRIVER-002`（超时）和
`X09-DRIVER-003`（控制字段不可表示）。公共契约回归位于
`core/rust/crates/xiao-driver/tests/b0_c_driver.rs`，私有单元测试另守损坏 TAC 的执行前拒绝。

本批没有修改 `FORMAT_VERSION = 3`、opcode `0..40`、09R3 冻结报告、共享向量或
`tests/benchmarks/reports/`；UseDocs 和模块登记与驱动器一起更新。

## Agent 交接上下文

### 接手前提

1. [09-B0. 字节码最小运行闭环](09b0-bytecode-closure.md) —— **方向稿**，§5 是本批范围、
   §2.1 的别名层移除计划与本批有关（见第七节）。
2. [09-B0-B. 生产 VM 执行闭环](09b0b-production-vm.md) —— **上一批**。本批消费它的
   `RunRequest` / `run_request` / `RunOutcome`，**不重做**它的入口 ABI。
3. [09. 字节码运行模式](09-bytecode-runtime.md) —— `:44` 的「取消/超时边界」、`:47` 的
   「不得要求 CLI 依赖 Node.js」、`:120` 的「不得解析文本判断成功」、`:25` 的交接检查。
4. [09R2D. 两种机型与指令编码器交接文档](09r2d-machines-and-encoder.md) —— **开发规定
   主表**，第二章全部沿用。
5. [08A. U0 统一前端实现交接记录](08a-u0-frontend-implementation.md) —— `FrontendArtifact`
   的产出方与 `FrontendError` 的形状。
6. B0-B 的评审结论 —— 见第一节的待还债项（`tests/benchmarks/Cargo.lock` 与门禁盲区）。

### 现状盘点（2026-09-21 实测）

| 项 | 现状 |
| --- | --- |
| `FrontendArtifact` → `lower_program` 的连接 | ✅ 已接入 `xiao-driver/src/run.rs`；基准工具仍保留自己的一次性链路 |
| `xiao-driver` 规模 | `frontend.rs` 保持前端职责，新增 `run.rs` 作为生产 VM 编排层 |
| `xiao-driver` 登记 | `status = verified`、`stage = 09-B0-C`，新增 `compiler/driver/README.md` |
| 生产运行入口 | ✅ B0-B 已交付：`RunRequest` / `run_request`（别名 `run_production`），固定函数 0 与栈式载体 |
| 退出码 | ❌ 仍未定义（**有意为之**：B0-B 只冻结 `Success/Error/Fatal` 三分支，整数留给 11/X0） |
| 取消 / 超时 | ✅ 驱动器边界采样已实现；VM 中途检查点记为 `B0-C-CANCEL-001`，出口 `11/X0` |

### 本批交付与不负责

**交付**：`xiao-driver` 的内部运行驱动器（前端 → 降低 → 验证 → 执行）、三段错误的统一
结构化表示、取消/超时边界，以及上批待还的债。

**不负责**：`xiao run` 与 TypeScript CLI 接线（11/X0）、诊断窗口（X0）、多模块工程图（D/E）、
`.xiaoc` 容器（14）、优化（13）、LLVM（10）。

---

## 一、先还债

### 1.1 `tests/benchmarks/Cargo.lock` 的漏提交（B0-B 遗留）

B0-B 给 `xiao-vm` 加了 `xiao-source` 依赖，`core/rust/Cargo.lock` 提交了，
**独立 crate 的那份 lock 漏了**。本批已先用独立 `fix:` 提交补齐：

```diff
 name = "xiao-vm"
 dependencies = [ ..., + "xiao-source", ]
```

提交为 `c2f56ef fix: 补齐基准 crate 的 VM 依赖锁定`；后续依赖变更仍须复跑独立
`tests/benchmarks` 检查。

### 1.2 门禁必须带上 `tests/benchmarks`（**结构性盲区**）

1.1 的根因不是疏忽，是**盲区**：`tests/benchmarks` 是**独立 crate**，
`cargo test --workspace` 覆盖不到它。B0-B 的正文列了 7 条验证命令，没有一条碰到它。

**本批起，验证清单固定包含**：

```text
cargo check --manifest-path tests/benchmarks/Cargo.toml
```

`tests/differential/` 若也只有 README，请顺手确认它是否同样需要。

**为什么这条要写进交接文档**：它已经造成一次真实的漏交付，而靠记性守不住——
本批又要给 `xiao-driver` 加依赖，`Cargo.lock` 的连锁更新会再次发生。

---

## 二、落点与依赖方向

### 2.1 落点：`xiao-driver`

`00a-project-layout.md:44-63` 的依赖图里，`xiao-driver` 在 `xiao-bytecode` / `xiao-vm` 的
**上层**，本批要新增这两个依赖，方向合法：

```text
xiao-ir
    ├─ xiao-bytecode → xiao-vm → xiao-runtime
                    ↓
               xiao-driver          ← 本批在它这里加两条向下依赖
```

`xiao-driver` 的 crate 定位原文是「Xiao 编译、**运行**和构建请求编排接口」
（`lib.rs:1`）——运行两个字本来就在它的职责里。

**开工第一件事**：核对 00A 依赖图与 `xiao-driver/Cargo.toml` 现状，确认不引入环；
并确认新增依赖后 `core/rust/Cargo.lock` **和** `tests/benchmarks/Cargo.lock` 都要更新（§1.1）。

### 2.2 不要新建 crate

依赖图里没有多余的位置，且「不新增 crate」是更保守的选择。把驱动器放进已有的
编排层，比新开一个 `xiao-runner` 再让 `xiao-driver` 依赖它要少一层。

---

## 三、三段错误的统一（本批**核心设计**）

驱动器要跨越三个阶段，每个阶段都有自己的失败形态，本批必须给出**统一的结构化表示**：

| 阶段 | 现有类型 | 失败含义 |
| --- | --- | --- |
| 前端 | `FrontendError { diagnostics: Vec<Diagnostic> }` | 源码有词法/语法/类型/生命周期错误，**没有产物** |
| 降低 + 验证 | `TacVerificationError` / `RunRequestError`（B0-A/B0-B） | 产物结构不一致或请求参数非法，**有产物但不能执行** |
| 执行 | `RunResult::{Success, Error(XiaoError), Fatal(FatalError)}` + `RunOutcome.report` | 产物跑了，抛了未捕获错误或致命故障 |

**必须回答的问题**（不是抄一遍类型就能过去的）：

1. **前端失败与执行失败是不是同一种东西？** 它们在用户呈现、退出码与"有没有产物"上
   都不同，但都必须是**结构化失败**。给出你的结论与判据。
2. **驱动器返回一个类型还是两个？** 一个覆盖全链的枚举（前端失败 / 被拒 / 已执行），
   还是 `Result<RunOutcome, DriverError>`？**选哪个都行，但要写明为什么不选另一个。**
3. **每个失败点都要有稳定 `code`**（09 文档 `:120`）。B0-A/B0-B 已经给了
   `X09-BYTECODE-001/002`、`X09-VM-001/002`；前端诊断有自己的 `X0x-*` 体系。
   驱动器**新增**的失败点要沿用同一命名空间，不要另起一套。

> **不得让消费方解析人类可读文本判断成功与否**（09 文档 `:120`）。这条是硬约束，
> 也是 X0 退出条件第 6 条的前提。

---

## 四、不许复制的两样东西

### 4.1 基准工具的 `functions[0]` 搬迁

`tests/benchmarks/src/main.rs` 里有一段**只在工具里成立**的手法：

```rust
let Some(function) = program.functions.iter().find(|f| f.name == spec.entry).cloned() ...;
program.functions[0] = function;      // 把命名函数搬到入口槽 0
```

它存在的原因是：基准程序写成 `def` 函数，而降低器固定的入口是函数 0（脚本体）。

**生产驱动器不得复制它。** 生产入口就是函数 0，这是 B0-B 已经冻结并加了测试的
（`script_and_project_share_entry_zero`）：脚本模式跑顶层语句，`[main]` 模式跑表体，
**两者的体都降低为函数 0**。

**如果本批认为生产需要"指定任意函数为入口"，那是新增 ABI，不是补执行**——要么不做，
要么单开批次并同步 09R 的冻结记录。**不要在驱动器里偷偷加一个 `entry: String` 参数。**

### 4.2 任何重新推断语义的"补齐"

09 文档 `:15` 与 `:102` 都写死了：驱动器只消费已验证的前端产物，
**不得重新解析源码、不得重新推断类型或生命周期、不得重算 release 计划**。
`tests/benchmarks` 的 `compile_all` 是一个正面例子（它只做 `compile → lower → encode`），
可作为形状参考。

---

## 五、取消与超时边界

09 文档 `:44` 把它列为一级工程目标的一条：

> 定义稳定的运行请求、运行结果、结构化错误/诊断事件和**取消/超时边界**。传输采用
> 进程协议还是库 ABI 留到 CLI 集成阶段冻结，但字段语义必须先与第 07、08 阶段一致。

现状：B0-B 没有做 VM 中途检查点；本批已定义并实现驱动器层边界采样。

**本批采用方案 A**：在运行请求上定义取消/超时的**语义与字段**，并写清传输层
（进程协议 / 库 ABI）如何注入留到 11。

**但这里有一个必须正面回答的问题**：超时与取消在字节码 VM 里意味着什么？
它需要 VM 在指令循环的某个边界周期性检查——**那是性能敏感点，且 B0-B 已经交付**。

三条路，选一条并写明依据：

- **A（已选择）**：本批只定义驱动器层接口，VM 侧的检查点显式记为**具名债项**
  `B0-C-CANCEL-001`，出口批次为 `11/X0`；
- **B**：本批接通最小检查点（例如仅在函数调用边界检查），并说明它对 09R3 冻结的
  性能口径意味着什么；
- **C**：证明本批不需要它，并写明"09 文档 `:44` 的这条由哪个批次兑现"。

**不允许**定义了一个没人实现、也没有债项记账的接口——那是假完备，比不做更坏。

---

## 六、UseDocs 与登记（**`rust.xiao-driver` 是 `verified`**）

实测：`rust.xiao-driver` 的 `status = verified`、`stage = 09-B0-C`，UseDocs 指向
`docs/UseDocs/language/compiler/README.md`、`compiler/frontend/README.md` 和新增的
`compiler/driver/README.md`。

本批改了它，**必须同批**：

1. 复核上述两页（它们现在只描述前端流水线；驱动器改变了这个 crate 的能力边界）。
2. 决定是否需要**新增**一页 UseDoc 描述运行驱动器——若要新增，注意
   `a0.docs.orphan_page` 要求新页面有入链，且 UseDocs 的 front matter 必须带
   `module` / `stage` / `status`。
3. 更新 `docs/module-registry.json` 里 `rust.xiao-driver` 的 `stage`（`08A` 已不足以
   描述它）与 `code` / `tests` 数组。
4. 00B 的政策：**UseDocs 页面未达到 `verified` 不得把模块标记为已完成**。

---

## 七、顺带修正方向稿 §2.1 的一处不准

方向稿 §2.1 写的别名层移除触发条件是：「B0-C 交付且生产驱动器成为唯一消费方」。

**这句话不准确**：别名层（`research::`）的消费方**不是**生产驱动器——
驱动器会用生产路径。真正的消费方是 **`tests/benchmarks/src/main.rs` 与 6 个 `r2_*.rs`**。

所以准确的移除条件是：**那些测试与工具改到生产路径之后**。本批**不做**移除，
但要把这条修正写进方向稿，避免后来者按错误条件判断"可以删了"。

---

## 八、最可能翻车的地方

1. **复制基准工具的 `functions[0]` 搬迁**（§4.1），把工具的权宜之计渗进生产入口。
2. **三段错误各返回各的类型**，驱动器再加一层包装，于是消费方要 switch 四次才能
   判断"到底成没成功"（§3）。
3. **定义了取消/超时接口但没有任何实现与记账**（§5）。
4. **跳过 `verify_for_execution` 的既有落点**：驱动器必须走 B0-B 的 `run_request`，
   **不得自己拼 VM**——否则会绕过 B0-A 的结构关卡。
5. **在驱动器里重新推断语义**（§4.2）。
6. **忘了 `tests/benchmarks` 的 lock 与编译检查**（§1.1/§1.2）。
7. **动了冻结项**：`FORMAT_VERSION = 3`、opcode `0..40`、`tests/spec/` 共享向量、
   `tests/benchmarks/reports/`。本批**不应**产生这些文件的 diff。
8. **把 `xiao run` 或 CLI 顺手做了**。09R `:649` 明说留到 11/X0。

---

## 九、硬性约束

门禁、区分度验证、工具规定、单一来源原则、解耦约束**全部沿用 09R2D 文档第二章**。

### ★ 提交标题必须带规范前缀

`09r2d:417` 要求 `feat:`/`fix:`/`test:`/`docs:`/`chore:` 前缀。B0-A 两个提交违规，
B0-B 的 `feat: 接入生产 VM 运行契约` **已改正**——保持。本批第一个提交（补 lock）用 `fix:`。

### ★ 提交正文必须写「为什么」

B0-B 的正文是三段"为什么"加一段"验证："，**这是本仓目前最好的形态**，保持。
本批会触碰 `verified` 模块的 UseDocs 与登记，以及三段错误类型的取舍——那些正是
审核者最需要解释的地方。

### 其他

- 新增的每个 `pub` 项都要有文档注释（公共 API 100%、全仓 ≥90% 是硬门槛）。
- 新增公开项要按 B0-A 的既有做法在 `research::` 别名层补一条，保持别名层完整
  （`tests/benchmarks` 是它的现成回归夹具）。
- `A0-SIZE-001`：`frontend.rs` 344 行，本批若显著增长，超 2500 行要走旁置 md 豁免。

---

## 十、验收

沿用 09R2D 的「撤掉实现 → 用例必须失败 → 还原 → 通过」。**关键验收不是「测试通过」**：

1. **端到端跑通（已通过）**：`b0_c_driver.rs` 用真实源码走
   `FrontendCompiler` → `lower_program` → `verify_for_execution` → `run_request`，脚本模式与
   `[main]` 工程模式各一条。
2. **三段失败各有断言（已通过）**：前端失败、验证失败、执行失败都返回结构化结果，
   且各自的 `code` 稳定；请求字段拒绝另有断言。
3. **区分度（已通过）**：私有单元测试注入结构错误的 `TacProgram`，驱动器在 VM 前返回
   `X09-BYTECODE-001`，公共测试另验证损坏 IR 的 `X09-BYTECODE-002`。
4. **§4.1 有守卫（已通过）**：公开 `DriverRequest` 没有指定入口函数或机型选择字段，
   函数零由 B0-B 生产 ABI 固定。
5. **§5 有结论（已通过，保留债项）**：取消/超时采用方案 A；`B0-C-CANCEL-001` 的出口
   批次是 `11/X0`。
6. **§6 同批完成（已通过）**：UseDocs、模块登记和入链已同步。
7. **门禁（已通过）**：Rust workspace 测试、clippy、格式化、文档、`tests/benchmarks`
   检查、Bun 测试、仓库检查和文档覆盖率均通过。

---

## 十一、不负责与不要重复做的事

- **不要接 `xiao run` / TypeScript CLI**（11/X0）、**不要做诊断窗口**（X0）。
- **不要重做 B0-B 的入口 ABI**：`RunRequest` / `run_request` 已交付并有测试。
- **不要重做 B0-A 的验证器**：驱动器是**消费**它，不是重写。
- **不要移除 `research::` 别名层**（§7），也不要在本批改测试与基准工具的路径。
- **不要为多模块做预留设计**（方向稿 §2.3）。
- **不要删掉 `tests/benchmarks/` 或三个载体**。

## 相关页面

- [09-B0. 字节码最小运行闭环](09b0-bytecode-closure.md) —— 方向稿，§5 是本批范围
- [09-B0-B. 生产 VM 执行闭环](09b0b-production-vm.md) —— 上一批，本批消费它的入口
- [09. 字节码运行模式](09-bytecode-runtime.md) —— `:44` 取消超时边界、`:120` 结构化结果
- [09R. 字节码寄存器机型特别研究](09r-bytecode-machine-research.md) —— `:648-649` 是本批的权威定义
- [09R2D. 两种机型与指令编码器交接文档](09r2d-machines-and-encoder.md) —— 开发规定主表
- [08A. U0 统一前端实现交接记录](08a-u0-frontend-implementation.md) —— `FrontendArtifact` 产出方
- [00A. 工程框架与目录布局](00a-project-layout.md) —— 依赖方向图
- [12. 测试与开发里程碑](12-tests-and-milestones.md) —— B0 退出条件原文 `:672-679`
