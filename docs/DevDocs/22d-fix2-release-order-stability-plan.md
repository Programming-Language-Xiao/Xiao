# 22D-FIX.2. 并发释放差分稳定性修复规划

> **这是 22D 的第二个收尾补丁批，不是 B3-4，也不是 22E 的返工。**
> 22D-FIX.1 在 `ab92df1` 上完成了当时的 12/16/1 黄金矩阵；但 2026-11-19 之后的
> B6-4、B7 和 B3-3 维护记录持续出现三处释放差分失败，`22d_open=true` 仍然有效。
> 本批只处理这三处间歇失败：先把“投影误报”和“真实释放缺失”分开，再修判据或 Runtime，
> 最后用三平台重复执行关闭 22D。**本批不改语言 API、ABI、Send/Sync、诊断编号、
> 标准库函数体或 registry。**
>
> 状态 **已验收（2026-11-27，`22d_open=false`）**。规划基线 `0dd9c7b7`，入口产品
> `2460e75f`（ABI 1.20、生成器 25、标准库接口 1）。修复产品 `3506455e` 的生成器版本为 26。
> B3-3 的 fs 已完成内部验收，B3-4 未启动。
> 三平台重复差分、专项目标与门禁均已通过。[收口证据](assets/22d-fix2/entry-20261126/README.md)

## 一、入口事实与当前矛盾

### 1.1 三条已知失败

| 编号 | 夹具 | 首次失败身份 | 断言事实 | 后续观测 |
| --- | --- | --- | --- | --- |
| F1 | `native-lock-wait-027` | `97d5f288`，B6-4 Windows 原生维护，run `44464422497` / job `135358661620` | 原生稳定轨迹只有一组 `shared_destroy`/`weak_release`，VM 有两组 | 同提交重跑两次通过；Runner 镜像由 `20261103.298.1` 换为 `20261116.315.1`；本机单夹具 25 轮通过 |
| F2 | `xcon-bounded-string-success` | `58176ff5` 本机完整入口第 2 轮。Windows 维护以产品 `2460e75f`、Runner 提交 `97ba4b89` 执行，attempt 1，job `139867799304` | 原生多出被其他对象隔开的 `strong_release` 别名，两侧均有三个 `destroy` | 同一源码的 attempt 2 通过，job `139878903980`。本机失败轮保留两侧原始轨迹 |
| F3 | `spawn-join-str-multiargument` | `58176ff5` 本机完整入口第 4 轮 | VM 多出被其他对象隔开的 `strong_release` 别名；两侧均有三个 `destroy` | 仅本机完整入口出现；单夹具未单独复现 |

B3-3 修复归档在 `windows_maintenance.22d_open=true` 仍明确记录 F2；B6-4、
B6-5、B7 和 B2-C 的交接也继续使用 `22d_open=true`。因此不能把 22D 的“已完成”
当作当前状态。

### 1.2 文档矛盾的处理

22D §10 和 DevDocs 索引的 22D 行仍写着“已完成（2026-10-31）”。这应解释为
**22D-FIX.1 在 `ab92df1` 上的验收点已完成**，而不是“后续维护回归不会重开阶段”。
本批不改写 `ab92df1`、B6-4 或 B3-3 的历史证据，只做三件事：

1. 在 22D 状态头、§10、§11 和索引行增加“后续维护回归重开、`22d_open=true`”的说明；
2. 新建本页，承接三处失败；
3. 在本批关闭后新建派生收口记录，不改写 B3-3 `status.json` 中当时的快照字段。

### 1.3 修复前的判据

入口快照的投影位于 `core/rust/crates/xiao-driver/tests/22d_llvm_concurrency.rs`：

- `release_trace_sequence` 过滤 `concurrency_release`，按首次出现归一化对象 ID，
  只压缩**相邻且完全相同**的事件；
- `assert_case_release_trace` 对普通成功/错误场景比较整个事件向量，对四个取消场景
  只比较 `shared_destroy`/`weak_release`，对 detached 允许独立打印对象交换。

22D §5.2 已经写明：“临时别名引用计数不属于差分相等条件”。但实现只压缩相邻重复，
没有压缩“被其他对象隔开的同一对象别名释放”。F2/F3 的 `left`/`right` 正是这种形态：

```text
F2 原生：strong(0), strong(1), strong(0), destroy(0), strong(1), destroy(1), strong(2), destroy(2)
F2 VM  ：strong(0), destroy(0), strong(1), destroy(1), strong(2), destroy(2)
F3 VM  ：strong(0), strong(1), destroy(0), strong(1), destroy(1), strong(2), destroy(2)
F3 原生：strong(0), destroy(0), strong(1), destroy(1), strong(2), destroy(2)
```

两侧终态对象数相同，缺失的不是 `destroy`，而是别名计数被交错后进入了比较。
这与 §5.2 的声明不一致，属于**投影实现缺陷候选**，不是可以靠重跑消除的噪声。
F1 不同：它缺少一个稳定的 `shared_destroy`/`weak_release` 对，不能只靠改投影解释。

开工核对使用 Windows 的 `2460e75f` Runtime 和 Rust 1.96.0。隔离工作树的临时测试入口
保留现有断言和原始轨迹，完整执行通过 29 个夹具，保留 58 份 TSV。F1 单独运行 100 轮，
每轮 VM 和原生各保留两组共享对象终态。历史失败只有断言输出，F1 仍待归因。
对 F2/F3 的本机历史失败轨迹按对象压缩强引用释放，两侧均有三个销毁且终态动作流一致。
[历史轨迹来源](assets/20o-b2-b/entry-20261119-b6-4-followup/README.md#windows-22d)

## 二、范围与不负责

### 2.1 本批要做

1. 固定基线、复现三处失败并保留原始 VM/原生轨迹；
2. 判断 F2/F3 是否仅为别名投影误报，判断 F1 是真实释放缺失、轨迹丢失还是判据问题；
3. 修正释放投影口径，使 §5.2 的“别名计数不参与相等”真正生效；
4. 若 F1 是 Runtime 清理/等待缺陷，修产品清理顺序；若只是轨迹丢失，修轨迹观察；
5. 增加正反例，证明新投影没有放宽到“删掉 `destroy` 也能过”；
6. 在三平台重复执行并重新封存证据，最后才关闭 22D。

### 2.2 明确不做

- 不启动 B3-4 `env`，不实现 B3-5/B3-6/B3-7；
- 沿用 ABI 1.20、标准库接口 1，以及 `std.concurrent` 的既有签名、32B/16B 布局、标签 18 和错误 027–032。F1 的修复会改变原生指令，生成器由 25 升到 26，以区分同源旧缓存。
- 不修改 `stdlib/concurrent.xiao` 函数体、原生产物元数据、required-feature 或 registry；
- 不跑旧 721，不核销 19/11/22 的其他历史债务；
- 不修改 B6-4/B6-5/B7/B3-3 的历史日志、原始作业身份或已封存轨迹；
- 不把“重跑通过”当作 22D 关闭证据；
- 不用删除断言、跳过夹具、扩大可交换集合或改超时来换绿色。

## 三、三例归因方案

### 3.1 F2/F3：先按“投影误报”处理

现有证据已经足以把 F2/F3 与 F1 分开：

- F2 的失败轮两侧各有三个对象、三个 `destroy`；差异只是对象 0/1 的额外
  `strong_release` 出现在另一个对象事件之间；
- F3 同理，只是额外别名在 VM 侧；
- 两者都没有缺失 `destroy`、`shared_destroy` 或 `weak_release`；
- 失败发生在完整入口的后续夹具，而单夹具或重跑可通过，符合“别名交错位置变化”。

因此先实施投影 v2（§四）。**在投影 v2 下重新执行 F2/F3，如果仍失败，再按
真实对象缺失升级为 Runtime 缺陷，不得把 v2 当作无条件放行。**

### 3.2 F1：不能只改投影

F1 的断言是：

```text
left : [(0, "shared_destroy"), (0, "weak_release")]
right: [(0, "shared_destroy"), (0, "weak_release"), (1, "shared_destroy"), (1, "weak_release")]
```

这里不是别名计数差异，而是一个共享对象的完整终态组缺失。按以下顺序判别：

| 假设 | 判别证据 | 处置 |
| --- | --- | --- |
| H1 真实清理缺失 | `join` 完成时 worker 持有的部分对象仍未释放，包括取消展开漏归还临时值或等待提前返回 | 修实际持有引用的清理路径，增设受控原生取消与 Runtime join 不变量回归 |
| H2 轨迹丢失：事件已发生，但文件未写入或进程退出前未落盘 | 同一失败轮内存事件计数完整，文件缺少事件；或 `flush` 与 worker 追加并发时丢行 | 修轨迹会话/落盘/同步；不改语言语义 |
| H3 投影误合并：`shared_destroy` 与 `weak_release` 被错误归到同一对象或过滤 | 原始 TSV 中存在第二个对象的两个事件，投影后消失 | 修投影；必须用原始轨迹证明 |
| H4 夹具本身少持有一个对象 | 源码参数或对象计数与预期不符 | 只修夹具/黄金，说明原失败不可归因于产品 |

H1 与 H2 的区分须在失败轮保留完整原生 TSV、VM TSV、进程退出码、
Runner 镜像、Runtime/工具 SHA、对象分配/销毁计数、以及 `22D-CASE` 前后日志。
如果只能拿到断言投影，必须重跑到能保留原始轨迹为止，不能直接写“已定位”。

F1 的本地 25 轮通过只覆盖一个夹具，不能证明 Rare H1 不存在；本批把 F1 的重复轮次
提高到 §六的门槛，并要求至少一次失败样本或等价 Runtime 回归。

### 3.3 对象身份与跨后端顺序

当前 `release_trace_sequence` 按“首次释放出现顺序”归一化对象 ID。这个 ID 不是语言
可观察身份，VM 和 native 的分配顺序也不保证相同。投影 v2 不再把跨对象身份当作语义：

- 比较每个对象的**生命周期形状**和数量；
- 比较按生命周期签名标注的**终态动作流**：同签名对象互换不区分，不同签名对象的
  先后顺序保留；
- 不要求两个后端把同一个内部对象分配到同一个序号。

这不是放宽到“只看有没有任何事件”，因为 `destroy`/`shared_destroy`/`weak_release`/
`lock_guard_release` 的数量、每个对象的生命周期形状和终态动作顺序仍逐项核对。

## 四、投影 v2 设计

### 4.1 算法

对每个夹具的 TSV 执行：

1. 校验行号连续、字段数正确；任何格式错误直接失败；
2. 丢弃 `concurrency_release`（native 包装层自身的引用归还，不是语言对象释放）；
3. 按原始 `object_id` 分组，得到每个对象的事件子序列；
4. **按对象子序列压缩相邻相同动作**：同一对象连续出现多个 `strong_release`/
   `shared_release` 时只保留一次，即使这些事件在全局文件中被其他对象隔开；
5. 校验生命周期形状：
   - 含 `destroy` 的对象：恰好一次 `destroy`，之前至少一次 `strong_release`，之后无事件；
   - 含 `shared_destroy` 的对象：恰好一次 `shared_destroy`，之前至少一次
     `shared_release`，之后至少一次 `weak_release`，再之后无事件；
   - 含 `lock_guard_release` 的对象：恰好一次，且没有其他动作；
   - 只有被过滤动作的对象不进入比较；其他形状直接失败；
6. 比较两侧的**对象生命周期多重集合**（排序后逐项相等）；
7. 比较两侧的**终态动作流**：按文件顺序只保留 `destroy`、`shared_destroy`、
   `weak_release`、`lock_guard_release`，以每个对象的生命周期签名作为标签；同签名
   对象互换不区分，不同签名对象的先后顺序保留，同时比较动作名和计数；
8. 对每个夹具额外打印 `22D-TRACE` 摘要：夹具名、VM/native 对象数、终态事件数、
   投影版本；摘要进入原始日志。

“生命周期签名”指第 4 步压缩后的单个对象动作序列，例如 `[strong_release, destroy]`
或 `[shared_release, shared_destroy, weak_release]`。标签只用于终态流的跨对象顺序，
不把原始对象 ID 当作跨后端身份。

### 4.2 特殊夹具

| 夹具类别 | 口径 |
| --- | --- |
| 普通成功/错误 | 投影 v2 全量生命周期多重集合 + 终态动作流 |
| `detach-success` | 保留“独立打印对象可交换”的声明；每个对象的生命周期形状仍按 v2 校验 |
| 四个取消夹具 | 先校验生命周期形状，再比较稳定共享对象。由 `shared_release` 或 `shared_destroy` 识别共享对象，缺少共享对象销毁或弱引用释放即失败。两侧各须有 `lock_guard_release`，轮询临时 guard 的数量及交错随调度变化 |
| `native-running-cancellation-finally` | 继续核对 027、退出码、finally 的精确输出及两侧各有释放记录 |
| `lock-guard-scope-release` | 额外要求恰好两个不同 guard 对象，各一次 `lock_guard_release` |

F1 属于取消夹具。两侧各须观察到 `gate` 和 `ready` 对应的两组完整
`shared_destroy`/`weak_release` 终态，再比较投影。任一侧缺一组即失败。

### 4.3 正反例

投影 v2 必须带以下自反例，输出到临时文件，不改写历史轨迹：

| 编号 | 构造 | 期望 |
| --- | --- | --- |
| P-1 | 在对象 0 和对象 1 的事件之间插入额外 `strong_release` | 通过；证明别名交错不再误报 |
| P-2 | 删除一个对象的 `destroy` | 失败；不能漏真实终态 |
| P-3 | 复制一个 `destroy` | 失败 |
| P-4 | 删除一个 `shared_destroy` 后的 `weak_release` | 失败 |
| P-5 | 删除一个 `lock_guard_release` | 失败 |
| P-6 | 把 `shared_destroy` 改成 `shared_release` | 失败；生命周期形状不成立 |
| P-7 | 两个**生命周期签名相同**的独立对象交换终态动作顺序 | 通过；同签名对象的内部身份不是语言可观察量 |
| P-8 | 保留 `concurrency_release` 但不改变其他事件 | 通过；包装释放被过滤 |
| P-9 | 交换两个**生命周期签名不同**的对象终态顺序 | 失败；跨对象释放先后仍受约束 |

P-1 和 P-2 必须同时存在；只有 P-1 会退化成“删断言”，只有 P-2 会保留原误报。

## 五、F1 的 Runtime 调查与修复

### 5.1 调查入口

1. 固定工作入口 `0dd9c7b7`、产品 `2460e75f` 及实际测试所用的 Runtime、LLVM、Clang、Runner 镜像和任务根。
2. 用只选择 `native-lock-wait-027` 的临时测试入口运行至少 100 轮，不修改源码夹具；
3. 每轮保留原生/VM TSV 和进程回执；失败轮立即复制原始文件，不重跑覆盖；
4. 在 Runtime 层增加 join 后不变量测试。夹具归还自己的强引用，仅保留观测用弱句柄。
   取消并 join 持有共享对象的 worker 后，确认弱句柄无法升级，再归还观测句柄并核对最后的弱释放事件。
5. 对照 `xiao-runtime/src/scheduler.rs` 的 `join_interruptible*`、
   `xiao-runtime/src/abi_concurrency.rs` 的 `join`/`finish` 与
   `xiao-runtime/src/concurrency.rs` 的 `SharedHandle` 释放路径，定位返回时点。

### 5.2 修复分支

- **若 H1 成立**：修 `join`/`finish`/取消清理，使返回 027 前 worker 帧拥有的共享句柄
  已释放；或让入口收尾等待所有由本次入口拥有的工作线程，而不只是 detached 回调。
  修复后必须新增 Runtime 回归，证明“join 返回后对象已释放”，不能只靠 TSV 变绿；
- **若 H2 成立**：修轨迹会话的开始/刷新/退出顺序，必要时在入口结束显式刷新并等待
  已登记写入；不得把事件丢失写成语言层成功；
- **若 H3 成立**：只修投影，并保留原始 TSV 证明第二组事件存在；
- **若 H4 成立**：只修夹具并撤回“产品缺陷”表述，登记原失败不可归因。

### 5.3 产品与证据边界

如果 F1 修复触碰 Runtime 清理/等待代码，产品提交会晚于 `2460e75f`。此时：

1. B3-3 的 fs 证据必须在新产品提交上重跑至少 fs 专项和三平台 fs 教程，
   或明确登记“B3-3 证据只覆盖旧产品，新 Runtime 变更未回验”；
2. 不得继续用 `2460e75f` 作为 B3-4 的唯一入口产品；
3. 如果只是测试/投影/观察代码变更，不改 Runtime 行为，则 B3-3 产品身份可保持
   `2460e75f`，但仍需重跑受影响的 22D 与 fs 精确用例。

本次改动在 LLVM 并发调用的失败清理边，实际 Runtime 实现沿用入口版本。生成器升到 26
会改变 B3-3 产物指纹，因此按产品变更要求回验 fs 专项、三平台教程与 Windows COFF `Complete`。
验收还读取生成器 24 和 25 的封存产物，核对旧元数据仍可验证。

## 六、执行批次

### 22D-FIX.2A：固定证据与投影

1. 固定基线、三处失败日志、Runner 镜像、工具/Runtime 身份；
2. 新增原始轨迹保留目录和 `22D-TRACE` 摘要；
3. 实现投影 v2 和 P-1～P-9 自反例；
4. 先用 F2/F3 的原始轨迹离线验证：v2 应判通过；P-2～P-6 应判失败；
5. 不修改 Runtime，不关闭 22D。

退出：投影 v2 的正反例可复核；F2/F3 的原始轨迹在 v2 下有明确结论。

### 22D-FIX.2B：F1 归因与修复

1. 运行 F1 100 轮并保留失败样本；
2. 按 H1～H4 判别，写出证据链；
3. 若为 H1/H2，实施 Runtime/轨迹修复并新增对应回归；
4. 若为 H3/H4，只修投影/夹具并说明原失败不可归因于产品；
5. 重新执行 F1 100 轮；不允许用重试次数掩盖失败。

退出：F1 的根因有原始证据；修复或归因结论能解释“为什么此前 25 轮通过”。

### 22D-FIX.2C：三平台重复执行

1. Windows 完整 22D 差分至少 10 轮，单夹具 F1/F2/F3 各至少 50 轮；
2. Linux、macOS 完整 22D 差分各至少 10 轮，单夹具各至少 30 轮；
3. 每轮记录实际退出码、`22D-CASE` 行、对象数/终态事件数；失败轮保留两侧 TSV；
4. 三平台原生 maintenance 和 workspace gates 使用实际 run/job/attempt；
5. 记录 22D 内部回归产物的实际验证等级。B3-3 复验的 Windows fs 产物沿用产品级
   COFF `Complete` 验收。

退出：无失败、无跳过的重复执行矩阵；失败/重试不计作通过。

### 22D-FIX.2D：收口与状态传播

1. 更新 22D 状态、§5.2、§11 和索引；
2. 新增本批证据目录与机器可读 `status.json`；
3. 只在全部退出条件满足后写 `22d_open=false`；
4. 说明 B6-4 的 `b6_4_passed=false` 不会因 22D 关闭自动变真；
5. B3-4 仍未启动；若 Runtime 变更，B3-3 按 §5.3 重新绑定产品。

## 七、验证矩阵

| 层 | 必测 |
| --- | --- |
| 投影 | F2/F3 原始轨迹离线回放；P-1～P-9 正反例；对象数/终态事件数摘要 |
| F1 | 100 轮单夹具；失败轮原始 TSV；Runtime join 后不变量 |
| 差分 | 22D 12/16/1 全量、`native_concurrency_llvm_assembly_fixtures`、C 布局探针 |
| Runtime | 入口产品的全量 Runtime 测试、`native_cleanup_checkpoint_defers_repeated_cancellation`、取消四位置。以实跑回执登记当前总数 |
| 22C | 26/26 VM 回归不退化 |
| B3 | fs 受控原生/VM 教程和排他重命名专项；B1 七库精确入口不退化 |
| 平台 | Windows/Linux/macOS 完整与单夹具重复执行；实际 run/job/attempt |
| 门禁 | workspace、clippy、fmt、docs/usedocs/layout、`git diff --check` |
| 边界 | 无 API/ABI/诊断/stdlib/registry/721 改动 |

## 八、退出条件

1. F2/F3 的失败已用原始轨迹证明为别名投影，或已修真实对象缺失；
2. F1 的根因有原始证据，H1/H2/H3/H4 中恰有一个结论，且修复/归因可复核；
3. 投影 v2 的 P-1、P-7、P-8 通过，P-2～P-6 和 P-9 均按预期拒绝。终态的数量、形状和顺序均通过正反例核对。
4. Windows 单夹具 50 轮、Linux/macOS 各 30 轮无失败；完整 22D 三平台各 10 轮无失败；
5. 22C 26/26、Runtime 现有回归、B3 fs 受影响入口和完整门禁无退化；
6. 三平台实际 CI 证据齐全，失败/重试保留，不以单平台或本地日志代替远端；
7. 若 Runtime 变更，B3-3 产品身份已重新绑定并回验；若未变更，明确记录仅测试/投影变更；
8. 以上满足后才写 `22d_open=false`；B6-4/B6-5/B7 的其他 false 字段不变，B3-4 仍未启动。

## 九、风险与禁止

- **把投影修订做成删断言**：只允许压缩别名计数，不允许删除 `destroy`、
  `shared_destroy`、`weak_release`、`lock_guard_release` 的数量/形状比较；
- **把 F1 当 F2**：缺一组稳定共享对象不是别名交错，不能用 v2 放行；
- **重跑到绿**：重试通过只作为复现信息，不作为关闭证据；
- **覆盖失败样本**：原始 TSV、作业号、Runner 镜像和断言输出只追加，不改写；
- **改 Runner 镜像/超时掩盖**：环境变化必须登记，不能用镜像升级代替根因；
- **顺手扩大范围**：不实现 B3-4、不改标准库、不改 ABI/诊断、不跑 721；
- **把 22D 关闭写成 B6-4 通过**：B6-4 还有 Windows COFF 和
  `followup_passed=false`，22D 只是其中一个条件。

## 十、落点与待批准项

### 10.1 代码与证据落点

```text
docs/DevDocs/22d-fix2-release-order-stability-plan.md          本规划
docs/DevDocs/22d-llvm-concurrency-differential.md              状态、§5.2、§11 更新
docs/DevDocs/assets/22d-fix2/entry-20261126/                    本批证据
core/rust/crates/xiao-driver/tests/22d_llvm_concurrency.rs      投影 v2 与轨迹保留
core/rust/crates/xiao-driver/tests/concurrency_closure/         仅在需要时修夹具
core/rust/crates/xiao-runtime/src/scheduler.rs                  F1 若为 H1 的等待/清理
core/rust/crates/xiao-runtime/src/abi_concurrency.rs            join/finish 与轨迹刷新
core/rust/crates/xiao-runtime/src/memory/mod.rs                  F1 若为 H2 的轨迹会话
```

### 10.2 实施口径

| 编号 | 事项 | 本批执行 |
| --- | --- | --- |
| P-1 | 按对象压缩别名 | 采用 §四的投影 v2，核对生命周期多重集合和终态动作流 |
| P-2 | 三平台重复轮次 | Windows 单夹具 50 轮，Linux/macOS 单夹具 30 轮，完整入口各 10 轮 |
| P-3 | F1 产品修复与 B3-3 回验 | 同一批修复实际丢失实参的路径，使用修复产品回验 B3-3 |
| P-4 | 关闭条件 | 按 §八检查受控失败证据和实际三平台回执 |

## 十一、新 Agent 先读

1. [22D. LLVM 并发执行、差分与跨平台](22d-llvm-concurrency-differential.md) §5.2、§9.3、§11；
2. [22D-FIX.1. 并发差分矩阵收尾](22d-fix1-concurrency-differential-closure.md)；
3. 本页 §三、§四、§五；
4. `core/rust/crates/xiao-driver/tests/concurrency_closure/release_trace.rs` 的生命周期校验和
   `22d_llvm_concurrency.rs` 的同源差分入口；
5. [B6-4 后续证据](assets/20o-b2-b/entry-20261119-b6-4-followup/README.md) 的
   Windows 22D 段和原始失败日志；
6. [B3-3 修复复验](assets/20o-b3/entry-20261126-b3-3-rename-fix/README.md) 的
   `windows_maintenance`。

## 十二、实施记录

F2/F3 的两对历史 TSV 已离线通过投影 v2。VM 和原生各销毁三个对象，归并各对象的强引用
别名后，终态动作流逐项一致。P-1～P-9 分别命中预期的接受与拒绝结果。

F1 归入 H1。受控回归在 worker 写入 `ready` 并复制 `lock(gate)` 的实参之后，固定在
原生锁入口的取消预检查处消费取消。旧版错误边直接进入外层展开，跳过了实参释放。
原生以退出码 3 回报 027 和清理输出，但 `gate` 的轨迹只有 `shared_release`，另一个
共享对象有完整的 `shared_destroy` 和 `weak_release`。VM 保留两组。修复让实参求值、
取消检查和 ABI 调用的错误边共用清理块，按逆序归还已经求值的实参。受控运行再次观察到
两组终态，Runtime 的弱句柄检查同时确认真实锁阻塞取消后，`join` 返回时对象已经销毁。

Windows 历史的 25 轮和当前修复前的 100 轮都未命中取消预检查窗口。取消若由锁 ABI 返回，
旧代码已经有实参归还路径。原始远端失败只有投影断言，本批使用同一夹具的受控回归记录
缺失的原始事件。修复后 Windows 单跑 100 轮均观察到两组终态。

三平台完整差分各 10 轮，Windows 三个单夹具各 50 轮，Linux 和 macOS 各 30 轮，均通过。
汇编和 C 布局探针、Runtime 取消回归、22C 既有 26 项和后续新增 3 项、B1 七库精确入口、
三平台原生维护与工作区门禁均已对账。B3-3 已在 `3506455e` 回验，B3-4 从该产品接续，
当前尚未启动。B6-4 按该批自身门禁仍为 `b6_4_passed=false`。[机器可读回执](assets/22d-fix2/entry-20261126/status.json)
