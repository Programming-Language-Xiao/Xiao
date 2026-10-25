# 22B. 并发类型、生命周期与 IR

> **这是 22 阶段的第二批：把 22A/22A-FIX 的并发契约落成静态类型、生命周期事实和 IR，
> 不接 Runtime、VM、LLVM 或标准库实现。**
> 22A-FIX 已固定 `std.concurrent` 的 API/类型名、Send/Sync 类型表、CrossThread 字段、
> 诊断映射和产物门禁；本批只让 22C 能按这些事实实现 Runtime，不在实现阶段重新解释语义。
>
> 状态：**已实施（2026-10-25）**。D-1 取乙（专用注解语法），22B-FIX/FIX.1 的 D-1～D-5
> 取甲；类型、生命周期模型、IR 动作/verifier、参数化入口、作用域消费、BoundaryFacts、
> shared 能力和取消检查均已接入。Runtime/VM/LLVM、标准库函数体和产物门禁仍由 22C–22E
> 负责。

## 一、入口条件与冻结输入

### 1.1 必读

1. [22. 并发执行与共享句柄](22-concurrency-execution-and-shared-handles.md)
   ——阶段分解、22B/22C 边界；
2. [22A. 并发入口冻结](22a-concurrency-freeze.md) §2.9——最终修正契约；
3. [22A-FIX. 契约冲突与收口](22a-fix-concurrency-freeze.md)——八项甲裁定和补丁；
4. [07-D. 并发模型研究](07d-concurrency-model-research.md) §10–§11——候选 A、
   `CrossThread` 和不以名称推断能力的约束；
5. [21A. 单线程 RC、强环与并发模型交接](21a-rc-cycles-and-concurrency-handoff.md)
   §3.2/§五——普通句柄的单线程边界；
6. [12. 测试与开发里程碑](12-tests-and-milestones.md) P0——阶段出口。

### 1.2 已冻结的输入

22B 不得重新选择的项：

| 项 | 冻结内容 | 出处 |
| --- | --- | --- |
| 类型名 | `thread<T>`、`task<T>`、`runtime`、`pool`、`shared<T>`、`shared_mut<T>`、`lock_guard<T>` | [22A §2.6](22a-concurrency-freeze.md) |
| API | `current_runtime`/`runtime_create`/`runtime_close`、`spawn(runtime, fn, ...)`、`join`/`detach`/`cancel`、`shared_read`/`shared_mut`、`read`/`lock`/`lock_read`/`lock_write`、`pool_create(runtime, capacity)`/`submit`/`wait`/`shutdown` | [22A §2.6/§2.9](22a-concurrency-freeze.md) |
| Send/Sync | 标量、固定宽度数值、`bool`、`str`、递归安全 `tuple` 接受；当前可变 `array`/`set`/`dict`/字典表/字典列、普通句柄、动态值、资源拒绝；显式共享句柄按 tag 接受 | [22A §2.9](22a-concurrency-freeze.md) |
| CrossThread | 事实名 `TaskBoundary`/`ThreadBoundary`；IR 字段 `runtime_id: u64`、`task_boundary: bool`、`thread_boundary: bool`；诊断字段 `boundary_kind`、`runtime_id`、`source_span` | [22A §2.9](22a-concurrency-freeze.md) |
| 诊断 | `X03-TYPE-030..032`、`X06-LIFETIME-006..008`；不新建 `X22` 码族 | [22A §2.9](22a-concurrency-freeze.md) |
| 值边界 | 逐参数深拷贝、失败逆序清理、主错误与 `suppressed`；长度前缀框架只作为边界动作，不在 22B 写序列化字节 | [22A §2.2](22a-concurrency-freeze.md) |
| 产物门禁 | `.xiaoc`/`.xar` 的 ABI 1.17 与 feature 位；原生并发产物后置到 22E | [22A §2.9](22a-concurrency-freeze.md) |

证据：22A 的 32B 布局见 `40093591873`；22A-FIX 的 workspace gates 见
`40104492341`/`40104894749`/`40106553383`。22B 不重新测量 ABI，也不改 32B 头。

## 二、实施前核出的现状

| 面 | 当前事实 | 对 22B 的影响 |
| --- | --- | --- |
| 类型模型 | `Type` 只有标量、`none`、函数、容器、表、资源、动态；**没有**并发句柄类型 | 需要新增 8 类类型及其显示/统一/降低 |
| 类型注解 | `DeclaredType` 只有标量、`set`、资源；`FunctionTypeAnnotation` 只有标量、`none`、资源、`set`、`[T]`、`dict`、裸类型参数；没有通用泛型语法 | D-1 取乙：增加并发类型专用注解语法，不引入通用泛型 |
| 能力推导 | 类型层没有 `Send`/`Sync` 或等价能力表 | 需要按 22A 的实际类型表实现递归判定 |
| 普通句柄 | `StrongHandle`/`WeakHandle` 是 `xiao-runtime` 的 Rust 句柄，不是 Xiao `Type`；语言层没有普通句柄类型 | D-2 取甲：内部 `OrdinaryHandle`，`X03-TYPE-031` 有内部生产者 |
| 生命周期事实 | `EscapeReason::CrossThread` 已存在（`core/rust/crates/xiao-lifetime/src/model.rs:136-144`），但没有生产者 | 需要显式 lowering 产生事实，不能按名称启发式 |
| 生命周期结果 | `LifetimeResult` 只有 scopes/values/edges/release plans/dynamic checks，没有 boundary facts | 需要新增事实容器和诊断 |
| IR 类型 | `IrType` 没有并发类型；`lower_type`/`restore_type` 需要同步 | 新增类型必须可往返、可验证 |
| IR 所有权 | `IrOwnership` 只有 scopes/values/edges/release plans/dynamic checks，没有跨线程事实、边界动作或取消检查 | 需要新增 IR 形状和 verifier |
| 快照 | `IR_VERSION = 1`、`IR_SNAPSHOT_VERSION = 1`，已有稳定 JSON 快照和版本守卫 | D-5 取甲：扩 `IrOwnership` + `serde(default)`，旧快照先保持可读 |
| 标准库签名 | `xiao-intrinsics`/`stdlib` 中没有 `std.concurrent`；UseDocs 只有 `planned` 页面 | D-6 取甲：`xiao-types` 签名表；22B 不实现标准库 |
| 测试入口 | `xiao-types/tests`、`xiao-lifetime/tests/a06_lifetime.rs`、`xiao-ir/tests` 已分别有类型、生命周期、快照/verifier 规格 | 22B 应按这三层加正/负例和快照 |

## 三、22B 的目标与边界

### 3.1 目标

1. 在 `xiao-types` 增加并发类型和候选 A 能力判定；按 D-1 乙在 `xiao-syntax`
   增加 `thread<T>`、`task<T>`、`shared<T>`、`shared_mut<T>`、`lock_guard<T>` 和
   `runtime`/`pool` 的专用注解语法；
2. 让类型检查器能识别 `std.concurrent` 的签名、多边界值、返回类型和诊断；
3. 在 `xiao-lifetime` 产生 `TaskBoundary`/`ThreadBoundary` 事实和 Runtime ID；
4. 在 `xiao-ir` 记录跨线程值、边界动作、所有权/释放、取消检查，并把 verifier 补全；
5. 让 22C 可以只消费这些类型/事实/动作，不再决定名称、能力表或事实语义。

### 3.2 不负责

- 不实现 `std.concurrent` 的 Runtime、VM 或标准库函数体；
- 不实现 32B 原子头、弱升级、锁等待、真实取消或序列化字节；
- 不改 `StrongHandle`/`WeakHandle` 布局，不改 Runtime ABI 1.17；
- 不改 `.xiaoc`/`.xar` feature 位或产物矩阵；
- 不实现任务池、线程、共享句柄的运行时行为；
- 不把 `CrossThread` 由变量名/函数名推断产生。

## 四、实施前必须裁定的设计点

以下不是重新选择 22A 语义，而是 22B 的落地形状。星崽已于 2026-10-24 完成裁定：
**D-1 取乙，D-2～D-8 取甲**；各小节保留备选作历史记录。

### D-1（类型注解语法）是否开放 `thread<T>` 等显式注解？

- **甲**：22B 只加类型表示和推导，不扩用户类型注解语法；`thread<T>` 等由
  `std.concurrent` 签名和表达式推断得到。22E 若需要显式注解再补语法。
- **乙（最终选择）**：在 `DeclaredType`/`FunctionTypeAnnotation` 增加 `thread<T>`、`task<T>`、
  `shared<T>`、`shared_mut<T>`、`lock_guard<T>` 的专用语法，同步 parser/AST/type checker；
  `runtime`/`pool` 为非泛型注解。
- **丙**：一次引入通用泛型类型语法，支持所有 `Name<...>`；范围远大于 22B。

**最终选择：乙。** 只扩这组并发类型的专用解析，不引入通用泛型语法；需要同步
`DeclaredType`、`FunctionTypeAnnotation`、`TypeTerm` 的使用点、`type_checker` 的注解转换、
`xiao-ir` 的 `lower_declared_type`/`lower_function_annotation` 和语法负例测试。

### D-2（普通句柄的身份）`X03-TYPE-031` 对准哪些值？

现状是 `StrongHandle`/`WeakHandle` 只存在于 `xiao-runtime`，语言 `Type` 没有普通句柄。
可选：

- **甲（最终选择）**：新增内部 `Type::OrdinaryHandle`（不开放用户语法），代表普通
  `StrongHandle`/`WeakHandle` 或不可共享的运行时句柄；`thread`/`task`/`runtime`/`pool`/
  `lock_guard` 同属不可跨线程句柄族；`X03-TYPE-031` 对它们生效。
- **乙**：不引入普通句柄类型，所有无共享能力的值都按 `X03-TYPE-030` 拒绝；这会让 031
  没有语言层生产者，和 22A 的两类诊断不一致。
- **丙**：向用户开放 `handle<T>` 语法和构造；超出 22B 范围。

**最终选择：甲。** 普通句柄只作为内部类型/IR 身份，不进入 D-1 的用户注解语法；22B 的
测试夹具和 intrinsic/runtime 签名可以产生它，22C 再决定实际 Runtime 值如何暴露身份。

### D-3（句柄所有权）`join`/`detach`/`wait` 的“消费”怎么强制？

22A 规定 `join`/`detach`/`wait` 消费句柄，`cancel` 不消费；`runtime`/`pool` 是可复制
别名。现有类型系统没有线性/使用后移动检查。

- **甲（最终选择）**：只对 `thread<T>`/`task<T>` 增加“已消费”状态，在类型检查或 lowering
  拒绝再次使用；`runtime`/`pool`/`shared<T>` 可复制；`lock_guard` 受作用域约束。
- **乙**：消费只作为 Runtime 错误，类型层不强制；22C 负责。
- **丙**：引入通用线性类型系统；范围过大。

**最终选择：甲。** `join`/`detach`/`wait` 标记 thread/task 已消费，`cancel` 只借用；
`runtime`/`pool` 保持可复制别名，`shared<T>` 可复制，`lock_guard` 不可逃逸。使用后移动的
诊断优先复用现有绑定/函数参数诊断；若现有身份不足，再单独登记，不复用 030–032 的语义。

### D-4（边界事实映射）哪些操作产生哪个事实？

**最终选择：甲。**

| 操作 | `task_boundary` | `thread_boundary` | Runtime ID |
| --- | --- | --- | --- |
| `spawn(runtime, ...)` | true | true | 目标 Runtime |
| `submit(pool, ...)` | true | true（池任务可能换 worker） | 池所属 Runtime |
| 同线程挂起/未来协程 | true | false | 当前 Runtime |
| `join`/`wait`/`detach` | 不产生新事实 | 不产生新事实 | 沿用所属句柄 |

所有事实只由显式 `spawn`/`submit` lowering 产生，不能按变量名或函数名推断。备选是
`submit` 的 `thread_boundary` 先记 false；这与“池任务可迁移”的模型冲突，不采用。

### D-5（IR 形状与快照兼容）

**最终选择：甲。**

- 在 `IrOwnership` 增加可选字段：
  - `cross_thread_facts: Vec<IrCrossThreadFact>`，字段
    `value: u32`、`runtime_id: u64`、可选 `source_runtime_id: Option<u64>`、
    `task_boundary: bool`、`thread_boundary: bool`、`span: IrSpan`；
  - `boundary_transfers: Vec<IrBoundaryTransfer>`，记录参数列表、目标 Runtime、拷贝类别
    （深拷贝/序列化）和清理顺序；
  - `cancellation_checks: Vec<IrCancellationCheck>`，记录安全点/等待点位置。
- 新增字段使用 `#[serde(default)]`，先保证旧快照可读；如果 `IrType` 变体或必填字段导致
  旧快照不能往返，则提升 `IR_VERSION`/`IR_SNAPSHOT_VERSION` 并更新所有 spec 快照。
- verifier 不变量：
  1. 每个边界值恰有一个事实；事实的 `runtime_id` 等于 transfer 目标；
  2. 普通句柄、`shared_mut<T>` 跨 Runtime 拒绝；`shared<T>` 只读跨 Runtime 允许；
  3. 参数按位置深拷贝，失败清理顺序与 22A 一致；
  4. `lock_guard` 不进入边界事实；
  5. 没有显式 `spawn`/`submit` lowering 就不得产生事实；
  6. 取消检查不能出现在清理动作之后，取消等待点与锁/任务操作对应。

不采用新的顶层 `IrConcurrency`；本批按 `IrOwnership` 扩展，减少 `IrProgram` 顶层变化。
新增字段必须用 `#[serde(default)]`，旧快照先保持可读；只有必填结构或 `IrType` 变体破坏
往返时才提升版本。

### D-6（`std.concurrent` 的签名从哪里来）

22B 需要类型检查 `spawn`/`submit`/`shared_read` 等调用，但 22E 才实现标准库。

- **甲（最终选择）**：22B 在 `xiao-types`（或测试夹具）提供**签名-only**的
  `std.concurrent` 类型表；不生成 `stdlib/concurrent.xiao`，不接 VM/LLVM。
- **乙**：在 `xiao-intrinsics` 增加 `status=planned` 的声明（无 VM/ABI 绑定），22C/22E 接手。
- **丙**：22B 直接写 `stdlib/concurrent.xiao` 函数体；超出 22B 边界，不建议。

**最终选择：甲。** 22B 只提供类型层签名和测试夹具；22E 才负责用户可导入的真实模块，
22C 负责 Runtime/ABI 接线。

### D-7（多边界值的静态上限）

22A 规定最多 64 个边界参数、单值 16 MiB、整帧 64 MiB；超限的运行时错误是
`X06-RUNTIME-029`。22B 需要决定 65 个参数在类型检查阶段报什么：

- **甲（最终选择）**：复用函数调用参数诊断（不新增 X03 码），并在 lowering 生成边界
  动作前拒绝；
- **乙**：新增一个并发类型诊断（需扩展 030–032 之外，和 22A 固定映射冲突）；
- **丙**：只留到 Runtime 029；类型检查接受 65 个参数。

**最终选择：甲。** 超过 64 个实参在调用参数检查阶段拒绝，复用现有函数参数诊断；
`X06-RUNTIME-029` 只保留给运行时单值/整帧上限，不把静态错误伪装成运行时错误。

### D-8（取消/边界动作的 IR 粒度）

- **甲（最终选择）**：只记录边界动作、取消安全点和等待点，不改现有 CFG 形状；
  `IrCancellationCheck` 用位置/类别表达，22C 负责真实等待。
- **乙**：新增取消专用控制流边；会扩 IR verifier 和快照。
- **丙**：22B 完全不记录，交给 22C；与 22 阶段“取消边界”目标不符。

**最终选择：甲。** 22B 记录取消安全点/等待点，不扩取消专用 CFG；真实持锁等待、唤醒和
超时由 22C 实现。这样 22C 有明确输入，且不把 Runtime 调度细节写进类型 IR。

## 五、实施步骤

| 步 | 内容 | 验证 |
| --- | --- | --- |
| 0 | 基线核对：22A-FIX 契约、当前 `Type`/`IrType`/lifetime 结构、快照版本 | 文档/代码读取 |
| 1 | 类型模型与语法：新增 `Thread`/`Task`/`Runtime`/`Pool`/`Shared`/`SharedMut`/`LockGuard` 和内部 `OrdinaryHandle`；按 D-1 乙扩 `DeclaredType`/`FunctionTypeAnnotation`/parser；补 Display/统一/lower/restore | `xiao-types`/`xiao-syntax` 单测；`xiao-ir` 类型往返 |
| 2 | 能力表：实现候选 A 递归判定和诊断；shared/shared_mut 按 tag；可变容器/动态/资源/普通句柄拒绝 | 正/负例、`X03-TYPE-030..032` |
| 3 | 签名面：按 D-6 在 `xiao-types` 提供签名-only `std.concurrent`；检查多值、返回类型、消费状态 | 调用类型检查；不执行 Runtime |
| 4 | 生命周期：新增 boundary fact 容器；在显式 spawn/submit lowering 产生事实；Runtime ID 传播；release plan | `a06_lifetime` 扩展；无名称启发式 |
| 5 | IR：新增事实/边界动作/取消检查；补 verifier；处理快照兼容和版本 | JSON 往返、非法结构、旧快照 |
| 6 | 诊断/i18n：新增 030–032、006–008 常量与中英消息；参数结构化 | 诊断断言 code/message_id |
| 7 | 回归：类型、生命周期、IR 正/负例；确定性；不执行产品代码 | `cargo test`、快照 |
| 8 | 文档：22/12/README 更新，22C 交接表；不提前写 UseDocs `verified` | docs/usedocs/layout |

## 六、测试与验收

### 6.1 类型层

- 接受：`none`、固定宽度数值、`bool`、`str`、递归安全 `tuple`、显式 `shared<T>`；
- 拒绝：`array`/`set`/`dict`/字典表/字典列、普通句柄、`dynamic`、resource；
- `shared_mut<T>` 跨 Runtime 拒绝；`shared<T>` 只读跨 Runtime 接受；
- `thread<T>`/`task<T>` 的 `join`/`detach`/`wait` 消费与 `cancel` 不消费；
- D-1 乙的专用注解语法：`thread<int>`、`shared<str>`、`runtime`、`pool` 可往返；
  非法/泛型越界（如 `foo<int>`、缺参数）产生稳定语法诊断；
- 多值参数、返回类型、64 个上限的诊断（按 D-7）。

### 6.2 生命周期层

- `spawn` 产生 `TaskBoundary + ThreadBoundary`，Runtime ID 为目标 Runtime；
- `submit` 按 D-4 产生事实；同线程挂起只产生 `TaskBoundary`；
- 事实只由显式 lowering 产生；变量名/函数名不能触发；
- 普通句柄、`shared_mut` 跨 Runtime、动态/资源边界产生 031/008/030；
- 深拷贝失败按逆序清理，主错误和 `suppressed` 顺序正确；
- `lock_guard` 在作用域结束释放，不逃逸。

### 6.3 IR 层

- 新事实/动作/取消检查可 JSON 往返，字段顺序稳定；
- verifier 拒绝：事实引用未知值、Runtime ID 不匹配、普通句柄或 `shared_mut` 跨 Runtime、
  `lock_guard` 进入边界、事实无显式 lowering、清理顺序错误、取消检查在清理之后；
- 旧快照按 D-5 的兼容策略读取；如果提升版本，更新所有 spec 快照和版本守卫测试；
- `xiao-ir` 快照逐条执行，不以“结构非空”代替语义断言。

### 6.4 门禁

```bash
cargo test --manifest-path core/rust/Cargo.toml -p xiao-types -p xiao-lifetime -p xiao-ir
cargo test --manifest-path core/rust/Cargo.toml --workspace
bun run check:docs
bun run check:usedocs
bun run check:layout
git diff --check
```

## 七、22B 退出条件

1. 类型模型、能力表、签名面、生命周期事实、IR 事实/动作/verifier 全部有唯一落点；
2. D-1～D-8 已裁定并写死，不在 22C 重新决定；
3. 正/负例覆盖普通句柄、共享句柄、多边界值、Runtime ID、动态值、资源、跨 Runtime；
4. IR 快照和旧快照兼容/版本策略通过；
5. 没有实现 Runtime、VM、LLVM、标准库函数体或 32B ABI；
6. [22B-FIX](22b-fix-concurrency-wiring.md) 已完成，22C 可以只按 22B 的类型/事实/动作
   实现，不再补静态语义。

## 八、不许

1. **不许**在 22B 实现 `spawn`/`join`/`lock` 的 Runtime 行为或序列化字节；
2. **不许**修改 32B 头、Runtime ABI 1.17、`.xiaoc`/`.xar` feature 位或产物矩阵；
3. **不许**按变量名/函数名推断 `CrossThread` 事实；
4. **不许**把普通句柄或 `shared_mut` 跨 Runtime 放行；
5. **不许**用运行时 029 代替 64 参数上限的静态诊断（按 D-7 裁定）；
6. **不许**重新选择已裁定的 D-1～D-8（D-1 乙、其余甲）；类型语法和 IR 形状按本文写死；
7. **不许**把 `planned` 的 UseDocs 页面改成 `verified`；
8. **不许**在没有旧快照/版本策略的情况下改 `IrType`/`IrOwnership` 的必填结构。

## 九、风险

1. **普通句柄身份**：D-2 已选内部 `OrdinaryHandle`；若 22C 没有实际值产生该类型，
   `X03-TYPE-031` 只能由测试夹具/内部签名触发，不能假装已有用户路径。
2. **消费语义需要线性检查**：现有类型系统没有 affine 状态，D-3 选择甲会扩类型检查器。
3. **`submit` 的线程边界**：D-4 取 true 是保守事实；若 22C 固定 worker，需要在事实/验证中
   明确是否仍保留保守值，不能静默改写 22B 记录。
4. **IR 快照兼容**：新增字段和 `IrType` 变体可能破坏旧快照；D-5 的兼容/版本策略必须通过。
5. **签名面漂移**：D-6 的 `xiao-types` 签名表必须与 22E 的真实 `std.concurrent` 一致；
   类型层和标准库实现不能各有一套名称/所有权。
6. **事实来源漂移**：只有显式 lowering 能产生事实；名称启发式会让 VM/LLVM 分叉。
7. **诊断撞号**：X03/X06 新号必须与现有注册表对账；不能用 029 冒充静态错误。
8. **D-1 扩语法**：专用注解语法会触及 parser/AST/type checker；只扩并发类型，不能顺手
   引入通用泛型或改变既有 `set`/`dict` 注解。

## 十、落点

```text
docs/DevDocs/22b-concurrency-types-lifetime-ir.md       本文
docs/DevDocs/22-concurrency-execution-and-shared-handles.md  22B 链接与交接
docs/DevDocs/12-tests-and-milestones.md                 P0 的 22B 入口
docs/DevDocs/README.md                                  22B 索引
core/rust/crates/xiao-types/                            类型与能力表
core/rust/crates/xiao-syntax/                           D-1 乙的专用注解语法/parser
core/rust/crates/xiao-lifetime/                         boundary facts/诊断
core/rust/crates/xiao-ir/                               IrType/IrOwnership/verifier/快照
core/rust/crates/xiao-i18n/                             新诊断中英消息
```

不接入 `xiao-runtime`、`xiao-vm`、`xiao-codegen-llvm`、`stdlib/` 的并发行为；这些属于
22C–22E。为新增 `Type`/`IrType` 补穷举拒绝分支是允许的兼容改动，不等于 22C 实现。
22B 结束时只交付静态类型、生命周期和 IR 契约。

## 十一、实施结果与证据

22B 已落地到以下提交链：

- `d1ba1c7`、`d81280e`、`7dc64ac`：专用并发注解 AST/parser、并发 `Type` 变体、候选 A
  能力表、签名-only `std.concurrent` 夹具、句柄消费状态和 64 参数上限；
- `0b1c6da`、`fd9dbc4`、`2fc937b`、`60cb67a`、`40e3cc7`、`52aaf78`：显式
  `TaskBoundary`/`ThreadBoundary` 事实、Runtime ID、跨 Runtime/句柄能力诊断、`lock_guard`
  逃逸约束和兼容视图；
- `ecba419`、`ccf503b`、`f5c8d7f`：`IrType` 并发变体、`IrOwnership` 的 facts/transfers/
  cancellation 字段、旧快照默认值、lower/restore、verifier 和字节码类型标签往返；
- `4512c07`：下游静态穷举的兼容拒绝分支；未接入任何并发 Runtime 行为。
- `22B-FIX` 接线：前端注册 `std.concurrent`/`stdlib.concurrent` 签名并记录已解析调用；
  `xiao-lifetime` 生成带来源 Runtime 的边界事实和取消检查；`xiao-ir` 消费
  `cross_thread_facts`、`boundary_transfers`、`cancellation_checks`；并收紧
  `BoundaryValue`、句柄消费和 `shared_mut` API。

验证已通过：`cargo check --manifest-path core/rust/Cargo.toml --workspace`、
`cargo test --manifest-path core/rust/Cargo.toml --workspace`、`cargo fmt --manifest-path
core/rust/Cargo.toml --all -- --check`，以及 `check:docs`、`check:usedocs`、`check:layout`、
`git diff --check`；`xiao-driver/tests/22b_concurrency_wiring.rs` 覆盖真实源码正/负例。

**证据边界**：本地测试同时覆盖模型/API、parser 注解、真实 `std.concurrent` 源码调用、
生命周期边界、IR facts/transfers/cancellation 和旧快照兼容；远端 workspace gates 运行号为
`40201575737`（提交 `b3ded8b`）。22B-FIX 的接线结果与回归清单见
[22B-FIX](22b-fix-concurrency-wiring.md)。

## 相关页面

- [22A. 并发入口冻结](22a-concurrency-freeze.md)——最终 API/类型/诊断契约
- [22A-FIX. 契约冲突与收口](22a-fix-concurrency-freeze.md)——八项甲裁定和补丁
- [22. 并发执行与共享句柄](22-concurrency-execution-and-shared-handles.md)——阶段分解
- [07-D. 并发模型研究](07d-concurrency-model-research.md)——候选 A/CrossThread 输入
- [21A. 单线程 RC、强环与并发模型交接](21a-rc-cycles-and-concurrency-handoff.md)——普通句柄边界
- [12. 测试与开发里程碑](12-tests-and-milestones.md)——P0 出口条件
