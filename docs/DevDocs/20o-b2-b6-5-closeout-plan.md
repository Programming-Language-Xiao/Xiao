# 20O-B2-B6-5. B6 整批收口与 B7/B2-C 交接规划

> **状态（2026-11-20）　B6-5 已完成内部收口。** 固定产品 `e79fab3c` 的 Linux 工程门禁通过，`b6_5_passed=true`、`b7_entry_ready=true`、`b6_accepted=false`。B6-1 维持受限，B6-4 五字段按正式回执交接。B7 与 B2-C 未开工。[收口证据与只读审计](assets/20o-b2-b/entry-20261120-b6-5/README.md)
> 入口：[B6-4 交接状态](assets/20o-b2-b/entry-20261119-b6-4-followup/status.json)、[B6-2 实测](assets/20o-b2-b/entry-20261118-b6-2/README.md)、[B6-3 实测](assets/20o-b2-b/entry-20261118-b6-3/README.md)、[B6-1 重跑](assets/20o-b2-b/entry-20261118-b6-1-rerun/README.md)、[B6-1 macOS 专项](assets/20o-b2-b/entry-20261118-b6-1-macos/README.md)。
> B6-5 不重跑 B6-2 全量性能、不重跑 B6-3 全边界、不跑旧 721、不改产品/ABI/registry/教程/工作流阈值，不关闭 22D 或 Windows COFF 缺项。

## 一、入口与范围

| 项 | B6-5 接收状态 |
| --- | --- |
| B6-1 | `restricted/stage_passed=false`；三平台功能通过；Windows COFF `UnverifiedCoffExports`；Linux Clippy 历史未执行 |
| B6-2 | `perf_pass=true`，三平台 9 组×183 项、8,235 个独立进程，绑定 `79ea0375`；source-boundary 证明随交接引用 |
| B6-3 | `boundary_passed=true`，三平台 85×9=765 次，C ABI 11 项/平台，绑定 `58176ff5`；Windows COFF 未验证 |
| B6-4 | 受限交接；`b6_4_passed=false`、`original_passed=false`、`followup_passed=false`、`windows_coff_verified=false`、`22d_open=true` |
| 22D | 三处间歇性释放差分未归因：`native-lock-wait-027`、`xcon-bounded-string-success`、`spawn-join-str-multiargument` |
| Windows COFF | `windows_coff_verified=false`；B6-1/B6-3/B6-4 均不得写成 `Complete` |
| 当前产品 | ABI 1.19、生成器 22、标准库接口 1、intrinsic 1～48、ID 100 墓碑；B6-5 开工时固定最终受保护产品树 |
| 本批目标 | 汇总 B6 各阶段、对账证据与平台范围、跑最终工程门禁、形成 B7 入口和 B2-C 交接清单 |
| 本批不做 | B6-2 全量重测、B6-3 全边界重跑、旧 721、B2-C 最终发布、22D 归因、COFF 工具改造 |

B6-5 是 **B6 内部收口批**，不是 B6 通过批，也不是 B2-C 发布批。B6-1/B6-4 的受限字段是入口事实；B6-5 只能记录、对账和交接，不能通过重跑或改字段把它们变成通过。

### 1.1 要做

1. 固定 B6-5 最终产品提交、受保护树/blob、Cargo.lock、Runtime、工具链、目标和平台边界；
2. 汇总 B6-1、B6-2、B6-3、B6-4 及其后续补验，逐阶段登记源码/Runtime/工具/平台/证据身份；
3. 对账 B6-2 `79ea0375`、B6-3 `58176ff5` 到 B6-5 最终产品树的受保护差异；只有已证明与性能/边界路径无关时才沿用原结论；
4. 保留 B6-4 五字段、22D 三处失败、B6-1 两项缺项和 Windows COFF 验证等级，逐项写明后续归属；
5. 在最终产品树上运行完整 workspace 工程门禁，记录 CI run/job/attempt 或本机进程身份；
6. 复核 B6-4 的 `local-checks.zip`（Linux 164 导出、Windows 定向 Clippy）与 B6-4 只读审计；最终树不一致时重跑受影响项；
7. 形成 B7 入口：B1 回归、A0～A7/B0～B6 对账、22D 追踪、教程/registry 边界；
8. 形成 B2-C 交接清单：完整 721、三平台公共产物、最终教程/registry、Windows COFF 与平台补验；
9. 证据落在 `assets/20o-b2-b/entry-<date>-b6-5/`，按 SHA-256 内容寻址；定稿后单次 docs 门禁。

### 1.2 明确不做

- 不把 B6-1 `restricted`、B6-4 五字段或 Windows COFF 未验证写成通过；
- 不重跑 B6-2 全量性能或 B6-3 全边界；只做受影响项补验；
- 不运行旧 721，不启动 B2-C；
- 不改 ABI 1.19、生成器 22、接口 1、intrinsic 1～48/ID 100，不新增 C 符号或容器 schema；
- 不改产品、测试、预算、看门狗、工作流阈值或工作流默认超时来补绿；
- 不在 B6-5 内修 22D、COFF 工具或 B6-1/B6-4 的产品缺陷；发现缺陷保留失败并退回所属批次；
- 不改写 B6-1/B6-2/B6-3/B6-4/FIX.2/B5 历史证据和原始失败。

### 1.3 结论字段

B6-5 用两组结论分开记录，禁止互相替换：

```text
b6_accepted =
  b6_1_stage_passed
  and b6_2_perf_pass
  and b6_2_final_tree_covered
  and b6_3_boundary_passed
  and b6_3_final_tree_covered
  and b6_4_passed
```

当前 `b6_accepted=false`，因为 B6-1 与 B6-4 都是受限状态。`b6_2_final_tree_covered` 和 `b6_3_final_tree_covered` 只有在 B6-2/B6-3 快照到最终树的受保护差异已证明不触及性能/边界路径，或受影响项已补测后，才能为 true。B6-5 不得把 `b6_accepted` 改成 true，除非各阶段都有正式通过回执且最终树覆盖成立。

```text
b6_5_passed =
  evidence_inventory_complete
  and stage_reconciliation_complete
  and final_workspace_gates_passed
  and status_fields_consistent
  and open_items_assigned
```

`b6_5_passed=true` 只表示“收口证据、最终工程门禁和交接完成”，不表示 B6 通过，也不表示 `perf_pass` 覆盖最终树。B6-5 证据和状态必须同时写明：

- `open_items_assigned=true` 才允许 B7 入口；
- 22D → B7；
- Windows COFF → B2-C；
- B6-1 Linux Clippy 历史缺项 → B7-4 最终工程门禁；
- `b7_entry_ready` 只有在 `b6_5_passed && open_items_assigned`，且 `b6_accepted` 已按四个阶段的正式回执明确（当前为 false，不得写成 true）时才能为 true。

## 二、开工闸门

1. B6-1/B6-2/B6-3/B6-4 的证据审计通过；B6-4 五字段与 [status.json](assets/20o-b2-b/entry-20261119-b6-4-followup/status.json) 一致；
2. B6-2 `79ea0375`、B6-3 `58176ff5` 的固定证据可读；source-boundary 证明和 B6-2 review 可复核；
3. 22D 三处失败、Windows COFF `UnverifiedCoffExports`、B6-1 Linux Clippy 未执行均有原始回执和后续归属；
4. B6-5 最终产品树、ABI/生成器/接口、intrinsic/ID 100、两个 Cargo.lock 和平台工具链已固定；
5. 最终树与 B6-2/B6-3 快照的受保护差异已列出；性能/边界路径是否有变化已判定，需补验项已列清单；
6. Linux/Rust 1.96.0 的 workspace 测试、Clippy、fmt、`check:lock`、`repo-check` 入口可用；缺项不得写成通过；
7. 唯一任务根、独立 target/Runtime/缓存、每平台/后端/负载独占输出路径已建立；
8. B7/B2-C 未开始；B6-5 只新增收口文档和证据，不改产品。

## 三、分批执行

### B6-5A：最终快照与证据清单

1. 固定最终受保护产品提交/tree/blob，至少覆盖 `core/`、`stdlib/`、`.cargo/`、`tests/`、`docs/UseDocs/`、`docs/module-registry.json`、两个 Cargo.lock；
2. 记录 ABI 1.19、生成器 22、标准库接口 1、intrinsic 1～48、ID 100 墓碑；确认无新增 49+ 或 C 符号；
3. 建立 B6 证据清单：B6-1 重跑/macOS、B6-2+review、B6-3+review、B6-4+followup+local-checks、22D 记录、B6-2 source-boundary、B6-5 新证据；
4. 每个路径绑定 Git commit/tree/blob、内容 SHA-256、平台、Runtime/工具身份；历史目录只读，不改写原始失败；
5. 生成 `79ea0375`→最终树、`58176ff5`→最终树的受保护差异表，标注性能路径、cmath/lfloat 路径、归档/加载器、组件登记、测试/语料变化；
6. 判定哪些差异需要补测；不按版本号相同继承性能或边界结论。

退出：最终身份、证据清单、差异表和补测清单冻结。

### B6-5B：阶段对账与受限状态

1. B6-1：保留 `restricted/stage_passed=false`、三平台功能通过、Windows COFF 未验证、Linux Clippy 历史未执行；不得用 B6-4 的 Linux Clippy 或 B6-5 最终门禁改写 B6-1 历史；
2. B6-2：保留 `perf_pass=true` 和 `79ea0375` 绑定；核对九组×183、8,235 进程、全转义 URL、O0/O3、内存/翻倍比和 source-boundary；差异触及性能路径时只补测受影响项，不重跑全量；
3. B6-3：保留 `boundary_passed=true` 和 `58176ff5` 绑定；核对 765 次、C ABI 11 项/平台、B1/B4 回归和 28/28 回放；差异触及边界路径时只补验受影响项；
4. B6-4：保留五字段和原始/后续区分；核对 `original_passed=false`、`followup_passed=false`、`windows_coff_verified=false`、`22d_open=true`、Linux 164 导出、Windows 定向 Clippy、Windows 22D 三处失败；
5. B5：固定 1.19/21 矩阵只作历史输入；当前 1.19/22 与 21→22 增量分层引用，不把 B5 历史矩阵写成 B6-5 成绩；
6. 22D：列出三处失败、原始轨迹和复现条件，归 B7；B6-5 不做新归因，不把失败改记为不可验证；
7. Windows COFF：列 B6-1/B6-3/B6-4 的实际等级和缺项，归 B2-C；不把 Windows 功能通过写成 COFF `Complete`；
8. B6-1 Linux Clippy：保留历史 `not_run`；B6-5 最终 Linux Clippy 另存新回执，不能回填 B6-1。

退出：每个阶段的状态、证据、差异、失败和后续归属逐项可对账。

### B6-5C：最终工程门禁

1. 在 B6-5 最终产品树上使用 Rust 1.96.0、`--locked`、独立 target/Runtime 运行：

```text
cargo +1.96.0 test --manifest-path core/rust/Cargo.toml --workspace --locked
cargo +1.96.0 clippy --manifest-path core/rust/Cargo.toml --workspace --all-targets --locked -- -D warnings
cargo +1.96.0 fmt --manifest-path core/rust/Cargo.toml --all -- --check
bun run check:lock
bun tools/repo-check/src/cli.ts all
```

2. Linux workspace Clippy 必须实际执行；Windows 定向 Clippy 不能替代 Linux Clippy；缺组件时记 `not_run`，不得写成通过；
3. 保存 CI run/job/attempt 或本机命令/PID、源码前后摘要、Runtime/工具/目标、日志和退出码；最终树变化后不得直接引用 B6-4 旧门禁；
4. 最大负载和 B6-2 性能仍由独立 release 测量入口处理，不塞入默认 debug 测试；B6-5 不重跑全量性能；
5. 复核 `local-checks.zip` 的 Linux 164 导出和 Windows 定向 Clippy；最终受保护树与 `780398e5` 不一致时重跑受影响项；
6. 任一门禁失败或未执行：`b6_5_passed=false`，保留失败并退回所属阶段；不得改阈值、删测试、加 ignore 或把失败改写为不可验证。

退出：最终树上的工程门禁逐项有回执，失败/未执行项明确。

### B6-5D：状态派生与交接状态

1. 生成 B6-5 状态文件，至少包含：

```text
b6_5_passed
b6_accepted
b6_1_stage_passed
b6_2_perf_pass
b6_2_final_tree_covered
b6_3_boundary_passed
b6_3_final_tree_covered
b6_4_passed
windows_coff_verified
22d_open
open_items_assigned
b7_entry_ready
```

2. `b6_accepted` 按 §1.3 合取计算；当前必须为 false；
3. `b6_5_passed` 只按收口条件计算，不能把 `b6_accepted=false` 覆盖成 true；
4. `open_items_assigned=true` 要求 22D→B7、Windows COFF→B2-C、B6-1 Linux Clippy→B7-4 均有明确条目；
5. `b7_entry_ready` 必须在 B6-4 五字段、22D、COFF、B6-1 受限范围和 B2-C 清单全部写入后才为 true；
6. 审计脚本从固定证据和实际门禁回执派生上述字段，不能手工写 true；至少包含 128 组状态组合和伪造状态反例。

退出：状态文件、审计输出、交接文档三者一致，且 `b6_accepted=false`。

### B6-5E：B7 与 B2-C 入口

1. B7 入口表：B6-1 受限、B6-2 性能绑定、B6-3 边界绑定、B6-4 五字段、22D 三处失败、Windows COFF 未验证、B5 历史矩阵分层；
2. B7 必须明确：不得把 B6-4 `followup_passed=false` 写成通过，不得把 COFF 未验证写成 `Complete`，不得关闭 22D；
3. B2-C 入口表：完整 721、三平台公共产物、最终教程/registry、Windows COFF/平台补验、B6 新增数值负载；
4. B2-C 未开始；B6-5 不运行 721、不升级 registry/教程；
5. 生成 B6-5 证据归档和只读审计；定稿后单次 docs 门禁。

退出：B7 入口和 B2-C 清单完整，B2-C 未启动。

## 四、证据矩阵

| 层 | 必测 |
| --- | --- |
| 快照 | 最终产品提交/tree/blob、两个 Cargo.lock、Runtime/工具/目标、ABI/生成器/接口、intrinsic/ID 100 |
| 阶段 | B6-1 受限字段、B6-2 `79ea0375`、B6-3 `58176ff5`、B6-4 五字段、B5 历史矩阵 |
| 差异 | `79ea0375`/`58176ff5`→最终树的受保护差异、性能/边界路径判定、补测清单 |
| 门禁 | Linux workspace 测试、Clippy、fmt、`check:lock`、`repo-check`；实际 CI/进程身份 |
| 补验 | Linux 164 导出、Windows 定向 Clippy、受影响性能/边界项（若有） |
| 受限项 | 22D 三处、Windows COFF、B6-1 Linux Clippy；原始失败保留 |
| 状态 | `b6_accepted`、`b6_5_passed`、`open_items_assigned`、`b7_entry_ready` |
| 交接 | B7 入口、B2-C 721/三平台/教程/registry 清单；B2-C 未开始 |
| 历史 | B6-1/B6-2/B6-3/B6-4/FIX.2/B5 原始证据未改写；旧 721 未跑 |

## 五、退出条件

1. B6-5A～E 完成，最终快照、证据清单、差异表和补测清单冻结；
2. 最终树上的 workspace 测试、Linux Clippy、fmt、`check:lock`、`repo-check` 全部有实际回执；任一失败/未执行则 `b6_5_passed=false`；
3. B6-1/B6-2/B6-3/B6-4 状态逐项对账；B6-4 五字段不变，`b6_accepted=false`；
4. B6-2 性能与 B6-3 边界只在已证明的源码范围内引用；受影响项已补测或明确转交；
5. 22D 三处、Windows COFF、B6-1 Linux Clippy 的原始失败和后续归属完整；不得改写为通过；
6. 无产品/测试/ABI/registry/教程/工作流阈值变更；旧 721 未跑；历史证据未改写；
7. B7 入口和 B2-C 清单完整；B2-C 未开始；
8. 状态文件、审计输出和交接文档一致；定稿后单次 docs 门禁。

## 六、风险与禁止

- 把 `b6_5_passed=true` 写成 B6 通过、`b6_accepted=true` 或 `perf_pass` 覆盖最终树；
- 把 B6-4 `followup_passed=false`、`b6_4_passed=false` 改成 true，或重写原批失败；
- 把 Windows COFF `UnverifiedCoffExports` 写成 `Complete`；
- 把 Windows 定向 Clippy 当作 Linux Clippy，或把 B6-1 历史 `not_run` 回填；
- 用 B6-2/B6-3 旧快照结论直接覆盖最终树，不核对源码差异；
- 重跑 B6-2 全量或 B6-3 全边界来冒充收口；
- 改工作流阈值/超时、删测试、加 ignore 或改预算取得绿色；
- 运行旧 721、启动 B2-C、提前升级 registry/教程；
- 在 B6-5 内修 22D/COFF/产品缺陷，或把“已登记”当作签收。

## 七、交接

B6-5 已在最终树上完成 Linux workspace、Clippy、fmt、锁和仓库门禁，默认 workspace 为 1,760 通过、0 失败、56 项原有忽略。B6-2 的最终树覆盖来自源码路径核对，测量成绩固定于原快照。B6-3 的最终树 Linux 定向补验通过，Windows 和 macOS 最终树产物验证归 B2-C。B7 接收 [交接清单](assets/20o-b2-b/entry-20261120-b6-5/handoff.json)。

B6-5 完成后，B7 按以下入口接收：

- B6-1 `restricted/stage_passed=false`；
- B6-2 `perf_pass=true` 绑定 `79ea0375`，附最终树差异结论；
- B6-3 `boundary_passed=true` 绑定 `58176ff5`，附最终树差异结论；
- B6-4 五字段受限交接；
- 22D 三处失败 → B7；
- Windows COFF → B2-C；
- B6-1 Linux Clippy 历史缺项 → B7-4 最终工程门禁。

B2-C 仍后置；B6-5 只准备入口，不执行最终 721、三平台公共产物、教程/registry 升级或发布。

## 八、落点

```text
docs/DevDocs/20o-b2-b6-5-closeout-plan.md              本规划
docs/DevDocs/assets/20o-b2-b/entry-<date>-b6-5/        B6-5 证据
docs/DevDocs/20o-b2-b7-closeout-plan.md                B7 入口
docs/DevDocs/20o-b2-c-final-closeout-plan.md           B2-C 清单
core/rust/Cargo.lock                                  只读复核
tests/benchmarks/Cargo.lock                           只读复核
```

本页只做 B6-5；B7/B2-C 仍后置。

## 九、新 Agent 先读

1. [B6 资源性能规划](20o-b2-b6-resource-performance-plan.md)；
2. [B6-4 交接状态](assets/20o-b2-b/entry-20261119-b6-4-followup/status.json)；
3. [B6-2 实测与自审](assets/20o-b2-b/entry-20261118-b6-2/README.md)；
4. [B6-3 实测](assets/20o-b2-b/entry-20261118-b6-3/README.md)；
5. [B6-1 重跑](assets/20o-b2-b/entry-20261118-b6-1-rerun/README.md)；
6. [B7 收口规划](20o-b2-b7-closeout-plan.md)；
7. [B2-C 最终收口规划](20o-b2-c-final-closeout-plan.md)。
