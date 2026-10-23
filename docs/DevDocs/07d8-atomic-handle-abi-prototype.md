# 07-D.8. 原子共享句柄对象头/ABI 原型

> **这是隔离 ABI 原型批次，不是并发实现阶段。** 07-D.7 已把原型门禁修绿，但当前
> 07-D.6 的“原子共享句柄”只是 `Arc`/`Weak`/`Mutex` 参考模型；`00-decisions.md` 第 6 项
> 要求对象头、锁、取消、清理和跨平台 ABI 在同一轮原型中验证，这一步还没有完成。
>
> **本批一句话**：**用一个自定义对象头把强/弱计数、升级、锁、取消、析构和跨平台尺寸
> 真正测出来；仍然不写产品 Runtime、语言语法、IR 或标准库公开契约。**
>
> 状态：**本地 ABI 原型、Linux workspace gates `39963374595` 与四平台复现 `39965322134` 已完成；Linux/macOS 测量待补（2026-10-23）**。只改 `tests/research/concurrency` 隔离原型、门禁和
> 文档；不登记新阶段代号，不预留 `H0-C`、`07-D 实现`、`std.concurrent` 或任何语法/API。

## 一、交接上下文

### 1.1 必读

1. [00. 决策基线](00-decisions.md) §07-D 并发模型裁定（2026-10-22），特别是第 6 项
   “原子共享句柄首阶段实现，范围只读 + 共享可变”；
2. [07-D.6. 最小 OS 线程原型](07d6-minimal-os-thread-prototype.md) §7——当前参考模型
   和限制；
3. [07-D.7. 原型门禁与证据收口](07d7-prototype-gate-and-evidence-fixes.md)
   ——layout、CI 接线和测量标签修正；
4. [07-D. 并发模型研究](07d-concurrency-model-research.md) §10.5——原子共享句柄 ABI
   前置清单；
5. [21A. 单线程 RC、强环与并发模型决策交接](21a-rc-cycles-and-concurrency-handoff.md)
   §3.2/§五——新阶段前置；
6. [12. 测试与开发里程碑](12-tests-and-milestones.md)——测试分层和阶段出口。

### 1.2 当前状态（2026-10-23 实跑）

| 环节 | 现状 | 结论 |
| --- | --- | --- |
| 取证提交 | `a8aaa4b` 已推送；CI/平台证据已回填 | ABI 原型和门禁修复已取证 |
| 本地门禁 | `bun run check`、`check:layout`、原型 test/Clippy/fmt 通过 | 本地证据成立 |
| 原型测试 | 9 passed，包含自定义对象头/句柄测试 | 参考模型和 ABI 原型可运行 |
| 测量 | Windows x86_64；干净头 24 bytes、干净 `u64` 对象 32 bytes/偏移 24 bytes；含探针对象 48 bytes/偏移 40 bytes；align32 干净对象 64 bytes/偏移 32 bytes | 自定义原型布局，不是产品 ABI |
| CI | `39963374595` 绑定 `a8aaa4b` 的 workspace gates success | 新代码 CI 已取证 |
| 决策第 6 项 | 对象头/锁/取消/清理已由自定义原型和四平台复现验证 | Linux/macOS 测量待补，不冻结产品 ABI |
| 跨平台测量 | 四平台测试已通过；测量仅 Windows x86_64 | Linux/macOS 待测；见 F-3 |

### 1.3 本批边界

| 项 | 本批 | 说明 |
| --- | --- | --- |
| 自定义原子对象头 | **原型** | 强/弱计数、tag、布局、对齐、恰好一次析构 |
| 只读共享 | **原型** | 原子读取、弱升级竞态、对象头回收 |
| 共享可变 | **原型** | 显式锁、取消等待、异常/毒化、清理顺序 |
| 普通/共享句柄区分 | **原型** | tag/ABI 身份，禁止隐式升级 |
| 跨平台测量 | **做** | Linux amd64/arm64、macOS arm64 的尺寸和操作成本 |
| CI 取证 | **做** | 推送 07-D.7 修复，确认原型测试进入 workspace gates |
| 产品代码/语法/IR/ABI | **不做** | 不进入 `xiao-runtime` 或语言契约 |
| 任务池/官方库/新阶段 | **不做** | 原型结果出来后再决定 |

## 二、审核发现

### F-1 **决策第 6 项还没有被完整验证（已补本地 ABI 原型，CI/跨平台待证）**

`00-decisions.md` 第 6 项要求：

> 对象头、锁、取消、清理顺序和跨平台 ABI 必须在同一轮原型中一起验证。

当前 `tests/research/concurrency` 的原子句柄是：

- `ReadHandle`/`SharedMutableHandle` 内部都是 `Arc`（8 字节包装器）；
- `ReadState`/`MutableState` 是 `payload + drops: Arc<AtomicUsize>`；
- `read_state_layout_bytes=16` 是载荷 + 计数指针，不是对象头；
- 没有自定义强/弱计数、tag、内存顺序或普通/共享句柄 ABI 身份。

文档已正确标注“不是 Xiao 对象头/ABI”，但这意味着决策第 6 项只完成了参考模型验证。
**处理结果**：已补自定义 `#[repr(C)]` 对象头、强/弱计数、tag、弱升级、对象头回收、显式
锁和 acquire/release 发布路径；本地 9 个测试通过。跨平台和 CI 证据仍待补齐，决策第 6 项
暂保持“本地 ABI 原型已验证、正式 ABI 未冻结”。

### F-2 **07-D.7 已有 Linux CI；本批新代码 CI 已补（P2，已关闭）**

07-D.7 已把 `tests/research/**` 加入 workspace gates paths，并在 `run.sh` 中执行原型
fmt/test/Clippy；`39963374595` 已在绑定 `a8aaa4b` 的提交上确认 9 个原型测试通过。
推送后必须确认：

- workspace gates 运行了 `tests/research/concurrency`；
- 原型测试/Clippy/fmt 在 CI 通过；
- 没有把本地 `bun run check` 写成 CI 结果。

**当前结果**：workspace gates 接线已由 07-D.7 完成；`39963374595` 已覆盖本批提交并通过。

### F-3 **跨平台测量仍缺（P2，待补）**

当前测量只有 Windows x86_64。对象头的原子宽度、对齐、`size_of`、锁成本和弱升级竞态
在不同平台可能不同；不能用单平台结果冻结 ABI。需要在 Linux amd64/arm64、macOS arm64
用同一命令和重复次数补测，并记录环境。

### F-4 **取消等待测试的固定 sleep 可能带来门禁抖动（已修复）**

`atomic_mutable_demo` 原先在启动等待线程后固定 `sleep(2ms)` 再设置取消标记。该测试现在
使用 Barrier 确认等待线程已进入轮询，再设置取消标记；仍保留超时轮询作为兜底，不依赖固定
睡眠时长。

### F-5 **文档状态仍是“CI 待取证”（P3）**

07-D.6/07-D.7 的“本地通过、CI 待取证”是准确的；本批取得 CI 运行号和新 ABI 原型结果后
再更新最终状态。不要把本地 ABI 原型写成产品 ABI 已完成。

## 三、原型范围

### 3.1 自定义对象头

建议在隔离 crate 内新增一个 `#[repr(C)]` 的实验对象头，至少包含：

1. 强计数（原子整数）与弱计数（原子整数）；
2. 普通句柄/共享句柄的身份 tag（或等价的可区分表示）；
3. 载荷指针/偏移与对齐要求；
4. 弱引用归零、最后强引用归零、对象头回收；
5. 溢出/下溢检测策略（原型可记录为 panic/错误，不承诺产品语义）。

原型只需要验证模型和布局，不接入 `xiao-runtime` 的对象头、GC/RC 或字节码。

### 3.2 操作与场景

1. **克隆/释放**：强计数增减、弱计数增减、最后强引用恰好一次析构；
2. **弱升级**：并发升级与强引用归零的竞态；升级失败后载荷不得复活；
3. **对象头回收**：强/弱计数都归零后只回收一次；弱引用保留时对象头存活；
4. **只读共享**：多线程读取载荷，无数据竞争；
5. **共享可变**：显式锁、取消等待、异常/毒化锁、清理顺序；
6. **普通/共享区分**：普通句柄不能伪装成共享句柄，不能隐式跨线程；
7. **内存顺序**：至少验证 acquire/release 的发布-订阅路径；更细的模型可在风险节记录；
8. **取消/清理**：沿用 07-D §10.4 的步骤，验证主错误与 suppressed 错误顺序。

### 3.3 测量

在每个可用平台记录：

- `size_of`/`align_of` 对象头、共享句柄、弱句柄、普通句柄；
- 原子增减成本、弱升级成功/失败成本；
- 锁保护写入、取消等待、析构成本；
- 环境、命令、重复次数和方差；数据只作为原型参考，不冻结 ABI。

### 3.4 隔离要求

- 继续放在 `tests/research/concurrency`，不加入 `core/rust` workspace；
- 不复制进 `xiao-runtime`，不被产品 crate 引用；
- 不修改对象布局、IR、Runtime ABI、诊断字段或标准库公开接口；
- 原型结果只用于回答“是否进入实现阶段 / 首版 ABI 怎么冻结”。

### 3.5 本地 ABI 原型结果

Windows x86_64 下运行 `cargo run --release --manifest-path tests/research/concurrency/Cargo.toml
-- --iterations 1000`，命令行输出的自定义布局为：

| 项目 | 尺寸/结果 |
| --- | ---: |
| 干净 `AtomicHeader`（不含测试探针） | 24 bytes，8-byte 对齐 |
| 含探针对象的载荷偏移 | 40 bytes（`offset_of!`，探针旁路字段占 16 bytes） |
| 共享强句柄包装器 | 8 bytes |
| 弱句柄包装器 | 8 bytes |
| 普通句柄包装器 | 16 bytes |
| 含探针 `u64` 载荷对象 | 48 bytes，8-byte 对齐 |
| 32-byte 对齐载荷对象 | 96 bytes；载荷偏移 64 bytes |
| 干净 `u64` 载荷对象 | 32 bytes；载荷偏移 24 bytes |
| 干净 32-byte 对齐载荷对象 | 64 bytes；载荷偏移 32 bytes |
| 原子强/弱计数 | 弱升级失败后不能复活载荷；载荷和对象头各析构一次 |
| 内存顺序 | acquire/release 发布路径通过 |
| 共享可变 | 显式 `Mutex`、取消等待、异常/毒化锁恢复通过 |

`--iterations 1000` 的自定义操作成本为：clone/drop `19 ns`、弱升级成功 `19 ns`、弱升级
失败 `<1 ns`（整数纳秒取整为 `0`）、锁保护增量 `20 ns`、取消标记检查 `<1 ns`（取整为
`0`）、含探针分配/析构 `264 ns`、干净 Box 分配/释放 `48 ns`（不含载荷析构）。这些成本只代表 Windows x86_64 当前构建。

这些数字只描述隔离原型的 `#[repr(C)]` 结构和当前机器，不能冻结 Xiao Runtime ABI。Linux
amd64、Linux arm64、macOS arm64 仍需用同一命令补测；CI 运行号取得前，07-D.8 保持本地
原型完成、外部证据待取证。

## 四、实施步骤

| 步 | 内容 | 验证 |
| --- | --- | --- |
| 0 | 推送 07-D.7 修复，确认 workspace gates 执行原型检查和测试 | CI 运行号 |
| 1 | 新增自定义原子对象头/共享句柄原型 | 场景 1–3 |
| 2 | 加入只读、共享可变、普通/共享区分和内存顺序场景 | 场景 4–7 |
| 3 | 补取消/清理与错误汇总 | 场景 8 |
| 4 | 在 Windows/Linux/macOS 运行测量并记录环境/重复次数 | 测量表 |
| 5 | 评估决策第 6 项是否可视为验证；否则修订 00 或继续原型 | 决策记录 |
| 6 | 文档/门禁收口，推送并记录 CI | 本地/CI |

**本批不写产品代码**；不改语言、类型、IR、Runtime ABI 或标准库公开契约。

## 五、验收

1. **对象头**：自定义强/弱计数、tag、布局和对齐可观察；不是 `Arc` 包装器；
2. **竞态**：弱升级/强归零、对象头回收、恰好一次析构有可重复测试；
3. **共享可变**：锁、取消、异常/毒化、清理顺序覆盖；
4. **普通/共享边界**：普通句柄不能被隐式升级为共享句柄；
5. **跨平台**：Windows/Linux amd64/Linux arm64/macOS arm64 至少记录尺寸和测量；
6. **CI**：最终 HEAD 的 workspace gates 运行原型测试并通过；旧运行号不能冒充新 HEAD；
7. **文档**：07-D.6/07-D.8 和 00 决策记录写清“参考模型 → ABI 原型”的状态，不把原型
   写成产品 ABI；
8. **不越界**：无语言语法、类型、IR、Runtime ABI、对象布局或标准库公开契约变更。

## 六、不负责

- 不实现完整并发 Runtime、任务池、Actor、序列化或官方库；
- 不把原型对象头接入 `xiao-runtime` 或字节码/LLVM；
- 不修改 `StrongHandle`/`WeakHandle` 布局或 `PhantomData` 边界；
- 不冻结跨平台 ABI、内存顺序或诊断字段；
- 不登记新阶段代号、`std.concurrent` 或任何语言/API 名称；
- 不用单平台测量或参考模型替代对象头/ABI 证据。

## 七、落点与取证

```text
docs/DevDocs/07d8-atomic-handle-abi-prototype.md  本文
docs/DevDocs/07d6-minimal-os-thread-prototype.md  参考模型与 ABI 状态
docs/DevDocs/07d7-prototype-gate-and-evidence-fixes.md  门禁/CI 接线
docs/DevDocs/00-decisions.md                      第 6 项状态
tests/research/concurrency/src/lib.rs             自定义对象头/句柄原型
tests/research/concurrency/src/README.md          隔离边界
docs/module-registry.json                         原型登记（如新增文件/目录）
tools/gates/run.sh                                原型测试入口
```

```bash
# 原型
cargo test --manifest-path tests/research/concurrency/Cargo.toml
cargo clippy --manifest-path tests/research/concurrency/Cargo.toml --all-targets -- -D warnings
cargo fmt --manifest-path tests/research/concurrency/Cargo.toml -- --check
cargo run --release --manifest-path tests/research/concurrency/Cargo.toml -- --iterations 1000

# 门禁/CI
bun run check
bun run check:layout
gh run list --workflow 379178019 --limit 3
```

## 八、风险

1. **把参考模型当 ABI**：`Arc`/`Weak`/`Mutex` 的尺寸、计数和内存顺序不是 Xiao 共享
   句柄 ABI；必须用自定义对象头重新验证。
2. **原子宽度/对齐**：不同平台和指针宽度会影响布局；跨平台测量前不能冻结对象头。
3. **内存顺序**：只测 acquire/release 可能漏掉 relaxed 路径问题；原型要记录哪些顺序
   已验证、哪些未验证。
4. **锁/取消抖动**：固定 sleep 在 CI 上可能不稳定；优先用可观察信号。
5. **范围蔓延**：对象头 + 共享可变很容易变成产品 Runtime；原型只做隔离模型。
6. **CI 成本**：跨平台测量可能不适合每个 push 都跑；可放到平台复现或受控任务，但必须
   记录运行号和环境，不能只在本地写数字。
