# 07-D.9. ABI 布局清理与成本取证

> **这是 07-D.8 自定义 ABI 原型的收口批次，不是并发实现阶段。** 原型已经能跑，9 个测试
> 通过；但对象头尺寸混入了测试探针、通用载荷偏移计算不正确，自定义句柄的 clone/upgrade/
> drop/lock 成本还没有测。首批提交已推送，Linux workspace gates 已确认运行 9 个原型测试；
> 平台复现和跨平台测量仍未取证。
>
> **本批一句话**：**把“可比较的 ABI 布局”和“测试探针”分开，把偏移/对齐算对，把自定义
> 句柄成本测出来，再补平台/跨平台证据；仍然不接产品 Runtime 或语言契约。**
>
> 状态：**ABI 清理与本地成本证据已完成，待平台复现与跨平台测量（2026-10-23）**。只改隔离
> 原型、门禁和文档；不登记新阶段代号，不预留 `H0-C`、`07-D 实现`、`std.concurrent` 或
> 任何语法/API。

## 一、交接上下文

### 1.1 必读

1. [07-D.8. 原子共享句柄对象头/ABI 原型](07d8-atomic-handle-abi-prototype.md)
   —— 当前自定义对象头、结果表和待证项；
2. [07-D.7. 原型门禁与证据收口](07d7-prototype-gate-and-evidence-fixes.md)
   —— layout、CI 接线和测量标签；
3. [00. 决策基线](00-decisions.md) §07-D 并发模型裁定（2026-10-22）第 6 项；
4. [07-D. 并发模型研究](07d-concurrency-model-research.md) §10.5
   —— 原子共享句柄 ABI 前置清单；
5. [21A. 单线程 RC、强环与并发模型决策交接](21a-rc-cycles-and-concurrency-handoff.md)
   §3.2/§五——新阶段前置；
6. [12. 测试与开发里程碑](12-tests-and-milestones.md)——测试分层和阶段出口。

### 1.2 当前状态（2026-10-22 实跑）

| 环节 | 现状 | 结论 |
| --- | --- | --- |
| HEAD | `0ab538b` 已推送；07-D.13 改动在本地、尚未推送 | ABI 清理代码待新 CI |
| 原型测试 | 9 passed（本地）；Linux workspace gates 也运行并通过 9 tests | 自定义 ABI 场景可运行 |
| Clippy / fmt | 通过 | 代码质量通过 |
| `bun run check:layout` | 通过 | 目录/注册已修 |
| 完整 `bun run check` | 通过 | 本地门禁成立 |
| workspace gates | `39873277968` 绑定 `0ab538b` 的 Linux 作业 success；日志包含 research concurrency fmt/test/Clippy | 本地 ABI 清理之后的 CI 仍待新运行 |
| 平台复现 | 未重跑；上次 `39563381227` 早于原型测试进入 `run.sh` | macOS/Windows 原型测试待新运行；见 F-6 |
| 布局输出 | 干净头 24B；干净对象 `u64` 32B/24B、align32 64B/32B；含探针对象 48B/40B、align32 96B/64B | 仅 Windows x86_64 模型 |
| 自定义成本 | clone/drop 19 ns、升级成功 19 ns、升级失败/取消检查 `<1 ns`、锁 20 ns、含探针析构 264 ns、干净 Box 分配/释放 48 ns（不含载荷析构） | Windows x86_64，1000 次，仍待跨平台 |
| 文档 | 已统一干净头、探针对象和高对齐布局的数字 | F-1/F-2/F-3 已处理 |
| 跨平台测量 | 仅 Windows x86_64；Linux 只跑测试未跑测量 | 待补；见 F-6 |

### 1.3 本批边界

| 项 | 本批 | 说明 |
| --- | --- | --- |
| 布局数字/文档 | **做** | 统一 48/56 或修正后的真实数字，分列“干净头/含探针” |
| 测试探针分离 | **做** | 不把 `Arc<AtomicUsize>` 探针算进 ABI 头 |
| 通用偏移/对齐 | **做** | 用真实 `offset_of`，补高对齐载荷测试 |
| 自定义句柄成本 | **做** | clone/drop、弱升级成功/失败、锁、取消、析构 |
| 推送/CI | **做** | 推送 7 个提交，确认 workspace gates 运行原型测试 |
| 跨平台测量 | **做/登记** | Linux amd64/arm64、macOS arm64；不能跑的记录原因 |
| 产品代码/语法/IR/ABI | **不做** | 不进入 `xiao-runtime` 或语言契约 |
| 新阶段代号 | **不预留** | 原型结果出来后再决定 |

## 二、审核发现

### F-1 **文档内部尺寸不一致（已关闭）**

`07d8-atomic-handle-abi-prototype.md:36` 写：

> 自定义对象头 **40 bytes**……`u64` 对象 **48 bytes**

但同文档 `:151` 结果表和实际输出是：

> `AtomicHeader` **48 bytes**……`u64` 载荷对象 **56 bytes**

两处必须统一。若之后分离测试探针，应同时给出“干净 ABI 头”和“含探针原型头”两组数字，
并说明差异来源。

**处理结果**：07-D.8、00 决策基线和本批结果表已统一为干净头 24B、含探针对象 48B、
`u64` 载荷偏移 40B；高对齐对象另列 96B/64B。

### F-2 **48 字节对象头包含测试探针（已关闭）**

`tests/research/concurrency/src/atomic_abi.rs` 的 `AtomicHeader` 包含：

- `strong`、`weak`、`tag`、`payload_offset`；
- `payload_drops: Arc<AtomicUsize>`、`header_drops: Arc<AtomicUsize>`——这两个是测试
  析构探针，占 16 字节；
- `payload_offset` 是冗余字段，又占 8 字节。

因此 48 字节不是可比较的 ABI 候选尺寸；真实候选应在 24–32 字节量级。处理要求：

1. 把探针移出对象头（测试侧旁路表、全局计数或包装器），或至少同时报告“含探针”和
   “干净头”两组尺寸；
2. 去掉或明确标注冗余 `payload_offset` 字段；
3. 文档结果表不得把含探针尺寸写成“自定义对象头 ABI 尺寸”。

**处理结果**：`payload_drops`/`header_drops` 已移出 `AtomicHeader`，干净头尺寸单独报告；
含探针对象只作为分配模型尺寸记录，不再称作 ABI 头。

### F-3 **载荷偏移硬编码，通用对齐错误（已关闭）**

`atomic_abi.rs:151-155`：

```rust
payload_offset: size_of::<AtomicHeader>(),
```

这只对 `u64` 成立。对于 `align_of::<T>() > align_of::<AtomicHeader>()` 的载荷，
`AtomicObject<T>` 的 `#[repr(C)]` 布局会在头和载荷之间插入 padding，实际偏移大于头尺寸；
当前 `layout()` 和测试 `payload_offset == header_bytes` 会给出错误结论。

处理要求：

1. 用 `std::mem::offset_of!(AtomicObject<T>, payload)`（或等价安全计算）得到真实偏移；
2. 补一个高对齐载荷测试（例如 `#[repr(align(32))]`），断言偏移、对象尺寸和对齐；
3. 如果对象头不再保存 `payload_offset`，测试应直接比较实际偏移，而不是头尺寸。

**处理结果**：头中已移除冗余 `payload_offset` 字段，布局使用 `offset_of!` 计算，并加入
32-byte 对齐载荷断言；本地结果为 `u64` 偏移 40B、高对齐载荷偏移 64B。

### F-4 **没有测自定义句柄的操作成本（已关闭，本地）**

07-D.8 §3.3 要求测原子增减、弱升级成功/失败、锁保护、取消等待和析构成本。当前输出只有
布局和正确性；`measure()` 的 `atomic_op_ns`/`lock_op_ns` 仍测标准库 `AtomicUsize`/
`Mutex`，不是自定义 `SharedHandle`/`WeakHandle`。

处理要求：为自定义句柄补：

- clone/drop（强计数）；
- downgrade/upgrade 成功与失败；
- 共享可变锁、取消等待；
- 载荷/对象头析构；
- 环境、重复次数和方差；结果表分开“标准库参考”和“自定义原型”。

**处理结果**：新增自定义 clone/drop、弱升级成功/失败、锁、取消检查和分配/析构成本；
Windows x86_64、1000 次结果为 clone/drop `19 ns`、升级成功 `19 ns`、升级失败 `<1 ns`、
锁 `20 ns`、取消检查 `<1 ns`、含探针分配/析构 `264 ns`、干净 Box 分配/释放 `48 ns`（不含载荷析构）。跨平台测量仍待补。

### F-5 **workspace gates 已取证，平台复现仍未跑（部分关闭）**

07-D.7 已把 `tests/research/**` 接入 workspace gates，并把原型 fmt/test/Clippy 放进
`run.sh`；`39873277968` 在绑定 `0ab538b` 的 Linux 作业中 success，日志确认
`research concurrency cargo fmt/test/Clippy` 都执行，原型 **9 tests** 在 Linux 通过。
更早的 `39813718835` 只保留为历史来源，均不覆盖 07-D.13 的本地改动。

仍未关闭的是平台复现：上一次 `39563381227` 在 d6112e4，早于原型测试进入 `run.sh`，因此
macOS/Windows 还没有运行过这 9 个原型测试。要在 07-D.13 的本地改动完成后手动触发
`365649226`，并记录四平台作业结果；不能只用 Linux workspace gates 代替 macOS/Windows。

### F-6 **跨平台测试与测量仍缺（P3）**

Linux workspace gates 只证明原型测试能在 Linux 跑；对象头布局、原子宽度、锁成本和弱升级
竞态仍只有 Windows x86_64 的测量。需要：

1. 在新的平台复现运行中让 macOS/Windows 执行 `run.sh` 里的原型测试；
2. 在 Linux amd64/arm64、macOS arm64 用同一命令补测布局和成本；
3. 若平台复现不跑测量，另加受控测量步骤或记录缺失原因；不能用测试通过代替测量。

### F-7 **“本地 ABI 原型已完成”表述过宽（P3）**

更准确的状态是“`u64` 本地布局模型和 Linux CI 测试通过；通用偏移、探针分离、自定义操作
成本、平台复现和跨平台测量待补”。在 F-1–F-4 关闭前，不把当前结果写成可冻结的 ABI 候选。

## 三、实施步骤

| 步 | 内容 | 验证 |
| --- | --- | --- |
| 0 | 统一 07-D.8 状态表/结果表数字；区分干净头与含探针头 | 文档 diff |
| 1 | 把测试探针移出对象头或旁置；移除冗余 `payload_offset` 字段 | 布局输出 |
| 2 | 用 `offset_of!` 计算载荷偏移；补高对齐载荷测试 | 单元测试 |
| 3 | 补自定义句柄 clone/drop/upgrade/lock/cancel/drop 成本测量 | 结果表 |
| 4 | 运行原型 test/Clippy/fmt、layout、完整 `bun run check` | 本地门禁 |
| 5 | 推送（已完成）；ABI/测量修改后触发平台复现，记录 macOS/Windows 原型测试 | workspace gates + 平台运行号 |
| 6 | 补 Linux amd64/arm64、macOS arm64 的布局/成本测量，或记录缺失原因 | 跨平台表 |
| 7 | 更新 00/07-D.8/README 的 ABI 状态 | 文档 |

**本批不写产品代码**；不改语言、类型、IR、Runtime ABI、对象布局或标准库公开契约。

## 四、验收

1. **布局**：文档数字一致；干净 ABI 头与含探针原型头分开；不再把 48 字节直接写成 ABI 候选；
2. **通用性**：载荷偏移用真实 `offset_of!`；高对齐载荷测试通过；
3. **成本**：自定义句柄的 clone/drop、upgrade 成功/失败、锁、取消和析构都有测量；
4. **门禁**：原型 9+ 测试、Clippy、fmt、layout、完整 `bun run check` 通过；
5. **CI**：Linux workspace gates `39813718835` 已运行原型测试并通过；ABI/测量修改后的最终
   HEAD 需新的 workspace gates；平台复现要另行触发并记录 macOS/Windows 作业；
6. **跨平台**：至少记录 Windows/Linux/macOS 的布局和成本，或明确缺失原因；原型测试通过
   不能替代测量；
7. **不越界**：无语言、IR、Runtime ABI、对象布局或标准库公开契约变更；
8. **状态**：取得上述证据后再把 07-D.8 写成完成；之前保持“本地 `u64` 模型、证据待补”。

## 五、不负责

- 不接产品 Runtime、对象头、字节码或 LLVM；
- 不修改 `StrongHandle`/`WeakHandle` 布局或 `PhantomData` 边界；
- 不冻结跨平台 ABI、内存顺序或诊断字段；
- 不实现任务池、Actor、序列化、官方库或新阶段；
- 不登记 `H0-C`、`07-D 实现`、`std.concurrent` 或语言/API 名称；
- 不用含测试探针的尺寸或单平台数字冻结 ABI。

## 六、落点与取证

```text
docs/DevDocs/07d9-abi-layout-cleanup-and-cost-evidence.md  本文
docs/DevDocs/07d8-atomic-handle-abi-prototype.md           状态/结果表/数字
docs/DevDocs/00-decisions.md                               第 6 项 ABI 状态
docs/DevDocs/README.md                                     07-D.9 索引
tests/research/concurrency/src/atomic_abi.rs               对象头/布局/成本
tests/research/concurrency/src/lib.rs                      测量入口
tools/gates/run.sh                                         原型测试入口
.github/workflows/workspace-gates.yml                      tests/research 路径
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

## 七、风险

1. **探针混入 ABI**：测试计数器不能算进对象头尺寸；必须旁置或分列。
2. **对齐/偏移**：`#[repr(C)]` 的 padding 取决于载荷对齐；不能用头尺寸代替真实偏移。
3. **测量噪声**：自定义原子操作和锁成本受机器/负载影响；记录环境、重复次数和方差。
4. **单平台结论**：Windows 数字不能代表 Linux/macOS；跨平台前不冻结 ABI。
5. **范围蔓延**：对象头原型很容易被当成产品 Runtime；保持隔离 crate，不接产品代码。
6. **CI 成本**：自定义句柄成本测量可能不宜每个 push 都跑；可放平台复现或受控任务，
   但必须记录运行号和环境。
