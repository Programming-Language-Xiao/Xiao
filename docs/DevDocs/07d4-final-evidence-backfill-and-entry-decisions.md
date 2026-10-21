# 07-D.4. 终验回填与并发入口裁定

> **这是 07-D 的最后一段文档收口，不是并发实现阶段。** 07-D.3 的 Windows 链接器和
> macOS REPL 修复已经由平台复现 `39563381227` 验证通过；本批把新运行号写回文档，
> 修正几处“已关闭但正文仍像当前缺口”的记录，并把 8 项并发入口裁定送进
> [00. 决策基线](00-decisions.md)。
>
> **本批一句话**：**平台证据已经齐了，先把文档写成真的，再让星崽做 8 项裁定；
> 仍然不写并发语法、执行器、`Send`/`Sync`、原子 RC、共享句柄或 `std.concurrent`。**
>
> 状态：**平台证据与文档门禁已通过，待星崽裁定（2026-10-22）**。本批只改文档和裁定记录；不修改
> `xiao-syntax`、`xiao-types`、IR、Runtime ABI 或标准库公开契约，不登记新阶段代号。
> 若发现平台证据与文档不一致，以 CI 作业日志为准，不用 ordered summary 代替单平台结果。

## 一、Agent 交接上下文

### 1.1 接手前提

按顺序读：

1. [07-D.3. 平台复现修复与 07-D 终验](07d3-platform-reproduction-fix-and-closeout.md)
   —— 直接前置：Windows 链接器、macOS REPL 修复和平台证据格式；
2. [07-D.2. 并发入口裁定与平台收口](07d2-entry-decisions-and-platform-closeout.md)
   —— 文档归属、8 项裁定和平台证据口径；
3. [07-D.1. 07-D 收口与并发入口决策准备](07d-closeout-and-concurrency-decisions.md)
   —— 菱形 DAG、Send/Sync doctest、CrossThread 契约证据；
4. [07-D. 并发模型研究](07d-concurrency-model-research.md)
   —— 六项交付物、成本矩阵、两套边界候选、取消清理顺序；
5. [21A. 单线程 RC、强环与并发模型决策交接](21a-rc-cycles-and-concurrency-handoff.md)
   —— §五验收与 §3.2 新阶段前置；
6. [20L. 字典方法式 API 与 20K 收口](20l-dict-methods-and-20k-closeout.md)
   —— F-1–F-6 的历史来源和平台口径；
7. [00. 决策基线](00-decisions.md) §待定决策——本批的裁定落点。

### 1.2 当前状态（2026-10-21 实跑/CI）

| 环节 | 现状 | 结论 |
| --- | --- | --- |
| HEAD | `d6112e4` 已推送；本地有后继纯文档提交 | 07-D.3 修复已进入远端 |
| 工作区门禁 `39563178068` | Linux workspace gates 成功 | 文档、格式、Bun 全量测试和门禁通过 |
| 终验工作区门禁 `39624910950` | `17a89b0` 成功 | 纯文档终验提交的 workspace 门禁通过 |
| 平台复现 `39563381227` | linux-amd64、linux-arm64、macos-arm64、windows-amd64 **全部成功**；ordered summary 成功 | 四平台证据成立；见 §三 |
| Windows 日志 | `Cargo linker (CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER): ...\Microsoft Visual Studio\18\Enterprise\...\link.exe` | 不再命中 Git 的 `link.exe` |
| macOS 日志 | `(pass) SGR 鼠标点击和拖动建立选区...`；`298 pass` | REPL 时序修复成立 |
| 本地 REPL 测试 | 隔离运行 5 次全部通过 | 与 CI 一致 |
| 全量 Bun | 298 pass / 5 skip / 0 fail | 与 macOS 日志一致 |
| 维护回归 | `95981a8` 改的是 `tools/**` 和 `cli/**`，不触发维护回归；最后一次核心代码提交 f119ed0 的 `39517248193` 仍全绿 | 口径需写清；见 F-2 |
| 文档状态 | 07-D.3、07-D.2、07-D.1 的平台运行号和关闭状态已回填，文档/UseDocs 门禁通过 | 入口只剩 8 项裁定 |
| 并发裁定 | `00-decisions.md` 记录 8 项“待星崽裁定” | 入口阻断；见 F-4 |

### 1.3 本批边界

| 项 | 本批 | 说明 |
| --- | --- | --- |
| 平台运行号回填 | **做** | `39563381227` 与 `39563178068` 写入现有文档 |
| 文档残留修正 | **做** | 修正“已关闭但正文仍写当前问题”的段落 |
| 维护回归口径 | **做** | 明确 tools/cli 由 workspace/平台覆盖，或手动补一次 |
| 8 项并发裁定 | **收口** | 写入 00；未裁定就保持入口阻断 |
| 代码/脚本修改 | **不做** | 平台修复已在 d6112e4 完成，本批不重做 |
| 并发语法/执行器 | **不做** | 等新独立阶段 |
| 新阶段代号/`std.concurrent` | **不预留** | 入口条件满足前不登记 |

## 二、审核发现

### F-1 **07-D.3 曾声称残留已关闭，但 07-D.2/07-D.1 的正文没有同步（已关闭）**

上一轮审核记录了 07-D.3 的 F-4 与前置文档之间的措辞不同步：当时 07-D.2 的 F-5
还引用 00 中已经废弃的“**后置**、未裁定”，F-2 标题还写“**仍是**过期状态”，
07-D.1 的 F-5 也曾保留“移除动作外部验证”的旧承诺。当前 00 已统一为“**待星崽
裁定**”，07-D.2 的 F-5/F-2、07-D.1 的 F-5 均已改成历史说明或关闭记录，旧的
移除验证已明确为不适用。

**处理结果**：上述内容均已按当前文档正文收口；本项只保留审核来源，不再作为当前缺口。

### F-2 **维护回归口径仍然写成“必须新运行”（已明确）**

`07d3...:145-160` 的 F-5 写 `95981a8` “推送后必须取得新的维护回归”，但：

- 维护回归的 `paths` 只含 `.github/workflows/maintenance-regression.yml`、
  `core/rust/**`、`tests/fuzz/**`、`tests/benchmarks/**`、`tests/fixtures/**`、
  `stdlib/**`、`.gitattributes`，**不含 `tools/**` 和 `cli/**`**；
- `95981a8` 改的是 `tools/platform-reproduction/reproduce.ps1`、
  `cli/ts/src/repl/multiline.test.ts` 和文档，实际只触发工作区门禁
  `39563178068`；
- 平台脚本由 `39563381227` 覆盖，REPL 测试由工作区门禁的全量 Bun 和 macOS
  平台作业覆盖。

**处理结果**：采用推荐口径：维护回归以最后一次核心代码提交 f119ed0 的
`39517248193` 为准；95981a8 的 tools/cli 改动由 workspace 门禁 `39563178068` 和平台复现
`39563381227` 覆盖；纯文档后继提交不再要求重复维护回归。

历史选项记录：

1. 推荐口径：维护回归以最后一次核心代码提交 f119ed0 的 `39517248193` 为准；
   `95981a8` 的 tools/cli 改动由 workspace 门禁 `39563178068` 和平台复现
   `39563381227` 覆盖；
2. 若流程要求每个 HEAD 都有维护回归，手动执行
   `gh workflow run 376663825 --ref main` 并记录新运行号；不能写成 push 自动触发。

### F-3 **07-D.3 与新运行号没有回填（已关闭）**

`07d3-platform-reproduction-fix-and-closeout.md`、07-D、07D.1、07D.2、20L 和 README
已回填 `39563381227`、HEAD `d6112e4`、四个平台 success、Windows Cargo linker、macOS
SGR/298 pass、工作区门禁 `39563178068` 和终验工作区门禁 `39624910950`（绑定
`17a89b0`）。

### F-4 **8 项并发入口问题仍未裁定（P1，入口阻断）**

平台证据已不再是阻断项；真正的阻断只剩 8 项裁定。`00-decisions.md` 目前记录
“待星崽裁定”，没有选择。未裁定前不能建立并发实现阶段，不能预留 `H0-C`、
`07-D 实现`、`std.concurrent` 或任何语法/API 名称。

**处理要求**：按 §四逐项记录选择或明确后置。若选择后置，保持 07-D 入口条件不变，
不要把“暂不决定”写成“已决定不做”。

### F-5 **相对提交号的可追溯性（已关闭）**

平台证据已明确绑定 `d6112e4`；本批之后的纯文档提交不改变平台代码证据，相关状态写成
“d6112e4 平台基线 + 本地文档后继”，避免把未运行的文档提交冒充平台执行提交。

## 三、平台证据记录

| 运行 | HEAD | 作业 | 结果 |
| --- | --- | --- | --- |
| `39563381227` | `d6112e4` | linux-amd64 | success |
| `39563381227` | `d6112e4` | linux-arm64 | success |
| `39563381227` | `d6112e4` | macos-arm64 | success |
| `39563381227` | `d6112e4` | windows-amd64 | success |
| `39563381227` | `d6112e4` | Ordered platform evidence summary | success |
| `39563178068` | `d6112e4` | Linux workspace gates | success |

Windows 关键日志：

```text
Cargo linker (CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER):
C:\Program Files\Microsoft Visual Studio\18\Enterprise\VC\Tools\MSVC\14.51.36231\bin\Hostx64\x64\link.exe
```

macOS 关键日志：

```text
(pass) SGR 鼠标点击和拖动建立选区，滚轮忽略且退出关闭鼠标上报
298 pass
```

旧运行 `39546679818` 只保留为历史红证据；`39563381227` 才是当前平台通过证据。

## 四、8 项并发入口裁定

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

记录格式：

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

## 五、实施步骤

| 步 | 内容 | 验证 |
| --- | --- | --- |
| 0 | 回填 `39563381227`/`39563178068` 与 `d6112e4`；关闭 07-D.3 的 F-1/F-2 | 文档 diff |
| 1 | 修正 07-D.2/07-D.1 的 F-1/F-2/F-5 正文与标题 | grep 对账 |
| 2 | 明确维护回归口径（推荐第 1 种） | 文档 diff |
| 3 | 把 8 项裁定或明确后置写入 00 | `00-decisions.md` |
| 4 | 最终审核：文档/UseDocs、平台与工作区运行号、`XIAO_GATE_SELF_TEST=1` | 本地/CI 证据 |

**本批不写产品代码**；平台脚本和 REPL 测试已在 `d6112e4` 修好，不重做。若文档改动
提交后需要 CI，纯文档提交由 workspace gates 覆盖，平台复现无需重复触发，除非文档
改动涉及平台脚本/测试夹具。

## 六、验收

1. **平台证据**：`39563381227` 四平台与 ordered summary 全绿；Windows/macOS 关键日志
   已回填；旧 `39546679818` 只作历史红；
2. **工作区**：`39563178068` 成功；最终文档提交的 workspace gates 通过；
3. **文档一致性**：07-D.3 不再写“待平台复跑”；07-D.2/07-D.1 的 F-1/F-2/F-5 没有
   现在时的旧问题描述；不再有无标注的“当前本地后继提交”，改用 `d6112e4` 或最终哈希；
4. **维护口径**：F-2 的推荐口径或手动运行号已记录，不能把未触发的维护回归写成通过；
5. **裁定**：00 中 8 项有明确选择或明确后置；未登记新阶段代号、语法或公开 API；
6. **不越界**：无并发语法、`Send`/`Sync`/`async`/`Actor`/`Weak`、原子 RC、共享句柄、
   IR/ABI/标准库公开契约变更；
7. **门禁**：文档/UseDocs、覆盖率（如涉及）和 `XIAO_GATE_SELF_TEST=1` 通过。

## 七、不负责

- 不重新修改平台脚本或 REPL 测试；它们的修复已验证；
- 不实现线程、任务池、Actor、协程、`async`/`await`、`spawn`/`send`；
- 不实现 `Send`/`Sync` 类型能力、冻结、共享可变、Weak 语法或 `std.concurrent`；
- 不实现原子 RC、共享句柄、对象头/锁或新的 Runtime ABI；
- 不替星崽做 8 项并发裁定；
- 不用 ordered summary 的成功替代单平台作业结果。

## 八、落点与取证命令

```text
docs/DevDocs/07d4-final-evidence-backfill-and-entry-decisions.md  本文
docs/DevDocs/07d3-platform-reproduction-fix-and-closeout.md       状态/F-1/F-2 回填
docs/DevDocs/07d2-entry-decisions-and-platform-closeout.md        F-2/F-5 关闭记录
docs/DevDocs/07d-closeout-and-concurrency-decisions.md            F-1/F-5 关闭记录
docs/DevDocs/07d-concurrency-model-research.md                    §1.2/§9.1/§11.3 运行号
docs/DevDocs/20l-dict-methods-and-20k-closeout.md                 F-6 平台证据
docs/DevDocs/00-decisions.md                                      8 项并发裁定/后置
docs/DevDocs/README.md                                            07-D.4 索引与状态
```

```bash
# 运行号与作业
gh run view 39563381227 --json jobs --jq '.jobs[] | "\(.name) \(.conclusion)"'
gh run view 39563178068 --json jobs --jq '.jobs[] | "\(.name) \(.conclusion)"'

# 文档对账（应只剩明确标注为历史/审核时的引用）
grep -rn "当前本地后继提交\|仍待新的四平台复现\|仍写.*后置" docs/DevDocs
grep -rn "39563381227\|39563178068" docs/DevDocs

# 本地门禁
bun run check:docs
bun run check:usedocs
XIAO_GATE_SELF_TEST=1 bash tools/gates/run.sh
```

## 九、风险与注意事项

1. **不要因为平台全绿就跳过文档修正**：这次绿灯绑定的是 `d6112e4`，旧文档仍写着
   `39546679818` 的红或“待复跑”。
2. **维护回归不是 push 自动触发**：`tools/**`/`cli/**` 不在 paths 内；推荐用
   workspace/平台覆盖，或手动 dispatch 并记录。
3. **裁定未发生**：平台证据只解除外部阻断，8 项问题仍需要星崽选择；没有选择时保持
   入口阻断。
4. **相对提交号会漂移**：最终回填应写实际哈希，不要继续使用“当前本地后继提交”。
5. **ordered summary 不是平台绿**：本次 summary 恰好对应四平台全绿，但验收仍以
   单平台作业和关键日志为准。
