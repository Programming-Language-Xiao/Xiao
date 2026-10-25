# 22B-FIX.1. 并发静态接线的残余修复

> **这是 22B-FIX 与 22C 之间的补丁批，不实现 Runtime/VM/LLVM。**
> 2026-10-25 复核 `07e64f0` 后发现：`std.concurrent` 的签名/能力/边界事实/取消检查已经
> 接入真实流水线，simple `spawn`/`join`、`submit`/`wait` 用例可运行；但 workspace gates
> 红，参数化入口函数、消费状态作用域、`shared_read`/`shared_mut`/`lock_write` 消费和
> shared 构造参数能力仍有缺口。照当前状态进入 22C 会把这些缺口当成已冻结语义。
>
> 状态：**已完成（2026-10-25）**。本批只修 22B 静态接线，不接 Runtime；D-1～D-5
> 已按甲方案落实。workspace gates `40201575737` 已通过（提交 `b3ded8b`）。

## 一、已确认证据

| 项 | 结果 | 说明 |
| --- | --- | --- |
| 本地 workspace 测试 | 通过 | `cargo test --workspace` |
| 本地格式/文档 | 通过 | `cargo fmt --check`、`check:docs`、`check:usedocs`、`check:layout`、`git diff --check` |
| workspace gates `40201575737` | **通过** | 提交 `b3ded8b` 的 Linux workspace gates 全部通过；旧失败 `40161893418` 已修复 |
| maintenance regression `40201575725` | **通过** | 同一提交 `b3ded8b` 的维护回归通过；本批不接 Runtime/LLVM |
| 简单源级接线 | 通过 | `spawn`/`join`、`submit`/`wait`、`shared_mut` 跨 Runtime、`lock_guard` 逃逸用例已有 |
| 参数化入口函数 | **通过** | 真实函数类型按位置检查边界实参，混合 `int`/`str` 参数通过 |
| 混合边界实参 | **通过** | 参数类型不匹配稳定报 `X04-TYPE-002` |
| 消费状态作用域 | **通过** | 绑定身份覆盖别名；分支、循环和 try 路径在合流点保守合并；遮蔽与重新赋值有回归 |
| 控制流出口合流 | **通过** | `return`、`break`、`continue` 的消费状态按可达出口合流；不可达语句不污染后续状态 |
| `shared_read` 消费 | **通过** | 源值复用稳定报 `X04-TYPE-002` |

> 上述历史失败已转为 `22b_concurrency_wiring.rs` 的正式源级回归；本批不保留临时夹具。

## 二、残余缺陷

### F-1（阻断，已修复）workspace gates 红

`core/rust/crates/xiao-driver/tests/22b_concurrency_wiring.rs` 只有
`#![allow(missing_docs)]`，没有 `//!` 模块文档；`A0-COVERAGE-002` 把测试 crate 本身当作
公共声明并报错。CI 运行 `40161893418` 已失败，不能把本地测试通过当成远端验收。

**已完成**：在文件顶部增加 `//! 22B 并发静态接线回归。`；没有用
`allow(missing_docs)` 绕过覆盖规则。

### F-2（阻断，已修复）`spawn`/`submit` 只接受无参入口函数

`std_concurrent_signatures` 把入口函数固定为：

```text
Type::Function { parameters: Vec::new(), return_type: T }
```

见 `core/rust/crates/xiao-types/src/concurrency.rs:316-319`；`spawn` 的参数见
`:351-356`。边界实参在 `check_concurrency_contract` 中也只做 Send 能力检查，没有和入口
函数的参数类型逐一对应（`core/rust/crates/xiao-types/src/checker/imports.rs:145-189`）。

**后果**：22A 要求的“函数值显式声明参数、多边界值按位置传入”无法使用；带参数函数和
混合类型边界实参都会失败。

**已完成**：在 `std.concurrent.spawn`/`submit` 调用检查中读取第二个实参的实际函数类型，
按位置检查边界实参与函数参数，返回值使用函数返回类型实例化 `thread<T>`/`task<T>`；不要
再依赖空参数 `Function` 占位。

### F-3（阻断，已修复）消费状态按名称全局保存

`concurrency_handle_states` 是 `BTreeMap<String, HandleConsumption>`
（`core/rust/crates/xiao-types/src/checker.rs:110`），键只来自 `name_key`
（`ascii:`/`backtick:` + 文本，`core/rust/crates/xiao-types/src/checker/diagnostic.rs:19-25`），
没有作用域或绑定身份。

**后果**：不同函数/分支中的同名 `thread`/`task` 共享消费状态；变量遮蔽、重新赋值或
绑定新句柄都会被误判。

**已完成**：线程/任务句柄及会被消费的命名值使用绑定身份；别名共享消费状态。函数和表方法
使用作用域快照隔离，局部绑定在作用域退出时清理，重新赋值建立新身份。`if`/`elif`/`else`
分支互相隔离后在合流点合并，循环拒绝重复消费循环入口已有值并允许每次迭代先创建新值，
`try`/`catch`/`finally` 也按保守合流传播可能路径的消费结果；`return`、`break`、`continue`
分别保留函数、循环和下一轮迭代的出口状态，不可达语句不会继续污染当前路径。源码回归覆盖跨函数、
别名、重新赋值、互斥分支、分支后复用、循环入口/循环出口复用、循环内新值、try 后复用及三类提前退出。

### F-4（高，已修复）`shared_read`/`shared_mut`/`lock_write` 没有强制消费

`check_concurrency_contract` 只对 `join`/`detach`/`wait`/`cancel` 更新状态
（`imports.rs:191-243`）。`ConcurrentSignature` 里的 `consumed_parameters` 在转换成
`FunctionSignature` 时没有保留，也没有被调用检查消费：

- `shared_read(T)` 的 `T` 应被消费；
- `shared_mut(T)` 的 `T` 应被消费；
- `lock_write(guard, T)` 的新 `T` 应被消费；
- `read`/`lock`/`lock_read` 只借用，不消费。

**已完成**：在并发调用检查中统一消费 `shared_read`/`shared_mut`/`lock_write` 的指定参数；
对命名值使用 F-3 的绑定状态，对临时表达式不保留可复用身份。

### F-5（高，已修复）shared 构造参数没有能力检查

当前只在 `spawn`/`submit` 的边界实参上调用 `concurrency_capabilities`；`shared_read([1])`
和 `shared_mut([1])` 这类构造没有对应的能力拒绝路径，尽管 22A 要求共享句柄的 `T` 也必须
满足递归能力表。

**已完成**：对 `shared_read`/`shared_mut` 的 `T` 使用同一候选 A 表；`shared_mut` 还要保留
同 Runtime 约束。补可变容器、普通句柄、动态值、resource 的负例。

### F-6（高，已修复）源码级回归仍不完整

已有 `22b_concurrency_wiring.rs` 覆盖简单调用，但没有：

- 参数化入口函数和混合类型边界实参；
- 同名/遮蔽/重新赋值的作用域消费；
- `return`/`break`/`continue` 提前退出后的消费状态合流；
- `shared_read`/`shared_mut`/`lock_write` 的消费；
- shared 构造参数能力拒绝；
- `pool_create` 容量边界（若本批不检查，登记给 22C）。

**已完成**：补源级正/负例并断言稳定诊断码；覆盖参数化入口、消费作用域、控制流提前退出、
三种消费操作、shared 能力和 `source_runtime_id` 相关 IR 路径。

### F-7（中，已修复）`source_runtime_id` 是 22A 字段表之外的 IR 扩展

`IrCrossThreadFact` 新增了 `source_runtime_id`（`core/rust/crates/xiao-ir/src/model.rs`），
用于校验 `shared_mut` 同 Runtime；22A 冻结的 IR 字段只列了 `runtime_id`、`task_boundary`、
`thread_boundary`。

**已完成**：保留为可选、`serde(default)` 的 IR 字段，并在 22A-FIX/22B-FIX.1 契约中登记；
旧快照缺少该字段时还原为空。

### F-8（中，已修复）`shared_mut` 来源 Runtime 固定为 0

修复前 `escape.rs` 把 `shared_mut` 的来源 Runtime 写成 0，依赖隐含假设。

**已完成**：来源 Runtime 从 `BoundaryFacts` 的 lowering 上下文读取。`current_runtime` 使用与
显式 Runtime 值身份区间分离的非零静态身份标签，显式 Runtime 使用由值身份导出的非零稳定标签；
IR verifier 拒绝目标或来源 Runtime ID 为 0，符合 22A 对 0 无效值的冻结约束。

### F-9（中，已修复）文档/证据状态仍需回写

22B-FIX、22B、22 和 README 已回写为 FIX.1 已完成；本批远端 workspace gates 使用运行号
`40201575737`，不把旧远端失败运行当作本批通过证据。

## 三、目标与边界

### 3.1 目标

1. 先让 workspace gates 回绿；
2. 支持参数化入口函数和边界实参类型对应；
3. 让消费状态按作用域/绑定工作；
4. 覆盖 `shared_read`/`shared_mut`/`lock_write` 消费和 shared 构造能力；
5. 补源级回归，明确 `source_runtime_id` 契约；
6. 22C 才能按稳定事实进入 Runtime。

### 3.2 不负责

- 不实现 Runtime/VM/LLVM/标准库函数体；
- 不实现 32B 原子头、锁等待、真实取消、序列化字节或任务池执行；
- 不改 `.xiaoc`/`.xar` feature 位；
- 不把 `shared_mut` 来源 Runtime 变成运行时探测；
- 不进入 22C 的 ABI/平台工作。

## 四、需要裁定的选择

| 编号 | 问题 | 建议 | 备选 |
| --- | --- | --- | --- |
| D-1 | `spawn`/`submit` 如何支持带参函数及额外实参 | **甲：在调用检查中特殊处理第二个实参的真实函数类型，边界实参必须与函数形参数量严格相等并按位置检查；额外实参拒绝，不引入隐式闭包捕获** | 乙：扩展 `FunctionSignature` 支持函数型可变参数；丙：继续限定零参函数（与 22A 冲突） |
| D-2 | 消费状态的身份 | **甲：作用域/绑定身份，退出作用域清理，重新声明重置** | 乙：按名称但在函数/分支边界清理；丙：全局名称（当前，已知错误） |
| D-3 | shared 构造与锁写入消费 | **甲：`shared_read`/`shared_mut` 消费 T，`lock_write` 消费新值，`read`/`lock_read` 借用** | 乙：只做运行时检查，22C 再补 |
| D-4 | `source_runtime_id` | **甲：保留为可选、serde default 的 IR 字段并写入 22A/22B-FIX 契约** | 乙：移出 fact，使用独立 Runtime ID 表 |
| D-5 | `pool_create` 容量 1–256 | **甲：本批只登记为 22C 运行时检查** | 乙：22B 对字面量做静态检查 |

## 五、实施步骤

| 步 | 内容 | 验证 |
| --- | --- | --- |
| 0 | 给 `22b_concurrency_wiring.rs` 加模块文档 | 已完成；旧覆盖错误已消除 |
| 1 | 按 D-1 修 `spawn`/`submit` 入口函数和边界实参检查 | 已完成；参数化函数和混合类型实参通过 |
| 2 | 按 D-2 把消费状态改为作用域/绑定身份 | 已完成；同名跨函数、遮蔽重置、重新赋值有回归 |
| 3 | 按 D-3 接入 `shared_read`/`shared_mut`/`lock_write` 消费 | 已完成；复用源值被拒绝 |
| 4 | 补 shared 构造参数能力检查 | 已完成；容器负例报 030 |
| 5 | 按 D-4 记录 `source_runtime_id` 或迁移 | 已完成；可选字段与 lowering 上下文已登记 |
| 6 | 补源级回归并更新 22B-FIX/22/12/README | 已完成 |
| 7 | 推送并记录 CI；22C 门禁改为 FIX.1 完成 | 已完成；workspace gates `40201575737` 通过 |

## 六、验收

1. workspace gates `40201575737`、本地 Rust workspace、独立文档门禁和源码级回归均通过；
2. `spawn(rt, work, args...)` 对带参数函数和混合类型实参通过，错误类型被拒绝；
3. 同名跨函数/遮蔽/重新赋值的消费状态正确；
4. `shared_read`/`shared_mut`/`lock_write` 消费和 shared 构造能力负例通过；
5. `source_runtime_id` 契约有唯一落点；
6. 源码级测试覆盖 F-2～F-6，不只测模型 API；
7. 22B-FIX/22/12/README 状态一致，22C 入口明确为 FIX.1 完成；
8. 没有实现 Runtime/VM/LLVM/标准库函数体。

## 七点一、实施结果

1. 测试 crate 增加模块文档；`spawn`/`submit` 的第二个实参现在按真实函数类型检查，
   函数返回类型实例化 `thread<T>`/`task<T>`；边界实参数量必须与函数形参数量严格一致，
   额外实参拒绝，不表示隐式闭包捕获。
2. 消费状态按绑定身份追踪别名，并在类型检查器的词法作用域快照中隔离；函数参数、循环变量、
   catch 绑定和表方法参数会创建新的局部状态，重新赋值会建立新身份；分支、循环与 try 路径在
   控制流合流处传播消费，`return`、`break`、`continue` 按各自可达出口保留状态，循环会拒绝重复
   消费入口已有值，不可达语句不会污染后续路径。
3. `FunctionSignature` 保留 `consumed_parameters`；`shared_read`、`shared_mut` 和
   `lock_write` 在调用点消费命名源值，`read`、`lock`、`lock_read` 保持借用语义。
4. `shared_read`/`shared_mut` 对输入值执行候选 A Send/Sync 检查；数组、动态值、资源和
   其他不满足能力表的值被静态拒绝。
5. `source_runtime_id` 作为可选、旧快照兼容的 IR 字段登记；`shared_mut` 来源 Runtime
   从 lowering 上下文读取，不再在表达式 lowering 中写死来源值；当前 Runtime 使用与显式值身份区间
   分离的非零静态标签，IR verifier 拒绝无效的零 Runtime ID。
6. `cancel` 保持借用语义；取消后仍可再次 `cancel` 或执行 `join`/`wait`，消费操作才会
   标记句柄不可复用。

本地验证命令：

```text
cargo check --manifest-path core/rust/Cargo.toml --workspace
cargo test --manifest-path core/rust/Cargo.toml --workspace
cargo clippy --manifest-path core/rust/Cargo.toml --workspace --all-targets -- -D warnings
cargo fmt --manifest-path core/rust/Cargo.toml --all -- --check
bun run check:docs
bun run check:usedocs
bun run check:layout
git diff --check
```

## 七、不许

1. **不许**用 `allow(missing_docs)` 或类似豁免绕过 A0 覆盖；
2. **不许**用 `Dynamic` 或空参数 `Function` 假装支持参数化入口；
3. **不许**继续用全局名称保存消费状态；
4. **不许**让 `shared_read`/`shared_mut`/`lock_write` 的消费只存在于签名表；
5. **不许**把 `spawn`/`submit` 的事实改成名称启发式；
6. **不许**把 workspace gates 红写成通过；
7. **不许**在 FIX.1 关闭前进入 22C。

## 八、落点

```text
docs/DevDocs/22b-fix1-concurrency-wiring-residuals.md   本文
docs/DevDocs/22b-fix-concurrency-wiring.md              状态/证据修正
docs/DevDocs/22b-concurrency-types-lifetime-ir.md       22C 交接状态
docs/DevDocs/22-concurrency-execution-and-shared-handles.md  22C 门禁
docs/DevDocs/12-tests-and-milestones.md                 P0 的 FIX.1 前置
docs/DevDocs/README.md                                  索引与状态
core/rust/crates/xiao-driver/tests/22b_concurrency_wiring.rs  模块文档 + 源级回归
core/rust/crates/xiao-types/src/checker/imports.rs      入口函数/消费/能力
core/rust/crates/xiao-types/src/checker.rs              消费状态身份
core/rust/crates/xiao-types/src/checker/diagnostic.rs   name/绑定键
core/rust/crates/xiao-lifetime/src/escape.rs            边界事实 lowering
core/rust/crates/xiao-ir/src/model.rs                   source_runtime_id 契约
```

不修改 `xiao-runtime`、`xiao-vm`、`xiao-codegen-llvm`、`stdlib/` 的并发行为。

## 九、风险

1. **入口函数特殊处理**：如果绕过 `FunctionSignature` 的泛型统一，容易把普通函数调用规则
   复制成第二套；必须只限 `std.concurrent.spawn`/`submit`。
2. **消费身份改造**：绑定身份要跟现有作用域/遮蔽模型一致，不能引入新的名字解析规则。
3. **shared 构造语义**：`shared_read(T)` 消费 T 但 `read` 借用；两者不能混成同一状态。
4. **IR 字段扩展**：`source_runtime_id` 若保留，必须写进 22A/22B-FIX 的可选字段说明和旧快照
   兼容测试。
5. **文档状态**：FIX.1 未完成前不能把 22B/22B-FIX 写成已完成。

## 相关页面

- [22B-FIX. 并发静态契约接线与收口](22b-fix-concurrency-wiring.md)
- [22B. 并发类型、生命周期与 IR](22b-concurrency-types-lifetime-ir.md)
- [22A. 并发入口冻结](22a-concurrency-freeze.md)
- [22. 并发执行与共享句柄](22-concurrency-execution-and-shared-handles.md)
- [12. 测试与开发里程碑](12-tests-and-milestones.md)
