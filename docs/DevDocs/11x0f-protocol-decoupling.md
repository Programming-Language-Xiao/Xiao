# 11X0-F. `protocol.rs` 解耦交接文档

> **本批是预防性拆分，不是功能开发。** 起因是 `A0-SIZE-001` 的阈值门禁虽然通过，
> 但 `xiao-driver/src/protocol.rs` 已经到 **2354 / 2500 = 94%**，而协议层**天然单调增长**。
> 等它撞线再拆，就要在"必须先拆才能加功能"的压力下动手。
>
> **模板是 [09R2E](09r2e-research-encoder-decoupling.md)**：它把 `encode.rs` 拆成 `encode/` 子模块，
> 保持门面路径与兼容契约不变，并留下源码级架构回归测试。本批照它的形状做。

## Agent 交接上下文

### 接手前提

1. [09R2E. 研究编码器模块解耦交接记录](09r2e-research-encoder-decoupling.md) —— **本批的模板**。
   §「解耦后的结构」给出 DAG 画法，§「兼容契约」给出"公开路径不变"的要求，
   §「验证与交付」给出必须在同一提交更新的四样东西。
2. [00E. 单文件行数门禁交接](00e-file-size-gate.md) —— `A0-SIZE-001` 的判定与旁置 md 豁免。
   **本批的目标是根本不需要豁免。**
3. [00F. 解析器第二批模块解耦交接](00f-parser-decoupling.md) —— 另一次同类拆分的先例
   （`parser.rs` → `parser/statements.rs`）。
4. [11X0-A. 跨平台工具链](11x0-cli-protocol-and-toolchain.md) §2 —— 协议的**单一来源**
   对策（方案 C：共享 fixture + 双向回环测试）。拆分**不得**破坏它。
5. [11X0. 跨平台工具链](11x0-cli-protocol-and-toolchain.md) —— 方向稿与批次边界。
6. [09R2D. 两种机型与指令编码器交接文档](09r2d-machines-and-encoder.md) —— 开发规定主表，
   特别是 §2.5 的解耦约束。

### 现状盘点（2026-09-23 实测）

```text
core/rust/crates/xiao-driver/src/protocol.rs        2354 行 → 占阈值 94%，其中实现 2350 行
core/rust/crates/xiao-driver/src/protocol_tests.rs   432 行（测试已独立，这点做得对）
```

**关键数据**：`protocol.rs` 里 `#[cfg(test)]` 从**第 2351 行**才开始——也就是说
**2354 行里几乎全是实现**，不是测试撑大的。同样处于高位的还有 `xiao-types/src/checker.rs`
（2200 / 88%）与 `xiao-codegen-llvm/src/dynamic.rs`（2172 / 87%）——
**那两份各有独立的解耦交接文档，本批先完成 `protocol.rs` 这一份。**

**为什么现在拆**：

- **余量只剩 150 行**；
- 它**还会继续长**：`X0-T`（项目测试语义与结果协议）、`09-B0-E`（VM 中途取消检查点）
  都要动协议，将来还有 `.xar`（17）与包管理（11A）的新命令与字段；
- **协议层是单调增长的**——它只会加类型、加字段、加端点。这不是"某次写胖了"，
  是**结构性趋势**。

### 本批交付与不负责

**交付**：把 `protocol.rs` 按职责拆为门面 + 子模块，公开 API 与行为**完全不变**。

**不负责**：任何协议语义变更、新命令、新字段。**本批不新增能力**——
它是一次纯组织调整，只是跨越多个提交。

---

## 二、切面：**按职责切，不按行数切**

先看清它现在有哪几块职责（行号为 2026-09-23 实测，**接手时先按名字定位**）：

| 职责 | 大致范围 | 大致体量 |
| --- | --- | --- |
| **常量与版本** | `:43-64` | 版本、帧参数、7 个错误码 |
| **配置与请求类型** | `:68-389` | `CoreVersions`、`ProtocolTarget`、`OptimizationConfig`、`DiagnosticConfig`、`SourceIdentity`、`RunOptions`、`ToolchainSpec`、`ProtocolRequest`、`ProtocolParam` |
| **协议消息类型** | `:390-649` | `ProtocolSpan/Diagnostic/StackFrame/BackendLocation/Report/Event/Metrics/Value/ErrorBody`、`ProtocolResponse`、`ProtocolArtifact`、`ProtocolRuntimeConfig`、`ProtocolDiagnosticActivation` |
| **错误类型** | `:650-786` | `FrameError`、`ProtocolError` |
| **帧编解码** | `:787-873` | `encode_frame`、`decode_frame`、`read_frame`、`write_frame`、`read_request` |
| **内部 → 协议映射** | `:874-1145` | `protocol_diagnostic` / `_report` / `_metrics` / `_event` / `_value` / `_stack_frame`、`exit_name`、`protocol_error_body` |
| **校验** | `:1145-1190` | `validate_versions` / `_source` / `_target` |
| **run 路径** | `:1191-1378` | `run_options`、`run_response`、`run_with_diagnostics`、`rejected_response`、`executed_response` |
| **build 路径** | `:1379-1722` | `build_toolchain`、**`build_response`（单函数 301 行）**、`diagnostics_component_path`、`stage_diagnostics_component`、`cleanup_native_outputs` |
| **运行时配置固化** | `:1723-1900` | `freeze_runtime_config`、`config_document_value`、`config_value`、`write_runtime_config`、`runtime_config_path` |
| **服务与分发** | `:1901-2350` | `dispatch`（150 行）、`worker_response`、`serve`（144 行）、`serve_stdio`、`core_crash_response` |

**建议的落点**（名称可调，**分层原则不能调**）：

```text
protocol.rs          门面：版本常量、协议错误码、pub use 子模块（保持现有公开路径）
protocol/frame.rs    帧编解码 + FrameError（叶子：只依赖 serde 与 std::io）
protocol/message.rs  协议消息类型（叶子：只依赖 serde）
protocol/request.rs  配置与请求类型（叶子）
protocol/mapping.rs  内部 → 协议映射（依赖 message + 各内部 crate）
protocol/validate.rs 请求校验（依赖 request + message）
protocol/run.rs      run 路径
protocol/build.rs    build 路径（注意那个 301 行的函数要再拆）
protocol/config.rs   运行时配置固化
protocol/service.rs  dispatch / worker / serve（依赖以上全部）
```

**三个大函数要在拆分中一并处理**，否则它们会把新文件重新撑胖：

- `build_response`（301 行）——按"工具链准备 / 编译链接 / 产物整理"再切；
- `dispatch`（150 行）——按命令分支切；
- `serve`（144 行）——按"读循环 / 分发 / 写回 / 崩溃处理"切。

---

## 三、硬约束

### 3.1 公开 API **一个都不能少**

`protocol.rs` 现在是 `xiao-driver` 的公开面，`lib.rs` 从它重导出。拆分后：

- `protocol.rs` **保留为门面**，用 `pub use` 把子模块的项重新导出；
- **既有调用方（含 `protocol_tests.rs` 与 `cli/ts` 的协议夹具）不需要改 import**——
  这是 [09R2E](09r2e-research-encoder-decoupling.md) §「兼容契约」的同一条；
- 拆完后 `cargo doc` 的公开项清单应当**逐条不变**（可作为一条人工核对项）。

### 3.2 依赖 DAG 与**架构回归测试**

照 [09R2E](09r2e-research-encoder-decoupling.md) 的做法，在文档里写明允许/禁止依赖，
并加一条**源码级测试**（它那边的名字是 `module_dependency_direction_is_acyclic`）。

**必须成立的方向**：

```text
frame / message / request   ← 叶子，不依赖任何上层
       ↓
mapping / validate
       ↓
run / build / config
       ↓
service                     ← 允许依赖以上全部，且只有它依赖 std::thread / Arc
```

**禁止**：`encoder`/`decoder` 式的横向依赖；叶子依赖服务层；任何子模块反向依赖门面。

### 3.3 `protocol_tests.rs` 与共享 fixture

- **`tests/spec/11x0-protocol/` 的 fixture 不动**——它是协议的**单一来源证据**（方案 C），
  拆分**不得**触碰它；
- `protocol_tests.rs`（432 行）**理想情况下一个字符不改**——如果它因为拆分需要改 import，
  说明 §3.1 的门面重导出**没做全**，那是拆分本身的缺陷，不是测试的问题。

### 3.4 每一处都要复核的四样东西

照 [09R2E](09r2e-research-encoder-decoupling.md) §「解耦后的结构」：

1. 局部 `README.md`（`xiao-driver/src/README.md` 与新增子目录的）；
2. 允许/禁止依赖的说明；
3. 架构回归测试；
4. `docs/module-registry.json` 的 `code` 数组与 `docs/DevDocs/README.md` 的相关行。

---

## 四、分步提交（**不要一笔拆完**）

2350 行一次性挪动，评审时无法判断"有没有语义变更"。**按"叶子优先"分批**，
每批独立可审：

| 步 | 内容 | 为什么这个顺序 |
| --- | --- | --- |
| **1** | 搬 `frame.rs` + `FrameError` | **最纯的叶子**，只依赖 serde 与 `std::io`，搬完立刻能验证 |
| **2** | 搬 `message.rs` + `request.rs` | 两个纯类型模块，无行为 |
| **3** | 搬 `mapping.rs` | 纯函数，输入输出都是类型 |
| **4** | 搬 `validate.rs` | 小而独立 |
| **5** | 搬 `run.rs` + `config.rs` | 服务路径的前半 |
| **6** | 搬 `build.rs`，**并拆分那个 301 行的函数** | build 路径最重 |
| **7** | 搬 `service.rs`，**并拆分 `dispatch` / `serve`** | 最后，它依赖前面全部 |

**每一步的验收**：`protocol_tests.rs` 不改、fixture 不改、公开 API 不变、门禁全绿。

**第 1 步应当是纯移动**——用 `git show -M --stat` 应当看到重命名而非增删
（差异只来自 `use` 路径改写）。

---

## 五、最可能翻车的地方

1. **顺手改了行为**。本批**不新增任何能力**；如果拆的过程中"发现某处可以顺便修一下"，
   **另开提交**。
2. **门面重导出不全**，逼得 `protocol_tests.rs` 改 import（§3.3）——那是拆分没做对。
3. **碰了 `tests/spec/11x0-protocol/` 的 fixture**（§3.3）。它们是跨语言单一来源证据。
4. **只拆文件不拆大函数**（§2 末）——`build_response` 301 行会把新的 `build.rs` 重新撑胖。
5. **叶子依赖了服务层**（§3.2 的 DAG）——那等于把耦合从"行数"换成了"方向"，更糟。
6. **一笔拆完**（§4）——2350 行的单次评审无法判定语义未变。
7. **忘了锁文件**：拆分**不应**增删 Rust 依赖，若增删了，`check:lock` 会拦住（已固化）。

---

## 六、验收

沿用 09R2D 的「撤掉实现 → 用例必须失败 → 还原 → 通过」。**关键验收不是「测试通过」**：

1. **`check:layout` 干净**，且 `protocol.rs`（门面）**显著低于**阈值，
   **不需要**旁置 md 豁免；
2. **公开 API 逐条不变**——`cargo doc` 的 `xiao-driver` 公开项清单与拆分前一致；
3. **`protocol_tests.rs` 与 `tests/spec/11x0-protocol/` 一个字符没改**；
4. **架构回归测试到位**（§3.2 的 DAG 有源码级断言）；
5. **三个大函数已被拆分**（§2 末），没有一个新文件重新逼近阈值；
6. **`README.md` / 依赖说明 / 模块登记同批更新**（§3.4）；
7. **门禁全绿**，含 `check:lock`、`bunx tsc`。

---

## 七、不负责与不要重复做的事

- **不改协议语义**、不加字段、不加命令。
- **不动 `tests/spec/11x0-protocol/` 的 fixture**。
- **不重排公开项的可见性**（`pub` 仍是 `pub`）。
- **不要顺手拆其他高位文件**——`checker.rs` 与 `dynamic.rs` **各有独立交接文档**，
  按各自的批次做。
- **不要用旁置 md 豁免替代拆分**（`00E` 明说它是最后手段）。

## 相关页面

- [09R2E. 研究编码器模块解耦交接记录](09r2e-research-encoder-decoupling.md) —— **本批的模板**
- [11X0-A. 跨平台工具链](11x0-cli-protocol-and-toolchain.md) §2 —— 协议的单一来源（方案 C）
- [00E. 单文件行数门禁交接](00e-file-size-gate.md) —— `A0-SIZE-001` 与豁免机制
- [00F. 解析器第二批模块解耦交接](00f-parser-decoupling.md) —— 同类拆分的先例
- [09R2D. 两种机型与指令编码器交接文档](09r2d-machines-and-encoder.md) —— 开发规定主表
