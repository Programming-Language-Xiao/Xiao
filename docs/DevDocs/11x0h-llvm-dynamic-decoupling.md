# 11X0-H. `dynamic.rs` 解耦交接文档

> **本批是预防性拆分，不是功能开发。** `xiao-codegen-llvm/src/dynamic.rs` 已到
> **2172 / 2500 = 87%**，而它是**原生后端的动态值降低器**——原生侧每加一类动态能力
> （新容器、表方法、错误路径）都要经过它。
>
> 同轮发现的另两份见 [11X0-F. `protocol.rs` 解耦](11x0f-protocol-decoupling.md)（94%）
> 与 [11X0-G. `checker.rs` 解耦](11x0g-type-checker-decoupling.md)（88%）。
> **三份各自独立做。**
>
> **模板是 [09R2E](09r2e-research-encoder-decoupling.md)**。
>
> **⚠️ 接手前先读 §8 与 §9**：§8 是上一批（`checker.rs`）**尚未收尾的两件事**
> （一个门禁失败 + 一批空正文），**先做完再动本批**；§9 是**新会话必读**的跨批次约束
> （门禁清单、公共 API 硬门槛、`check:lock`、提交正文要求等）——它们不因批次更替失效。

## 一、Agent 交接上下文

### 接手前提

1. [09R2E. 研究编码器模块解耦交接记录](09r2e-research-encoder-decoupling.md) —— **本批的模板**。
2. [10A. LLVM 原生构建闭环](10a-n0-native-closure.md) —— **本模块的来源**。§1.1 的
   `CODEGEN_VERSION`、§1.2 的批次表、§3.3 的 `str` 决策都在这里；
   **`dynamic.rs` 是 N0-B 的产物**。
3. [10B. N0-B Runtime ABI 交接文档](10b-n0-runtime-abi.md) —— 动态值的 ABI 形状
   （tagged value、句柄协议、`%xiao.value`）。**拆分不得改变它。**
4. [00E. 单文件行数门禁交接](00e-file-size-gate.md) —— `A0-SIZE-001` 与旁置 md 豁免。
   **本批的目标是根本不需要豁免。**
5. [09R2D. 两种机型与指令编码器交接文档](09r2d-machines-and-encoder.md) —— 开发规定主表。
6. [11X0-F. `protocol.rs` 解耦交接](11x0f-protocol-decoupling.md) —— 同轮发现的另一份，
   分步提交与门面保留的做法相同。

### 拆分前基线（2026-09-23 实测）

```text
core/rust/crates/xiao-codegen-llvm/src/dynamic.rs   2172 行 → 占阈值 87%，无内联测试
core/rust/crates/xiao-codegen-llvm/src/ir.rs        1799 行 → 72%（本批不动它）
core/rust/crates/xiao-codegen-llvm/tests/n0_a.rs      测试在独立目录，且大量用 #[ignore] 门控
```

**一个有利条件**：`dynamic.rs` 与 `ir.rs` 的**分工本来就是清楚的**——模块文档自己写了
「本模块与 N0-A 的静态标量降低器**分开维护**。它只在 IR 明确包含字符串、容器或表时启用……
静态程序仍由 `crate::ir::lower_static_program` 生成原生 `i64`/`f64`/`i1`，
**不会因为本模块存在而链接 Runtime**」。

**所以本批不是"把一团糊拆开"，而是"把一个已经清晰的分工，在文件层面落实"。**

**为什么现在拆**：余量 328 行，但原生侧还有很多没做——**N0-C**（统一错误路径与
`try`/`catch`/`finally`）、**N0-D**（固定宽度与裁剪），以及第 15 阶段的优化。
每一个都要改这个文件。

### 本批交付与不负责

**交付**：把 `dynamic.rs` 按职责拆为门面 + 子模块，**生成的 LLVM IR 一个字节不变**。

**不负责**：任何降低语义变更、新容器、新指令。**本批不新增能力**。

---

## 二、切面：**按职责切，不按行数切**

现在的职责块（行号为 2026-09-23 实测，**接手时先按名字定位**）：

| 职责 | 大致范围 | 大致体量 |
| --- | --- | --- |
| **IR 谓词族** | `:33-368` | `abi_field_type`、`name_key`、`scope_is_ancestor`；`type/statement/expression_uses_runtime`；`type/statement/expression_uses_container_abi` |
| **生成器状态** | `:369-490` | `Slot`、`LoopLabels`、`DynamicGenerator`、`new`、`generate` |
| **槽收集** | `:491-644` | `collect_slots`、`collect_table_initializers`、`insert_slot`、`collect_value_slots`、`validate_release_scope_boundary` |
| **运行时声明与 ABI 调用** | `:720-886` | `declare_runtime`、`value_return_is_indirect`、`bytes_parameter_type`、`emit_bytes_argument`、`value_declaration`、`emit_value_call` |
| **入口** | `:887-996` | `emit_entry`、`main_adapter`、`emit_observation_return`、`record_observation` |
| **语句与控制流** | `:997-1276` | `emit_statement`、`emit_if`、`emit_elif_chain`、`emit_while`、`emit_condition` |
| **表达式与字面量** | `:1277-1738` | `emit_expression`、`emit_bytes_value`、`emit_literal`、`emit_string_literal`、`emit_text_value`、`emit_sequence`、`emit_dictionary`、`emit_set`、`emit_new_call` |
| **表描述符** | `:1739-1869` | `emit_table_descriptor`（约 130 行） |
| **槽读写与释放** | `:1870-2044` | `load_slot`、`store_slot`、`none_value`、`release_value`、`release_for_exit`、`release_all_slots_fallback` |
| **发射原语** | `:2045-2088` | `emit`、`emit_label`、`next_temp`、`next_label`、`check_status`、`checked_status_call` |
| **纯文本辅助** | `:2089-2172` | `parse_i64`、`parse_i32`、`format_float`、`is_identity_cast`、`unquote`、`escape_bytes`、`escape_llvm`、`stable_hash` |

**建议的落点**（名称可调，**分层原则不能调**）：

```text
dynamic.rs                   门面：DynamicGenerator + generate + pub use 子模块
dynamic/predicate.rs         IR 谓词族（两组 uses_* —— 纯函数，无 self）
dynamic/slot.rs              槽收集与读写
dynamic/runtime_abi.rs       运行时声明与 ABI 调用原语
dynamic/entry.rs             入口与 main 适配器
dynamic/control.rs           语句与控制流
dynamic/expression.rs        表达式与字面量发射
dynamic/container.rs         容器构造与表描述符
dynamic/release.rs           释放计划发射
dynamic/text.rs              纯文本辅助（转义、解析、哈希）
```

`DynamicGenerator` 的 `impl` 块**可以分散在多个文件**。

### 2.1 已实测：两个辅助**各有一份重复实现**（**既有 A 类病**）

`:2089-2172` 的纯文本辅助里，有三个在 `ir.rs` 也有：

```text
escape_llvm    ir.rs:1787   fn escape_llvm(text: &str) -> String
               dynamic.rs:2160  fn escape_llvm(text: &str) -> String
stable_hash    ir.rs:1792   fn stable_hash(bytes: &[u8]) -> String
               dynamic.rs:2165  fn stable_hash(bytes: &[u8]) -> String
format_float   ir.rs:1747   fn format_float(value: f64) -> String
               dynamic.rs:2109  fn format_float(text: &str, span: IrSpan) -> Result<String>
```

**前两个的签名与实现逐字节相同**——这正是本仓登记过 **7 次**的 A 类病形态
（「同一条规则在两层各写一份，然后漂移」）。而它们是**产物相关**的：

- `escape_llvm` 决定 `.ll` 文本里字符串常量的转义；
- `stable_hash` 决定**构建指纹**（`10A` §1.1 已定它进工具链指纹）。

**当前两份相同，所以没有症状**——但改一处忘另一处是迟早的事，而症状会是
「静态路径与动态路径产出的 `.ll` 头不一致」，极难定位。

**第三个不是重复**：`format_float` 两侧签名不同（一个收 `f64` 返 `String`，
一个收 `&str` + `IrSpan` 返 `Result`），是**同名但职责不同的两个函数**，
**不要合并**。

**本批要做的**：把 `escape_llvm` 与 `stable_hash` **合并为一份**，放到 crate 级公共位置
（例如 `codegen/text.rs`），由 `ir.rs` 与 `dynamic/` 共同消费。

**这既解决了 §2 里"`text.rs` 落在哪一层"的问题，也顺手治好一个既有缺陷。**
判据：合并后两侧都不再各自定义它们，且 §3.1 的 `.ll` 逐字节比对仍然通过
（证明合并没有改变行为）。

---

## 三、硬约束

### 3.1 生成的 LLVM IR **一个字节不变**

最强的一条验收不是"测试通过"，而是：**同一份输入产生的 `.ll` 文本逐字节相同**。

`:2089-2172` 的 `format_float`、`escape_llvm`、`stable_hash` 直接决定输出文本——
**搬动它们时任何一处行为偏差都会改变产物**，而 `10A` §1.1 已定 `CODEGEN_VERSION`
进构建指纹：**IR 变了就该升版本号**。本批的目标是**不升**。

**做法**：拆分前后各生成一次同一批程序的 `.ll`，`diff` 结果必须为空（可写成一个临时脚本）。

### 3.2 静态 / 动态边界**不能模糊**

`ir.rs:90` 的 `LlvmModule.uses_runtime` 是 N0 的核心不变量，`ir.rs:293` 明确写死
静态路径产 `uses_runtime: false`，而 **N0-A 的 `assert!(!module.uses_runtime)` 是这条的
探测器**。

**拆分不得**让静态路径意外依赖 `dynamic/` 的任何东西。**架构回归测试要覆盖这条**：

```text
ir.rs       不得 use crate::dynamic::*
dynamic/*   可以 use crate::ir::{LlvmModule, CodegenOptions, ...}
```

**方向只有一个**——与 09R2D §2.5 的解耦约束一致，也与 `09R2E` 的 DAG 画法一致。

### 3.3 公开 API 不变

`dynamic.rs` 的公开面（`DynamicGenerator` 及其 `generate` 等）被 `lib.rs` 重导出。
拆分后 `dynamic.rs` **保留为门面**，`pub use` 子模块；调用方不改 import。

### 3.4 每一处都要复核的四样东西

照 [09R2E](09r2e-research-encoder-decoupling.md)：局部 `README.md`、允许/禁止依赖说明、
架构回归测试、`docs/module-registry.json` 与 `docs/DevDocs/README.md` 的相关行。

---

## 四、分步提交（**不要一笔拆完**）

| 步 | 内容 | 为什么这个顺序 |
| --- | --- | --- |
| **1** | 搬 `predicate.rs`（`:33-368` 的两组 `uses_*`） | **纯函数、无 `self`**，最安全 |
| **2** | **先合并**重复的 `escape_llvm`/`stable_hash`（§2.1），再搬其余文本辅助 | 合并影响产物文本，单独一步便于逐字节验证 |
| **3** | 搬 `runtime_abi.rs` + `entry.rs` | ABI 调用与入口，边界清晰 |
| **4** | 搬 `slot.rs` + `release.rs` | 槽与释放耦合紧，一起搬 |
| **5** | 搬 `control.rs` + `expression.rs` + `container.rs` | 最重的一块，占半数行数 |
| **6** | 门面收尾 | |

**每一步的验收**：`.ll` 逐字节不变（§3.1）、门禁全绿、`uses_runtime` 断言通过。

**第 1 步应当是纯移动**——`git show -M --stat` 应当看到重命名而非增删。

---

## 五、最可能翻车的地方

1. **产物文本变了**（§3.1）——浮点格式化、转义、哈希这三处最容易。
   **每一批都要 diff `.ll`**，不要等到最后。
2. **把重复的 `escape_llvm`/`stable_hash` 搬进 `dynamic/` 私有模块**（§2.1）——
   那会**固化**现有的两份实现，或逼 `ir.rs` 反向依赖 `dynamic/`（**方向错了**）。
3. **静态路径意外依赖 `dynamic/`**（§3.2）——`!uses_runtime` 断言会抓到它，
   但那意味着拆分方向已经错了。
4. **顺手改了降低语义**。本批**不新增任何能力**。
5. **忘了 `CODEGEN_VERSION`**——如果最终产物确实变了，就必须升它（`10A` §1.1），
   并同步 16 阶段的构建指纹说明。**本批的目标是不升**；若不得不升，要写明原因。
6. **一笔拆完**（§4）。
7. **忘了锁文件**：拆分**不应**增删 Rust 依赖，若增删了，`check:lock` 会拦住。

---

## 六、验收

沿用 09R2D 的「撤掉实现 → 用例必须失败 → 还原 → 通过」。**关键验收不是「测试通过」**：

1. **`.ll` 逐字节不变**（§3.1）——同一批程序在拆分前后产生相同文本；
2. **`CODEGEN_VERSION` 未升**（若升了，说明产物变了，必须解释）；
3. **`!module.uses_runtime` 断言仍通过**，静态路径不依赖 `dynamic/`（§3.2）；
4. **公开 API 逐条不变**；
5. **`check:layout` 干净**，`dynamic.rs`（门面）显著低于阈值，**不需要**旁置 md 豁免；
6. **重复实现已合并**——`escape_llvm` 与 `stable_hash` 在两侧都不再各自定义（§2.1）；
7. **架构回归测试到位**，`README.md` / 依赖说明 / 模块登记同批更新；
8. **门禁全绿**，含 `check:lock`、`bunx tsc`。

---

## 七、不负责与不要重复做的事

- **不改降低语义**、不加容器、不加指令。
- **不动 `ir.rs`**（1799 行，72%）——它是本批要**守住边界**的对象，不是拆分对象。
- **不升 `CODEGEN_VERSION`**（除非产物确实变了并写明原因）。
- **不要顺手拆其他高位文件**——`protocol.rs` 与 `checker.rs` **各有独立交接文档**。
- **不要用旁置 md 豁免替代拆分**（`00E` 明说它是最后手段）。

## 八、**接手前先做**：上一批（`checker.rs`）的收尾

`11X0-G`（`checker.rs`）已经落地——形状完全按文档做到了（2200 → 166 行门面，
六个子模块与 §2 的落点表一字不差，`RuntimeCheckKind` 仍可达，`tests/` 零 diff）。
**但它留下了两件必须收尾的事，先处理完再动 `dynamic.rs`。**

### 8.1 一个**门禁失败**要修（`bun run check` exit 1）

```text
[error] A0-COVERAGE-001 公共 API 文档覆盖率 99.97% 低于 100%
[error] A0-COVERAGE-002 xiao-types/src/checker.rs:63 reexport use：公共声明缺少代码文档
公共 API：3001/3002
```

**根因就一行**——门面里那个显式重导出没有文档注释：

```rust
pub use self::result::{RuntimeCheck, RuntimeCheckKind, TypeCheckResult, TypedNode};
```

**`pub use` 本身算「公开声明」**，而公共 API 覆盖率是 **100% 的硬门槛**
（不是"全仓 ≥90%"那条）。另有 **13 个内部项**缺文档（`constant.rs` 2 项、
`expression.rs` 若干等），它们是 warning 但也要补。

**修法**：给该 `pub use` 加文档注释、补齐那 13 项，然后 `bun run check` 必须 **exit 0**。

### 8.2 一批提交**缺正文**，用下一笔补说明

`checker.rs` 那批有 **8 个提交正文为空**（6 个 `refactor(types)` + 2 个 `docs(types)`），
而其中 6 个 `refactor` **正是最需要说明"这只是移动、没有语义变更"的**。

**不要改写历史**（成本高且无必要）。**用下一笔提交说明这批改动的性质**，
并给出"无语义变更"的证据——**最强的证据已经存在**：

> 四个快照测试（`c0c1_snapshots.rs`、`c2a_snapshots.rs`、`c2b_snapshots.rs`、
> `c2c_snapshots.rs`）**一个字符没改却全部通过**。它们比对的是**类型检查结果的结构**，
> 所以"结构没变"是被测试锁住的，不是靠声称。

**这条要做在 `dynamic.rs` 之前**，免得两批的说明混在一起。

---

## 九、本批的硬约束（**新会话必读**）

接手者可能没有前序批次的上下文。以下约束**跨批次有效，不因批次更替失效**；
本批（`dynamic.rs`）与后续任何解耦批次都适用。

### 9.1 门禁清单（每次提交前全跑）

```text
cargo test --manifest-path core/rust/Cargo.toml --workspace
cargo clippy --manifest-path core/rust/Cargo.toml --workspace --all-targets -- -D warnings
cargo fmt --manifest-path core/rust/Cargo.toml --all -- --check
cargo doc --manifest-path core/rust/Cargo.toml --workspace --no-deps
bun test
bunx tsc --noEmit -p tsconfig.json
bun run check              ← 含 check:lock，见 9.3
bun run check:coverage     ← 公共 API 硬门槛，见 9.2
```

### 9.2 **公共 API 覆盖率 100% 是硬门槛**（§8.1 就是踩了这条）

- **`pub use` 算公开声明**，重导出行**也要有文档注释**；
- `pub` 项、`export` 项、公共模块入口都是 100%；
- 全仓声明项是 **≥90%**——**两条门槛不同，别混**。

### 9.3 `check:lock` 是**已固化**的门禁

`tests/benchmarks` 是**独立 crate**，它的 `Cargo.lock` 不在 `core/rust` workspace 覆盖内。
这个盲区造成过**三次**漏提交，写文档提醒三次都没生效，现已固化为 `check:lock`
并**并入 `bun run check`**。**本批若给任何 Rust crate 增删依赖**，跑完 `cargo check`
后锁文件有 diff 就是漏提交。判据与来由见 [00A.1](00a-a0-workspace-and-checkers.md)。

### 9.4 **`bun run check` 要看完整输出，不要 `| tail`**

它的 warning **不影响退出码**。`| tail -3` 会显示"通过"而**吞掉几十条告警**——
本仓真的漏看过 83 条。核对时用 `grep -c` 计数。

### 9.5 ★ 两条提交约束**分别**核对

1. **标题带规范前缀**（`feat:`/`fix:`/`test:`/`docs:`/`chore:`；`refactor:` 与带 scope 的
   `feat(x0-e):` 同样合规）；
2. **正文说明为什么**——**空正文是违规**（§8.2 就是踩了这条）。

复发史：`261d88e`/`66f0cec`/B0-A 两个（标题缺）→ `2a33680`（正文缺）→
之后各批守住 → **`checker.rs` 那批 8 个提交正文为空，再次复发**。

**对解耦批次尤其重要**：审核者必须能**只读正文**就知道"这笔只是移动、没有语义变更"，
而不是读 diff 反推。

### 9.6 环境依赖测试按 [10D](10d-environment-gated-test-spec.md) 写

本仓的原生相关测试大量依赖外部工具链。**一律用显式的跳过机制**
（`#[test] #[ignore]`），**不允许条件 `return`**——那会让"没跑"和"通过"在汇总里
长得一模一样。跳过的数量要在默认门禁里**可见**（`cargo test` 会打印 `N ignored`）。

### 9.7 单一来源原则（本仓的**头号病**）

「同一条规则在两层各写一份，然后漂移」——**已登记过 7 次**。
`11X0-H` §2.1 发现的 `escape_llvm`/`stable_hash` 重复实现**就是这个病**，
只是还没发作。**解耦时如果发现重复实现，顺手合并掉**，别把它搬进新结构里固化下来。

---

## 相关页面

- [09R2E. 研究编码器模块解耦交接记录](09r2e-research-encoder-decoupling.md) —— **本批的模板**
- [11X0-F. `protocol.rs` 解耦交接](11x0f-protocol-decoupling.md) —— 同轮发现的另一份
- [11X0-G. `checker.rs` 解耦交接](11x0g-type-checker-decoupling.md) —— 同轮发现的第三份
- [10A. LLVM 原生构建闭环](10a-n0-native-closure.md) —— 本模块的来源与 `CODEGEN_VERSION`
- [10B. N0-B Runtime ABI 交接文档](10b-n0-runtime-abi.md) —— 动态值的 ABI 形状
- [00E. 单文件行数门禁交接](00e-file-size-gate.md) —— `A0-SIZE-001` 与豁免机制
- [00A.1 工作区与质量门禁](00a-a0-workspace-and-checkers.md) —— `check:lock` 的判据（§9.3）
- [11X0-G. `checker.rs` 解耦交接](11x0g-type-checker-decoupling.md) —— 上一批，其收尾见 §8

---

## 十、本批落地记录（2026-09-23）

- `dynamic.rs` 已从拆分前的 2172 行收敛到 248 行门面；实现按职责落在 `predicate.rs`、
  `text.rs`、`runtime_abi.rs`、`entry.rs`、`slot.rs`、`release.rs`、`control.rs`、
  `expression.rs` 和 `container.rs`。
- `escape_llvm` 与 `stable_hash` 已合并到 crate 级 `src/text.rs`，静态 `ir.rs` 与动态
  门面共同消费；动态文本解析仍保留在 `dynamic/text.rs`，没有固化重复实现。
- 新增 `dynamic_architecture_tests.rs`、`src/dynamic/README.md` 和两级 LLVM README，
  源码级锁定门面不回流职责实现、`ir.rs` 不依赖 `dynamic/`、子模块不形成兄弟反向依赖。
- `CODEGEN_VERSION` 保持为 2，`xiao-runtime-abi` 形状、静态/动态选择、所有权释放顺序、
  Runtime 组件清单和 LLVM 发射调用顺序均未改变；既有 N0-A/N0-B 测试计数保持不变。
- 分步提交为 `c2f0b2b`、`a5aa9b1`、`6ee24f5`、`75d014d`、`13acaf4`；每笔提交均带
  规范标题和正文，并记录“仅移动、无语义变更”的验证依据。
- 本地定向门禁通过：`cargo fmt --manifest-path core/rust/Cargo.toml --all -- --check`、
  `cargo check -p xiao-codegen-llvm`、LLVM crate 全套测试、严格 `clippy`，以及动态架构
  回归测试 `dynamic::architecture_tests::module_dependency_direction_is_acyclic`。
