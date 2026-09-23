# 11X0-G. `checker.rs` 解耦交接文档

> **本批是预防性拆分，不是功能开发。** `xiao-types/src/checker.rs` 已到 **2200 / 2500 = 88%**，
> 而它是类型层的核心——**每个新语法、每个新容器规则都要经过它**。
>
> 与本文同批的还有 [11X0-F. `protocol.rs` 解耦](11x0f-protocol-decoupling.md)（94%）与
> `11X0-H. dynamic.rs 解耦`（87%）。三份是同一轮阈值盘点发现的，**按各自批次独立做**。
>
> **模板是 [09R2E](09r2e-research-encoder-decoupling.md)**：门面保留、公开路径不变、
> 留源码级架构回归测试。

## Agent 交接上下文

### 接手前提

1. [09R2E. 研究编码器模块解耦交接记录](09r2e-research-encoder-decoupling.md) —— **本批的模板**。
2. [00E. 单文件行数门禁交接](00e-file-size-gate.md) —— `A0-SIZE-001` 的判定与旁置 md 豁免。
   **本批的目标是根本不需要豁免。**
3. [00F. 解析器第二批模块解耦交接](00f-parser-decoupling.md) —— 同类拆分的先例。
4. [02. 类型与值系统](02-type-system.md) 与 [02A. P2-B/S0 静态标量类型实现交接记录](02a-p2-static-types.md)
   —— `checker.rs` 的**职责来源**；拆分不得改变它声明的任何规则。
5. [09-B0-A. 字节码最小运行闭环](09b0-bytecode-closure.md) §3.3 —— **`RuntimeCheckKind` 的
   特殊地位**（见 §3.2）。这是本批最容易踩的一处。
6. [09R2D. 两种机型与指令编码器交接文档](09r2d-machines-and-encoder.md) —— 开发规定主表。

### 现状盘点（2026-09-23 实测）

```text
core/rust/crates/xiao-types/src/checker.rs   2200 行 → 占阈值 88%，无内联测试
core/rust/crates/xiao-types/tests/           测试在独立目录（c0/c1/c2a/c2b/c05/... 十余个文件）
```

**两点是有利的**：

- **测试不在 `src/`**——内部重组**不会碰到它们**，比 `protocol.rs`（需要门面重导出兜住
  `protocol_tests.rs`）的风险更低；
- **后半部分是纯函数族**——`:1959-2200` 的常量求值（`eval_const_binary` 到 `finite_float`）
  自成一体，**是最安全的第一个切面**。

**为什么现在拆**：余量 300 行，但类型层是**每个语言特性都要落地的必经之路**——
`Result` 泛型、模式匹配、错误码/条件（07-C/07-D 登记过的）、并发阶段的类型轴，
每一样都要改这里。它不是"某次写胖了"，是**结构性趋势**。

### 本批交付与不负责

**交付**：把 `checker.rs` 按职责拆为门面 + 子模块，**公开 API 与类型规则完全不变**。

**不负责**：任何类型语义变更、新检查规则、新诊断码。**本批不新增能力**。

---

## 二、切面：**按职责切，不按行数切**

现在的职责块（行号为 2026-09-23 实测，**接手时先按名字定位**）：

| 职责 | 大致范围 | 大致体量 |
| --- | --- | --- |
| **公开结果类型** | `:57-300` | `RuntimeCheckKind`、`RuntimeCheck`、`TypedNode`、`TypeCheckResult` |
| **`TypeChecker` 与入口** | `:301-395`、`:1831` | struct 定义、`new`、`check`/`check_program`、`check()` 自由函数 |
| **语句检查** | `:396-861` | `check_statement`、`check_simple_assignment`、**`check_extended_assignment`（150 行）**、`check_declaration` / `_scalar_` / `_const_`、`check_explicit_target` |
| **表达式检查** | `:862-1435` | `check_expression`、`literal_type`、`check_name`、`check_unary`、`check_binary`、**`infer_binary_with_variables`（125 行）**、**`check_call`（144 行）**、`check_new_call`、`check_cast` |
| **转换与常量目标** | `:1436-1684` | `scalar_callee`、`static_conversion_value_is_valid`、`static_target_range_is_valid`、`check_explicit_constant_target`、`eval_const`、`check_constant_target` |
| **诊断上报** | `:1700-1815` | `undefined_name`、`environment_error`、`numeric_error`、`unification_error`、`type_error` / `_with_params`、`push_runtime_check`、`has_errors_since` |
| **常量求值（纯函数）** | `:1816-2200` | `is_random_seed_callee`、`assignment_binary_operator`、`binary_requires_runtime_check`、`decode_string`、`convert_constant`、**`eval_const_binary`**、**`eval_numeric_binary`（90 行）**、`eval_const_unary`、`constant_as_f64`、`constants_equal`、`constants_ordering`、`normalize_decimal`、`finite_float` |

**建议的落点**（名称可调，**分层原则不能调**）：

```text
checker.rs               门面：TypeChecker + check() 入口 + pub use 子模块
checker/result.rs        公开结果类型（RuntimeCheckKind / RuntimeCheck / TypedNode / TypeCheckResult）
checker/statement.rs     语句检查
checker/expression.rs    表达式检查（注意两个大函数要再切）
checker/constant.rs      常量求值（纯函数族，最安全的起点）
checker/conversion.rs    转换与常量目标
checker/diagnostic.rs    诊断上报（所有 *_error 与 push_runtime_check）
```

`TypeChecker` 的 `impl` 块**可以分散在多个文件**——Rust 允许，且这是本批能成立的前提。
**不需要**把 `TypeChecker` 本身拆开。

**三个大函数要在拆分中一并处理**，否则它们会把新文件重新撑胖：
`check_extended_assignment`（150 行）、`infer_binary_with_variables`（125 行）、
`check_call`（144 行）。

---

## 三、硬约束

### 3.1 公开 API **一个都不能少**

`checker.rs` 是 `xiao-types` 的公开面，`lib.rs` 从它重导出。拆分后：

- `checker.rs` **保留为门面**，用 `pub use` 重导出子模块的项；
- **既有调用方不需要改 import**——`xiao-ir` 的降低器、各 `tests/*.rs` 都从 `xiao_types::*` 取，
  路径不变；
- `cargo doc` 的公开项清单**逐条不变**（人工核对项）。

### 3.2 ⚠️ `RuntimeCheckKind` 是**跨 crate 的单一来源**，路径不能变

这是本批**最容易踩的一处**。`RuntimeCheckKind` 定义在 `checker.rs:57`，但：

- `09-B0-B` 把它的**稳定名称与反查**（`as_name` / `from_name`）定为**唯一来源**，
  `xiao-ir` 与 `xiao-bytecode` **都消费这一个入口**——那正是为了消除「两份名单漂移」；
- `09R2F1`/`09R2G`/`B0-A` 的穷尽性守卫与 `intentionally_unsupported` 都建立在它上面。

**所以**：

- 它可以**移到 `checker/result.rs`**（或留在门面），但**必须仍然从 `xiao_types::RuntimeCheckKind`
  可达**；
- `as_name` / `from_name` 的**行为一个字都不能改**；
- `09-B0-A` 的穷尽性守卫测试（遍历 14 个变体）**必须继续通过**。

**动手前先 grep 一遍谁在用它**（`xiao-ir`、`xiao-bytecode`、各测试），确认迁移后路径不变。

### 3.3 测试与快照

- `xiao-types/tests/` 下的十余个测试文件**不应当因为拆分而修改**；
- **快照类测试**（`c0c1_snapshots.rs`、`c2a_snapshots.rs` 等）尤其要留意——
  它们比对的是**类型检查结果的结构**，如果拆分过程中不慎调整了 `TypeCheckResult` 的
  字段顺序或诊断顺序，会被它们抓到。**这是好事**：它们是本批的安全网。

### 3.4 每一处都要复核的四样东西

照 [09R2E](09r2e-research-encoder-decoupling.md)：局部 `README.md`、允许/禁止依赖说明、
架构回归测试、`docs/module-registry.json` 与 `docs/DevDocs/README.md` 的相关行。

---

## 四、分步提交（**不要一笔拆完**）

按"**纯函数优先**"的顺序，每批独立可审：

| 步 | 内容 | 为什么这个顺序 |
| --- | --- | --- |
| **1** | 搬 `constant.rs`（`:1959-2200` 的 `eval_*` 族 + `convert_constant` + `normalize_decimal` + `finite_float`） | **纯函数、无 `self`**，最安全；搬完立刻能验证 |
| **2** | 搬 `result.rs`（公开结果类型） | 无行为；**这一步之后要立刻复核 §3.2 的路径** |
| **3** | 搬 `diagnostic.rs`（诊断上报族） | 只写 `self.diagnostics`，边界清晰 |
| **4** | 搬 `statement.rs` | |
| **5** | 搬 `expression.rs`，**并拆分三个大函数** | 最重的一块 |
| **6** | 搬 `conversion.rs`，门面收尾 | |

**每一步的验收**：`xiao-types/tests/` 不改、公开 API 不变、门禁全绿。

**第 1 步应当是纯移动**——`git show -M --stat` 应当看到重命名而非增删。

---

## 五、最可能翻车的地方

1. **`RuntimeCheckKind` 的路径变了**（§3.2）——会同时打断 `xiao-ir`、`xiao-bytecode`
   与两处穷尽性守卫。**这是本批的头号风险。**
2. **顺手改了类型规则**。本批**不新增任何能力**；发现可以顺手修的地方，**另开提交**。
3. **动了 `TypeCheckResult` 的字段或诊断顺序**——快照测试会抓到，但那意味着"纯组织调整"
   实际上动了可观察结果。
4. **只拆文件不拆大函数**（§2 末）。
5. **一笔拆完**（§4）。
6. **忘了锁文件**：拆分**不应**增删 Rust 依赖，若增删了，`check:lock` 会拦住。

---

## 六、验收

沿用 09R2D 的「撤掉实现 → 用例必须失败 → 还原 → 通过」。**关键验收不是「测试通过」**：

1. **`check:layout` 干净**，且 `checker.rs`（门面）**显著低于**阈值，**不需要**旁置 md 豁免；
2. **公开 API 逐条不变**——`cargo doc` 的 `xiao-types` 公开项清单与拆分前一致；
3. **`RuntimeCheckKind` 仍从 `xiao_types::RuntimeCheckKind` 可达**，`as_name`/`from_name`
   行为不变，**两处穷尽性守卫仍通过**（§3.2）；
4. **`xiao-types/tests/` 一个字符没改**，含全部快照测试；
5. **三个大函数已被拆分**，没有一个新文件重新逼近阈值；
6. **架构回归测试到位**，`README.md` / 依赖说明 / 模块登记同批更新；
7. **门禁全绿**，含 `check:lock`、`bunx tsc`。

---

## 七、不负责与不要重复做的事

- **不改类型语义**、不加检查规则、不加诊断码。
- **不改 `TypeCheckResult` 的公开形状**。
- **不要顺手拆其他高位文件**——`protocol.rs` 与 `dynamic.rs` **各有独立交接文档**，
  按各自的批次做。
- **不要用旁置 md 豁免替代拆分**（`00E` 明说它是最后手段）。

## 相关页面

- [09R2E. 研究编码器模块解耦交接记录](09r2e-research-encoder-decoupling.md) —— **本批的模板**
- [11X0-F. `protocol.rs` 解耦交接](11x0f-protocol-decoupling.md) —— 同轮发现的另一份
- [02. 类型与值系统](02-type-system.md) —— `checker.rs` 的职责来源
- [09-B0-A. 字节码最小运行闭环](09b0-bytecode-closure.md) §3.3 —— `RuntimeCheckKind` 的单一来源
- [00E. 单文件行数门禁交接](00e-file-size-gate.md) —— `A0-SIZE-001` 与豁免机制
- [00F. 解析器第二批模块解耦交接](00f-parser-decoupling.md) —— 同类拆分的先例
