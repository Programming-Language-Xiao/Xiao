# 06A. 生命周期静态闭环交接记录

> 本文是 06-A 的实现与交接记录。本阶段已经建立作用域、控制流、逃逸、
> 强/弱所有权图和确定性释放计划；它不分配 Runtime 对象、不执行引用计数或
> `drop`，也不生成字节码/LLVM 指令。

## Agent 交接上下文

### 接手前必须阅读

1. [00. 决策基线](00-decisions.md)：无追踪式 GC、强类型、国际化字段和低耦合总约束。
2. [04. 函数与控制流](04-functions-and-control.md)：函数、分支、循环和退出语句 AST。
3. [05C. 表静态闭环](05c-table-static-closure.md)：`new/init/drop` 的静态签名边界。
4. [06. 内存与运行时语义](06-memory-and-runtime.md)：第 06 阶段总目标和后续 Runtime 债项。
5. [08. 前端与统一中间表示](08-frontend-pipeline.md)：生命周期结果的首个后置消费者。
6. [12. 测试与开发里程碑](12-tests-and-milestones.md)：R0-A 退出条件。

### 本阶段输入

- `xiao-source::SourceFile`：名称解码和所有诊断区间的唯一源码来源。
- `xiao-syntax::Program`：不修改的 AST；包含顺序语句、函数、表、分支、循环和退出语句。
- `xiao-types::TypeCheckResult`：按 `SourceSpan` 查询的类型、运行时检查标记和函数/表签名。

生命周期层自行规范化 `ascii:`/`backtick:` 名称，只消费上述公开 API。禁止给
`TypeCheckResult` 增加生命周期字段，也禁止让 `xiao-types` 反向依赖本 crate。

### 本阶段交付

| 交付物 | 位置 | 状态 |
| --- | --- | --- |
| 生命周期公开模型 | `core/rust/crates/xiao-lifetime/src/model.rs` | 已完成 |
| 强/弱图与拓扑排序 | `core/rust/crates/xiao-lifetime/src/graph.rs` | 已完成 |
| AST 逃逸/控制流分析 | `core/rust/crates/xiao-lifetime/src/escape.rs` | 已完成 |
| 作用域退出释放计划 | `core/rust/crates/xiao-lifetime/src/release.rs` | 已完成 |
| 稳定诊断编号 | `core/rust/crates/xiao-lifetime/src/diagnostics.rs` | 已完成 |
| 规格测试 | `core/rust/crates/xiao-lifetime/tests/a06_lifetime.rs` | 已完成 |
| 用户文档 | `docs/UseDocs/language/memory/` | 已完成 |

### 明确不负责

- 不创建引用计数对象头、堆分配器、弱引用升级或真实 Runtime 值。
- 不执行 `init`、`drop`、容器修改、错误展开或任何 Xiao 程序。
- 不添加 `Weak` 表面语法；06-A 只冻结 `Strong`/`Weak` 静态边 API。
- 不添加线程、任务池、Actor、`async/await` 或跨线程表面语法；`CrossThread` 只保留事实类别。
- 不添加 `try/catch/finally/raise`，也不接入 CLI、VM、LLVM、日志或国际化目录。

## 一级工程目标：独立生命周期模型

### A1.1 身份与作用域

`ScopeId`、`ValueId` 和 `BlockId` 是从零开始、按分析顺序分配的稳定身份。程序、函数、
分支、循环和表体分别建立 `ScopeInfo`；每个作用域保存父级、深度、源码区间和值声明顺序。
匿名表达式对象与名称绑定使用不同 `ValueId`，因此普通重绑定共享对象不会被误判为对象环。

`ValueInfo` 保存规范化名称、所属作用域、类型、存储类别、参数/常量/临时值标记和逃逸原因。
固定宽度数值、`bool`、`none` 及全部成员可留栈的元组使用 `Stack`；`str`、动态值、容器、
函数和表值使用 `HeapStrong`。`HeapWeak` 是后续 Weak lowering 的静态句柄类别。

### A1.2 公开入口

稳定门面为：

```rust
let result = LifetimeAnalyzer::new(&source).analyze(&program, &type_result);
let plan = result.release_plan(scope_id, ExitKind::Return);
let graph = result.ownership_graph()?;
```

`LifetimeResult` 同时提供作用域、值、强边、弱边、释放计划、控制流图、动态检查和诊断。
后置阶段只能消费这些结构，不得重新从源码标点推导所有权。

## 一级工程目标：控制流与逃逸

### A2.1 控制流范围

分析器覆盖顺序语句、`if/elif/else` 合流、`for`、`while`、`return`、`break` 和
`continue`。函数边界会隔离外层循环；循环头有真假后继，`continue` 回到循环头，
`break` 到循环后继，`return` 到函数退出块。动态检查和构造失败通过错误后继边记录。

每个程序/函数/分支/循环/表作用域均生成以下八种独立计划模板：

- `Normal`
- `Return`
- `Break`
- `Continue`
- `Error`
- `ConstructFailure`
- `DynamicCheckFailure`
- `Fatal`

当前 AST 尚无普通错误、致命退出和异常语句生产者，但预先统一退出类别可以让第 07/08
阶段接入而不改变 06-A 的计划键。

### A2.2 逃逸规则

- 返回的绑定/对象及其强拥有闭包标记为 `Returned`，从相应 `Return` 计划移除。
- 嵌套函数读取非全局外层局部值时生成 `ClosureCapture` 强边，并把被捕获值提升到堆。
- 内层值存入外层绑定或容器时标记为 `StoredInLongerLivedContainer`，沿正常展开边转移。
- 类型为 `Dynamic`、未解析类型变量或缺失类型记录时采用 `HeapStrong`，并生成
  `DynamicLifetimeCheck` 与 `X06-LIFETIME-005` 警告。
- 类型层已有 Runtime 检查的表达式也保留动态失败清理边，即使其最终静态结果类型已知。

> **端到端观察记录（2026-09-22，来自 11X0-B 的 CLI 验证）**：脚本模式下，
> `value = 1 + 2` 与带注解的 `int value = 1 + 2` 都会为各自的绑定产生一条
> `X06-LIFETIME-005`（两个绑定即两条）。来源是上面 `Dynamic`/未解析类型/缺失类型记录
> 那条规则，与 CLI 层无关。
>
> **本记录只陈述观察与来源，不评估该行为是否必要。** 记在这里是因为它是 11X0-B 端到端
> 验证时直接可见的程序输出，也是用户运行最小程序时会看到的诊断；是否调整属于逃逸分析
> 的后续评估范围。相关入口见 [11X0-B](11x0b-cli-shell.md)。

`CrossThread` 已作为稳定逃逸原因保留，但在并发表面模型冻结前没有 AST 生产者；第 07/08
阶段必须通过显式 lowering 事实接入，不能凭函数名猜测线程行为。

## 一级工程目标：所有权图与释放顺序

### A3.1 强边与弱边

边方向固定为 `A -> B` 表示 A 必须先于 B 释放。绑定到根对象使用 `Alias` 边；容器对象
到其元素对象使用 `ContainerElement`；闭包到捕获值使用 `ClosureCapture`；表到字段使用
`TableMember`。强边延长目标生命周期，弱边既不延长生命周期，也不参与强环/强拓扑。

强对象环使用 `X06-LIFETIME-001` 静态拒绝。普通名称别名通过绑定与对象分离表示，不会把
`a = b`、`b = a` 错当成容器互持；对象直接持有自身属于单节点强环，多个容器互持属于
多节点强环，两者都必须拒绝。

### A3.2 确定性拓扑

释放顺序采用强边拓扑排序。每个拓扑层内按作用域声明序号逆序，同序号再按 `ValueId`
逆序；同一计划对值去重并保持连续的 `order`。返回或转移到外层的值只出现在
`transferred`，不出现在该退出计划的 `actions`。

损坏图输入返回 `GraphError`，不 panic。分析结果若检测到强环，仍以声明逆序生成完整的
错误路径兜底计划，且不会因环而遗漏其他独立值；后端必须先检查诊断，不能执行该产物。

## 二级工程 SOP

### A4.1 修改模型

1. 新增退出种类、存储类别或边原因前，先更新 `00-decisions.md` 和本文。
2. 在 `model.rs` 添加有完整 Rustdoc 的值对象；不要把 AST 或 Runtime 对象嵌入模型。
3. 图不变量只放在 `graph.rs`，AST 遍历只放在 `escape.rs`，计划编排只放在 `release.rs`。
4. 同步正例、负例、动态回退和损坏输入测试。

### A4.2 接入新语法

1. 先在语法/类型阶段冻结节点和类型语义，再让 `escape.rs` 消费公开节点。
2. 为所有正常和提前退出路径确定作用域展开边。
3. 为新对象关系选择已有边原因；确需新原因时同步图测试和 IR 契约。
4. 无法证明时生成动态检查，禁止静默假设为栈值或跳过清理。
5. 函数体退出不得传播到函数声明所在语句序列；同一函数内跨分支/循环读取也不得误记为闭包捕获。

### A4.3 后端消费

1. 第 08 阶段把 `LifetimeResult` 映射到类型化 IR，保留所有稳定身份与源码区间。
2. IR 验证器检查未知身份、重复释放、强环和退出计划完整性。
3. 第 09/10 阶段分别降低同一计划；两条后端不得反转边方向或重排 `drop`。
4. 真正的引用计数增减、Weak 对象和构造失败展开留给 06-B/Runtime。

### A4.4 质量门禁

```text
cargo fmt --all -- --check
cargo check --workspace
cargo test -p xiao-lifetime
cargo test --workspace
cargo clippy -p xiao-lifetime --all-targets -- -D warnings
cargo doc -p xiao-lifetime --no-deps
bun tools/repo-check/src/cli.ts all --format text
```

公共 API/模块/类型/字段 Rustdoc 覆盖率必须为 100%，全仓库声明项覆盖率不得低于 90%。
代码、测试、crate README、分层 UseDocs、DevDocs 和模块登记必须在同一提交中完成。

## 稳定诊断

| 编号 | 消息身份 | 含义 |
| --- | --- | --- |
| `X06-LIFETIME-001` | `x06.lifetime.strong_cycle` | 强引用对象环 |
| `X06-LIFETIME-002` | `x06.lifetime.invalid_edge` | 所有权边无效 |
| `X06-LIFETIME-003` | `x06.lifetime.unknown_value` | 作用域/值身份不存在 |
| `X06-LIFETIME-004` | `x06.lifetime.fact_conflict` | 生命周期事实冲突（预留） |
| `X06-LIFETIME-005` | `x06.lifetime.dynamic_check` | 需要 Runtime 检查的动态边界 |

`code`、`message_id`、参数和 `SourceSpan` 是机器接口；当前中文文本只是预览。后续国际化
接入只能替换展示文本，不能改变生命周期结果、退出类别或释放顺序。

## 后续交接

06-B 应在 `xiao-runtime` 中实现引用计数对象头、Strong/Weak 句柄和确定性 `drop`，但不把
运行时布局倒灌进本 crate。第 08 阶段先把本结果接入统一 IR，再由第 09/10 阶段验证字节码
与 LLVM 的释放记录一致。线程/任务模型未冻结前，`CrossThread` 不得通过名称启发式产生。
