# 09R2C. 异常控制流实现交接文档

> 本文是 09R2 第三批的实现交接文档，按仓库交接记录体例组织。它把 `try`/`catch`/`finally`/`raise`
> 与运行时检查从已完成的静态闭环（07-B）推进到三地址降低、解释器处理器路由与可执行语义。
> **接手 Agent 必须先读完「接手前提」列出的四份材料再动代码。**

## Agent 交接上下文

### 接手前提

1. [09R. 字节码寄存器机型特别研究](09r-bytecode-machine-research.md) —— 尤其 **R1-L 调用约定**、
   **R1-T handler 表**、**R1-U 11 种 ExitKind 转移**、**R1-V finally 子程序**、**R1-W drop 不可重排**。
   这些是已冻结的设计，本批实现它们，**不得自行改设计**。
2. [07. 错误模型与并发安全边界](07-concurrency-and-errors.md) —— 尤其「07-B 已完成」小节与
   「作用域展开与资源清理」：展开顺序固定为 `finally -> drop -> 匹配 catch / 继续传播`，
   清理错误进 `suppressed` 且不覆盖主错误。
3. [06. 内存与运行时语义](06-memory-and-runtime.md) —— 释放顺序的冻结规则。
4. 已交付研究代码的 README：`core/rust/crates/xiao-bytecode/src/research/README.md`、
   `core/rust/crates/xiao-vm/src/research/README.md`。

### 当前进度

09R2 已交付两批，全部在 `main` 上：

| 批次 | 内容 | 提交 |
| --- | --- | --- |
| 批次 1 | TAC 模型与降低、对账验证器、栈式参考解释器、共享语义向量 | `51ceaac` `efe9789` `f035279` `a9b68ab` |
| 批次 2 | 容器运行时对象、精确索引、临时值释放修复 | `1f8d560` `a93fd3f` `885a278` `7dc998a` |
| 跨层审计 | 12 项缺陷修复 + 契约清理 | `7a67af3` `3f6aa94` `85a11d8` `3d1956f` `59a0317` `2bb5dc8` `ccba90a` `e900f3b` |

**现状**：`lower/stmt.rs` 把 `IrStatementKind::Try` 归入 `record_unsupported`；解释器到达
`TacOp::Raise` / `TacOp::Check` 直接报「尚未实现」；`TacHandler` 已定义但 `TacFunction.handlers`
**永远是空表**（`lower/mod.rs:365` 写死 `Vec::new()`）；解释器的 `unwind`
（`semantics/exec.rs:363`）只跑 `(scope, "error")` 计划，**完全不查处理器**。

异常控制流是 R1 施工图里 R2a 唯一还没达成的验收条件——09R 文档要求语义向量「同时覆盖函数调用、
递归、循环、`try`/`catch`/`finally`、`raise`」。

### 本阶段交付与不负责事项

**交付**：运行时错误对象、错误类型名单一来源、handler 表与 catch 路由、`finally` 子程序、
`raise` 构造式与重抛、`Check` 降低、语义向量与文档同步。

**不负责**：R2b 选择器全量（多选/区间/步长/随机）、分类型寄存器机型与混合式机型、指令编码器、
`for` 与表声明、`Result` 泛型与 `?` 传播（07 文档明确后置）、正式 `.xiaoc` 格式。

## 开发规定与硬约束

这一节是本仓库在这一阶段**反复踩出来的**规矩，不是风格建议。

### 单一来源原则（最高优先级）

**任何跨层使用的映射或判定只能有一处定义。** 跨层审计发现的 12 个缺陷里，**5 个是同一个病**：
同一条规则在两层各写一份，然后悄悄漂移。

已经建立、禁止再开第二份的唯一来源：

| 规则 | 唯一来源 |
| --- | --- |
| 退出边拼写 | `xiao_lifetime::ExitKind::as_name` / `from_name` |
| 释放动作类别拼写 | `xiao_lifetime::ReleaseActionKind::as_name` / `from_name` |
| 字符串字面量解码与转义表 | `xiao_types::decode_string_literal` / `decode_escape` |
| 负索引规范化 | `xiao_types::normalize_index` |
| 标量名拼写 | `xiao_syntax::ScalarType::as_str` / `from_name`（靠 `scalar_names_round_trip` 往返用例钉住） |
| 数值提升与加宽 | `xiao_types::conversion`（`promote_numeric_scalars` / `numeric_rank` / `integer_for_rank` / `float_for_rank`） |
| 赋值兼容判定 | `xiao_types::can_assign`（运行时 `runtime_value_matches` 也消费它） |
| 字典键规范化 | `xiao_types::decode_string_literal`（IR 降低与类型层同源） |

**本批要新增的唯一来源**：错误类型名表，落在 `xiao-diagnostics`。
**本批禁止再开新来源**：字符串解码、退出边拼写、索引归一化都走上面的既有函数。

### 工具使用规定

**不要用 shell heredoc 或 Python 字符串替换去改含中文或转义的源码文件。**

本仓库每个文件都有中文 Rustdoc，命中率极高。实测过的失败模式：

- `cat > f <<'EOF'` 与 `python - <<'PY'` 在处理中文、`\n` 转义和 markdown 反引号时会**提前终止**
  或**替换静默不生效**（不报错也不改动）。
- 一次 `\\t` 被折成真制表符，导致探针测的是**假输入**，据此误报了一个并不存在的前端缺陷。
- 一次 `\n` 被折成真换行，把注释截断成字节串字面量，直接编译失败。

**规定**：新建文件用 `Write` 工具；改文件用 `Edit` 工具。只有纯 ASCII 的机械替换才用 shell。
**不得用 `git checkout --` 还原被临时改动的文件**——它会把你未提交的新增代码一起回退。

### 兜底匹配的陷阱

`RuntimeValue` 有三处**不会编译报错**的兜底匹配，新增变体时会静默走错分支：

| 位置 | 兜底行为 | 不加分支的后果 |
| --- | --- | --- |
| `value/mod.rs` 的 `PartialEq::eq` | `_ => false` | `Error(e) != Error(e.clone())`，自己和自己不等 |
| `containers/mod.rs` 的 `is_hashable` | **否定式** `!matches!(...)` | 新变体默认被当作**可哈希**，错误对象能进集合，而静态 `hashability()` 未必同意 → 静态通过、运行时报错 |
| `value/mod.rs` 的 `type_name()` | `_ => "dynamic"` | `type_mismatch` 的参数显示 `dynamic`，误导排查 |

新增变体时必须逐一检查这三处并加显式分支，且**各写一条用例钉住**。
另有两处会**编译报错**因而安全：`Hash::hash` 与 `scalar_type()` 的 `match` 没有兜底分支。

### 区分度验证

**不许用「测试通过」结案。** 每条修复都要证明用例**有区分度**：
撤掉修复 → 用例必须失败 → 还原 → 通过。

本仓库此前已这样验过：`ExitKind` 拼写锁、临时值释放、`Copy` 与 `Move` 的分工、`not` 的比较、
解耦叶子测试。本批同样适用，见「验收标准」。

### 门禁

每次提交前全跑：

```bash
cargo test --manifest-path core/rust/Cargo.toml --workspace
cargo clippy --manifest-path core/rust/Cargo.toml --workspace --all-targets -- -D warnings
cargo fmt --manifest-path core/rust/Cargo.toml --all -- --check
cargo doc --manifest-path core/rust/Cargo.toml --workspace --no-deps
bun run check && bun run check:coverage && bun test
git diff --check
```

补充规定：

- **不要用 `#[allow]` 掩盖警告或覆盖率缺口。** `bun run check` 要求新增 `pub` 项 100% 有 Rustdoc。
- **新增含 `.rs` 的目录必须自带 `README.md`**（目录判定**不继承父目录**），且该目录必须出现在
  `docs/module-registry.json` 某条目的 `code`/`tests` 路径中，否则报 `A0-LAYOUT-002`。
- **新建 `tests/` 目录必须带 `tests/README.md`。**
- 提交信息用 `feat:` / `fix:` / `test:` / `docs:` / `chore:` 前缀，正文说明**为什么**而不只是做了什么。

### 解耦约束

1. **语义核与载体分离**：`xiao-vm/src/research/semantics/` 不认识「栈」这个词，只依赖
   `carrier.rs` 的窄接口。已有可执行测试 `research_module_stays_a_leaf` 守住「生产 `src/` 不得
   引用 `crate::research`」，不要破坏它。
2. **不引入 trait object 进热路径**：载体、值运算都用静态分发。
3. **值运算收口**：Runtime 侧在 `value/ops.rs`，VM 侧在 `research/ops.rs`。语义核不散落裸
   `RuntimeValue` 调用。
4. **`VmEventSink` 只在粗粒度边界调用**（模块/函数/作用域/释放/错误），不做逐指令回调——
   它会影响 09R3 的性能基准。
5. **`xiao-bytecode` 不得重新推断类型、重算生命周期、重排释放顺序。** 它只做 1:1 语义展开。

## 一级工程目标：错误身份与错误对象

### 任务 1：`xiao-diagnostics` —— 错误类型名的单一来源

**背景（真缺陷）**：前端与运行时用的**不是同一张表**。

- 前端 `xiao-types/src/control_checker.rs:275-279`：

  ```rust
  fn is_error_type_name(name: Name, source: &xiao_source::SourceFile) -> bool {
      let text = name.unquoted_text(source);
      !name.backticked && (text == "Error" || text == "XiaoError" || text.ends_with("Error"))
  }
  ```

  这是**无穷集**——`FooError`、`WhateverError` 全部通过。
- 运行时 `xiao-runtime/src/testing/mod.rs:144-159` 的 `dispatch_catch`：**8 个精确名字**的白名单
  （`Error`/`XiaoError`/`ArithmeticError`/`MemoryError`/`TableError`/`ConcurrencyError`/
  `ResourceError`/`TypeError`）。

后果：`catch err as FooError` **静态通过、运行时静默不匹配**，错误直接穿过处理器，没有任何诊断。

**交付**：在 `xiao-diagnostics` 定义权威表并附 `error_kind_of` 之类的查询函数；`xiao-types` 与
`xiao-runtime` 都改为消费它。`FatalError` 作为**独立条目**标记为不可被普通 catch 捕获。
**这是行为变更**（静态检查收紧），必须登记。

### 任务 2：`xiao-runtime` —— 错误对象

`XiaoError` 是 `#[derive(Clone, Debug, Eq, PartialEq)] pub struct XiaoError(Box<XiaoErrorData>)`，
**可以直接作为 `RuntimeValue::Error(Box<XiaoError>)` 变体**。注意：

- 它带全局自增的 `error_id`，是**身份语义**（`e.clone() == e` 为真，两个独立创建的同内容错误不等）。
  与容器一致、与标量不一致，登记为已知语义而非缺陷。
- 无 `Hash`；`RuntimeValue::Hash` 对该变体只哈希判别式（不相等的值允许同哈希，符合契约）。
- **必须处理「兜底匹配的陷阱」一节的四处**。
- 提供窄接口读取 `code()` / `kind()` / `message_id()`，供后续 `err.code` 成员访问复用。

## 一级工程目标：异常控制流与运行时检查

### 任务 3：`xiao-bytecode` —— 异常指令、handler 表与 `finally` 子程序

`research/tac.rs` 新增：

```text
MakeError { type_name: String, code: Option<VReg>, message: Option<VReg> }
CallSub { sub: BlockId }
RetFromSub
Check { kind: String, value: VReg, on_failure: BlockId }   // 现有 Check 需补 value 操作数
```

`TacHandler` **已经存在**（`tac.rs:591`），字段为
`{ protected: (BlockId, BlockId), handler: BlockId, scope: u32, exit: String, catch_type: Option<String>, binding: Option<VReg> }`。
本批要做的是**把它填起来**（现在永远是空表）。

`raise` 有两条路径：

- **构造式**：`raise ArithmeticError(code = "X", message = "y")` 在 IR 里只是一个普通 `Call`
  （callee 是裸名字、`ty` 是 `Dynamic`）。TAC 必须自己识别 callee 是否为合法错误类型名 →
  `MakeError` + `Raise`。`code`/`message` 的字符串字面量**必须经 `xiao_types::decode_string_literal`
  解码**——IR 的 `Literal.text` 是**含引号的源码切片**。
- **重抛**：`raise <已有错误值>` 按普通表达式求值后直接 `Raise`。

`finally` **按 R1-V 用子程序**：finally 体只发一份，各退出路径发 `CallSub`，子块末尾发 `RetFromSub`。
**不要内联展开**——R1 冻结子程序是因为内联会让编码体积按层数爆炸，直接违反 09R3
「编码体积不得恶化超过 10%」的门槛。

**R1-U 的 11 种 ExitKind 转移逐一落地**，其中 **`Fatal` 不查 handler 表、不执行任何释放计划**——
这是刻意的与其余十种的不对称（致命故障下继续跑释放钩子已经不安全）。

### 任务 4：`Check` 降低与执行

`IrRuntimeCheck` 只在 IR 里排队，全仓没有任何地方构造 `TacOp::Check`。

本批只实现**能真正判定**的类别：`boolean_condition`、`arithmetic`、`numeric_range`、
`dynamic_conversion`。其余（`selector_bounds`/`selector_step`/`random_count`/`random_seed`/
`set_*`/`iterable`）依赖 R2b 或 `for`，继续记入 `unsupported`，**不要假装实现**。

失败走 R1 冻结的 `DynamicCheckFailure` 退出边。

### 任务 5：`xiao-vm` —— catch 路由与子程序执行

- `Fault::Error` 不再直接拆帧：先在**当前帧的 handler 表**里按「块号落在 protected 区间 +
  `catch_type` 匹配」查找；命中则按 `exit` 执行对应释放计划、把错误绑进 `binding` 寄存器、
  跳到 handler 块**继续执行**；未命中才拆帧传播（现有 `unwind` 的行为）。
- **类型匹配必须经 `dispatch_catch`**，不自己再写一份名字到类别的映射。
- `Fatal` 保持现状：绕过所有释放计划、走 `FatalError` 通道、不进 catch、不合并 `suppressed`。
- 子程序：帧内维护挂起的退出类别栈；`CallSub` 压入、`RetFromSub` 弹出并按它分派。
- `VmEventSink` 增加**粗粒度**的处理器事件（进入/命中/未匹配），不做逐指令回调。

## 一级工程目标：语义向量与文档

### 任务 6

- 扩展 `tests/spec/09-bytecode/errors.json`：覆盖 `try`/`catch`/`finally`/`raise` 的正常、命中、
  未匹配、`finally` 清理、重抛、嵌套；**至少一条同时覆盖函数调用、递归、循环、`try`/`finally`、
  `raise` 与字符串堆值释放**（这是 09R 文档写死的验收条件原文）。
- 向量**不得含机型专属字段**——后续两种机型必须复用同一组期望值。
- 更新 [09R 交接记录](09r-bytecode-machine-research.md)（本批交付与两处冲突登记）、crate README、
  `value/README.md`、DevDocs 主表与 [12. 测试与开发里程碑](12-tests-and-milestones.md)。

## 调研已确认的关键事实

以下结论已完成调查，接手时不必重复。

### 生命周期阶段的形状

源码：

```xiao
try
    value = 1
catch err as Error
    value = 2
finally
    done = true
```

`IrOwnership.scopes` = **4 个**：`program`(0) / `try`(1) / `catch`(2) / `finally`(3)。
**三者的 `parent` 都是 program**——catch 与 finally 是 try 的**兄弟**，不是子节点
（`escape.rs:910-916`、`:973-975`）。这正是 07-B「catch 绑定不能读取已释放的 try 局部绑定」的实现方式。

`IrOwnership.values` = 4 个，其中 **catch 里的 `value = 2` 声明的是一个全新值**（id=2），与 try 里的
`value`（id=0）互不相干——try 的绑定栈在 catch 分析前已经弹掉。

`release_plans` = **44 条**（4 作用域 × 11 退出边），**只有 scope 2 的 11 条非空**，每条都是同一份
`[ReleaseAction { value: 1 /* err */, order: 0, kind: "strong" }]`。原因是只有 catch 绑定 `err` 是
`heap_strong`（`escape.rs:915-922` 硬编码 `Type::Dynamic` + `HeapStrong`）。

### 两处必须绕开的坑

**catch 入口块可能没有前驱。** `escape.rs:913` 的 `connect_exit_sources(&try_errors, catch_block, Error)`
在 `try_errors` 为空时**一条边都不建**，catch 块在 CFG 上是孤岛。**TAC 不能把「没有前驱」当作
「不可达」**，否则空 try 体的 catch 会被整个丢掉。

**`exits` 不是完备描述。** `ExitKind::Normal` 从来没有被 `record_block_exit` 写过——正常路径只体现为
`next` 后继边。块的 `exits` **只记异常与非局部控制转移**，不能拿它判断可达性。

### `escape.rs` 与 07-B 的一处冲突

`escape.rs:887-901` 把 `ExitKind::Fatal` 列进了「可被 catch 吃掉」的集合，与 07-B「Fatal 绕过所有
释放计划、不进 catch」**冲突**。handler 表必须按 `xiao-types` 的 `CATCH_FATAL_CODE` 边界自己判定，
**不能依赖 `summary.exits`**。本批**不改 `xiao-lifetime`**（那是 07-B 的冻结产物），只登记这处不一致。

### 释放计划是不可裁剪的笛卡尔积

`release.rs:48-59` 外层循环「每个作用域」、内层无条件 `for exit in ExitKind::ALL`，**没有任何裁剪**。
唯一过滤是值级的（`!temporary` 且 `needs_release() && !transferred`）。

因此 handler 表**不能**用「某个 `(scope, exit)` 计划是否存在」来表达 Fatal 与 Unmatched 的差别。
这条已写进 `xiao-ir/tests/r2_release_reconciliation.rs` 的文件头注释。

### 可复用的现成资产

- `xiao-runtime/src/testing/mod.rs` 的 `RuntimeDriver::dispatch_catch(error, handler_types) -> CatchRoute`
  与 `dispatch_fatal`——按名字匹配的路由表已经写好，本批改的是让它的名字表与前端同源。
  `CatchRoute` 有 `Matched` / `Propagate` / `Fatal` 三变体。
- `xiao-runtime` 的 `ErrorAccumulator`（主错误与 `suppressed` 语义已冻结）。
- `xiao-vm` 的 `FatalError` 与 `XiaoError` 双通道（`Fault::{Error, Fatal}`），Fatal 的不对称已实现。
- `xiao-bytecode` 的 `run_plan(scope, exit, span)` / `scope_chain_to(kind)` / `exit_scope`——
  按 `(scope, 退出边)` 查冻结计划，handler 路由直接接这套。

## 提交切分

按可独立验证的单元分六次提交，前两次是纯生产 crate 改动：

1. `xiao-diagnostics` 错误类型名表 + `xiao-types` 与 `xiao-runtime` 共同消费（**修掉静默分歧**）+ 测试。
2. `xiao-runtime` 错误对象变体 + 三处兜底匹配的显式分支 + 测试。
3. `xiao-bytecode` 异常指令、handler 表、`finally` 子程序、`raise` 两条路径 + 测试。
4. `Check` 降低与执行 + 测试。
5. `xiao-vm` catch 路由与子程序执行 + 测试。
6. 语义向量扩展 + 文档同步。

**若中途必须停**：第 1、2 次提交本身完整可验证，不会留下半成品。

## 验收标准

命令见「门禁」一节。**关键验收不是「测试通过」**：

1. **类型名分歧真的被修掉**：`catch err as FooError` 现在编译期被拒；两侧清单来自同一常量，
   改一处即同时生效（改常量应让两侧用例同时失败）。
2. **catch 真的接住**：`raise` 抛出的错误被匹配的 handler 捕获、绑定可用、程序继续；
   未匹配的错误穿过 handler 继续传播，且**原错误身份不被改写**。
3. **`finally` 恰好执行一次**：正常路径、异常路径与 `return` 路径各一次；`finally` 里再抛错
   不覆盖主错误（进 `suppressed`）。
4. **`Fatal` 不被吞**：栈溢出等致命故障绕过所有释放计划、不进 catch。
   **撤掉这条不对称必须让对应用例失败**——它最容易写成永不触发的装饰。
5. **catch 孤岛块不被丢弃**：try 体没有 `raise` 时，catch 体仍被降低且在异常时可达。
6. **`Check` 真的会失败**：动态条件不是 `bool` 时报出稳定错误码；**撤掉检查必须让对应用例失败**。
7. **三处兜底匹配各有用例**：`Error(e) == Error(e.clone())`、错误对象不可作集合元素、
   `type_name()` 返回 `"error"` 而不是 `"dynamic"`。
8. **不算假通过**：新增 `pub` 项 100% Rustdoc；不用 `#[allow]` 掩盖。

## 已知风险与未决

1. **本批跨度大**（6 次提交跨 4 个 crate，含一次静态检查行为变更）。前两批的教训是：
   **跨层缺陷只在接起来时才显现**，所以每批都必须有端到端向量，不能只靠单元测试。
2. **`finally` 子程序是本批最复杂的一块**：解释器要维护「挂起的退出类别」栈。R1 冻结子程序是为
   编码体积（09R3 门槛），09R3 的机型与编码器要沿用同一套转移语义。
3. **`XiaoError` 是身份语义**：`catch` 绑定与另一个等值错误比较会判不等。登记为已知语义。
4. **`escape.rs` 的 `Fatal` 归类与 07-B 冲突**：按 07-B 实现并登记，不改 `xiao-lifetime`。
5. **`RuntimeValue` 与诊断域耦合**：把 `XiaoError` 塞进 `RuntimeValue` 会让 Runtime 值域与
   `xiao-diagnostics`（文档定位是「语言无关的机器字段」）耦合。若后续认为不可接受，替代方案是
   给错误对象包一层轻量句柄（仿 `StringHandle` 对 `StringObject` 的两层范式）。本批采用直接内嵌，
   因为它是原型且改动最小。

## 不要重复做的事

以下已在跨层审计中修完，**不要重新调查或再修一遍**：

- `a = b` 的所有权语义（已补 `TacOp::Copy`；临时值用 `Move`，具名绑定之间用 `Copy`）
- 反引号名与普通名的区分（`name_key(name, backticked)`，三处前缀拼法已统一）
- 一元 `not` 与一元负号（`not` 与 `false` 比较；负号是 `0 - x` 且零常量同宽度）
- 数值提升的桥（降低器按类型层规则插入显式 `Cast`）
- `*args` 与 `**kwargs`（记入 `unsupported`，不静默当作普通实参）
- 字符串转义三份实现（统一到 `decode_string_literal` 与 `decode_escape`）
- 字段写入判定（`runtime_value_matches` 复用 `can_assign`）
- 4 个无 harness 的快照（已补 `xiao-types/tests/c0c1_snapshots.rs`）
- 跨 crate 重名常量（`xiao-config` 的三个已加 `CONFIG_` 前缀）
- 3 个死码（`X03-PARSE-003`、`X03-TYPE-007`、`X04-TYPE-008` 已删，文档承诺已清）
- 诊断重复上报（同码同跨度只保留首次）
- 18 个测试文件的诊断码字面值（已全部改为常量引用）
