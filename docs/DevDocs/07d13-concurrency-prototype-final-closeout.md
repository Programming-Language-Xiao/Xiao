# 07-D.13. 并发原型最终收口

> **这是 07-D 原型阶段的最终收口批次，不是并发实现阶段。** 07-D.12 已把主要文档行改真，
> 但 README/00 仍混用“Linux CI 已绿”和“平台/跨平台待取证”，`4f326eb` 仍未推送、没有
> 新 CI，`custom_abi_*` 入口没有 64 位保护，平台/跨平台测量仍缺。
>
> **本批一句话**：**把最后的文档、CI、入口和平台证据收干净，然后正式关闭 07-D 原型阶段
> 或明确登记缺口；不再继续增加 ABI 微批次。**
>
> 状态：**待实施（2026-10-23）**。只改隔离原型、门禁和文档；不登记新阶段代号，不预留
> `H0-C`、`07-D 实现`、`std.concurrent` 或任何语法/API；不进入产品 Runtime/语言/ABI。

## 一、交接上下文

### 1.1 必读

1. [07-D.12. 07-D 终验、文档同步与跨平台证据](07d12-closeout-doc-sync-and-cross-platform-evidence.md)
   —— 直接前置：文档同步、F-1–F-6；
2. [07-D.11. ABI 成本语义与证据终验](07d11-abi-cost-semantics-and-evidence-closeout.md)
   —— 成本语义和 64 位边界；
3. [07-D.10. 干净 ABI 布局与证据收口](07d10-clean-abi-layout-and-evidence-closeout.md)
   —— 干净布局；
4. [07-D.8. 原子共享句柄对象头/ABI 原型](07d8-atomic-handle-abi-prototype.md)
   —— ABI 原型；
5. [00. 决策基线](00-decisions.md) §07-D 并发模型裁定（2026-10-22）第 6 项；
6. [21A. 单线程 RC、强环与并发模型决策交接](21a-rc-cycles-and-concurrency-handoff.md)
   §3.2/§五——新阶段前置；
7. [12. 测试与开发里程碑](12-tests-and-milestones.md)——测试分层和阶段出口。

### 1.2 当前状态（2026-10-23 实跑）

| 环节 | 现状 | 结论 |
| --- | --- | --- |
| HEAD | `17bf342`；本地领先 origin/main **4 个提交**（`f445c74`、`4f326eb`、`086e746`、`17bf342`），未推送 | 新代码/文档无 CI |
| 原型测试 | 9 passed | 本地通过 |
| Clippy / fmt / 完整 check / layout | 通过 | 本地门禁成立 |
| 布局 | 干净 `u64` 32B/24B、align32 64B/32B；含探针 48B/40B、96B/64B | 干净/含探针已分列 |
| 成本 | clone/drop 19 ns、升级成功 19 ns、失败/取消检查 <1 ns、锁 20 ns、含探针 259–264 ns、干净 Box 47–48 ns | 单次波动，标签已区分 |
| CI | 最近是 `0ab538b` 的 `39873277968` | 不覆盖 4f326eb/17bf342 |
| 文档 | 07-D.8/09 主要行已修；README/00 仍有“全部待取证”的旧口径 | 见 F-1 |
| 平台/跨平台 | 未重跑；测量仅 Windows x86_64 | 见 F-5 |

### 1.3 本批边界

| 项 | 本批 | 说明 |
| --- | --- | --- |
| README/00 状态区分 | **做** | Linux CI 已绿 ≠ 平台/跨平台已取证 |
| 64 位入口 | **做** | `custom_abi_*` 加 guard，或明确只支持 `run_demo`/CLI |
| 推送/CI | **做** | 推送 4 个提交，取得新 workspace gates |
| 平台复现 | **做** | 触发 365649226，记录 macOS/Windows 9 tests |
| 跨平台测量 | **做/登记** | Linux/macOS 布局/成本，或明确缺失原因 |
| 取消等待 | **登记缺口** | 真实等待路径留给实现阶段，不写成完成 |
| 07-D 收口 | **决定** | 证据齐备后关闭原型阶段；否则明确剩余缺口 |
| 产品代码/新阶段 | **不做** | 不进入 Runtime/语言/IR/ABI/新阶段 |

## 二、审核发现

### F-1 **README/00 仍把“Linux CI 已绿”和“平台/跨平台待取证”混在一起（P3）**

`README.md` 的 07-D.6/07-D.7 仍写“CI 待取证”，07-D.8 仍写“CI/跨平台待取证”，
07-D.10/07-D.11 仍写“CI/平台/跨平台待取证”；但 Linux workspace gates `39873277968`
已经运行 9 个原型测试。`00-decisions.md:522` 仍写“Linux amd64、Linux arm64、macOS arm64
和推送后的 workspace gates 仍待取证”，也没有区分 `0ab538b` 已有的 Linux CI 与 `4f326eb`
尚未推送的新代码。

处理要求：统一写成“Linux workspace gates 已绿（具体运行号）；macOS/Windows 平台测试和
Linux/macOS 测量待取证；新代码的 workspace gates 待推送”。旧状态只能作为历史记录。

### F-2 **新代码没有 CI（P2，外部取证）**

`4f326eb`/`17bf342` 未推送；`39873277968` 只绑定 `0ab538b`，不覆盖 64 位 guard、成本
标签和最新文档。推送后必须取得新的 workspace gates，不能用旧运行号关闭 CI 项。

### F-3 **`custom_abi_*` 入口没有 64 位保护（P3）**

CLI 的 `main` 先调用 `run_all_scenarios`（含 `run_demo` 的 64 位 guard），所以命令行路径
安全；但库导出的 `custom_abi_layout_report()`/`custom_abi_cost_report()` 可以直接调用
`layout()`/`measure_costs()`，在 32 位目标上仍可能产生半套数据。处理要求：

1. 在这两个入口加同样的 64 位检查；或
2. 在文档中明确“只有 `run_demo`/CLI 是受支持入口，直接调用测量函数不受保护”，并把
   这两个函数降为测试专用（`#[cfg(test)]`/私有）。

### F-4 **取消等待仍是未来实现阶段缺口（P3）**

`cancel_check_ns` 只是标记检查；真实“持锁等待 → 取消 → 返回”的路径没有测。文档已明确
登记为未来实现阶段缺口，这是可接受的收口口径；但 07-D 的“完成”描述不能包含“取消等待
已验证”。

### F-5 **平台/跨平台证据仍缺（P3）**

- macOS/Windows 还没有运行 9 tests；
- 布局/成本只有 Windows x86_64；
- 平台复现只跑测试，不跑测量。

处理要求：

1. 触发 `365649226`，记录四平台作业；
2. 在 Linux/macOS 用同一命令补测布局/成本，或记录无法执行的原因；
3. 如果以测试通过代替测量，必须在文档中明确。

### F-6 **测量值有单次波动（P3）**

含探针 259–264 ns、干净 Box 47–48 ns；这是单次机器结果，不是稳定基准。结果表已注明
Windows x86_64/1000 次；跨平台比较需统一环境和重复次数。

## 三、实施步骤

| 步 | 内容 | 验证 |
| --- | --- | --- |
| 0 | 修 README/00 的当前状态区分 | 文档 diff；grep |
| 1 | 给 `custom_abi_*` 加 64 位 guard，或明确只支持 `run_demo`/CLI | 代码/文档 |
| 2 | 运行原型 test/Clippy/fmt、layout、完整 `bun run check` | 本地门禁 |
| 3 | 推送 4 个提交，记录新的 workspace gates | CI 运行号 |
| 4 | 触发平台复现，记录 macOS/Windows 9 tests | 新平台运行号 |
| 5 | 补 Linux/macOS 测量或记录缺失原因 | 跨平台表 |
| 6 | 更新 07-D.13/00/README，声明 07-D 原型收口或剩余缺口 | 文档 |

**本批不写产品代码**；不改语言、类型、IR、Runtime ABI、对象布局或标准库公开契约。

## 四、验收与收口

1. **文档**：README/00/07-D.8–13 清楚区分 Linux CI、平台测试、跨平台测量和新代码 CI；
2. **CI**：最终 HEAD 的 workspace gates 运行原型测试并通过；
3. **64 位**：`custom_abi_*` 入口也拒绝 32 位，或明确只支持 `run_demo`/CLI；
4. **取消等待**：登记为实现阶段缺口，不写成完成；
5. **平台/跨平台**：macOS/Windows 测试、Linux/macOS 测量有运行记录或缺失说明；
6. **门禁**：9+ 测试、Clippy、fmt、layout、完整 `bun run check` 通过；
7. **不越界**：无语言、IR、Runtime ABI、对象布局或标准库公开契约变更。

**07-D 原型收口判定**：

- 若 1–6 齐备：把 07-D 原型阶段标为“研究/原型完成；正式实现仍需 21A §3.2 与 07-D
  §11.3 的新阶段登记、B0/N0 黄金场景、跨平台 ABI 冻结和星崽裁定”；
- 若平台/跨平台无法在本批完成：在 07-D.13 明确登记为未来实现阶段的已知缺口，不阻塞
  原型收口，但不得写成“跨平台已验证”。

本批之后不再增加 ABI 微批次；后续工作要么进入新的并发实现阶段（需满足入口条件），
要么保持 07-D 研究状态。

## 五、不负责

- 不接产品 Runtime、对象头、字节码或 LLVM；
- 不修改 `StrongHandle`/`WeakHandle` 布局或 `PhantomData` 边界；
- 不冻结跨平台 ABI、内存顺序或诊断字段；
- 不实现任务池、Actor、序列化、官方库或新阶段；
- 不登记 `H0-C`、`07-D 实现`、`std.concurrent` 或语言/API 名称；
- 不用单次测量、旧 CI 或单平台数字冻结 ABI。

## 六、落点与取证

```text
docs/DevDocs/07d13-concurrency-prototype-final-closeout.md  本文
docs/DevDocs/README.md                                       07-D.6–13 状态
docs/DevDocs/00-decisions.md                                 第 6 项状态
docs/DevDocs/07d8-atomic-handle-abi-prototype.md             状态/CI
docs/DevDocs/07d9-abi-layout-cleanup-and-cost-evidence.md    状态/跨平台
docs/DevDocs/07d10-clean-abi-layout-and-evidence-closeout.md 状态
docs/DevDocs/07d11-abi-cost-semantics-and-evidence-closeout.md F-1/F-3
docs/DevDocs/07d12-closeout-doc-sync-and-cross-platform-evidence.md F-2/F-5
tests/research/concurrency/src/atomic_abi.rs                 64 位入口/测量
tools/gates/run.sh                                           原型测试入口
```

```bash
# 原型
cargo test --manifest-path tests/research/concurrency/Cargo.toml
cargo clippy --manifest-path tests/research/concurrency/Cargo.toml --all-targets -- -D warnings
cargo fmt --manifest-path tests/research/concurrency/Cargo.toml -- --check
cargo run --release --manifest-path tests/research/concurrency/Cargo.toml -- --iterations 1000

# 门禁/平台
bun run check
bun run check:layout
gh workflow run 365649226 --ref main
gh run list --workflow 365649226 --limit 3
```

## 七、风险

1. **继续微批次**：本批应明确收口或登记缺口，不再为单个文档行新增批次。
2. **文档再次漂移**：HEAD/CI 每次提交都会变；写具体哈希和运行号，并标明历史。
3. **把 Linux CI 当跨平台**：Linux workspace gates 不能替代 macOS/Windows 测试或测量。
4. **64 位入口半保护**：只 guard `run_demo` 不算入口统一；补齐或明确限制。
5. **单次测量**：259–264 / 47–48 的波动不能当性能结论。
6. **范围蔓延**：07-D 是研究/原型，不是产品并发实现。
