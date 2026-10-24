# 22B. 并发类型、生命周期与 IR

> **这是 22 阶段的第二批：把 22A/22A-FIX 的并发契约落成静态类型、生命周期事实和 IR，
> 不接 Runtime、VM、LLVM 或标准库实现。**
> 22A-FIX 已固定 `std.concurrent` 的 API/类型名、Send/Sync 类型表、CrossThread 字段、
> 诊断映射和产物门禁；本批只让 22C 能按这些事实实现 Runtime，不在实现阶段重新解释语义。
>
> 状态：**规划完成（2026-10-24），待星崽裁定 §四的 D-1～D-8 后实施**。本文只规划，
> 不写产品代码；实施由另起 Agent 按本文执行，且不得越过 22B 边界。

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
| 类型模型 | `Type` 只有标量、`none`、函数、容器、表、资源、动态；**没有**并发句柄类型 | 需要新增 7 类类型及其显示/统一/降低 |
| 类型注解 | `DeclaredType` 只有标量、`set`、资源；`FunctionTypeAnnotation` 只有标量、`none`、资源、`set`、`[T]`、`dict`、裸类型参数；没有通用泛型语法 | `thread<T>` 等是否开放显式注解必须先裁定（D-1） |
| 能力推导 | 类型层没有 `Send`/`Sync` 或等价能力表 | 需要按 22A 的实际类型表实现递归判定 |
| 普通句柄 | `StrongHandle`/`WeakHandle` 是 `xiao-runtime` 的 Rust 句柄，不是 Xiao `Type`；语言层没有普通句柄类型 | `X03-TYPE-031` 目前没有语言层生产者（D-2） |
| 生命周期事实 | `EscapeReason::CrossThread` 已存在（`core/rust/crates/xiao-lifetime/src/model.rs:136-144`），但没有生产者 | 需要显式 lowering 产生事实，不能按名称启发式 |
| 生命周期结果 | `LifetimeResult` 只有 scopes/values/edges/release plans/dynamic checks，没有 boundary facts | 需要新增事实容器和诊断 |
| IR 类型 | `IrType` 没有并发类型；`lower_type`/`restore_type` 需要同步 | 新增类型必须可往返、可验证 |
| IR 所有权 | `IrOwnership` 只有 scopes/values/edges/release plans/dynamic checks，没有跨线程事实、边界动作或取消检查 | 需要新增 IR 形状和 verifier |
| 快照 | `IR_VERSION = 1`、`IR_SNAPSHOT_VERSION = 1`，已有稳定 JSON 快照和版本守卫 | 新增字段是否兼容旧快照必须裁定（D-5） |
| 标准库签名 | `xiao-intrinsics`/`stdlib` 中没有 `std.concurrent`；UseDocs 只有 `planned` 页面 | 22B 只能做签名/类型检查输入，不能实现标准库（D-6） |
| 测试入口 | `xiao-types/tests`、`xiao-lifetime/tests/a06_lifetime.rs`、`xiao-ir/tests` 已分别有类型、生命周期、快照/verifier 规格 | 22B 应按这三层加正/负例和快照 |

## 三、22B 的目标与边界

### 3.1 目标

1. 在 `xiao-types` 增加并发类型和候选 A 能力判定；
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

以下不是重新选择 22A 语义，而是 22B 的落地形状；建议值不自动生效。

### D-1（类型注解语法）是否开放 `thread<T>` 等显式注解？

- **甲（建议）**：22B 只加类型表示和推导，不扩用户类型注解语法；`thread<T>` 等由
  `std.concurrent` 签名和表达式推断得到。22E 若需要显式注解再补语法。
- **乙**：在 `DeclaredType`/`FunctionTypeAnnotation` 增加 `thread<T>`、`task<T>`、
  `shared<T>`、`shared_mut<T>`、`lock_guard<T>` 的专用语法，同步 parser/AST/type checker。
- **丙**：一次引入通用泛型类型语法，支持所有 `Name<...>`；范围远大于 22B。

甲能避免把 22B 变成泛型语法批次；乙适合“用户必须能写类型”的场景；丙不建议。

### D-2（普通句柄的身份）`X03-TYPE-031` 对准哪些值？

现状是 `StrongHandle`/`WeakHandle` 只存在于 `xiao-runtime`，语言 `Type` 没有普通句柄。
可选：

- **甲（建议）**：新增内部 `Type::OrdinaryHandle`（不开放用户语法），代表普通
  `StrongHandle`/`WeakHandle` 或不可共享的运行时句柄；`thread`/`task`/`runtime`/`pool`/
  `lock_guard` 同属不可跨线程句柄族；`X03-TYPE-031` 对它们生效。
- **乙**：不引入普通句柄类型，所有无共享能力的值都按 `X03-TYPE-030` 拒绝；这会让 031
  没有语言层生产者，和 22A 的两类诊断不一致。
- **丙**：向用户开放 `handle<T>` 语法和构造；超出 22B 范围。

甲给出 031 的真实类型身份；必须在实现前确认 `xiao-runtime` 的句柄是否会进入类型检查器。

### D-3（句柄所有权）`join`/`detach`/`wait` 的“消费”怎么强制？

22A 规定 `join`/`detach`/`wait` 消费句柄，`cancel` 不消费；`runtime`/`pool` 是可复制
别名。现有类型系统没有线性/使用后移动检查。

- **甲（建议）**：只对 `thread<T>`/`task<T>` 增加“已消费”状态，在类型检查或 lowering
  拒绝再次使用；`runtime`/`pool`/`shared<T>` 可复制；`lock_guard` 受作用域约束。
- **乙**：消费只作为 Runtime 错误，类型层不强制；22C 负责。
- **丙**：引入通用线性类型系统；范围过大。

甲更符合 22A 的“消费”措辞；需要确认 use-after-move 诊断的身份（建议复用现有函数/绑定诊断，
不新增 X03 码，除非另行裁定）。

### D-4（边界事实映射）哪些操作产生哪个事实？

建议：

| 操作 | `task_boundary` | `thread_boundary` | Runtime ID |
| --- | --- | --- | --- |
| `spawn(runtime, ...)` | true | true | 目标 Runtime |
| `submit(pool, ...)` | true | true（池任务可能换 worker） | 池所属 Runtime |
| 同线程挂起/未来协程 | true | false | 当前 Runtime |
| `join`/`wait`/`detach` | 不产生新事实 | 不产生新事实 | 沿用所属句柄 |

备选：`submit` 的 `thread_boundary` 先记 false，等 22C 证明 worker 固定；但这会和
“池任务可迁移”的模型冲突。建议按上表，且所有事实只由显式 lowering 产生。

### D-5（IR 形状与快照兼容）

建议：

- 在 `IrOwnership` 增加可选字段：
  - `cross_thread_facts: Vec<IrCrossThreadFact>`，字段
    `value: u32`、`runtime_id: u64`、`task_boundary: bool`、`thread_boundary: bool`、
    `span: IrSpan`；
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

备选：把三类数据放到新的顶层 `IrConcurrency`；快照结构更清晰，但会扩 `IrProgram`。
建议先按 `IrOwnership` 扩展，减少顶层结构变化。

### D-6（`std.concurrent` 的签名从哪里来）

22B 需要类型检查 `spawn`/`submit`/`shared_read` 等调用，但 22E 才实现标准库。

- **甲（建议）**：22B 在 `xiao-types`（或测试夹具）提供**签名-only**的
  `std.concurrent` 类型表；不生成 `stdlib/concurrent.xiao`，不接 VM/LLVM。
- **乙**：在 `xiao-intrinsics` 增加 `status=planned` 的声明（无 VM/ABI 绑定），22C/22E 接手。
- **丙**：22B 直接写 `stdlib/concurrent.xiao` 函数体；超出 22B 边界，不建议。

甲让 22B 可独立测试类型规则；乙需要扩 intrinsic schema 和校验。无论选哪个，22E 才负责
用户可导入的真实模块。

### D-7（多边界值的静态上限）

22A 规定最多 64 个边界参数、单值 16 MiB、整帧 64 MiB；超限的运行时错误是
`X06-RUNTIME-029`。22B 需要决定 65 个参数在类型检查阶段报什么：

- 甲（建议）：复用函数调用参数诊断（不新增 X03 码），并在 lowering 生成边界动作前拒绝；
- 乙：新增一个并发类型诊断（需扩展 030–032 之外，和 22A 固定映射冲突）；
- 丙：只留到 Runtime 029；类型检查接受 65 个参数。

甲避免把运行时上限伪装成静态语义；具体诊断身份实施前确认。

### D-8（取消/边界动作的 IR 粒度）

- **甲（建议）**：只记录边界动作、取消安全点和等待点，不改现有 CFG 形状；
  `IrCancellationCheck` 用位置/类别表达，22C 负责真实等待。
- **乙**：新增取消专用控制流边；会扩 IR verifier 和快照。
- **丙**：22B 完全不记录，交给 22C；与 22 阶段“取消边界”目标不符。

甲足够让 22C 消费，且不把 Runtime 调度细节写进类型 IR。

## 五、实施步骤

| 步 | 内容 | 验证 |
| --- | --- | --- |
| 0 | 基线核对：22A-FIX 契约、当前 `Type`/`IrType`/lifetime 结构、快照版本 | 文档/代码读取 |
| 1 | 类型模型：新增 `Thread`/`Task`/`Runtime`/`Pool`/`Shared`/`SharedMut`/`LockGuard`（以及 D-2 选择的普通句柄）；补 Display/统一/lower/restore | `xiao-types` 单测；`xiao-ir` 类型往返 |
| 2 | 能力表：实现候选 A 递归判定和诊断；shared/shared_mut 按 tag；可变容器/动态/资源/普通句柄拒绝 | 正/负例、`X03-TYPE-030..032` |
| 3 | 签名面：按 D-6 提供 `std.concurrent` 签名；检查多值、返回类型、消费状态 | 调用类型检查；不执行 Runtime |
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
6. 22C 可以只按 22B 的类型/事实/动作实现，不再补语义。

## 八、不许

1. **不许**在 22B 实现 `spawn`/`join`/`lock` 的 Runtime 行为或序列化字节；
2. **不许**修改 32B 头、Runtime ABI 1.17、`.xiaoc`/`.xar` feature 位或产物矩阵；
3. **不许**按变量名/函数名推断 `CrossThread` 事实；
4. **不许**把普通句柄或 `shared_mut` 跨 Runtime 放行；
5. **不许**用运行时 029 代替 64 参数上限的静态诊断（按 D-7 裁定）；
6. **不许**在未裁定 D-1～D-8 前写死类型语法或 IR 形状；
7. **不许**把 `planned` 的 UseDocs 页面改成 `verified`；
8. **不许**在没有旧快照/版本策略的情况下改 `IrType`/`IrOwnership` 的必填结构。

## 九、风险

1. **普通句柄没有语言类型**：不解决 D-2，`X03-TYPE-031` 可能没有真实生产者。
2. **消费语义需要线性检查**：现有类型系统没有 affine 状态，D-3 选择甲会扩类型检查器。
3. **`submit` 的线程边界**：池任务可能换 worker，D-4 取 false 会让 22C 与“可迁移”模型冲突。
4. **IR 快照兼容**：新增字段和 `IrType` 变体可能破坏旧快照；D-5 必须明确版本策略。
5. **签名面越过边界**：D-6 若选丙，会把 22E 的标准库实现提前到 22B。
6. **事实来源漂移**：只有显式 lowering 能产生事实；名称启发式会让 VM/LLVM 分叉。
7. **诊断撞号**：X03/X06 新号必须与现有注册表对账；不能用 029 冒充静态错误。

## 十、落点

```text
docs/DevDocs/22b-concurrency-types-lifetime-ir.md       本文
docs/DevDocs/22-concurrency-execution-and-shared-handles.md  22B 链接与交接
docs/DevDocs/12-tests-and-milestones.md                 P0 的 22B 入口
docs/DevDocs/README.md                                  22B 索引
core/rust/crates/xiao-types/                            类型与能力表
core/rust/crates/xiao-syntax/                           仅当 D-1 选乙/丙时
core/rust/crates/xiao-lifetime/                         boundary facts/诊断
core/rust/crates/xiao-ir/                               IrType/IrOwnership/verifier/快照
core/rust/crates/xiao-i18n/                             新诊断中英消息
```

不修改 `xiao-runtime`、`xiao-vm`、`xiao-codegen-llvm`、`stdlib/` 的并发实现；这些属于
22C–22E。22B 结束时只交付静态类型、生命周期和 IR 契约。

## 相关页面

- [22A. 并发入口冻结](22a-concurrency-freeze.md)——最终 API/类型/诊断契约
- [22A-FIX. 契约冲突与收口](22a-fix-concurrency-freeze.md)——八项甲裁定和补丁
- [22. 并发执行与共享句柄](22-concurrency-execution-and-shared-handles.md)——阶段分解
- [07-D. 并发模型研究](07d-concurrency-model-research.md)——候选 A/CrossThread 输入
- [21A. 单线程 RC、强环与并发模型交接](21a-rc-cycles-and-concurrency-handoff.md)——普通句柄边界
- [12. 测试与开发里程碑](12-tests-and-milestones.md)——P0 出口条件
