# 07-D.2. 并发入口裁定与平台收口

> **这是 07-D 的第二段收口，不是并发实现阶段。** 07-D.1 的本地证据已经由
> `f119ed0` 补齐并通过三平台维护回归；本批把提交归属、运行号和过期状态回填干净，
> 触发一次新的四平台复现，再把 8 项并发入口问题送进 [00. 决策基线](00-decisions.md)。
>
> **本批一句话**：**先把证据写对、把平台跑绿、把裁定问清；仍然不写并发语法、
> 执行器、`Send`/`Sync`、原子 RC、共享句柄或 `std.concurrent`。**
>
> 状态：**d6112e4 平台复现已全绿，8 项已裁定（2026-10-22），待最小 OS 线程原型**。
> 不修改 `xiao-syntax`、`xiao-types`、IR、Runtime ABI 或标准库公开契约；不登记新阶段代号。
> 平台复现是外部运行，不能用旧运行号或 ordered summary 的绿替代单平台结果。

## 一、Agent 交接上下文

### 1.1 接手前提

按顺序读：

1. [21A. 单线程 RC、强环与并发模型决策交接](21a-rc-cycles-and-concurrency-handoff.md)
   —— 权威边界、§五验收、§3.2 新阶段前置；
2. [07-D. 并发模型研究](07d-concurrency-model-research.md)
   —— 六项交付物、成本矩阵、两套 `Send`/`Sync` 候选、取消清理顺序；
3. [07-D.1. 07-D 收口与并发入口决策准备](07d-closeout-and-concurrency-decisions.md)
   —— 上一批的 F-1–F-6、裁定清单和本地实施记录；
4. [20L. 字典方法式 API 与 20K 收口](20l-dict-methods-and-20k-closeout.md)
   —— F-1–F-6 的来源和平台证据口径；
5. [06A. 生命周期静态闭环](06a-lifetime-static-closure.md) 与
   [06B. Runtime 对象与表生命周期](06b-runtime-objects-and-tables.md)
   —— 菱形 DAG、强环、`CrossThread` 无生产者和单线程句柄边界；
6. [12. 测试与开发里程碑](12-tests-and-milestones.md)
   —— B0/N0 前置和阶段出口；
7. [00. 决策基线](00-decisions.md) §待定决策——本批的裁定落点。

### 1.2 本批审核输入快照（2026-10-21 实跑/CI；收口结果见后续 F 段）

| 环节 | 现状 | 结论 |
| --- | --- | --- |
| `f119ed0` | 已推送；`origin/main` 指向 f119ed0，本地另有后继文档提交 | 07-D.1 的实现提交已进入远端 |
| 维护回归 `39517248193` | 六作业全绿：解析/安全、产物大小、Linux workspace、Windows/Linux/macOS 原生 | 三平台原生证据成立 |
| 原生夹具 | Windows/Linux/macOS 三个原生作业都运行 `first_modules_match_native_results ... ok` | `set-dict-alias` 已随 f119ed0 进入 CI |
| Doctest | Linux workspace 运行四个 `StrongHandle`/`WeakHandle` `compile_fail`，4 passed | Send/Sync 负向边界有 CI 证据 |
| 工作区门禁 `39517248607` | Linux workspace gates 成功，headSha 为 f119ed0 | 格式、Clippy、文档和测试门禁通过 |
| 菱形 DAG | `graph_accepts_shared_diamond_and_releases_child_once` 通过 | 共享子节点只释放一次，两个拥有者先于子节点 |
| 负向测试有效性 | 临时探针直接导入两个句柄，得到 `E0277`：`cannot be sent/shared between threads safely` | 四个 `compile_fail` 不是因导入/语法错误而空转 |
| 平台复现 `39227168623` | Windows 红：`xiao-doc-coverage-rust` 链接到 `C:\Program Files\Git\usr\bin\link.exe`；其余三平台和 ordered summary 绿 | 四平台证据仍不完整；见 F-3 |
| 平台脚本修复 | d6112e4 的 Cargo linker 环境变量修复已由 `39563381227` 验证 | Windows 不再命中 Git linker |
| 文档状态 | f119ed0、d6112e4 与 `39517248193`/`39517248607`/`39563381227` 已回填 | 见 F-4 |
| 并发裁定 | `00-decisions.md` 只登记“待裁定/研究深度边界”，8 项问题没有选择 | 入口阻断；见 F-4 |

### 1.3 本批边界

| 项 | 本批 | 说明 |
| --- | --- | --- |
| 文档归属/运行号回填 | **做** | 把 f119ed0 和两个 CI 运行号写进现有文档 |
| 平台复现 | **做** | 手动触发一次四平台运行，记录单平台结果和 ordered summary |
| 8 项并发裁定 | **收口** | 写入 00；没有裁定就保持后置，不代替星崽拍板 |
| `compile_fail` 加固 | **可选** | 本批不阻塞；如需长期防误报，另立 `trybuild`/正向探针任务 |
| 并发语法/执行器 | **不做** | 等新独立阶段 |
| `Send`/`Sync`/`async`/`Actor`/`Weak` | **不做** | 不新增语法或公开 API |
| 原子 RC/共享句柄/对象头 | **不做** | 不修改 Runtime 布局或 ABI |
| 新阶段代号/`std.concurrent` | **不预留** | 入口条件满足前不登记 |

## 二、审核发现与前置返工

### F-1 **提交归属写错、运行号未回填（已关闭）**

上一批文档曾写：

> `2248e81 ... 新增 set-dict-alias 原生夹具、空日志拒绝和 07D 证据补充`

实际归属已收窄：`2248e81` 只提供夹具和空日志修复，07D 证据、菱形 DAG、Send/Sync
doctest 和文档对账由 `f119ed0` 承载；`f119ed0` 已推送，`39517248193` 与
`39517248607` 的 `headSha` 均为 f119ed0。d6112e4 是平台复现代码基线，本批后续文档提交，
尚未有新的 CI 运行号。

**处理结果**：

1. 已把提交归属统一为 f119ed0（包含 2248e81）；
2. 已在 07-D.1、07-D、20L、README 回填：
   - 维护回归 `39517248193`（六作业全绿，三平台原生运行首批模块夹具）；
   - 工作区门禁 `39517248607`（Linux workspace 成功）；
   - 四个 `compile_fail` 在 Linux workspace 的 4 passed；
3. 历史提交只在说明来源的位置保留，当前状态均写 f119ed0 或 d6112e4 平台基线。

### F-2 **07-D §1.2 曾是过期状态（已关闭）**

`07d-concurrency-model-research.md` 的旧“现状盘点”曾写：

- `:43` 20L 回归“**阻断**”；实际 `39459004148` 三平台原生已绿；
- `:44` “最新测试 `1393cea` 未推送；平台复现仍是旧红”；`1393cea` 已在
  `origin/main` 历史中，52b897e 也已推送。

同一文档 §9.1 已按 f119ed0 的新运行号更新，当前 §1.2 已明确为审核时快照；
07D.1 的 F-2 段也已改成关闭记录，不再把历史快照当作当前缺口。

**处理结果**：§1.2 已标成“审核时快照”，F-1/F-2 已标成关闭记录；当前 HEAD 的平台
外部门禁已由 `39563381227` 单独核销。

### F-3 **平台复现（已关闭）**

历史平台复现 `39227168623` 曾因 Git `link.exe` 命中而红；当前平台复现 `39563381227`
已绑定 d6112e4，四个平台作业与 ordered summary 全绿，Windows Cargo linker 和 macOS
SGR/298 pass 证据已回填。

**关闭标准已满足**：最新有效平台 HEAD d6112e4 有四平台运行且 Windows 绿。

### F-4 **8 项并发入口问题仍未裁定（P1，入口阻断）**

`00-decisions.md` 目前只记录“桌面定性研究、不是测量成本”和“八项问题待裁定”。在
星崽给出选择前，不能建立并发实现阶段，也不能预留 `H0-C`、`07-D 实现`、
`std.concurrent` 或任何语法/API 名称。

**处理要求**：按 §三逐项记录“选择”或“明确后置”；若选择后置，保持 07-D 入口条件不变，
不要把“暂不决定”写成“已决定不做”。

### F-5 **00 中“后置”措辞有歧义（已关闭）**

上一轮审核曾记录 `00-decisions.md` 的“八项入口问题目前全部明确**后置**、未裁定”。
当前 `00-decisions.md` 已改成“均待星崽裁定；本批只记录候选与推荐”，不预先决定后置
或不做。

### F-6 **`compile_fail` 的长期防误报能力有限（P3，可选）**

当前四个 doctest 已确认因 `E0277` 失败，不是空转；但 `compile_fail` 只要求任意编译
失败。未来若导入、类型名或示例本身坏掉，仍可能误绿。若要把边界长期钉死，可另立
`trybuild` 或“正向导入探针”任务；本批不阻塞。

## 三、8 项并发入口裁定

本批只整理，不代替星崽拍板。裁定结果写入 `00-decisions.md` 的
“并发模型待定/已定”段落。

| # | 待裁定项 | 选项 | 当前推荐（非冻结） | 影响 |
| --- | --- | --- | --- | --- |
| 1 | 执行机制 | OS 线程 / 单线程任务 / 任务池 / Actor / 多 Runtime / 组合 | 单线程任务 + 消息隔离先做隔离原型 | 决定 IR 事实、调度器和 ABI 规模 |
| 2 | `Send`/`Sync` | 候选 A 递归能力推导 / 候选 B 边界值优先 / 两者原型 | 首版按候选 B；只读共享后置 | 决定类型层和值传递入口 |
| 3 | `CrossThread` | 保持一个事实 / 拆任务边界与线程边界 | 暂保持一个事实 | 拆分会影响 IR 与诊断字段 |
| 4 | 复制/序列化失败 | 深拷贝 / 版本化序列化 / 冻结 / 共享可变 | 深拷贝先做，序列化后置；共享可变暂缓 | 决定错误和清理顺序 |
| 5 | 取消清理顺序 | 07-D §10.4 的“标记取消 → 停止接纳 → drain → 安全点 → finally/drop → 释放边界值 → 汇总错误” | 作为首版验收候选 | 决定 VM/LLVM 黄金场景 |
| 6 | 原子共享句柄 | 首阶段做 / 后置 / 不做 | 后置，不承担对象头、锁和跨平台 ABI 成本 | 决定 06/ABI 的工作量 |
| 7 | 官方库结构 | 双向链表/树/观察者/图在模型冻结后进入 | 模型冻结后再评估 | 避免用包装层掩盖所有权问题 |
| 8 | 研究深度 | 只保留定性记录 / 补可执行原型 | 若用于冻结，先补原型；否则只作参考 | 决定成本数据是否可信 |

记录格式建议：

```text
### 并发模型（2026-10-21 星崽裁定）
- 执行机制：待定 / 选择 X
- Send/Sync：待定 / 候选 A / 候选 B
- CrossThread：保持一个事实 / 拆分
- 复制与序列化：待定 / 选择 X
- 取消清理顺序：待定 / 采用 07-D §10.4 的候选序列 / 修改为 X
- 原子共享句柄：后置 / 不做 / 进入首阶段
- 官方库结构：模型冻结后评估 / 其他
- 研究深度：定性记录 / 补可执行原型
```

## 四、实施步骤

| 步 | 内容 | 验证 |
| --- | --- | --- |
| 0 | 回填 f119ed0、d6112e4 与 `39517248193`/`39517248607`/`39563381227`；修正提交归属和过期状态 | 文档 diff；grep 对账 |
| 1 | 手动触发平台复现，记录四平台作业和 ordered summary | 新运行号 |
| 2 | 把 8 项裁定或明确后置写入 00；更新 07-D §11.3 与 21A 引用 | `00-decisions.md` |
| 3 | 收口审核：定向测试、`XIAO_GATE_SELF_TEST=1`、文档/覆盖率门禁 | 本地/CI 证据 |
| 4 | 可选：评估 `trybuild`/正向探针是否值得为 Send/Sync 边界另立任务 | 结论记录 |

**本批不写产品代码**；除文档与测试辅助外，不改 `xiao-syntax`/`xiao-types`/IR/Runtime
ABI/标准库公开契约。`52b897e` 的 codegen 修复保持现状，不做补丁叠补丁。

## 五、验收

1. **文档归属**：现有文档不再把 07D 证据归给 2248e81；维护 CI 归属 f119ed0，平台证据绑定
   d6112e4；`39517248193`、`39517248607` 和 `39563381227` 已回填；`grep` 无未标注的
   “1393cea 未推送”；
2. **平台复现**：d6112e4 有一次新的四平台运行；Windows 绿，历史红点已定位并完成复跑；
   ordered summary 只作为汇总，不作为单平台绿的替代；
3. **CI**：f119ed0 的维护回归与工作区门禁全绿，d6112e4 的四平台复现全绿；三平台原生作业实际运行
   `first_modules_match_native_results`；后继纯文档提交使用绑定的 workspace 门禁证据。
4. **裁定**：00 中 8 项已明确为“待星崽裁定”，没有把候选或推荐写成选择；未登记新阶段代号、语法或公开 API；
5. **不越界**：无并发语法、`Send`/`Sync`/`async`/`Actor`/`Weak`、原子 RC、共享句柄、
   IR/ABI/标准库公开契约变更；
6. **门禁**：定向测试通过；`XIAO_GATE_SELF_TEST=1` 通过；文档覆盖率 ≥90%；完整门禁
   或 CI 对最终 HEAD 通过。

## 六、不负责

- 不实现线程、任务池、Actor、协程、`async`/`await`、`spawn`/`send`；
- 不实现 `Send`/`Sync` 类型能力、冻结、共享可变、Weak 语法或 `std.concurrent`；
- 不实现原子 RC、共享句柄、对象头/锁或新的 Runtime ABI；
- 不修改现有单线程 `StrongHandle`/`WeakHandle` 布局和 `PhantomData` 边界；
- 不把 07-D 的研究伪代码或成本排序写成已冻结设计；
- 不替星崽做 §三的 8 项裁定；没有裁定时保持后置。

## 七、落点与取证命令

```text
docs/DevDocs/07d2-entry-decisions-and-platform-closeout.md  本文
docs/DevDocs/07d-closeout-and-concurrency-decisions.md      F-1/F-2 状态与实施记录回填
docs/DevDocs/07d-concurrency-model-research.md              §1.2/§9.1/§11.3 状态与运行号
docs/DevDocs/20l-dict-methods-and-20k-closeout.md           F-6 平台证据回填
docs/DevDocs/21a-rc-cycles-and-concurrency-handoff.md       验收 1–3 证据引用
docs/DevDocs/00-decisions.md                                8 项并发裁定/后置记录
docs/DevDocs/README.md                                      07-D.2 索引与状态
```

```bash
# 文档对账（2248e81/1393cea 只能出现在历史来源说明中）
grep -rn "2248e81\|1393cea" docs/DevDocs

# 推送证据
gh run view 39517248193 --json jobs --jq '.jobs[] | "\(.name) \(.conclusion)"'
gh run view 39517248607 --json jobs --jq '.jobs[] | "\(.name) \(.conclusion)"'
gh run view 39517248193 --job <Windows-native-job-id> --log | grep "first_modules_match_native_results"

# 平台复现（在当前 HEAD 上手动触发）
gh workflow run 365649226 --ref main
gh run list --workflow 365649226 --limit 3
gh run view <run-id> --json jobs --jq '.jobs[] | "\(.name) \(.conclusion)"'

# 本地门禁
cd core/rust
cargo test --workspace --no-fail-fast
cargo clippy --workspace --all-targets -- -D warnings
cd ../..
bun run check:docs
bun run check:coverage
XIAO_GATE_SELF_TEST=1 bash tools/gates/run.sh
```

## 八、风险与注意事项

1. **平台复现不是 push 触发**：`39517248193` 和 `39517248607` 只证明 f119ed0 的维护/工作区；
   四平台证据必须手动触发，且定时任务每周才跑一次。
2. **Windows `link.exe` 可能再次命中**：`e7597c4` 只改了脚本 PATH；必须以新运行验证，
   不能只凭脚本 diff 认定已修。
3. **裁定可能继续后置**：没有星崽选择时，不要用“推荐方案”冒充裁定，也不要提前写
   `Send`/`Sync` 或任务语法。
4. **提交归属容易再漂移**：后续提交会不断前移 HEAD；文档中写“当前 HEAD”时要同时
   写提交号，避免只写中间提交或“最新提交”。
5. **`compile_fail` 只保证编译失败**：当前已确认因 `E0277` 失败；长期若需要防误报，
   另立 `trybuild`/正向探针任务，不在本批偷偷扩依赖。
