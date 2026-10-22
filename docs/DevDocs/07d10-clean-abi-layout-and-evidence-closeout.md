# 07-D.10. 干净 ABI 布局与证据收口

> **这是 07-D.9 之后的隔离原型收口批次，不是并发实现阶段。** `0698e1d` 已经把测试探针
> 移出 `AtomicHeader`，加入 `offset_of!`、高对齐载荷和自定义句柄成本；但对象本体仍含
> 探针，干净 ABI 对象布局没有直接测到，文档还留着旧数字/旧 CI 状态，且新代码没有 CI。
>
> **本批一句话**：**把“干净 ABI 对象”单独测出来，把取消等待/析构成本标清，把文档和 CI
> 状态改真；仍然不接产品 Runtime、语言或标准库契约。**
>
> 状态：**本地 ABI 清理与成本证据已完成，CI/跨平台待取证（2026-10-23）**。只改隔离原型、门禁和文档；不登记新阶段代号，不预留
> `H0-C`、`07-D 实现`、`std.concurrent` 或任何语法/API。

## 一、交接上下文

### 1.1 必读

1. [07-D.9. ABI 布局清理与成本取证](07d9-abi-layout-cleanup-and-cost-evidence.md)
   —— 当前清理目标、F-1–F-7 和本地结果；
2. [07-D.8. 原子共享句柄对象头/ABI 原型](07d8-atomic-handle-abi-prototype.md)
   —— 原始 ABI 原型和结果表；
3. [00. 决策基线](00-decisions.md) §07-D 并发模型裁定（2026-10-22）第 6 项；
4. [07-D. 并发模型研究](07d-concurrency-model-research.md) §10.5
   —— ABI 前置清单；
5. [21A. 单线程 RC、强环与并发模型决策交接](21a-rc-cycles-and-concurrency-handoff.md)
   §3.2/§五——新阶段前置；
6. [12. 测试与开发里程碑](12-tests-and-milestones.md)——测试分层和阶段出口。

### 1.2 当前状态（2026-10-23 实跑）

| 环节 | 现状 | 结论 |
| --- | --- | --- |
| HEAD | `0698e1d` 已推送；本批改动在本地、尚未推送 | ABI 清理代码待新 CI |
| 原型测试 | 9 passed | 本地通过 |
| Clippy / fmt | 通过 | 本地通过 |
| 完整 `bun run check` | 通过 | 本地门禁成立 |
| 布局输出 | 干净头 24B；干净对象 `u64` 32B/偏移 24B、align32 64B/偏移 32B；含探针对象 48B/40B、align32 96B/64B | 干净/含探针已分列 |
| 自定义成本 | clone/drop 19 ns、升级成功 19 ns、失败/取消检查 `<1 ns`、锁 20 ns、含探针析构 264 ns、干净析构 48 ns | 取消等待真实路径和跨平台仍待补 |
| 文档 | 07-D.8/07-D.9 已同步干净布局、成本和当前 HEAD | F-1 已关闭 |
| CI | 最近 workspace gates 是 f608a5b 的 `39813718835` | 不覆盖 0698e1d；见 F-3 |
| 平台/跨平台 | 未重跑；测量仅 Windows x86_64 | 待补；见 F-7 |

### 1.3 本批边界

| 项 | 本批 | 说明 |
| --- | --- | --- |
| 干净 ABI 对象布局 | **做** | header + payload，不含测试探针；u64 与 align32 |
| 文档状态/数字 | **做** | 07-D.8/07-D.9/00/README 同步 |
| 取消等待成本 | **做/改名** | 真实等待路径，或明确只测“取消标记检查” |
| 干净分配/析构 | **做/标注** | 分开含探针与无探针成本 |
| 推送/CI | **做** | 推送 0698e1d 及本批修改，取得 workspace gates |
| 平台/跨平台测量 | **做/登记** | macOS/Windows 测试、Linux/macOS 布局成本 |
| 产品代码/语法/IR/ABI | **不做** | 不进入产品 Runtime 或语言契约 |
| 新阶段代号 | **不预留** | 原型结果出来后再决定 |

## 二、审核发现

### F-1 **文档仍有过期数字和状态（已关闭）**

- `07d8...:36` 的测量行仍写“自定义对象头 40 bytes……`u64` 对象 48 bytes”，但结果表
  `:151` 已是“干净头 24B、含探针对象 48B”；同一文件 HEAD/CI 行仍写“本地、未推送、无
  workspace gates”，而 `4008458` 已随 `f608a5b` 推送并有 `39813718835`。
- `07d9...:34` 的 HEAD 仍写 `f608a5b`、origin/main 一致；实际本地 `0698e1d` 领先 2 个
  提交。`39813718835` 是 f608a5b 的 CI，不是新 ABI 代码的 CI。

处理要求：统一 HEAD、CI、尺寸数字；旧运行号只能作为历史来源，不能写成新代码证据。

**处理结果**：07-D.8、07-D.9、00 决策基线和 README 已统一干净/含探针布局、成本和
本地改动待 CI 的状态；`39813718835` 仅保留为 f608a5b 的历史 Linux 证据。

### F-2 **干净 ABI 对象布局没有直接测到（已关闭）**

探针已移出 `AtomicHeader`，但仍留在 `AtomicObject`：

```rust
struct AtomicObject<T> {
    header: AtomicHeader,
    payload_drops: Arc<AtomicUsize>,
    header_drops: Arc<AtomicUsize>,
    payload: UnsafeCell<MaybeUninit<T>>,
}
```

因此当前 `payload_offset=40`、`object_u64_bytes=48`、`align32_offset=64`、
`object_align32_bytes=96` 都是**含探针对象**。干净 ABI 对象应为：

| 载荷 | 干净对象 | 干净偏移 |
| --- | ---: | ---: |
| `u64`（8 字节对齐） | 32 bytes | 24 bytes |
| `repr(align(32))` 载荷 | 64 bytes | 32 bytes |

处理要求：

1. 新增一个不带探针的 `CleanAtomicObject<T>`（或把探针旁置到对象外），用
   `offset_of!`/`size_of!`/`align_of!` 直接测量上表；
2. 保留现有含探针对象用于析构计数和行为测试，但结果表分列“干净对象”和“含探针对象”；
3. 高对齐行必须标明是否含探针。

**处理结果**：新增 `CleanAtomicObject<T>`，直接测得干净 `u64` 对象 32B/偏移 24B、
align32 对象 64B/偏移 32B；含探针对象仍单独记录。

### F-3 **新代码没有 CI（待新运行号，外部取证）**

`0698e1d` 未推送；最近 workspace gates `39813718835` 绑定 `f608a5b`，只覆盖旧版原型。
推送后必须确认：

- workspace gates 运行 `tests/research/concurrency` 的 9 个测试；
- fmt/Clippy 通过；
- 不能用旧运行号关闭新代码的 CI 项。

### F-4 **“取消检查”不是“取消等待”（已澄清）**

`cancel_check_ns` 只测 `AtomicU8::load`，没有测等待锁期间轮询取消、唤醒或安全点到达的
成本；0 ns 也低于计时器分辨率。F-4 不能因此写成“取消等待已测”。处理要求：

1. 要么测真实的“持锁 → 等待线程阻塞 → 设置取消 → 返回”的延迟/吞吐，并记录环境；
2. 要么把项改名为“取消标记检查”，并把“取消等待成本”继续列为待补；
3. 不要用单次 load 的 0 ns 代表等待路径。

**处理结果**：成本字段保留为 `cancel_check_ns`，文档明确它只是取消标记检查；真实持锁
等待、唤醒和安全点路径仍列为待补，不把 0 ns 写成取消等待成本。

### F-5 **分配/析构成本含测试探针（已关闭）**

`drop_ns` 每次迭代创建 `DropCounters::new()`（两个 Arc 分配）和含探针的
`SharedHandle`，所以 261 ns 是“含探针分配/析构”。处理要求：补干净对象的分配/析构
成本，或在表头/结果中明确标注含探针。

**处理结果**：新增 `clean_drop_ns`，本地结果为含探针 264 ns、干净对象 48 ns；两者已
分列记录。

### F-6 **源码注释仍把探针描述为对象头的一部分（已关闭）**

`atomic_abi.rs` 的 `AtomicHeader` 注释仍写“和实验析构探针”，但 header 已不含探针；
`AtomicObject` 注释也没有说明它包含探针字段。需同步注释，避免 24B 头与 48B 对象混淆。

**处理结果**：`AtomicHeader` 注释只描述计数/tag；`AtomicObject` 明确说明旁路析构探针，
`CleanAtomicObject` 作为无探针布局测量模型单独标注。

### F-7 **平台/跨平台证据仍缺（P3）**

- 平台复现未重跑；macOS/Windows 还没有运行原型 9 个测试；
- 布局/成本测量仍只有 Windows x86_64；
- 平台复现只跑测试，不跑测量；需要单独的受控测量步骤或记录缺失原因。

## 三、实施步骤

| 步 | 内容 | 验证 |
| --- | --- | --- |
| 0 | 更新 07-D.8/07-D.9/00/README 的 HEAD、CI、尺寸状态 | 文档 diff |
| 1 | 增加干净 `CleanAtomicObject<T>` 布局测量（u64/align32） | `offset_of/size_of/align_of` 断言 |
| 2 | 分列干净对象与含探针对象结果；高对齐行标注 | 结果表 |
| 3 | 测取消等待路径，或改名并保留待办 | 成本表/文档 |
| 4 | 补干净分配/析构成本或标注含探针 | 成本表 |
| 5 | 同步源码注释 | 代码 diff |
| 6 | 运行原型 test/Clippy/fmt、layout、完整 `bun run check` | 本地门禁 |
| 7 | 推送 `0698e1d` 及本批修改，记录 workspace gates | CI 运行号 |
| 8 | 触发平台复现；补 Linux/macOS 测量或记录原因 | 平台/跨平台表 |

**本批不写产品代码**；不改语言、类型、IR、Runtime ABI、对象布局或标准库公开契约。

## 四、验收

1. **干净布局**：干净 `u64` 对象 32B/偏移 24；align32 对象 64B/偏移 32；含探针对象
   另行列出；
2. **文档**：07-D.8/07-D.9/00/README 的 HEAD、CI、尺寸一致；不再用旧运行号覆盖新代码；
3. **成本**：取消等待路径有真实测量或明确改名/待办；分配/析构区分干净/含探针；
4. **门禁**：9+ 测试、Clippy、fmt、layout、完整 `bun run check` 通过；
5. **CI**：推送后的 workspace gates 运行原型测试并通过；
6. **平台/跨平台**：macOS/Windows 原型测试、Linux/macOS 布局成本有运行记录或缺失说明；
7. **不越界**：无语言、IR、Runtime ABI、对象布局或标准库公开契约变更；
8. **状态**：上述证据齐备后才把“ABI 原型/清理完成”写成完成。

## 五、不负责

- 不接产品 Runtime、对象头、字节码或 LLVM；
- 不修改 `StrongHandle`/`WeakHandle` 布局或 `PhantomData` 边界；
- 不冻结跨平台 ABI、内存顺序或诊断字段；
- 不实现任务池、Actor、序列化、官方库或新阶段；
- 不登记 `H0-C`、`07-D 实现`、`std.concurrent` 或语言/API 名称；
- 不用含探针尺寸、单次 load 的 0 ns 或单平台数字冻结 ABI。

## 六、落点与取证

```text
docs/DevDocs/07d10-clean-abi-layout-and-evidence-closeout.md  本文
docs/DevDocs/07d8-atomic-handle-abi-prototype.md              状态/尺寸/CI
docs/DevDocs/07d9-abi-layout-cleanup-and-cost-evidence.md     状态/F-4/F-5
docs/DevDocs/00-decisions.md                                  第 6 项 ABI 状态
docs/DevDocs/README.md                                        07-D.10 索引
tests/research/concurrency/src/atomic_abi.rs                 干净对象/成本/注释
tests/research/concurrency/src/lib.rs                         测量入口
tools/gates/run.sh                                            原型测试入口
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

1. **干净/含探针混淆**：任何布局或成本数字都要标明是否含探针；干净 ABI 对象必须单独测。
2. **对齐全测量**：align32 只是最小高对齐用例；更大对齐仍可能在头与载荷之间产生 padding，
   结果表要记录假设。
3. **取消等待测量噪声**：等待/唤醒成本高度依赖调度；记录环境、重复次数、分位数，不用
   单次平均值冻结结论。
4. **旧 CI 冒充新证据**：`39813718835` 只覆盖 f608a5b；新代码必须新运行号。
5. **单平台结论**：Windows 数字不能代表 Linux/macOS；跨平台前不冻结 ABI。
6. **范围蔓延**：对象头/成本原型容易变成产品 Runtime；保持隔离 crate，不接产品代码。
