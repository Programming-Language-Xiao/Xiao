# 07-D.11. ABI 成本语义与证据终验

> **这是 07-D.10 之后的隔离原型收口批次，不是并发实现阶段。** `a8aaa4b` 已推送，
> workspace gates `39963374595` 和四平台复现 `39965322134` 均通过，干净布局和干净/含探针成本已落纸；
> 干净析构成本不是等价比较，取消等待和 Linux/macOS 测量仍未补。
>
> **本批一句话**：**把成本标签改成它实际测到的东西，把文档 HEAD/CI 状态改真，再补
> 取消等待和 Linux/macOS 测量；仍然不接产品 Runtime、语言或标准库契约。**
>
> 状态：**本地成本语义、CI 与平台复现已收口，Linux/macOS 测量和取消等待待补（2026-10-23）**。只改隔离原型、门禁和文档；不登记新阶段代号，不预留
> `H0-C`、`07-D 实现`、`std.concurrent` 或任何语法/API。

## 一、交接上下文

### 1.1 必读

1. [07-D.10. 干净 ABI 布局与证据收口](07d10-clean-abi-layout-and-evidence-closeout.md)
   —— 直接前置：干净布局、成本和 F-1–F-7；
2. [07-D.9. ABI 布局清理与成本取证](07d9-abi-layout-cleanup-and-cost-evidence.md)
   —— 原始清理目标和 F-4/F-5；
3. [07-D.8. 原子共享句柄对象头/ABI 原型](07d8-atomic-handle-abi-prototype.md)
   —— 原始 ABI 原型结果；
4. [00. 决策基线](00-decisions.md) §07-D 并发模型裁定（2026-10-22）第 6 项；
5. [21A. 单线程 RC、强环与并发模型决策交接](21a-rc-cycles-and-concurrency-handoff.md)
   §3.2/§五——新阶段前置；
6. [12. 测试与开发里程碑](12-tests-and-milestones.md)——测试分层和阶段出口。

### 1.2 当前状态（2026-10-23 实跑/CI）

| 环节 | 现状 | 结论 |
| --- | --- | --- |
| 取证提交 | `a8aaa4b` 已推送；CI/平台证据已回填 | ABI 成本语义已取证 |
| workspace gates | `39963374595` **success**；Linux 运行原型 9 tests | 新代码 CI 已取证 |
| 原型测试 | 9 passed | 本地与 CI 一致 |
| 干净布局 | `u64` 32B/偏移 24B；align32 64B/偏移 32B | 干净对象已单独测到 |
| 含探针布局 | `u64` 48B/偏移 40B；align32 96B/偏移 64B | 与干净对象分开记录 |
| 成本 | clone/drop 19 ns、升级成功 19 ns、失败/取消检查 `<1 ns`、锁 20 ns、含探针析构 264 ns、干净 Box 分配/释放 48 ns（不含载荷析构） | 操作集合已分列 |
| 文档 | 07-D.8/07-D.9/07-D.10 的状态、CI 与平台证据已同步 | F-1 已处理 |
| 平台/跨平台 | `39965322134` 四个平台测试 success；测量仅 Windows x86_64 | Linux/macOS 测量待补；见 F-5 |

### 1.3 本批边界

| 项 | 本批 | 说明 |
| --- | --- | --- |
| 文档 HEAD/CI/决策行 | **已完成** | 已统一到取证提交 `a8aaa4b` / workspace gates `39963374595` |
| 干净析构语义 | **做** | 等价生命周期，或明确改成“Box 分配/释放” |
| 64 位假设 | **做** | 文档 + cfg/相对断言 |
| 取消等待成本 | **做/明确待补** | 真实等待路径，或保留“未测” |
| 平台/跨平台证据 | **做/登记** | 平台复现测试 + Linux/macOS 测量 |
| 产品代码/语法/IR/ABI | **不做** | 不进入产品 Runtime 或语言契约 |
| 新阶段代号 | **不预留** | 原型结果出来后再决定 |

## 二、审核发现

### F-1 **文档 HEAD/CI/决策行仍是旧状态（已关闭）**

- `07d8-atomic-handle-abi-prototype.md:32` 仍写“07-D.8 改动在本地、尚未推送”；
  `:37` 仍写“尚无 workspace gates 运行号”；`:38` 仍写“当前只验证了参考模型”。
- `07d9-abi-layout-cleanup-and-cost-evidence.md:33` 仍写 `0698e1d` 已推送、07-D.10 在本地。
- `07d10-clean-abi-layout-and-evidence-closeout.md:32` 仍写 `0698e1d` 已推送、本批改动在
  本地；`:39` 的 CI 仍写最近是 f608a5b 的 `39813718835`。

实际 HEAD 是 `0ab538b`，新 CI 是 `39873277968`，已经通过。`07d10` 的 F-1 标“已关闭”，
但 07-D.8 的 HEAD/CI/决策行并未同步；需要统一更新后再关闭。

**处理结果**：已统一 07-D.8/07-D.9/07-D.10/00/README 的本地 HEAD、历史 CI 和当前待取证
状态；`39873277968` 只绑定 `0ab538b`，不覆盖本批本地改动。

### F-2 **“干净析构 48 ns”不是等价生命周期（已澄清）**

`measure_costs` 的 `clean_box_alloc_release_ns`：

```rust
let object = Box::new(CleanAtomicObject {
    header: AtomicHeader::new(HandleTag::Shared),
    payload: UnsafeCell::new(MaybeUninit::new(1_u64)),
});
black_box(object);
```

`CleanAtomicObject` 没有 `Drop`，载荷是 `MaybeUninit`；这里只测了 Box 分配/释放，没有
载荷析构、强计数释放或对象头生命周期。`drop_ns` 则包含：

- `DropCounters::new()` 的两个 Arc 分配；
- 含探针 `SharedHandle` 的构造；
- 最后一次强引用释放、载荷析构、对象头释放和探针析构。

因此 48 ns 与 264 ns 不是“干净 vs 含探针”的等价比较。处理要求：

1. 要么给干净对象补等价的生命周期（载荷析构 + 计数/头释放），再测量；
2. 要么把标签改成“干净对象 Box 分配/释放（不含载荷析构）”，不要与 264 ns 直接对比；
3. 文档成本表必须写清两边包含的操作。

**处理结果**：字段改为 `clean_box_alloc_release_ns`，文档明确其只测干净 `Box` 分配/释放，
不含载荷析构；含探针的完整句柄分配/析构单独记录为 264 ns，不互相计算加速比。

### F-3 **干净布局断言硬编码 64 位（已明确）**

`run_demo` 断言干净 `u64` 对象恰好 32B/偏移 24B、align32 64B/偏移 32B。32 位目标上
`usize`/`AtomicUsize` 更小，布局会不同。若原型只承诺 64 位平台，应在文档和测试中明确；
否则应按 `cfg(target_pointer_width)` 或相对关系断言，并把跨平台尺寸记录为数据而不是
固定常量。

**处理结果**：原型显式要求 64 位目标，非 64 位运行返回稳定错误；文档保留 Linux/macOS
64 位待测项，不把常量扩写成所有目标平台 ABI。

### F-4 **取消等待仍未测（P3）**

文档已把 `cancel_check_ns` 明确为“取消标记检查”，没有冒充等待成本；但“持锁 → 等待
线程阻塞 → 设置取消 → 返回”的真实路径仍待补。F-4 只能保持“已澄清/未测”，不能写成
“取消等待已完成”。

### F-5 **平台复现已通过，跨平台测量仍缺（P3，部分关闭）**

- `39965322134` 的 Windows amd64、Linux amd64/arm64、macOS arm64 和汇总作业均成功；
- 布局/成本只有 Windows x86_64；
- 平台复现只跑测试，不跑测量；需要受控测量步骤或明确记录缺失原因。

## 三、实施步骤

| 步 | 内容 | 验证 |
| --- | --- | --- |
| 0 | 更新 07-D.8/07-D.9/07-D.10/00/README 的 HEAD、CI、决策行 | 文档 diff |
| 1 | 修正干净析构测量：补等价生命周期或改标签 | 成本表 |
| 2 | 明确 64 位假设；必要时改用 cfg/相对断言 | 布局测试 |
| 3 | 保留取消等待为待补，或补真实等待测量 | 成本表/文档 |
| 4 | 运行原型 test/Clippy/fmt、layout、完整 `bun run check` | 本地门禁 |
| 5 | 推送本批修改，记录新的 workspace gates | CI 运行号 |
| 6 | 触发平台复现；补 Linux/macOS 测量或记录原因 | 平台/跨平台表 |

**本批不写产品代码**；不改语言、类型、IR、Runtime ABI、对象布局或标准库公开契约。

## 四、验收

1. **文档**：07-D.8/07-D.9/07-D.10/00/README 的 HEAD、CI、决策行一致；旧运行号只作历史；
2. **成本语义**：干净 48 ns 要么是等价生命周期，要么明确标成“Box 分配/释放（不含载荷
   析构）”；不得与含探针 264 ns 直接得出加速比；
3. **64 位**：明确平台假设或使用 cfg/相对断言；
4. **取消等待**：有真实测量，或明确保持“未测/待补”，不写成完成；
5. **门禁**：9+ 测试、Clippy、fmt、layout、完整 `bun run check` 通过；
6. **CI**：最终 HEAD 的 workspace gates 运行原型测试并通过；
7. **平台/跨平台**：macOS/Windows 测试、Linux/macOS 测量有运行记录或缺失说明；
8. **不越界**：无语言、IR、Runtime ABI、对象布局或标准库公开契约变更。

## 五、不负责

- 不接产品 Runtime、对象头、字节码或 LLVM；
- 不修改 `StrongHandle`/`WeakHandle` 布局或 `PhantomData` 边界；
- 不冻结跨平台 ABI、内存顺序或诊断字段；
- 不实现任务池、Actor、序列化、官方库或新阶段；
- 不登记 `H0-C`、`07-D 实现`、`std.concurrent` 或语言/API 名称；
- 不用不等价成本、单平台数字或旧 CI 结果冻结 ABI。

## 六、落点与取证

```text
docs/DevDocs/07d11-abi-cost-semantics-and-evidence-closeout.md  本文
docs/DevDocs/07d8-atomic-handle-abi-prototype.md                状态/CI/决策行
docs/DevDocs/07d9-abi-layout-cleanup-and-cost-evidence.md       HEAD/成本
docs/DevDocs/07d10-clean-abi-layout-and-evidence-closeout.md    HEAD/CI/F-1
docs/DevDocs/00-decisions.md                                    第 6 项状态
docs/DevDocs/README.md                                          07-D.11 索引
tests/research/concurrency/src/atomic_abi.rs                    干净生命周期/断言
tools/gates/run.sh                                              原型测试入口
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

1. **不等价比较**：干净 Box 分配/释放不能代表完整析构；任何加速比都要写清操作集合。
2. **64 位假设**：布局常量在 32 位目标不成立；先明确平台范围，再写死断言。
3. **取消等待噪声**：等待/唤醒成本依赖调度；记录环境、重复次数和分位数。
4. **旧 CI 冒充新证据**：`39813718835` 是 f608a5b；`39873277968` 是 0ab538b；本批还要新
   运行号。
5. **单平台结论**：Windows 数字不能代表 Linux/macOS；跨平台前不冻结 ABI。
6. **范围蔓延**：原型容易变成产品 Runtime；保持隔离 crate，不接产品代码。
