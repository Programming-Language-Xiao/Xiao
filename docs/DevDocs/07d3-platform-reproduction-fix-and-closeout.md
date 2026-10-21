# 07-D.3. 平台复现修复与 07-D 终验

> **这是 07-D 的平台收口批次，不是并发实现阶段。** 07-D.2 已把文档归属和运行号
> 回填干净，但手动触发的四平台复现 `39546679818` 仍有 Windows 和 macOS 两个作业红：
> Windows 继续命中 Git 的 `link.exe`，macOS 的 REPL 鼠标选区测试失败。
>
> **本批一句话**：**先把两个平台红点修掉并重新跑绿，再做最后的文档/裁定收口；
> 仍然不写并发语法、执行器、`Send`/`Sync`、原子 RC、共享句柄或 `std.concurrent`。**
>
> 状态：**平台复现已通过，待文档终验与星崽裁定（2026-10-21）**。不修改 `xiao-syntax`、`xiao-types`、IR、
> Runtime ABI 或标准库公开契约；不登记新阶段代号。平台复现只有新运行号能关闭，
> ordered summary 的绿不能替代单平台作业结果。

## 一、Agent 交接上下文

### 1.1 接手前提

按顺序读：

1. [07-D.2. 并发入口裁定与平台收口](07d2-entry-decisions-and-platform-closeout.md)
   —— 本批的直接前置：文档归属、8 项裁定和平台证据口径；
2. [07-D.1. 07-D 收口与并发入口决策准备](07d-closeout-and-concurrency-decisions.md)
   —— 菱形 DAG、Send/Sync doctest、CrossThread 契约证据；
3. [07-D. 并发模型研究](07d-concurrency-model-research.md)
   —— 六项交付物、成本矩阵、两套边界候选、取消清理顺序；
4. [21A. 单线程 RC、强环与并发模型决策交接](21a-rc-cycles-and-concurrency-handoff.md)
   —— §五验收与 §3.2 新阶段前置；
5. [20L. 字典方法式 API 与 20K 收口](20l-dict-methods-and-20k-closeout.md)
   —— F-1–F-6 的历史来源和平台口径；
6. [12. 测试与开发里程碑](12-tests-and-milestones.md)
   —— B0/N0 前置和阶段出口；
7. [00. 决策基线](00-decisions.md) §待定决策——8 项裁定的落点。

### 1.2 当前状态（2026-10-21 实跑/CI）

| 环节 | 现状 | 结论 |
| --- | --- | --- |
| 推送 | `0191330` 已推送；`origin/main` 与本地一致 | 文档对账已进入远端 |
| 工作区门禁 `39544482322` | Linux workspace gates 成功 | 纯文档提交的文档/格式门禁通过 |
| 维护回归 | 纯文档提交不触发；最后一次代码提交 f119ed0 的 `39517248193` 六作业全绿 | 维护证据仍以 f119ed0 为准 |
| 平台复现 `39546679818` | Linux amd64/arm64 成功；**macOS 红**（REPL 鼠标选区测试）；**Windows 红**（Git `link.exe`）；ordered summary 成功 | 历史红点；见新绿运行 `39563381227` |
| Windows 失败点 | 历史日志显示 `xiao-doc-coverage-rust` 使用 `C:\Program Files\Git\usr\bin\link.exe` | 已由 `39563381227` 验证 MSVC Cargo linker |
| macOS 失败点 | 历史日志显示 SGR 鼠标选区测试固定等待后 `anchor` 为 `null` | 已由 `39563381227` 验证测试通过和 298 pass |
| 文档状态 | f119ed0、d6112e4 与 `39517248193`/`39517248607`/`39563381227`/`39563178068` 已回填 | 仍待本批文档门禁与星崽裁定 |
| 并发裁定 | `00-decisions.md` 记录 8 项“待星崽裁定” | 入口阻断；见 F-3 |

### 1.3 本批边界

| 项 | 本批 | 说明 |
| --- | --- | --- |
| Windows 链接器修复 | **做** | 让 Git Bash/cargo 子进程实际使用 MSVC `link.exe` |
| macOS REPL 抖动 | **做** | 复现并消除时序竞争；不无证据地加入放行名单 |
| 平台复现复跑 | **做** | 四平台 + ordered summary，Windows/macOS 都必须绿 |
| 文档残留清理 | **做** | 标关闭已修项，决定维护回归口径 |
| 8 项并发裁定 | **收口** | 写入 00；未裁定就保持入口阻断 |
| 并发语法/执行器 | **不做** | 等新独立阶段 |
| `Send`/`Sync`/`async`/`Actor`/`Weak` | **不做** | 不新增语法或公开 API |
| 原子 RC/共享句柄/对象头 | **不做** | 不修改 Runtime 布局或 ABI |
| 新阶段代号/`std.concurrent` | **不预留** | 入口条件满足前不登记 |

## 二、审核发现

### F-1 **Windows 平台仍使用 Git 的 `link.exe`（已关闭）**

`39546679818` 的 Windows 作业在 `Run Windows reproduction` 步骤失败。日志显示
`xiao-doc-coverage-rust` 的链接器是：

```text
"C:\Program Files\Git\usr\bin\link.exe"
```

并返回 `link.exe returned an unexpected error`。这不是业务测试失败，而是工具链选择错误。

`e7597c4` 已在 `tools/platform-reproduction/reproduce.ps1:70-71` 把 MSVC 链接器目录
放到 PowerShell 的 `$env:PATH` 首位，但 `reproduce.ps1:157` 通过 `bash tools/gates/run.sh`
启动 Git Bash。Git Bash 启动时会把它自己的 `/usr/bin`（即 `C:\Program Files\Git\usr\bin`）
放到 PATH 前面，于是 `link.exe` 又解析到 Git 的版本。**PowerShell 层的 PATH 修复没有
穿透到 bash/cargo 子进程。**

**处理要求**：修在 bash 实际看到的环境上，而不是只改 PowerShell PATH。可选方向：

1. 在调用 `bash tools/gates/run.sh` 时，把 MSVC `link.exe` 目录转换成 POSIX 路径并显式
   置于 bash 的 `PATH` 首位；
2. 为该子进程设置 Rust 的链接器覆盖（例如目标三元组对应的 linker 环境变量或
   `-C linker=<MSVC link.exe>`），让 cargo 不按 PATH 搜索 `link.exe`；
3. 若改 `tools/gates/run.sh`，必须保证本地普通 Windows 运行和 CI 使用同一口径，
   不能只在平台脚本里特判。

**关闭标准**：新的 Windows 平台复现日志中不再出现
`C:\Program Files\Git\usr\bin\link.exe`；`xiao-doc-coverage-rust` 与全量门禁通过。

**本地处理结果**：`Initialize-MsvcEnvironment` 现在返回真实 MSVC `link.exe` 路径，脚本
按当前 Rust 目标三元组设置 `CARGO_TARGET_<TRIPLE>_LINKER`；Git Bash/cargo 子进程不再依赖
PATH 竞争。PowerShell 语法检查通过，最终确认仍需新的 Windows 平台日志。

### F-2 **macOS 的 REPL 鼠标选区测试失败（已关闭）**

同一平台运行的 macOS 作业在 `bun test` 中失败：

```text
(fail) SGR 鼠标点击和拖动建立选区，滚轮忽略且退出关闭鼠标上报
cli/ts/src/repl/multiline.test.ts:718
expected: { line: 0, column: 0 }
received: null
```

测试在 `input.write(...)` 后只等待 `setTimeout(..., 5)` 就调用 `input.end()`，依赖固定
等待时间完成 SGR 事件处理。macOS 上这次没有及时建立 anchor，属于时序竞争；它不在
`tools/gates/run.sh` 的两条已知 REPL 抖动名单里，所以门禁按真实失败判红，这是正确行为。

**处理要求**：

1. 连续重复运行该测试，确认是时序抖动还是 macOS 上的真实行为差异；
2. 优先把固定 `setTimeout` 改成可观察的确定性等待（例如等待输出中的鼠标上报/选区建立
   信号），或同步处理输入后再结束会话；
3. 只有拿到“同一测试在隔离重跑中稳定通过、且失败只由环境调度造成”的证据后，才能考虑
   把它加入 `run.sh` 的已知抖动名单；不得为了平台变绿直接放行；
4. 加入名单时必须同时更新 `XIAO_GATE_SELF_TEST=1` 的覆盖。

**关闭标准**：macOS 平台复现的 `bun test` 通过；若走隔离放行，必须有重复运行证据和
分类器自测。

**本地处理结果**：测试等待初始重绘和鼠标事件处理后的第二次重绘，不再使用固定 5 ms
计时；同一用例连续 5 次通过，完整 Bun 测试为 298 pass / 5 skip / 0 fail。断言和
`run.sh` 放行名单均保持不变。

### F-3 **8 项并发入口问题仍未裁定（P1，入口阻断）**

`00-decisions.md` 已记录 8 项“待星崽裁定”，没有选择。执行机制、`Send`/`Sync` 候选、
`CrossThread` 是否拆分、复制/序列化/冻结/共享可变、取消清理顺序、原子共享句柄、
官方库结构和研究深度都不能在本批被默认冻结。

**处理要求**：按 07-D.2 §三逐项记录选择或明确后置；未裁定前不建立并发实现阶段，
不预留 `H0-C`、`07-D 实现`、`std.concurrent` 或任何语法/API 名称。

### F-4 **上一轮文档残留（已关闭）**

- 上一轮审核曾发现 07D.2 的“后置、未裁定”、F-2 标题矛盾、F-5 移除验证承诺和 F-1
  “未找到测试”措辞；这些内容已在对应文档改为“待星崽裁定”、关闭记录、移除验证不适用
  和当前测试证据。

**处理结果**：上一轮文档中的旧行号和旧措辞已按当前文件内容收窄；07D.2/07D.1 的
F-1/F-2/F-5 只保留历史来源或已关闭说明，不再作为当前缺口。

### F-5 **维护回归的验收口径（已明确，当前代码提交待取证）**

维护回归的 `paths` 不含 `docs/**`，纯文档提交不会触发；当前最后有效维护回归是
f119ed0 的 `39517248193`。07-D.2 验收要求“最终提交的维护回归与 workspace 门禁全绿”，
对纯文档提交无法字面满足。

**处理结果**：f119ed0 是最后一次已有维护回归的代码基线，`39517248193` 全绿；
`0191330` 是纯文档提交，`39544482322` 的 workspace 门禁覆盖它；`95981a8` 含平台
脚本和 REPL 测试改动，不能由旧运行号覆盖，推送后必须取得新的维护回归。平台复现仍需
单独手动触发，不能由文档提交的维护回归状态推导。

历史选项记录：

1. 对最终 HEAD 手动 `workflow_dispatch` 一次维护回归，取得绿运行号；
2. 历史口径是“最后一次代码提交 f119ed0 的维护回归全绿；后续纯文档提交由
   `39544482322` 的 workspace 门禁覆盖”；当前 `95981a8` 含代码改动，需新的维护运行。

### F-6 **平台基线的可追溯边界（已关闭）**

平台证据已明确绑定 `d6112e4` 与 `39563381227`；本批之后的纯文档提交不改变平台代码
证据，相关状态使用“d6112e4 平台基线 + 本地文档后继”，避免把未运行的文档提交冒充
平台执行提交。

## 三、平台证据记录

| 运行 | HEAD | 作业 | 结果 |
| --- | --- | --- | --- |
| `39546679818` | `0191330` | linux-amd64 | success |
| `39546679818` | `0191330` | linux-arm64 | success |
| `39546679818` | `0191330` | macos-arm64 | **failure**：REPL 鼠标选区测试 |
| `39546679818` | `0191330` | windows-amd64 | **failure**：Git `link.exe` |
| `39546679818` | `0191330` | Ordered platform evidence summary | success（只汇总，不代表平台绿） |

复跑后必须用同样格式记录新的运行号；旧运行号只作历史红证据，不能冒充当前通过。

## 四、实施步骤

| 步 | 内容 | 验证 |
| --- | --- | --- |
| 0 | 修 Windows 链接器穿透问题，确认 bash/cargo 使用 MSVC `link.exe` | 本地 PowerShell 检查；平台日志待补 |
| 1 | 复现并修 macOS REPL 鼠标选区测试时序 | 隔离 5 次通过；macOS 平台日志待补 |
| 2 | 清理 07-D.2/07-D.1 的残留段落，明确维护回归口径 | 文档 diff；文档门禁 |
| 3 | 推送修复，手动触发平台复现，记录四平台与 ordered summary | 新运行号 |
| 4 | 把 8 项裁定或明确后置写入 00；未裁定则保持入口阻断 | `00-decisions.md` |
| 5 | 收口审核：定向测试、`XIAO_GATE_SELF_TEST=1`、完整门禁/CI | 本地/CI 证据 |

**本批允许修改平台脚本和 REPL 测试夹具**；不改语言语法、类型、IR、Runtime ABI 或
标准库公开契约。不要为了平台变绿而弱化断言或无声扩大 Bun 放行名单。

## 五、验收

1. **Windows**：新的平台复现 Windows 作业绿；日志中不再出现 Git 的 `link.exe`；
   `xiao-doc-coverage-rust` 与全量门禁通过；
2. **macOS**：新的平台复现 macOS 作业绿；若 REPL 测试被隔离放行，必须有重复运行证据
   和 `XIAO_GATE_SELF_TEST=1` 覆盖；
3. **平台汇总**：四平台作业全绿，ordered summary 可查；旧 `39546679818` 只作历史红；
4. **文档**：07-D.2/07-D.1 的残留 F-1/F-5 已标关闭；维护回归口径二选一写清；
   当前 HEAD 写成具体提交号；
5. **裁定**：00 中 8 项有明确选择或明确后置；未登记新阶段代号、语法或公开 API；
6. **不越界**：无并发语法、`Send`/`Sync`/`async`/`Actor`/`Weak`、原子 RC、共享句柄、
   IR/ABI/标准库公开契约变更；
7. **门禁**：定向测试、`XIAO_GATE_SELF_TEST=1`、文档覆盖率和最终 CI 通过。

## 六、不负责

- 不实现线程、任务池、Actor、协程、`async`/`await`、`spawn`/`send`；
- 不实现 `Send`/`Sync` 类型能力、冻结、共享可变、Weak 语法或 `std.concurrent`；
- 不实现原子 RC、共享句柄、对象头/锁或新的 Runtime ABI；
- 不修改现有单线程 `StrongHandle`/`WeakHandle` 布局和 `PhantomData` 边界；
- 不替星崽做 8 项并发裁定；
- 不通过删除断言、跳过测试或扩大放行名单来换取平台绿。

## 七、落点与取证命令

```text
docs/DevDocs/07d3-platform-reproduction-fix-and-closeout.md  本文
docs/DevDocs/07d2-entry-decisions-and-platform-closeout.md   F-5/F-2 残留清理
docs/DevDocs/07d-closeout-and-concurrency-decisions.md       F-1/F-5 关闭记录
docs/DevDocs/07d-concurrency-model-research.md               §11.3 与平台运行号
docs/DevDocs/20l-dict-methods-and-20k-closeout.md            F-6 平台证据
docs/DevDocs/00-decisions.md                                 8 项并发裁定/后置
docs/DevDocs/README.md                                       07-D.3 索引与状态
tools/platform-reproduction/reproduce.ps1                    Windows 链接器穿透
cli/ts/src/repl/multiline.test.ts                            SGR 鼠标选区时序
tools/gates/run.sh                                           仅在证据充分时调整放行/自测
```

```bash
# 平台复现（当前 HEAD 推送后）
gh workflow run 365649226 --ref main
gh run list --workflow 365649226 --limit 3
gh api repos/Programming-Language-Xiao/Xiao/actions/runs/<run-id>/jobs \
  --jq '.jobs[] | "\(.name) \(.status) \(.conclusion)"'

# 失败日志
gh run view <run-id> --job <job-id> --log-failed
# 若 API 抖动，可直接用 token 下载作业日志：
TOKEN=$(gh auth token)
curl -sL -H "Authorization: Bearer $TOKEN" \
  -H "Accept: application/vnd.github+json" \
  "https://api.github.com/repos/Programming-Language-Xiao/Xiao/actions/jobs/<job-id>/logs" \
  -o job.log

# 本地复现 REPL 测试
bun test --test-name-pattern "SGR 鼠标点击和拖动建立选区" cli/ts/src/repl/multiline.test.ts

# 本地门禁
cd core/rust
cargo test --workspace --no-fail-fast
cargo clippy --workspace --all-targets -- -D warnings
cd ../..
bun run check:docs
bun run check:usedocs
bun run check:coverage
XIAO_GATE_SELF_TEST=1 bash tools/gates/run.sh
```

## 八、风险与注意事项

1. **Windows PATH 修复不能只改 PowerShell**：`reproduce.ps1` 调用的是 Git Bash；必须在
   bash/cargo 实际看到的环境里让 MSVC 链接器获胜，或直接指定链接器路径。
2. **macOS 可能是时序抖动，也可能是真实差异**：先重复运行和定位，不要直接加入已知
   抖动名单；若加入，必须有隔离重跑证据和分类器自测。
3. **ordered summary 不是平台绿**：`39546679818` 的 summary 成功但 Windows/macOS 红，
   不能作为通过证据。
4. **维护回归不会因纯文档提交自动运行**：收口时必须写明用 f119ed0 的维护证据覆盖，
   或手动 dispatch 一次。
5. **裁定仍未发生**：没有星崽选择时，8 项并发问题保持入口阻断，不能以“推荐方案”
   代替裁定。
