# 22C-FIX.1. 并发 VM 调用接线与端到端执行

> **这是 22C 的收口补丁批，不是 22D。**
> 22C 已交付 ABI 1.17、32B 共享头、Runtime/线程/任务池骨架、XCON 基础、
> Runtime 关闭协调和 `.xiaoc`/`.xar` feature 门禁。本批已把 22B facts/transfers/
> cancellation 接入 TAC 并发计划、隔离 VM 和 Runtime 绑定，并完成源码到 `.xar` 的端到端回归。
>
> 状态：**FIX.1 已完成（2026-10-28）**，26 条源码/产物端到端回归、workspace、benchmark、
> research concurrency、Bun、coverage 及安全/产物回归均通过；[workspace gates](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/40613747372)
> 通过。 [maintenance regression](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/40613821972)
> 中，Linux、Windows、macOS 各自运行 `22c_vm_concurrency`，每个平台 26/26 通过，全部 jobs 通过。
> 本批只接 VM、驱动和已有 Runtime API；
> 不接 LLVM，不写 `stdlib/concurrent.xiao` 函数体，不改 22A/22B 的 API、Send/Sync 表、
> 32B 布局、诊断码或产物 feature 位。

## 一、入口条件与冻结输入

### 1.1 前置

1. [22A. 并发入口冻结](22a-concurrency-freeze.md) §2.2/§2.6/§2.9：语言表面、32B ABI、
   错误和产物矩阵；
2. [22A-FIX. 契约冲突收口](22a-fix-concurrency-freeze.md)：关闭语义、pool/Runtime 入口、
   shared 句柄和产物错误；
3. [22B. 并发类型、生命周期与 IR](22b-concurrency-types-lifetime-ir.md)：类型、能力表、
   facts/transfers/cancellation、verifier；
4. [22B-FIX.1. 静态接线收口](22b-fix1-concurrency-wiring-residuals.md)：源码级静态事实
   和错误出口；
5. [22C. Runtime ABI、VM 与错误](22c-concurrency-runtime-abi-vm.md)：Runtime API、共享头、
   XCON、关闭协调和产物门禁；
6. [07-D. 并发模型研究](07d-concurrency-model-research.md) §10.4：取消/清理顺序。

### 1.2 本批不得重新选择的内容

| 项 | 冻结内容 |
| --- | --- |
| API | `std.concurrent` 的 17 个签名与消费规则 |
| 类型/能力 | `Send`/`Sync` 表、`Runtime`/`Thread`/`Task`/`Pool`/`Shared`/`SharedMut`/`LockGuard` |
| 事实 | `IrCrossThreadFact`、`IrBoundaryTransfer`、`IrCancellationCheck`、`source_runtime_id` |
| ABI | Runtime ABI 1.17；32B 共享头；普通 `StrongHandle`/`WeakHandle` 单线程 |
| 错误 | 027 取消、028 detached 失败、029 序列化、030 池关闭、031 Runtime 关闭、032 池参数 |
| 产物 | `.xiaoc`/`.xar` ABI 1.17 + required-feature；原生并发产物 22E |
| 取消 | 真实锁等待/唤醒、四位置、主/suppressed 顺序、finally/drop 恰好一次 |

## 二、实施前现状

| 面 | 当前事实 | 影响 |
| --- | --- | --- |
| VM 值 | `RuntimeValue::Concurrency` 包装 Runtime/Thread/Task/Pool/Shared/SharedMut/LockGuard；消费槽共享别名状态 | 句柄可在寄存器和调用帧中传递，普通容器键仍拒绝 |
| VM 调用 | TAC `CallConcurrency` 按计划 ID 执行 Runtime 绑定；不从源码成员名重新推断操作 | `std.concurrent` 入口走原生绑定，普通动态调用仍走既有路径 |
| 标准库 | `std.concurrent` 保持签名表；VM 对保留模块提供内建成员入口 | 不写 `stdlib/concurrent.xiao` 函数体，导入初始化不执行虚假模块 |
| TAC | `TacConcurrencyPlan` 保存 operation、类型、Runtime 标签、facts、transfer、取消检查；格式 4 追加计划分区，旧格式 3 继续解码 | verifier 同时检查调用引用、类型、逆序清理和 IR facts/cancellation |
| Runtime | 子 VM 用 XCON `VmTransferValue` 传递候选 A 值；共享句柄保留来源 Runtime ID；已有 Runtime 绑定提供 027–032 | 不跨线程搬运 VM 的 `Rc`/`Cell` 状态 |
| VM 线程模型 | 每个 spawn/pool 执行单元创建独立 VM、模块快照和事件汇聚槽 | 任务结果、panic、取消和 suppressed 错误可回到父 VM |
| 驱动 | `xiao-driver` 真实源码、`.xiaoc`、`.xar` 入口均复用并发计划和 feature preflight | `xiao run`/归档入口可执行并发黄金场景 |
| 测试 | 新增 `22c_vm_concurrency.rs` 覆盖 26 条源码/产物/错误回归 | 22C 退出条件 2、3、4、5、6、7 有直接证据 |

## 三、目标与边界

### 3.1 目标

1. 将 22B 的 `ConcurrencyCall`、`IrCrossThreadFact`、`IrBoundaryTransfer` 和
   `IrCancellationCheck` 转换为 VM 可消费的计划，不重新扫描源码名称；
2. 在 VM 值层表示 Runtime/Thread/Task/Pool/Shared/SharedMut/LockGuard；
3. 完成 `spawn/join/detach/cancel`、`pool_create/submit/wait/shutdown`、
   `shared_read/shared_mut/read/lock/lock_read/lock_write` 和 `runtime_create/current_runtime/runtime_close`；
4. 完成边界深拷贝/序列化、逆序清理、主/suppressed 错误；
5. 完成 VM 取消安全点、join/wait/lock 等待中断和 detached 错误通道；
6. 完成 Runtime ID 静态标签到实际 ID 的映射；
7. 用真实源码经 TAC/VM 跑通并发黄金场景，并保留 `.xiaoc`/`.xar` 门禁。

### 3.2 不负责

- 不实现 LLVM 并发 lowering 或 VM/LLVM 差分（22D）；
- 不实现 `stdlib/concurrent.xiao` 的官方函数体或 UseDocs 发布（22E）；
- 不新增/重命名 22A API、Send/Sync 能力或诊断码；
- 不改变 32B 共享头、普通句柄布局或 XCON 版本；
- 不把平台测试、单次测量或原型结构写成产品 ABI。

## 四、设计裁定点

### D-1（并发调用面：原生绑定 + TAC 计划）

**建议取甲**：在 TAC 增加只读的 **并发计划表**，由 22B facts/transfers/cancellation 生成；
VM 通过计划 ID 执行原生并发绑定，不按源码函数名重新查找。

- 计划记录 operation、target runtime ID、边界参数寄存器、transfer/cancellation 索引和 span；
- `std.concurrent` 函数仍由前端签名表识别，但 VM 只消费降低后的计划；
- 不把调度器塞进字节码；计划表不改变普通 TAC 指令语义；
- `.xiaoc` 需要新增可选分区/元数据并升格式或复用既有扩展；旧产物无计划时按无并发处理，
  由 ABI/feature 门禁拒绝旧 Runtime；
- 计划表在 verifier 中检查：operation 集合、ID 非零、参数数量与类型、runtime 引用。

备选：乙，为每个并发入口增加独立 TAC opcode；丙，让 VM 解析 `std.concurrent` 名称。
乙会扩大 opcode/编码矩阵；丙违反“消费 IR facts、不重解析名称”，不取。

### D-2（VM 值与句柄）

在 `RuntimeValue` 增加不透明并发句柄：

```text
Runtime(runtime_id)
Thread(thread_handle)
Task(task_handle)
Pool(pool_id)
Shared(shared_handle)
SharedMut(shared_mut_handle)
LockGuard(lock_guard_handle)
```

- 句柄只包装 Runtime 层对象，不把 Rust `Arc` 布局暴露给 TAC/LLVM；
- Runtime/Thread/Task/Pool 按 22B 规则消费；LockGuard 只在当前作用域；
- `Debug`/相等/哈希不把句柄内容当值；句柄不能进入集合或字典键；
- 关闭后继续使用由 VM 转成 031/030，不让底层 panic。

### D-3（执行单元：每线程 VM）

- `spawn` 和任务池 worker 使用新 OS 线程；每个执行单元建立**新的 VM 实例**，共享只读
  `TacProgram`（`Arc`/只读克隆）但不共享 `Rc`/`Cell` 状态；
- 任务入口携带函数 ID（或可执行函数计划）和已深拷贝/序列化的边界值；
- 子 VM 的返回值按边界序列化回父 VM；`join/wait` 只接收合法结果；
- 最后一个 VM `task` 别名未 `wait` 时，将结果接收器交给 detached 回收通道；失败保留 027/028 身份，普通子 VM 错误作为 028 的 cause；
- `detach` 不表示 fire-and-forget：VM 结束和 Runtime drain 仍等待 detached 活动；外部截止时间/取消先收束子 VM，再完成 drain；
- VM 退出清理自身创建的 Runtime 时按直接 owner 记录逐个回到 owner 上下文执行 `try_close()`，避免恢复外层上下文后把合法关闭误判为 031；此关闭不递归收束更深层的 Runtime；
- 模块初始化状态不在线程间隐式共享；需要共享只读模块状态时通过深拷贝/序列化，不把
  `VmSession` 的 `Rc` 状态跨线程；
- pool worker 迁移使用 22B 的 task/thread 双边界事实；Runtime ID 在每个子 VM 中保持；
- 父 VM 退出前必须 join 或记录 detached 失败；Runtime close 的 drain 语义按 22C。

### D-4（深拷贝与 XCON）

- 边界值只允许 22A/22B 的候选 A 子集：`none`、固定宽度数值、`bool`、`str`、递归安全
  `tuple`；动态值、resource、可变容器和普通句柄在编译期拒绝；
- VM 将边界 `RuntimeValue` 转成规范 `XconValue`；编码顺序、深度/节点/大小限制直接复用
  `xiao-runtime::serialization`；
- 序列化/复制失败使用 029；参数按 IR `cleanup_order` 逆序清理；主错误不被清理错误覆盖，
  清理错误进入 `suppressed`；
- 子执行单元启动前全部参数必须成功；失败不启动线程/任务；
- 结果返回同样受边界序列化限制；不可序列化结果不得被包装成不透明值跨边界。

### D-5（共享句柄与锁守卫）

- `shared_read`/`shared_mut` 创建共享句柄；`read` 返回深拷贝；`lock` 获取可变守卫，
  `lock_read`/`lock_write` 只通过守卫访问；
- 只读 shared 可跨 Runtime；shared_mut 只在同一 Runtime；
- 锁等待使用 `LockCancellation`，取消返回 027；守卫离开 VM 作用域或 finally/drop 时释放，
  释放只执行一次；
- VM 的 release plan 必须知道隐藏的 guard/句柄资源，不能在普通 `drop` 与计划释放之间
  重复释放；
- 32B 头、tag、weak upgrade 和对象析构不在 VM 重写。

### D-6（取消、错误与清理）

- VM 外部取消（`Fault::Cancelled`）与并发取消 027 分开：外部取消走 VM 取消通道，
  并发 join/wait/lock/cancel 走 `X06-RUNTIME-027`；
- 027 是不可被用户 `catch` 吞掉的并发取消出口；未开始任务取消会从池队列移除消息、释放队列容量并让 `wait` 立即返回 027；
- 22B cancellation checks（`safe_point`/`wait`/`lock_wait`）必须由 VM 消费，不能只依赖
  Runtime 层标记；
- `join`/`wait`/`lock` 等待必须可取消；取消后按 22C 顺序清理，finally/drop 恰好一次；
- 线程/任务 panic、detached 失败进入 028；主动取消进入 027；正常失败按原错误重抛；
- Runtime close 031、池关闭 030、池参数 032；错误码身份不得在 VM 重新发明。

### D-7（Runtime ID 与实际映射）

- 22B 静态 `runtime_id` 继续用于 facts/transfers 的同一性；
- `runtime_create` 分配实际 ID；VM 在每次调用处通过 TAC 值寄存器读取实际 Runtime 句柄，并核对静态标签，不维护可能过期的独立映射表；
- `current_runtime` 的静态哨兵映射到当前实际 Runtime，不把 `u64::MAX` 当实际 ID；
- 关闭 Runtime 时映射失效；旧句柄、任务、共享可变句柄按 031 拒绝；
- Runtime ID 不进入 32B 头；只读 shared 包装保留源 Runtime ID。

### D-8（产物与 VM 入口）

- `.xiaoc` 和 `.xar` 的 ABI 1.17/required-feature 门禁已在解码/执行前执行；
- VM 加载时必须把实际 Runtime feature 集传给门禁；缺 feature 不执行用户入口；
- 并发计划表随 `.xiaoc` 保存并有 verifier；旧产物缺计划时不伪造 facts；
- 不接 LLVM；`xiao build` 的并发 lowering 仍留给 22D/22E。

## 四A、22C 审核遗留并入

这些是 22C 关闭前后审核留下的边界项，必须在本批计划里显式承接，不能等 22D 再发现：

| 项 | 处理 |
| --- | --- |
| detached 队列只有深度（`detached_reaper_queue_depth`），没有上限/背压/丢弃策略 | 本批先保证 VM 不把无限增长隐藏成静默行为；Runtime 侧登记阈值和策略，最迟在 22E 前落地；黄金场景加入持续 detach 观测 |
| 入口线程或无执行上下文的普通 `submit` 没有提交前主动取消入口 | VM 任务入口必须继承执行上下文取消令牌；入口线程使用显式提交期取消入口或由 Runtime/pool 关闭终止等待 |
| `RuntimeHandle::close` 跨上下文是 no-op；`try_close()` 才返回 031 | VM 内部统一使用 `try_close()`；回归同时检查旧 `close()` 不会把跨上下文拒绝伪装成成功 |
| Runtime owner 只按直接创建关系记录，父 Runtime 关闭不递归关闭子 Runtime；VM 退出时必须用记录的 owner 上下文关闭本 VM 创建的 Runtime | VM 的 Runtime ID 映射和生命周期测试必须覆盖 owner 间接关系与退出清理；不得把父/祖 Runtime 的权限自动扩展 |
| 22C 的部分 Runtime 关闭测试是 Rust API 级，尚未经过 TAC/VM | 本批必须把这些行为重放到真实源码和 VM 执行路径，不能把 Rust 单测当作 VM 证据 |
| 22C `close`/`spawn` 的跨 Runtime 语义已写入 22A/22A-FIX，但 VM 未接线 | VM 入口测试必须覆盖 031；语言 `runtime_close` 与 Rust `try_close` 的边界不能混淆 |

## 五、实施步骤

| 步 | 内容 | 验证 |
| --- | --- | --- |
| 0 | 基线盘点：VM 调用、RuntimeValue、TAC 计划、driver 入口、现有 22C 门禁 | 文档/现有测试 |
| 1 | 增加并发计划表与 verifier；从 22B facts/transfers/cancellation 生成；不按源码名解析 | TAC 计划快照/负例 |
| 2 | 增加 RuntimeValue 并发句柄、Runtime ID 映射和错误转换 | 值/错误单测 |
| 3 | 完成 Runtime/线程/task/pool 原生绑定和每线程 VM 执行 | 最小 `spawn/join`、`submit/wait` |
| 4 | 边界深拷贝/XCON 编解码、逆序清理、主/suppressed | 序列化/失败顺序回归 |
| 5 | shared/lock/guard 与作用域释放 | 共享/锁/跨 Runtime 负例 |
| 6 | 取消安全点、可取消等待、detached 失败和 027/028 | 取消四位置/超时回归 |
| 7 | 多 Runtime、Runtime close、pool close、detached drain；detached 队列阈值/策略 | 关闭/泄漏/退出码/积压观测 |
| 8 | 源码级 `xiao run` 端到端和 `.xar` 入口 | 真实源码黄金场景 |
| 9 | 文档/UseDocs 保持 planned、门禁和 CI | docs/usedocs/layout/workspace |

## 六、测试与验收

### 6.1 最小黄金场景

```text
runtime_create/current_runtime
spawn(runtime, fn, args...) -> join
spawn(...).detach
pool_create/submit/wait/shutdown
shared_read/read
shared_mut/lock/lock_read/lock_write
多 Runtime 并行
```

每个场景同时覆盖：

- 成功结果；
- 边界参数/返回值；
- 线程 panic、detached 失败、取消；
- Runtime/pool 关闭和错误码；
- finally/drop 只一次；
- 主/suppressed 顺序。

### 6.2 反向场景

- 普通句柄跨线程、动态值/resource/可变容器跨边界：编译期拒绝；
- 跨 Runtime shared_mut：编译期拒绝；
- 旧 ABI/缺 feature 的 `.xiaoc`/`.xar`：执行前拒绝；
- 静态 Runtime ID 与实际 ID 混用：verifier/VM 拒绝；
- 恶意 XCON tag 13/14：029；
- 线程/任务取消和超时：027，不挂起。

### 6.3 门禁

```bash
cargo test --manifest-path core/rust/Cargo.toml --workspace
cargo clippy --manifest-path core/rust/Cargo.toml --workspace --all-targets -- -D warnings
cargo fmt --manifest-path core/rust/Cargo.toml --all -- --check
bun run check:docs
bun run check:usedocs
bun run check:layout
git diff --check
```

## 七、退出条件

1. 22C 退出条件 2、4、7 有直接证据：VM 执行、错误/清理、22D 可消费的稳定行为；
2. 真实源码经 `xiao run` 覆盖 spawn/join、pool、shared、多 Runtime、detach 和取消；
3. 深拷贝/序列化成功、失败和 suppressed 顺序有回归；
4. VM 消费 22B facts/transfers/cancellation，没有按源码名称重新解析；
5. Runtime ID 静态标签到实际 ID 的映射唯一且有跨 Runtime 负例；
6. `.xiaoc`/`.xar` ABI/feature 门禁仍在执行前生效；
7. 没有接 LLVM、没有 stdlib 函数体、没有原生并发产物；
8. workspace/maintenance/CI 证据齐全。

## 八、不许

1. **不许**把 VM 并发做成名称匹配或重新扫描源码；
2. **不许**用静态 `runtime_id` 代替实际 Runtime ID；
3. **不许**把 `Rc`/`Cell` VM 状态跨线程移动；
4. **不许**改写 32B 布局、Send/Sync 表、API 名称或诊断码；
5. **不许**在 22C-FIX.1 接 LLVM 或写 `stdlib/concurrent.xiao`；
6. **不许**跳过边界序列化/清理，把普通值当跨线程共享；
7. **不许**把 22D/22E 的差分或发布结果写成已验收。

## 九、落点

```text
docs/DevDocs/22c-fix1-concurrency-vm-wiring.md          本文
docs/DevDocs/22c-concurrency-runtime-abi-vm.md         22C 状态/退出
docs/DevDocs/22-concurrency-execution-and-shared-handles.md  22C-FIX.1 入口
docs/DevDocs/12-tests-and-milestones.md                  P0 的 22C-FIX.1
docs/DevDocs/README.md                                  索引
core/rust/crates/xiao-bytecode/                         TAC 并发计划/编码/verifier
core/rust/crates/xiao-vm/                               RuntimeValue/调用/线程执行/错误
core/rust/crates/xiao-driver/                           xiao run/xar 入口
core/rust/crates/xiao-runtime/                          Runtime/thread/pool/shared/serialization
core/rust/crates/xiao-types/                            仅消费已冻结签名
core/rust/crates/xiao-ir/                               仅消费已冻结事实
```

`xiao-codegen-llvm` 和 `stdlib/` 不在本批落点。

## 十、风险

1. **线程化 VM 状态**：`Rc`/`Cell` 不能跨线程，必须每执行单元新建 VM，不能共享帧或模块状态。
2. **TAC 计划兼容**：新增计划表会改变 `.xiaoc` 载荷；必须做 ABI/feature 门禁和旧产物负例。
3. **双 Runtime ID**：静态标签与实际 ID 混用会造成跨 Runtime 错码。
4. **边界值语义**：深拷贝、XCON、返回值和失败清理必须使用同一套值子集。
5. **取消分层**：VM 外部取消与并发 027 不能混用；等待必须可中断。
6. **范围蔓延**：容易顺手接 LLVM 或标准库；本批只做 VM/驱动和已有 Runtime API。
7. **测试替身**：原型/模拟调度不能当作源码级 VM 证据；必须是真实 `xiao run` 场景。

## 相关页面

- [22C. Runtime ABI、VM 与错误](22c-concurrency-runtime-abi-vm.md)
- [22B. 并发类型、生命周期与 IR](22b-concurrency-types-lifetime-ir.md)
- [22A. 并发入口冻结](22a-concurrency-freeze.md)
- [12. 测试与开发里程碑](12-tests-and-milestones.md)
