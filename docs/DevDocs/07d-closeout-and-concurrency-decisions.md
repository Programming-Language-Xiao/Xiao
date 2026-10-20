# 07-D.1. 07-D 收口与并发入口决策准备

> **这是 07-D 的收口批次，不是并发实现阶段。** 07-D 的研究记录已经落纸，但
> [21A](21a-rc-cycles-and-concurrency-handoff.md) §五验收和
> [07-D](07d-concurrency-model-research.md) §11.3 的入口条件还没有全部满足。
> 本批先把能实跑的证据、平台运行号和文档对账收干净，再把并发模型裁定送进
> [00. 决策基线](00-decisions.md)。
>
> **本批一句话**：**先把“证据缺口”和“决策缺口”分开关掉，仍然不写并发语法、
> 执行器、`Send`/`Sync`、原子 RC、共享句柄或 `std.concurrent`。**
>
> 状态：**待实施（2026-10-21）**。不修改 `xiao-syntax`、`xiao-types`、IR、
> Runtime ABI 或标准库公开契约；不登记新阶段代号。52b897e 已修复 20L 的原生
> 同名别名污染，本批不重做该修复。

## 一、Agent 交接上下文

### 1.1 接手前提

按顺序读：

1. [21A. 单线程 RC、强环与并发模型决策交接](21a-rc-cycles-and-concurrency-handoff.md)
   —— **权威边界**：§二已确认决策、§3.1 研究交付物、§3.2 新阶段前置、§五验收；
2. [07-D. 并发模型研究](07d-concurrency-model-research.md)
   —— 六项研究交付物、成本矩阵、两套 `Send`/`Sync` 候选、值传递比较、取消清理
   顺序和 ABI 前置清单；
3. [07. 错误模型与并发安全边界](07-concurrency-and-errors.md) §并发术语、
   §后置并发研究——「不允许静默竞争」；
4. [06A. 生命周期静态闭环](06a-lifetime-static-closure.md) 与
   [06B. Runtime 对象与表生命周期](06b-runtime-objects-and-tables.md)
   —— DAG、强环拒绝、`CrossThread` 事实类别、非原子 `StrongHandle`/`WeakHandle`；
5. [20L. 字典方法式 API 与 20K 收口](20l-dict-methods-and-20k-closeout.md)
   —— 07-D 的前置批次和 F-1–F-6 的来源；
6. [12. 测试与开发里程碑](12-tests-and-milestones.md)
   —— B0/N0 前置、阶段出口和测试分层；
7. [00. 决策基线](00-decisions.md) §待定决策——本批的裁定落点。

### 1.2 审核输入（2026-10-21 实跑/CI）

| 环节 | 现状 | 结论 |
| --- | --- | --- |
| 52b897e 推送 CI | 维护回归 `39459004148` 六作业全绿（含 Windows/Linux/macOS 原生）；工作区门禁 `39459004003` 成功 | 20L F-1 的原生 set-alias 红已关闭 |
| 2248e81 | 领先 origin/main 1 个提交，**未推送**；新增 `set-dict-alias` 原生夹具和空日志拒绝 | 只有本地证据；见 F-4 |
| 07-D 六项交付物 | 模型矩阵、RC/Weak/错误/取消/清理、Send/Sync 两套候选、值传递比较、竞争验收、ABI 前置清单均已记录 | 21A §3.1 六项有记录 |
| 21A 验收 1–3 | 强环有测试；**未找到**菱形/DAG 专用测试、`Send` 负向断言或实跑运行号 | 批次验收第 3 项未关闭；见 F-1 |
| `CrossThread` | 事实类别存在，06A 明确“冻结前没有 AST 生产者”；代码检索只见到匹配/序列化，没有构造点 | 静态边界成立，但没有运行证据 |
| 平台复现 `39227168623` | Windows 作业红：`bun run check` 中 `xiao-doc-coverage-rust` 调用到 `C:\Program Files\Git\usr\bin\link.exe`；其余三平台和 ordered summary 绿 | 四平台证据不完整；见 F-3 |
| 平台脚本修复 | `e7597c4` 在 `reproduce.ps1` 把 MSVC linker 放到 PATH 首位，**晚于** `39227168623` | 修复未被平台复现运行验证 |
| 文档状态 | `07d:252` 仍写 Linux/macOS 待取证；20L 文档仍写“待四平台 CI 与推送取证” | 与 39459004148/39459004003 不一致；见 F-2 |
| 补充探针 | 混合导入 `set.size` + `dict.is_empty/remove`，VM 与原生均输出 `2/true` | 未发现残余同名调用污染 |

### 1.3 本批边界

| 项 | 本批 | 说明 |
| --- | --- | --- |
| 21A 验收 1–3 证据 | **做** | 补测试或如实登记“不可实跑”的契约证据 |
| 07-D/20L 文档对账 | **做** | 写清 F-1–F-6 的当前状态和运行号 |
| 平台复现 | **做** | 手动触发一次四平台运行；不复用旧红/旧绿冒充当前证据 |
| 并发模型裁定 | **整理** | 把选项、推荐和影响送进 00；不代替星崽拍板 |
| 可执行差分原型 | **可选** | 若要拿成本矩阵冻结模型，必须先补原型与测量 |
| 并发语法/执行器 | **不做** | 等独立阶段 |
| `Send`/`Sync`/`async`/`Actor`/`Weak` | **不做** | 不新增语法或公开 API |
| 原子 RC/共享句柄/对象头 | **不做** | 不修改 Runtime 布局或 ABI |
| 新阶段代号/`std.concurrent` | **不预留** | 入口条件满足前不登记 |

## 二、前置返工项

### F-1 **21A 验收 1–3 缺少实跑证据（P2）**

07-D 规划 §六.3 要求“DAG/菱形、强环拒绝、跨线程句柄拒绝、`CrossThread` 只由显式
事实产生，均有实跑证据”，但研究记录 §9.2 只逐条对照了 21A §3.1 的六项交付物，
没有给出这批验收的测试名或运行号。现状：

- 强环有两层测试：`xiao-lifetime/tests/a06_lifetime.rs:349`
  `strong_cycle_is_reported`、`:872` `graph_rejects_strong_cycle`；
- 未找到 DAG 共享/菱形的专用测试；`graph_orders_and_weak_edges` 只验证弱边不成环；
- 未找到 `StrongHandle`/`WeakHandle` 的 `Send` 负向断言；单线程边界目前由
  `xiao-runtime/src/memory/mod.rs:387-394` 的 `PhantomData<Rc<()>>` 结构性保证；
- `CrossThread` 没有生产者：06A `:112` 已写明“冻结前没有 AST 生产者”，代码检索只见到
  `EscapeReason::CrossThread` 的匹配（`xiao-lifetime/src/diagnostics.rs:81`、
  `xiao-ir/src/lower.rs:1327`）。

**处理要求**：

1. 在 `a06_lifetime.rs` 增加最小菱形/DAG 用例：两个拥有者共享同一子节点，释放顺序
   成功且子节点只析构一次；单节点/多节点强环继续由现有测试覆盖。
2. 在 Runtime 侧增加 `StrongHandle`/`WeakHandle` 不是 `Send`/`Sync` 的编译期或静态
   断言；不得为此新增公开 API 或改变对象布局。
3. `CrossThread`：若能不新增语法/IR/ABI 构造显式事实，就补传播/诊断测试；若不能，
   在 07-D §9.1 明确写成“契约级证据：无生产者”，**不得**写成“运行测试已通过”。
4. 把测试名、本地运行结果和最新 CI 运行号回填 07-D §9.1/§11.3。

**关闭标准**：上述 1–3 有明确的测试名或如实的不可实跑说明；本地定向通过；CI 运行号可查。

### F-2 **07-D/20L 状态与最新 CI 不一致（P2）**

- `07d-concurrency-model-research.md:252` 仍写“Linux/macOS 三平台仍待维护回归取证”，
  但 `39459004148` 的三平台原生作业已绿；
- 同表 `:255` 仍写 `1393cea` 待推送；该提交已在历史中，维护回归实际覆盖的是其后续
  提交；
- `20l-dict-methods-and-20k-closeout.md` 顶部仍写“待四平台 CI 与推送取证（2026-10-20）”，
  §1.2.1 仍写“三平台维护回归仍需外部运行号才能关闭 F-6”；
- `README.md` 的 07-D 行仍写“待外部证据与星崽裁定”，应区分“维护/工作区已绿”和
  “平台复现仍缺”。

**处理要求**：按以下口径更新，不把旧运行号当新证据：

| 项 | 新口径 |
| --- | --- |
| 20L F-1 原生 set-alias | 已由 `39459004148` 关闭（三平台原生全绿） |
| 20L F-2 Bun 分类 | 本地自测通过；52b897e 的维护/工作区运行已绿；2248e81 的空日志自测待推送后取证 |
| 20L F-3 私有字典动作 | 私有动作 + 等价证据成立；“移除动作后的外部四路验证”措辞删除或改成不适用 |
| 20L F-4 推送 | 52b897e 已推送并取证；2248e81 待推送 |
| 20L F-6 平台复现 | 仍待一次新的四平台运行和 ordered summary；旧 `39227168623` 只作为历史红 |

### F-3 **平台复现仍缺当前运行号（P2）**

`39227168623` 的 Windows 失败点是 `bun run check` 里 `xiao-doc-coverage-rust` 的链接
步骤调用了 Git 的 `link.exe`，不是业务测试失败。`e7597c4` 的 PATH 修复晚于该运行，
且平台复现工作流只有定时（每周一 03:17 UTC）和 `workflow_dispatch` 两种触发方式，
**普通 push 不会触发**。

**处理要求**：

1. 先推送 2248e81，确认维护回归和 workspace 门禁对最新 HEAD 全绿；
2. 手动触发一次 `Xiao platform reproduction`（工作流 ID `365649226`）；
3. 记录新的运行号、四个平台作业结果和 ordered summary；
4. Windows 若仍红，抓取实际链接器路径与失败步骤，另开修复并**重新运行**；不得用
   ordered summary 的绿冒充 Windows 绿；
5. 把结果回填 07-D/20L 状态。

**关闭标准**：最新 HEAD 有一次四平台平台复现运行，Windows 绿；若仍红，有新的根因、
修复提交和复跑运行号。

### F-4 **2248e81 未推送（P3）**

`set-dict-alias` 夹具和空日志自测目前只有本地证据；52b897e 的 CI 不包含新增夹具。
推送后维护回归才会在 Windows/Linux/macOS 原生作业执行它。

**处理要求**：推送后核对三个原生作业确实运行了 `20c_first_modules`，并记录运行号。
不得用本地 10.39s 的通过代替 CI 取证。

### F-5 **20L F-3 的“移除动作外部验证”措辞没有对应机制（P3）**

`07d-concurrency-model-research.md:254` 写“移除动作后的外部四路验证仍归平台门禁”。
仓库中不存在“移除动作”的验证步骤，平台门禁也不会执行它。实际成立的是另一条分支：
`__dict_remove`/`__dict_keys`/`__dict_values` 保持编译器私有，不登记 intrinsic，并用
`by_name` 缺失、VM 输出和 LLVM 符号三类证据锁定。

**处理要求**：把该句改成“保持私有动作，不登记 intrinsic；移除验证不适用”，避免留下
无法核销的承诺。

### F-6 **研究深度需要如实标注（P3）**

07-D 的成本矩阵是定性排序（“矩阵结论是成本排序，不是语言选择”），交付物 4 明确写
“只作研究伪代码，不进入产品契约”。这与记录自身的边界声明一致，不算造假；但若后续
要据此冻结执行模型、对象头或 ABI，必须先补可执行差分原型与测量。

**处理要求**：在 07-D 状态和 00 决策记录中写清“当前是桌面研究/伪代码，不是实测成本”。
若星崽决定进入模型冻结，另立原型任务；本批不把伪代码升级成产品契约。

## 三、并发模型裁定准备

本批只整理选项，不代替星崽裁定。以下问题来自 07-D §11.2：

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

裁定结果写入 `00-decisions.md` 的新“并发模型待定/已定”段落；在决定前不登记
`H0-C`、`07-D 实现` 或 `std.concurrent`，也不新增 `async`/`await`/`spawn`/`send`
语法或公开名字。

## 四、实施步骤

| 步 | 内容 | 验证 |
| --- | --- | --- |
| 0 | 推送 2248e81；确认维护回归与 workspace 对最新 HEAD 全绿 | CI 运行号 |
| 1 | 补 21A 验收 1–3 的菱形/DAG、`Send` 负向、`CrossThread` 证据 | 定向测试 + 运行记录 |
| 2 | 同步 07-D/20L/README 的 F-1–F-6 状态，修 F-5 措辞 | 文档 diff |
| 3 | 手动触发平台复现，记录四平台与 ordered summary | 新运行号 |
| 4 | 把 8 项裁定或明确后置写入 00；更新 07-D §11.3 | `00-decisions.md` |
| 5 | 收口审核：定向测试、`XIAO_GATE_SELF_TEST=1`、必要时完整门禁 | 本地/CI 证据 |

**本批不写产品代码**；除测试与文档外，不改 `xiao-syntax`/`xiao-types`/IR/Runtime
ABI/标准库公开契约。52b897e 的 codegen 修复保持现状，不做补丁叠补丁。

## 五、验收

1. **推送**：2248e81 已推送；最新 HEAD 的维护回归与 workspace 门禁全绿，新增
   `set-dict-alias` 在三平台原生作业执行；
2. **平台复现**：有一次新的四平台运行；Windows 绿，或红因已定位并有复跑；
   ordered summary 只作为汇总，不作为单平台绿的替代；
3. **21A 验收 1–3**：菱形/DAG、强环、普通句柄边界、`CrossThread` 证据有测试名或
   如实说明；没有用名称启发式产生 `CrossThread`；
4. **文档**：07-D、20L、README、21A 的状态与运行号一致；不再出现“待三平台/待推送”
   的过期说法；20L F-3 的不可核销措辞已删除；
5. **决策**：00 中 8 项裁定有明确选择或明确后置；未登记新阶段代号/API；
6. **不越界**：无并发语法、`Send`/`Sync`/`async`/`Actor`/`Weak`、原子 RC、共享句柄、
   IR/ABI/标准库公开契约变更；测试与文档之外无产品行为变化；
7. **门禁**：定向测试通过；`XIAO_GATE_SELF_TEST=1` 通过；完整门禁或 CI 对最新 HEAD
   通过；文档覆盖率不低于 90%。

## 六、不负责

- 不实现线程、任务池、Actor、协程、`async`/`await`、`spawn`/`send`；
- 不实现 `Send`/`Sync` 类型能力、冻结、共享可变、Weak 语法或 `std.concurrent`；
- 不实现原子 RC、共享句柄、对象头/锁或新的 Runtime ABI；
- 不修改现有单线程 `StrongHandle`/`WeakHandle` 布局和 `PhantomData` 边界；
- 不把 07-D 的研究伪代码或成本排序写成已冻结设计；
- 不修 52b897e 之外的 codegen 行为；平台复现若仍红，另开修复并复跑。

## 七、落点与取证命令

```text
docs/DevDocs/07d-closeout-and-concurrency-decisions.md  本文
docs/DevDocs/07d-concurrency-model-research.md          F-1/F-2/F-5/F-6 状态与证据
docs/DevDocs/20l-dict-methods-and-20k-closeout.md       F-1–F-6 当前对账
docs/DevDocs/README.md                                  07-D.1 索引与状态
docs/DevDocs/00-decisions.md                            并发模型裁定/后置记录
docs/DevDocs/21a-rc-cycles-and-concurrency-handoff.md   验收 1–3 证据引用
core/rust/crates/xiao-lifetime/tests/a06_lifetime.rs    菱形/DAG 与 CrossThread 证据
core/rust/crates/xiao-runtime/src/memory/mod.rs         句柄 Send/Sync 负向断言
```

```bash
# 本地定向
cd core/rust
cargo test -p xiao-lifetime --test a06_lifetime
cargo test -p xiao-driver --test 20l_dict_methods
cargo test -p xiao-driver --test 20c_first_modules -- --ignored --exact first_modules_match_native_results
XIAO_GATE_SELF_TEST=1 bash tools/gates/run.sh

# 平台复现（推送最新 HEAD 后）
gh workflow run 365649226 --ref main
gh run list --workflow 365649226 --limit 3
gh run view <run-id> --json jobs --jq '.jobs[] | "\(.name) \(.conclusion)"'
```

原生测试需要受控 LLVM、Runtime staticlib 和目标三元组（见 10D §4）；Windows 上
优先在 `pwsh`/`cmd` 环境准备，避免 Git Bash 的 PATH 影响链接器选择。

## 八、风险与注意事项

1. **平台复现不是 push 触发**：推送 2248e81 只覆盖维护回归和 workspace；四平台证据
   必须手动触发，且定时任务每周才跑一次。
2. **Windows `link.exe` 可能再次命中**：`e7597c4` 只改了脚本 PATH；必须以新运行验证，
   不能只凭脚本 diff 认定已修。
3. **`Send` 负向断言可能引入测试依赖**：优先用编译期断言或现有测试工具完成；若必须
   新增 dev-dependency，先确认不改变 workspace 公共依赖和 ABI。
4. **`CrossThread` 可能无法在运行层证明**：没有语法生产者时，只能登记为契约级证据；
   不要把“代码检索不到”写成“运行测试通过”。
5. **成本矩阵是定性数据**：在没有可执行原型和测量前，不能据此冻结对象头、原子计数或
   跨线程 ABI。
