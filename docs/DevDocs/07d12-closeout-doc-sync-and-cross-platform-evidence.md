# 07-D.12. 07-D 终验、文档同步与跨平台证据

> **这是 07-D 原型阶段的最后一次收口批次，不是并发实现阶段。** `4f326eb` 已修好成本
> 标签和 64 位边界，本地 9 个测试/完整门禁通过；但 07-D.8/07-D.9 仍有旧 HEAD/CI 行，
> 新提交未推送、没有 CI，取消等待和平台/跨平台证据仍未补。
>
> **本批一句话**：**把最后几处文档写成真的，推送取 CI，补齐或明确登记平台/跨平台和
> 取消等待证据，然后决定 07-D 是否收口；仍然不接产品 Runtime、语言或标准库契约。**
>
> 状态：**本地文档与成本证据已收口，CI/平台/跨平台待取证（2026-10-23）**。只改隔离原型、门禁和文档；不登记新阶段代号，不预留
> `H0-C`、`07-D 实现`、`std.concurrent` 或任何语法/API。

## 一、交接上下文

### 1.1 必读

1. [07-D.11. ABI 成本语义与证据终验](07d11-abi-cost-semantics-and-evidence-closeout.md)
   —— 直接前置：成本语义、64 位边界和 F-1–F-5；
2. [07-D.10. 干净 ABI 布局与证据收口](07d10-clean-abi-layout-and-evidence-closeout.md)
   —— 干净布局与成本；
3. [07-D.9. ABI 布局清理与成本取证](07d9-abi-layout-cleanup-and-cost-evidence.md)
   —— 原始清理目标；
4. [07-D.8. 原子共享句柄对象头/ABI 原型](07d8-atomic-handle-abi-prototype.md)
   —— ABI 原型结果；
5. [00. 决策基线](00-decisions.md) §07-D 并发模型裁定（2026-10-22）第 6 项；
6. [21A. 单线程 RC、强环与并发模型决策交接](21a-rc-cycles-and-concurrency-handoff.md)
   §3.2/§五——新阶段前置；
7. [12. 测试与开发里程碑](12-tests-and-milestones.md)——测试分层和阶段出口。

### 1.2 当前状态（2026-10-23 实跑）

| 环节 | 现状 | 结论 |
| --- | --- | --- |
| HEAD | `0ab538b` 已推送；07-D.11/07-D.12 改动在本地、尚未推送 | 新 CI 待取 |
| 原型测试 | 9 passed | 本地通过 |
| Clippy / fmt | 通过 | 本地通过 |
| 完整 `bun run check` / layout | 通过 | 本地门禁成立 |
| 布局 | 干净 `u64` 32B/24B、align32 64B/32B；含探针 48B/40B、96B/64B | 干净/含探针已分列 |
| 成本 | clone/drop 19 ns、升级成功 19 ns、失败/取消检查 `<1 ns`、锁 20 ns、含探针析构 264 ns、干净 Box 分配/释放 48 ns（不含载荷析构） | 标签已区分；见 F-4 |
| CI | 最近是 `0ab538b` 的 `39873277968` | 不覆盖本批；见 F-2 |
| 文档 | 07-D.8/07-D.9/07-D.10/07-D.11 已同步本地状态、历史 CI 和成本语义 | F-1 已处理 |
| 平台/跨平台 | 未重跑；测量仅 Windows x86_64 | 待补；见 F-5 |

### 1.3 本批边界

| 项 | 本批 | 说明 |
| --- | --- | --- |
| 文档 HEAD/CI/决策行 | **做** | 07-D.8/07-D.9/07-D.10/07-D.11/00/README |
| 推送/CI | **做** | 推送 4f326eb，取得新 workspace gates |
| 64 位入口 | **做/记录** | main/measurement 也拒绝 32 位，或明确只走 run_demo |
| 取消等待 | **做/明确登记** | 真实测量，或作为实现阶段已知缺口 |
| 平台复现 | **做** | 触发 365649226，记录 macOS/Windows 9 tests |
| 跨平台测量 | **做/登记** | Linux/macOS 布局/成本，或记录缺失原因 |
| 产品代码/语法/IR/ABI | **不做** | 不进入产品 Runtime 或语言契约 |
| 新阶段代号 | **不预留** | 07-D 收口后再决定 |

## 二、审核发现

### F-1 **07-D.8/07-D.9 的旧行仍未修（已关闭）**

`07d8-atomic-handle-abi-prototype.md:32` 仍写“07-D.8 改动在本地、尚未推送”；`:37` 仍写
“尚无 workspace gates 运行号”；`:38` 仍写“当前只验证了参考模型”。但 `4008458` 已随
`f608a5b` 推送，`0ab538b` 有 `39873277968`，本地还已有干净 ABI 布局和 64 位 guard。

`07d9-abi-layout-cleanup-and-cost-evidence.md:39` 仍把 `39813718835` 当作 workspace gates
证据，没有回填 `39873277968` 和当前待取证状态。

`07d11` 的 F-1 标“已关闭”，但这些行没有同步；必须修完后再关闭。

**处理结果**：07-D.8/07-D.9/07-D.10/07-D.11/00/README 已统一当前本地状态、历史 CI 和
干净/含探针布局；旧运行号只保留来源，未用于覆盖本批改动。

### F-2 **新提交没有 CI（待取证）**

`4f326eb` 未推送；`39873277968` 绑定 `0ab538b`，不覆盖 64 位 guard、成本标签和文档
修改。推送后必须取得新的 workspace gates 运行号，不能用旧运行号关闭新代码的 CI 项。

**当前结果**：本批尚未取得新 CI 运行号，保持外部取证状态。

### F-3 **64 位 guard 只在 `run_demo`（已明确）**

`layout()` 和 `measure_costs()` 在 32 位目标上仍可调用；如果原型承诺只支持 64 位，建议
在 main/测量入口也拒绝，或在文档中明确 `run_demo` 是唯一受支持入口。

**处理结果**：原型入口已明确要求 64 位目标，非 64 位运行返回稳定错误；本批不把尺寸扩展
为 32 位目标结论。

### F-4 **取消等待仍未测（已登记缺口）**

`cancel_check_ns` 只是标记检查；真实“持锁等待 → 设置取消 → 返回”的路径仍未测。两种
处理方式：

1. 补真实等待路径的延迟/吞吐；
2. 明确把它登记为未来实现阶段的已知缺口，不阻塞 07-D 原型收口。

无论选哪种，文档都不能写成“取消等待已完成”。

**处理结果**：保留 `cancel_check_ns` 的标记检查语义，真实持锁等待/唤醒/安全点路径登记
为未来实现阶段缺口，不阻断本地原型收口。

### F-5 **平台/跨平台证据仍缺（待补）**

- 平台复现未重跑；macOS/Windows 还没有运行 9 tests；
- 布局/成本只有 Windows x86_64；
- 平台复现只跑测试，不跑测量。需要：
  1. 触发 `365649226`，记录四平台作业；
  2. 在 Linux/macOS 用同一命令补测布局/成本，或记录无法执行的原因；
  3. 如果用测试通过代替测量，必须在文档中明确。

### F-6 **测量值有单次波动（已标注）**

本次复跑为含探针 264 ns、干净 Box 48 ns；文档记录同一口径。波动本身正常，但结果表应
注明“单次结果/当前机器”，不要把它当稳定基准；跨平台比较需统一重复次数和环境。

**处理结果**：成本表统一标注 Windows x86_64、1000 次和单次机器结果；没有把运行间波动
写成性能结论。

## 三、实施步骤

| 步 | 内容 | 验证 |
| --- | --- | --- |
| 0 | 修 07-D.8/07-D.9/07-D.11 的 HEAD/CI/决策行与 F-1 状态 | 文档 diff；grep |
| 1 | 推送 `4f326eb` 及本批文档，记录 workspace gates | CI 运行号 |
| 2 | 明确 64 位入口边界（main/measurement 或文档） | 代码/文档 |
| 3 | 取消等待：补测或登记为实现阶段缺口 | 成本表/文档 |
| 4 | 触发平台复现，记录四平台测试 | 新平台运行号 |
| 5 | 补 Linux/macOS 测量或记录缺失原因 | 跨平台表 |
| 6 | 最终审计：文档/UseDocs、layout、完整 check、CI | 本地/CI |

**本批不写产品代码**；不改语言、类型、IR、Runtime ABI、对象布局或标准库公开契约。

## 四、验收

1. **文档**：07-D.8/07-D.9/07-D.10/07-D.11/00/README 的 HEAD、CI、决策和 F-1 状态一致；
2. **CI**：最终 HEAD 的 workspace gates 运行原型测试并通过；
3. **64 位**：入口边界明确，非 64 位不会产生半套数据；
4. **取消等待**：有真实测量，或明确登记为未来实现阶段缺口，不写成完成；
5. **平台/跨平台**：macOS/Windows 测试、Linux/macOS 测量有运行记录或缺失说明；
6. **门禁**：9+ 测试、Clippy、fmt、layout、完整 `bun run check` 通过；
7. **不越界**：无语言、IR、Runtime ABI、对象布局或标准库公开契约变更；
8. **收口**：上述证据齐备后，才决定 07-D 原型阶段是否关闭；未齐备前保持“本地原型完成、
   外部证据待补”。

## 五、不负责

- 不接产品 Runtime、对象头、字节码或 LLVM；
- 不修改 `StrongHandle`/`WeakHandle` 布局或 `PhantomData` 边界；
- 不冻结跨平台 ABI、内存顺序或诊断字段；
- 不实现任务池、Actor、序列化、官方库或新阶段；
- 不登记 `H0-C`、`07-D 实现`、`std.concurrent` 或语言/API 名称；
- 不用单次测量、旧 CI 或单平台数字冻结 ABI。

## 六、落点与取证

```text
docs/DevDocs/07d12-closeout-doc-sync-and-cross-platform-evidence.md  本文
docs/DevDocs/07d8-atomic-handle-abi-prototype.md                      HEAD/CI/决策行
docs/DevDocs/07d9-abi-layout-cleanup-and-cost-evidence.md             workspace gates 行
docs/DevDocs/07d10-clean-abi-layout-and-evidence-closeout.md          状态
docs/DevDocs/07d11-abi-cost-semantics-and-evidence-closeout.md        F-1 状态
docs/DevDocs/00-decisions.md                                          第 6 项状态
docs/DevDocs/README.md                                                07-D.12 索引
tests/research/concurrency/src/atomic_abi.rs                         64 位入口/等待测量
tools/gates/run.sh                                                    原型测试入口
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

1. **再次文档漂移**：HEAD/CI 每次提交都会变；本批应写具体哈希和运行号，不再用“当前本地
   后继提交”。
2. **把外部证据当完成**：平台测试通过不等于跨平台测量；测量仍需单独记录。
3. **取消等待被忽略**：要么测，要么明确列为实现阶段缺口，不能含混写成完成。
4. **64 位边界**：只 guard `run_demo` 可能留下半套数据；入口边界要统一。
5. **单次测量噪声**：259/264、47/48 的波动正常；不要用单次差异得出性能结论。
6. **范围蔓延**：这是 07-D 原型收口，不是并发实现；保持隔离 crate。
