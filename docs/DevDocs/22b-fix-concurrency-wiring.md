# 22B-FIX. 并发静态契约接线与收口

> **这是 22B 与 22C 之间的接线批，不实现 Runtime/VM/LLVM。**
> 2026-10-25 复核 `768bf13..HEAD` 后发现：22B 的类型、生命周期和 IR 数据结构/单元测试
> 已落地，但签名、能力、消费状态、边界事实和取消检查还没有接进真实源码流水线；照当前
> 状态进入 22C，22C 拿不到 `spawn`/`submit` 的事实，也无法可靠拒绝普通句柄、跨 Runtime
> `shared_mut` 或超过 64 个边界参数。
>
> 状态：**规划完成，待实施（2026-10-25）**。本批只接 22B 静态流水线，不接 22C Runtime；
> 实施由另起 Agent 按本文执行，先完成 §五的 D-1～D-5 裁定。

## 一、已确认的证据

| 项 | 结果 | 说明 |
| --- | --- | --- |
| 本地测试 | 通过 | `cargo test --workspace`、`cargo fmt --check`、`check:docs`、`check:usedocs`、`check:layout`、`git diff --check` 均通过 |
| 远端 CI | 无 | 当前 `main` 领先 `origin/main` 18 个提交，22B 没有远端运行号 |
| 32B/产物 | 未改 | 没有 32B ABI、`.xiaoc`/`.xar` feature 位或标准库函数体改动 |
| 下游兼容 | 仅穷举分支 | `4512c07` 只给 `xiao-runtime`/LLVM 的 `Type`/`IrType` 匹配加拒绝分支，未接并发行为 |
| 22B 状态 | 过宽 | 文档写成“已实施/22C 可消费”，实际只到模型/API 和单元测试 |

## 二、必须收口的缺陷

### F-1（阻断）签名、能力和消费状态没有接进类型检查器

- `std_concurrent_signatures()` 只在 `core/rust/crates/xiao-types/src/concurrency.rs:307`
  定义并被同文件单元测试使用；没有注册到 `std.concurrent`，也没有调用点。
- `concurrency_capabilities()`（`:48`）、`HandleConsumption`（`:235`）、
  `check_boundary_argument_count()`（`:152`）均没有生产代码调用。
- `X03-TYPE-030/031/032` 只在 `core/rust/crates/xiao-types/src/diagnostics.rs:84-88`
  声明，仓内没有实际发出点。
- 结果：源码中的 `spawn`/`join`/`submit`/`shared_read` 等不会被签名检查，也不会触发
  Send/Sync、普通句柄、64 参数或消费错误。

**修复**：把 `std.concurrent` 签名注册到 `standard_module_function_signatures`
（`core/rust/crates/xiao-driver/src/frontend.rs:702`），在调用检查
（`core/rust/crates/xiao-types/src/checker/expression.rs:547`）消费签名、能力表、
消费状态和参数上限；真正发出 030/031/032。

### F-2（阻断）源码不会产生 `TaskBoundary`/`ThreadBoundary`

- `core/rust/crates/xiao-driver/src/frontend.rs:1327` 仍调用普通
  `analyze_lifetime`，没有调用 `analyze_with_boundary_facts`。
- `analyze_with_boundary_facts` 在 `core/rust/crates/xiao-lifetime/src/lib.rs:85`
  定义，但仓内没有生产调用方。
- `BoundaryFacts::record_spawn`/`record_submit` 只出现在
  `core/rust/crates/xiao-lifetime/src/model.rs:1513` 的单元测试中；真实前端不会构造事实。
- 后果：真实 `LifetimeResult.boundary_facts` 为空；`xiao-ir` 的
  `cross_thread_facts`/`boundary_transfers` 也为空，22C 无法消费。

**修复**：在类型检查之后、生命周期分析之前增加显式 lowering 步骤，识别
`std.concurrent.spawn`/`submit` 调用，使用 `BoundaryValue::from_type` 构造事实，调用
`analyze_with_boundary_facts`；不得按变量名/函数名猜测。

### F-3（阻断）取消检查始终为空

`core/rust/crates/xiao-ir/src/lower.rs:1263` 固定写 `cancellation_checks: Vec::new()`；
`IrCancellationCheck` 只有构造器和 verifier 测试，没有 lowering 生产者。D-8 目前只有
数据形状，没有接入实际 IR。

**修复**：在同一边界 lowering 中为 `spawn`/`submit`/`join`/`wait`/`lock` 生成
`safe_point`/`wait`/`lock_wait` 检查，并传入 `IrOwnership.cancellation_checks`；
检查顺序由 verifier 继续约束。

### F-4（高）签名表丢失泛型 `T`

`std_concurrent_signatures` 目前用 `Type::Dynamic` 占位：

- `spawn -> thread<dynamic>`（`concurrency.rs:345-349`）；
- `join -> dynamic`（`:351-356`）；
- `wait -> dynamic`（`:422-427`）；
- `cancel(Dynamic)`（`:365`）。

即使接进类型检查器，也无法推导 `spawn` 的返回值、`join`/`wait` 的结果，或限制
`cancel` 只接受 thread/task。

**修复**：使用现有 `FunctionSignature`/`TypeScheme` 的类型参数表达 `spawn<T>`、
`join<T>`、`submit<T>`、`wait<T>`、`shared_read<T>` 等；为 `cancel` 设计两个重载或一个
可判定的句柄接口。不得继续用 `Dynamic` 当泛型。

### F-5（高）`HandleConsumption` 状态机错误

`core/rust/crates/xiao-types/src/concurrency.rs:235-242` 只在“已消费且本次操作也消费”
时拒绝；因此 `join` 后再 `cancel` 仍返回 `true`。消费后的 thread/task 不应再接受
`cancel` 或其他操作；现有测试只覆盖 `cancel -> join -> detach`。

**修复**：消费后所有操作（除显式关闭/查询等另行定义的操作）统一拒绝；补
`join -> cancel`、`detach -> join`、`wait -> cancel` 测试。

### F-6（高）边界事实 API 可绕过能力检查

- `BoundaryValueInput for ValueId` 把值直接标成 `Sendable`
  （`core/rust/crates/xiao-lifetime/src/model.rs:212-215`），`record_spawn([value])`
  因此不会拒绝普通句柄或动态值。
- `shared_mut` 只在 `source_runtime_id` 为 `Some` 时检查跨 Runtime
  （`model.rs:842-846`）；没有来源 Runtime 时直接放行。
- `record_fact` 是公开的直接插入入口，绕过 `validate_value`。

**修复**：生产路径只接受带类型的 `BoundaryValue`；删除或标注仅供测试的 `ValueId`
捷径；`shared_mut` 必须携带来源 Runtime 或拒绝；`record_fact` 收窄为受控内部入口。

### F-7（高）没有源码级回归

当前新增测试主要是 parser 注解、`BoundaryFacts` 模型、手工 IR 结构和字节码类型往返；
没有 `spawn`/`submit` 源码级类型检查/生命周期/IR 用例，也没有 030/031/032 触发用例。

**修复**：新增源级正/负例，至少覆盖：

- 合法 `spawn(runtime, fn, int)` 产生 `TaskBoundary + ThreadBoundary` 和 IR fact/transfer；
- 普通句柄、`shared_mut` 跨 Runtime、`dynamic`/resource 被拒绝；
- 65 个边界参数被静态拒绝；
- `join` 后再次使用/`cancel` 被拒绝；
- `lock_guard` 逃逸被拒绝；
- 取消检查出现在预期位置。

### F-8（中）字节码类型标签没有产物门禁

`core/rust/crates/xiao-bytecode/src/encode/encoder.rs:750-772` 新增类型标签 12–19；旧
运行时遇到这些标签会报通用未知类型错误。22B 没有设置 `.xiaoc` required-feature 位或
ABI 门禁，22C 必须补；本批只能先把它标为 22C 前置，不能宣称旧产物兼容。

### F-9（中）文档状态和证据过宽

22B 文档写成“已实施（2026-10-25）”“22C 可以只消费本批事实”
（`22b-concurrency-types-lifetime-ir.md:8-9`、§十一），22 阶段写成“22B 静态契约已完成”
（`22-concurrency-execution-and-shared-handles.md:3`、`:198`），但 F-1～F-3 未接线。
同时 18 个本地提交没有远端 CI。

**修复**：22B-FIX 完成前把 22B 状态改为“部分实现，待接线”；22C 门禁改为“22B-FIX 完成”；
补充推送后的运行号，不把本地测试写成远端证据。

### F-10（低）下游兼容边界措辞

`4512c07` 确实修改了 `xiao-runtime`/`xiao-codegen-llvm`，但只是穷举拒绝分支，没有并发
Runtime 行为。22B 计划写“不修改这些 crate”，应改成“不接入并发行为；允许为新增类型补
穷举兼容分支”，避免下次审核把兼容改动误判为越界。

## 三、目标与边界

### 3.1 目标

1. 让真实源码的 `std.concurrent` 调用进入签名/能力/消费检查；
2. 让 `spawn`/`submit` 产生显式边界事实并进入生命周期结果；
3. 让取消检查进入 `IrOwnership.cancellation_checks`；
4. 修复 F-4～F-7 的 API/测试问题；
5. 更新文档状态，明确 22C 的真实入口。

### 3.2 不负责

- 不实现 Runtime、VM、LLVM 或标准库函数体；
- 不实现 32B 原子头、锁等待、真实取消、深拷贝字节或任务池执行；
- 不改 `.xiaoc`/`.xar` feature 位；F-8 的产物门禁留给 22C；
- 不把 `spawn`/`submit` 的事实改成名称启发式；
- 不进入 22C 的 ABI/平台工作。

## 四、实施设计

### 4.1 类型/签名接线

- 在 `standard_module_function_signatures` 注册 `std.concurrent` 签名；用
  `FunctionSignature` 的 `type_parameters`/`type_parameter_ids` 表达泛型；
- `spawn`/`submit` 的函数参数和边界值参数要与 `FunctionSignature` 的参数模型兼容；
- 在 `check_call` 内：
  - 用 `concurrency_capabilities` 检查每个边界实参；
  - 用 `check_boundary_argument_count` 检查 64 上限；
  - 用 `HandleConsumption` 处理 `join`/`detach`/`wait`/`cancel` 的消费关系；
  - 发出 `X03-TYPE-030/031/032`；不得只在测试辅助函数里返回 `false`；
- 签名表不再用 `Type::Dynamic` 代替泛型返回/参数。

### 4.2 前端边界事实

在 `frontend.rs` 的类型检查结果之后增加一个显式 lowering 遍历：

- 只识别已解析的 `std.concurrent.spawn`/`submit` 调用；
- 从调用实参和类型结果构造 `BoundaryValue::from_type`/`from_type_in_runtime`；
- 目标 Runtime ID 来自 `runtime` 参数或 pool 所属 Runtime；
- 调用 `BoundaryFacts::record_spawn_values`/`record_submit_values`；
- 把事实传给 `LifetimeAnalyzer::analyze_with_boundary_facts`；
- 没有显式调用的值不得产生事实。

### 4.3 取消检查

同一遍历按调用生成 `IrCancellationCheck`：

- `spawn`/`submit`：`safe_point`；
- `join`/`wait`：`wait`；
- `lock`：`lock_wait`；
- 顺序按源码位置排序；清理动作之后不得再插入检查。

数据路径建议：在 `LifetimeResult` 增加取消检查事实，或由 IR lowering 直接接收同一
lowering 计划；二者择一，不能继续固定 `Vec::new()`。

### 4.4 API 修复

- `HandleConsumption`：消费后拒绝所有后续操作（`cancel` 也不例外）；补状态测试；
- `BoundaryValueInput for ValueId`：删除或标为测试专用，生产路径必须带类型；
- `shared_mut` 没有来源 Runtime 时拒绝或要求补齐；
- `record_fact` 收窄为 crate 内部或标注仅供已验证数据；
- `IrBoundaryTransfer` 只接受 `deep_copy`/`serialization`，去掉 `"serialize"` 别名。

## 五、需要先裁定的选择

| 编号 | 问题 | 建议 | 备选 |
| --- | --- | --- | --- |
| D-1 | 泛型签名怎么表达 | 复用 `FunctionSignature`/`TypeScheme` 的类型参数；`spawn<T>`/`join<T>` 等按现有泛型调用实例化 | 新增并发专用签名结构 |
| D-2 | 边界事实在哪一层产生 | 前端类型检查后的显式 lowering 遍历；复用 AST 调用和类型结果 | 放到 IR lowering（缺少调用点上下文） |
| D-3 | 取消检查数据路径 | 扩展 `LifetimeResult`，由 IR lowering 读取 | 由 IR lowering 直接接收独立计划 |
| D-4 | `cancel` 的签名 | 两个重载：`cancel(thread<T>)`/`cancel(task<T>)` | 保留 `Dynamic` 参数（无法静态拒绝） |
| D-5 | 字节码 feature 门禁 | 本批只登记为 22C 前置，22C 实现 `.xiaoc` 门禁 | 本批提前加门禁（会越过 22B 边界） |

## 六、实施步骤

| 步 | 内容 | 验证 |
| --- | --- | --- |
| 0 | 基线核对：当前 `Type`/IR/生命周期/签名结构，确认 F-1～F-10 | 代码/文档 |
| 1 | 签名表改成可实例化泛型；注册 `std.concurrent` | 类型调用测试 |
| 2 | 在调用检查中接能力、消费、64 上限和 030–032 | 正/负例 |
| 3 | 前端显式 lowering 生成 `BoundaryFacts`；调用 `analyze_with_boundary_facts` | 源码生命周期测试 |
| 4 | 生成取消检查并进入 IR；补 verifier 覆盖 | IR 快照/非法结构 |
| 5 | 修复 F-5/F-6 API 绕过；补状态测试 | 单元/源码负例 |
| 6 | 补 F-7 源码级回归和旧快照兼容 | `cargo test`、快照 |
| 7 | 更新 22B/22/12/README 状态；记录本地/远端证据 | docs/usedocs/layout |

## 七、验收

1. 源码 `spawn`/`submit` 产生 `TaskBoundary + ThreadBoundary` 和 IR fact/transfer；
2. 普通句柄、`shared_mut` 跨 Runtime、`dynamic`/resource、65 参数、消费后使用均被拒绝；
3. 取消检查在实际 IR 中非空，位置/类别通过 verifier；
4. `std.concurrent` 签名可推导 `thread<T>`/`task<T>` 和 `join`/`wait` 的 T；
5. 22B-FIX 之前的“已实施/22C 可消费”状态已改成“部分实现/待接线”，完成后才恢复；
6. 没有实现 Runtime/VM/LLVM/标准库函数体；F-8 的产物门禁明确归 22C；
7. workspace tests/fmt/docs gates 全绿；如声称远端证据，必须有对应推送和运行号。

## 八、不许

1. **不许**在 22B-FIX 实现 Runtime/VM/LLVM 并发行为或序列化字节；
2. **不许**用名称、变量名或函数名推断 `spawn`/`submit`；
3. **不许**继续让 `std_concurrent_signatures` 只被自身测试调用；
4. **不许**用 `Dynamic` 冒充 `spawn<T>`/`join<T>` 的泛型；
5. **不许**保留 `ValueId -> Sendable` 的生产捷径或无来源 `shared_mut` 放行；
6. **不许**把取消检查固定成空 `Vec`；
7. **不许**在 22B-FIX 关闭前进入 22C；
8. **不许**把字节码新类型标签写成已兼容旧运行时；门禁归 22C。

## 九、落点

```text
docs/DevDocs/22b-fix-concurrency-wiring.md              本文
docs/DevDocs/22b-concurrency-types-lifetime-ir.md       状态/证据/交接修正
docs/DevDocs/22-concurrency-execution-and-shared-handles.md  22C 门禁
docs/DevDocs/12-tests-and-milestones.md                 P0 的 22B-FIX 前置
docs/DevDocs/README.md                                  索引与状态
core/rust/crates/xiao-types/src/concurrency.rs          签名/能力/消费/上限接线
core/rust/crates/xiao-types/src/checker/                调用检查
core/rust/crates/xiao-driver/src/frontend.rs            签名注册与事实 lowering
core/rust/crates/xiao-lifetime/src/                     事实 API/诊断
core/rust/crates/xiao-ir/src/                          取消检查/verifier
core/rust/crates/xiao-types/tests/                      源码级类型测试
core/rust/crates/xiao-lifetime/tests/                   源码级生命周期测试
core/rust/crates/xiao-ir/tests/                         源码级 IR 测试
```

不修改 `xiao-runtime`、`xiao-vm`、`xiao-codegen-llvm` 的并发行为；为新增类型补穷举
拒绝分支是允许的，但必须标为兼容改动，不冒充 22C 实现。

## 十、风险

1. **泛型签名改造**：`spawn`/`submit` 的多值参数和函数参数不容易塞进现有
   `FunctionSignature`；D-1 选错会把类型检查器改成第二套调用系统。
2. **事实 lowering 位置**：在类型检查后、生命周期前做遍历，需要稳定的 AST→ValueId 映射；
   如果没有映射，会退化成按名称/位置猜测。
3. **取消检查顺序**：轻量检查点必须与清理动作的相对顺序一致，否则 verifier 会拒绝或
   22C 会误判。
4. **API 收紧**：删除 `ValueId -> Sendable` 捷径会影响现有单元测试；测试应改用带类型的
   `BoundaryValue`，不能为了测试保留不安全入口。
5. **文档再次过宽**：本批完成前不能把 22B 写成“已完成”；22C 门禁必须以实际源码级证据为准。

## 相关页面

- [22B. 并发类型、生命周期与 IR](22b-concurrency-types-lifetime-ir.md)
- [22A. 并发入口冻结](22a-concurrency-freeze.md)
- [22A-FIX. 契约冲突与收口](22a-fix-concurrency-freeze.md)
- [22. 并发执行与共享句柄](22-concurrency-execution-and-shared-handles.md)
- [12. 测试与开发里程碑](12-tests-and-milestones.md)
