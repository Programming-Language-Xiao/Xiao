# 22C. 并发 Runtime ABI、VM 与错误

> **这是 22 阶段的第三批：把 22B 已冻结的静态事实落成 Runtime ABI、VM 执行和错误语义，
> 不接 LLVM，也不实现标准库函数体。**
> 22B/22B-FIX.1 已交付类型、生命周期事实、IR 边界动作、取消检查、`source_runtime_id`
> 和 verifier；本批只消费这些字段，不重新决定 Send/Sync、CrossThread、API 名称或序列化格式。
>
> 状态：**基础 Runtime/ABI 与产物要求校验已实施，VM 调用接线仍待后续子批（2026-10-26）**。
> 代码提交 `df6abc3` 已完成 ABI 1.17、32B 共享头、Runtime ID 注册表、XCON v1 编解码、
> Runtime/线程/任务池骨架和 `.xiaoc` required-feature/ABI 校验 helper；workspace gates
> `40257127901`、maintenance regression `40257090391` 均通过。VM 调用接线和完整端到端并发
> 执行仍按本文后续步骤推进。

## 一、入口条件与冻结输入

### 1.1 前置

1. [22. 并发执行与共享句柄](22-concurrency-execution-and-shared-handles.md)——阶段分解；
2. [22A. 并发入口冻结](22a-concurrency-freeze.md) §2.9——API、32B ABI、诊断、产物门禁；
3. [22B. 并发类型、生命周期与 IR](22b-concurrency-types-lifetime-ir.md)——静态字段；
4. [22B-FIX.1. 静态接线收口](22b-fix1-concurrency-wiring-residuals.md)——源码级回归和
   `source_runtime_id`/不可达事实约束；
5. [07-D. 并发模型研究](07d-concurrency-model-research.md) §10.4——取消/清理顺序；
6. [12. 测试与开发里程碑](12-tests-and-milestones.md) P0——阶段出口。

### 1.2 不得重新选择的内容

| 项 | 冻结内容 | 出处 |
| --- | --- | --- |
| Runtime ABI | 次版本 `1.17`；普通句柄布局不变 | [22A §2.9](22a-concurrency-freeze.md) |
| 共享头 | 32B/8B：strong/weak/tag/lock_state；载荷偏移 32B | 22A §2.6 |
| tag | `1=ordinary`、`2=shared_read`、`3=shared_mut` | 22A §2.5 |
| 锁状态 | 低 4B atomic u32；最高位等待者；其余保留 | 22A §2.2/§2.9 |
| 诊断 | `X03-TYPE-030..032`、`X06-LIFETIME-006..008`、`X06-RUNTIME-027..032` | 22A §2.9 |
| 值边界 | 逐参数深拷贝/版本化序列化；失败逆序清理；主/suppressed | 22A §2.2 |
| 产物 | `.xiaoc`/`.xar` ABI 1.17 + required-feature；原生并发产物 22E | 22A §2.9 |
| 静态字段 | `IrCrossThreadFact`/`IrBoundaryTransfer`/`IrCancellationCheck`、`source_runtime_id` | 22B §2.9 |

## 二、实施前核出的现状

| 面 | 当前事实 | 对 22C 的影响 |
| --- | --- | --- |
| Runtime ABI | `ABI_MINOR_VERSION = 17`（`core/rust/crates/xiao-runtime-abi/src/lib.rs:18`） | 已升到 1.17；产物 feature 门禁仍待接线 |
| 普通句柄 | `StrongHandle`/`WeakHandle` 使用 `PhantomData<Rc<()>>`，单线程非原子（`xiao-runtime/src/memory/mod.rs:408/437`） | 不能复用为共享句柄；保持布局不变 |
| 现有对象头 | `ObjectHeader` 含类型、计数、布局、策略、销毁状态和载荷（`:314`），不是 32B 草案 | 需要独立的共享对象头，不能把探针字段混进普通头 |
| 32B ABI | `xiao-runtime::concurrency::SharedHeader` 已实现并锁定 32B/8B 布局 | 已验证 weak upgrade、锁和恰好一次析构基础 |
| Runtime ID | 22B 静态事实使用非零静态标签；当前 Runtime 是 `u64::MAX` 哨兵（`xiao-lifetime/src/model.rs:22`） | `RuntimeRegistry` 已提供静态标签到实际 ID 的映射和关闭失效 |
| IR 事实 | `IrOwnership` 已有 facts/transfers/cancellation；`source_runtime_id` 可选 | 22C 直接消费，不能重新解释 |
| 序列化 | `xiao-runtime::serialization` 已实现 XCON v1 大端节点编码、限制、确定性排序和 029 | VM 边界接线及失败清理仍待完成 |
| VM | 普通单线程执行闭环已存在；并发 Runtime 骨架已提供 | 仍需要接 spawn/join/submit/shared/lock/cancel 调用 |
| 标准库 | `std.concurrent` 只有类型签名表 | 本批不写 `stdlib/concurrent.xiao`；22E 接用户模块 |
| 字节码 | 已有并发 IrType 标签 12–19，ABI 版本已升到 1.17 | `.xiaoc` 已提供 required-feature/ABI 要求校验 helper；VM 加载入口仍待接线 |

## 三、目标与边界

### 3.1 目标

1. Runtime ABI 1.17 和共享句柄头；
2. VM 线程/任务池/多 Runtime/共享句柄执行；
3. 深拷贝/序列化和取消等待；
4. 031/027–032 等运行时错误和 detached 错误通道；
5. `.xiaoc`/`.xar` 的 ABI/feature 门禁；
6. VM 端到端回归消费 22B 的 facts/transfers/cancellation。

### 3.2 不负责

- 不实现 LLVM 并发 lowering（22D）；
- 不实现 `std.concurrent` 标准库函数体或 UseDocs 发布（22E）；
- 不改 32B 头布局、普通句柄布局、API 名称或 Send/Sync 表；
- 不实现原生并发产物元数据（22E）；
- 不实现 32 位目标；不支持 Actor/async/官方并发容器；
- 不把 Runtime 调度细节写回 22B 的类型/能力表。

## 四、设计裁定点

### D-1（Runtime ID 映射）

22B 的 `runtime_id` 是静态 lowering 身份：当前 Runtime 用 `u64::MAX` 哨兵，显式 Runtime 用
值身份导出的非零标签；Runtime 实际 ID 必须由创建流程分配。22C 需要定义：

- Runtime 创建时分配进程内单调递增的实际 ID（从 1 开始，0 无效）；
- IR/VM 的静态标签到实际 ID 的映射表，进入 Runtime 时建立、关闭时销毁；
- `current_runtime` 的静态哨兵要映射到当前的实际 ID，不能把 `u64::MAX` 直接当实际 ID；
- 关闭 Runtime 后映射失效，旧句柄/任务/共享句柄按 031 拒绝。

**建议**：在 Runtime 注册表保存 `static_label -> actual_id`，不改变 22B IR 字段。

### D-2（32B 共享头实现）

- 自定义 `#[repr(C)]` 头：`strong: AtomicU64`、`weak: AtomicU64`、`tag: AtomicU64`、
  `lock_state: AtomicU32`、`reserved: u32`；载荷从 32B 开始；
- 共享句柄只引用这个头，不复用普通 `StrongHandle`/`WeakHandle`；
- weak upgrade 非零 CAS，最后强/弱释放各一次 Acquire/Release；
- lock state CAS + 平台等待；取消令牌不写头；
- 32B 布局用四平台 CI 测量或现有 `40093591873` 作为输入，但产品实现必须重新验证。

**建议**：自定义头 + `AtomicU64`/`AtomicU32`，不直接用 `Arc` 内部布局。

### D-3（线程/任务池/多 Runtime 后端）

- OS 线程使用 `std::thread`；固定容量任务池；Runtime 注册表；detach 错误通道；
- 线程/Runtime 关闭顺序：停止接纳 → drain/等待 → 释放边界值 → 汇总主/suppressed；
- pool worker 迁移符合 22B `submit` 的双边界事实；
- 不为每个 Runtime 启动不必要的线程；多 Runtime 并行必须可观测 ID。

### D-4（深拷贝/序列化）

- 实现 22A 的 `XCON` 12B 头、版本 1、大端、tag 0–12；
- 深度 64、节点 1,000,000、单值 16 MiB、整帧 64 MiB；
- 资源/dynamic 只做防御性 029，不作为可传值；
- 失败按 IR `IrBoundaryTransfer.cleanup_order` 逆序清理，主错误/`suppressed` 不覆盖；
- 22C 只做 VM/Runtime 编解码，不写标准库公开 API。

### D-5（取消/错误/产物门禁）

- 真实持锁等待/唤醒；锁等待、join、wait 统一 027；detached 失败 028；
- Runtime 关闭 031；池自身关闭 030；池创建参数 032；
- `.xiaoc`/`.xar` ABI 1.17 + required-feature；旧 Runtime 在执行前拒绝；
- `.xiaoc` 类型标签 12–19 必须受 feature/ABI 门禁保护；
- native 并发产物 22E。

### D-6（VM 入口形态）

- VM 消费 `IrCrossThreadFact`/`IrBoundaryTransfer`/`IrCancellationCheck`，不重新解析源码名称；
- `std.concurrent` 的签名-only 表在 22B 已存在；22C 只实现 Runtime/VM 调用，不写 stdlib；
- 若需要 opcode，只添加最小调用/边界动作，不把调度器塞进字节码。

## 五、实施步骤

| 步 | 内容 | 验证 |
| --- | --- | --- |
| 0 | 基线：22A/22B 契约、IR 字段、ABI 1.17、VM 调用路径 | 已完成 |
| 1 | Runtime ABI 1.17：共享头/句柄/状态码/符号；普通句柄不变 | 已完成 ABI 单测/兼容矩阵基础 |
| 2 | 共享头：原子计数、tag、弱升级、锁状态、恰好一次析构 | 已完成共享头基础测试 |
| 3 | Runtime/线程/任务池/多 Runtime、Runtime ID 映射 | 已完成 Runtime/线程/池骨架和映射测试 |
| 4 | 深拷贝/序列化：XCON、限制、失败清理、suppressed | 已完成 XCON 编解码/限制基础；边界清理待 VM 接线 |
| 5 | VM：spawn/join/detach/cancel、shared/read/lock、pool submit/wait/shutdown | 源码级端到端 |
| 6 | 取消：锁等待/join/wait、四位置、唤醒/清理 | 取消/清理回归 |
| 7 | 产物：`.xiaoc`/`.xar` ABI 1.17 + feature；旧 Runtime 拒绝 | `.xiaoc` helper 已完成；`.xar`/VM 执行入口负例待接线 |
| 8 | 文档/门禁：22C 状态、UseDocs 入口保持 planned、CI | docs/usedocs/layout |

## 六、测试与验收

### 6.1 Runtime/ABI

- 32B 头尺寸/对齐/载荷偏移；tag/锁状态；
- 强弱计数并发、弱升级失败/成功、最后释放恰好一次；
- 锁等待/取消/唤醒，无死锁；
- 普通 `StrongHandle`/`WeakHandle` 仍为单线程边界。

### 6.2 VM 端到端

- `spawn`/`join`、`submit`/`wait`、`detach` 失败通道；
- 多 Runtime 并行、Runtime 关闭、池关闭；
- `shared_read`/`shared_mut`/`read`/`lock`/`lock_read`/`lock_write`；
- 多边界值/深拷贝/序列化失败清理；
- 取消四位置、主/suppressed 顺序、finally/drop 只执行一次；
- 资源/dynamic 编译期拒绝；恶意 tag 13/14 运行时防御性 029。

### 6.3 产物

- ABI 1.17 + feature 位在 `.xiaoc`/`.xar` 执行前检查；
- 旧 Runtime/旧归档拒绝稳定；
- 字节码类型标签 12–19 不会绕过门禁。

### 6.4 门禁

```bash
cargo test --manifest-path core/rust/Cargo.toml --workspace
cargo clippy --manifest-path core/rust/Cargo.toml --workspace --all-targets -- -D warnings
cargo fmt --manifest-path core/rust/Cargo.toml --all -- --check
bun run check:docs
bun run check:usedocs
bun run check:layout
git diff --check
```

## 七、22C 退出条件

1. ABI 1.17 与 32B 共享头实现，普通句柄布局未变；基础实现和 workspace/maintenance 证据已完成，需继续补 VM 证据；
2. VM 端到端覆盖线程/任务池/多 Runtime/共享/取消/序列化；
3. Runtime ID 静态标签到实际 ID 的映射唯一且有测试；
4. 错误 027–032 和 detached 通道按 22A 语义；
5. `.xiaoc`/`.xar` 门禁和旧 Runtime 拒绝有证据；
6. 无 LLVM/标准库函数体/原生并发产物；
7. 22D 可以只按本批 Runtime 行为做 VM/LLVM 差分。

## 八、不许

1. **不许**改 32B 布局、普通句柄布局、Send/Sync 表或 API 名称；
2. **不许**把 22B 静态 facts 当作实际 Runtime ID；
3. **不许**把 `.xiaoc`/`.xar` 的 ABI 1.17 门禁推迟到发布阶段；
4. **不许**在 22C 实现 LLVM 并发 lowering 或标准库函数体；
5. **不许**用名称/源码重扫描替代 IR facts/transfers；
6. **不许**把平台测试或单次测量当成 ABI 验证；
7. **不许**在 22C 关闭前把 22D/22E 的差分或发布结果写成已验收。

## 九、落点

```text
docs/DevDocs/22c-concurrency-runtime-abi-vm.md          本文
docs/DevDocs/22-concurrency-execution-and-shared-handles.md  22C 状态
docs/DevDocs/12-tests-and-milestones.md                 P0 的 22C 入口
docs/DevDocs/README.md                                  22C 索引
core/rust/crates/xiao-runtime-abi/                      ABI 1.17/共享头声明
core/rust/crates/xiao-runtime/                          Runtime/线程/池/共享句柄/序列化
core/rust/crates/xiao-vm/                               VM 调用与取消/错误
core/rust/crates/xiao-bytecode/                         类型标签/产物门禁接线
core/rust/crates/xiao-driver/                           编译/运行入口
core/rust/crates/xiao-lifetime/                         仅消费静态事实，不重定义
core/rust/crates/xiao-ir/                               仅消费字段，不改语义
```

不修改 `xiao-codegen-llvm` 的并发行为；LLVM 留给 22D；`stdlib/concurrent.xiao` 留给 22E。

## 十、风险

1. **Runtime ID 双身份**：静态标签与实际 ID 混淆会让跨 Runtime 检查和错误码错位。
2. **共享头布局**：把普通 `StrongHandle` 或 `Arc` 尺寸直接当 32B 头会破坏 ABI。
3. **取消唤醒**：只标记不等待会违反 22A 的首版承诺；锁/任务/线程等待必须统一。
4. **序列化确定性**：容器排序、长度上限和失败清理顺序必须按 IR/22A 契约。
5. **门禁遗漏**：`.xiaoc` 类型标签 12–19 若没有 ABI/feature 门禁，旧 Runtime 会静默或误拒。
6. **范围蔓延**：VM 实现容易顺手接 LLVM 或标准库；LLVM 22D、stdlib 22E。

## 相关页面

- [22B. 并发类型、生命周期与 IR](22b-concurrency-types-lifetime-ir.md)
- [22A. 并发入口冻结](22a-concurrency-freeze.md)
- [22A-FIX. 契约冲突与收口](22a-fix-concurrency-freeze.md)
- [07-D. 并发模型研究](07d-concurrency-model-research.md)
- [12. 测试与开发里程碑](12-tests-and-milestones.md)
