# 20O-B2-B6-3. cmath/lfloat 错误与资源边界负载实施规划

> **B6-3 三平台边界验收通过（2026-11-18）。** 固定 `58176ff5` 的 85 条用例在每平台的 VM、原生 O0 和 O3 各通过一次，共 765 次。每平台 11 项 C ABI 观察通过，B1/B4 回归及 28 条计数守卫通过。B6-4 待执行。[原始证据与复跑命令](assets/20o-b2-b/entry-20261118-b6-3/README.md)
> 入口：B6-2 固定 `79ea0375` 的九组性能证据与 [source-boundary 证明](assets/20o-b2-b/entry-20261118-b6-2/review/README.md)；当前产品为 `58176ff5`（ABI1.19/生成器22/接口1），B6-3 使用自身源码、Runtime 和工具身份。
> 只做 F-3 §6.1/F-12 错误、资源、原子性与 C ABI 状态边界；不重跑 B6-2 性能、不跑 721、不改产品/ABI/registry/教程。
> B6-1 仍为 `restricted/stage_passed=false`：Windows COFF 导出完整性未验证、Linux Clippy 未执行；B6-3 不得据此写成 B6-1 全通过。

## 一、入口与范围

| 项 | 当前状态 |
| --- | --- |
| B6-2 | 固定 `79ea0375`，三平台 VM/O0/O3 各 183 项、8,235 进程，`perf_pass=true`；后续源码仅有 Rustdoc/加载器移位，source-boundary 证明已归档 |
| 当前产品 | `58176ff5`，ABI1.19/生成器22/接口1；`58176ff5` 到当前 HEAD 的 `core/`、`stdlib/` 无产品差异 |
| B6-1 | `restricted/stage_passed=false`；三平台功能通过，Windows COFF 未验证、Linux Clippy 未执行 |
| 本批目标 | F-3/F-12 边界负载、错误优先级、原子性、C ABI 状态与 B1/B4 回归 |
| 本批结果 | `boundary_passed=true`，macOS 最终五项作业通过。Windows 和 Linux 各两项 B1 采集误判已按实际 Cargo 日志复算，原回执保留 |
| 未做 | B6-2 性能重跑、B6-4 三平台/CI、B6-5 收口/B7 交接、registry/教程、旧721 |

B6-3 按 F-3 §6.1 的资源接纳上限和 F-12 的错误身份判定。进程驻留峰值随执行回执保存。F-3 §6.2 的时间、内存增量和翻倍比结论沿用 B6-2 的固定快照。

### 1.1 要做

1. 冻结边界矩阵：逐项列出 F-3 §6.1/F-12 的 D/W/H、precision、指数、系数、scale、强消去、2/5 因子、非终止、输出 scale 与上限+1，并写明预期错误身份和优先级；
2. cmath：15 项函数的 NaN/Inf、域 009、非有限结果、有符号零、子正规、下溢；min/max 与共享 pow 的错误/位模式；VM 与原生 O0/O3；
3. lfloat：`lfloat_divide` precision=1/32,768 合法、0/32,769 越界；工作/结果/scale 边界；七种舍入与精确短结果不补零；`//`/`%` 的合法 scale 差 200,000 与 200,001 拒绝；lint/lfloat 非平凡底数幂的 1,000,000 与上限+1；
4. 错误优先级与求值顺序：类型002、precision/rounding 012、逐形参输入、除零013或数学域009、精确 `/` 非终止034、工作/结果033、受控分配011；文本内部先原文033、再语法012、再系数/指数/scale033；实参仍按源码顺序求值；
5. C ABI：状态 0/1/2/3/6 的 out 保留/替换与挂起槽行为；Fatal/8 仅在既有可达路径、独立进程取证；VM Rust 错误走 `Fault::Error`/catch，不借 ABI 挂起槽；
6. 原子性：失败无部分结果、输入不变、finally/release、临时值清理；
7. 回归：B1 20 入口/资源 18/双精确/28 守卫、B4 cmath 回归；28/28 是维护工作流计数自检，不代替产品测试；B5 固定矩阵只作历史输入；
8. 平台与身份：Windows/Linux 实际执行；macOS 有 Runner 则执行，否则记 `unverifiable`；逐边界记录 pass/fail/unverifiable，不得把未验证汇总为通过；
9. 证据：新建 `assets/20o-b2-b/entry-20261118-b6-3/`，日志按 SHA-256 内容寻址；B6-1/B6-2/FIX.2/B5 历史目录不改写；定稿后单次 docs 门禁。

### 1.2 明确不做

- 不改产品/测试/看门狗/预算/输入；不缩减边界场景或加 ignore；
- 不改 ABI1.19、生成器22、接口1、intrinsic 1～48/ID100、registry/教程；
- 不重跑 B6-2 性能、不跑旧721、不启动 B6-4/B6-5；
- 不覆盖 B6-1/B6-2/FIX.2/B5 历史证据，不把 B6-1 `restricted` 写成全通过；
- 不在 B6-3 内修产品；发现产品缺陷记录失败并退回所属阶段或另立修复批。

## 二、开工闸门

1. B6-2 审计与 source-boundary 证明通过；B6-1 的 restricted 口径可读；
2. 当前产品 `58176ff5` 到 HEAD 的 `core/`、`stdlib/` 无产品差异；ABI1.19/生成器22/接口1；
3. 边界矩阵已冻结，每项有预期错误身份/优先级和独立输入；
4. 三平台目标/工具链可用；macOS 缺失按 `unverifiable` 记录；
5. 唯一任务根、独立 target/Runtime/缓存已建立；每平台/后端/优化/边界独占输出路径；
6. 历史证据只读；B6-3 只做验证与证据，产品/测试变更须退回或另裁。

## 三、分批执行

### B6-3A：边界矩阵冻结

1. 列出 D/W/H、precision、指数、系数、scale、强消去、2/5、非终止、输出 scale、上限+1 的输入与预期错误；
2. 固定矩阵 SHA-256 和生成规则；运行前冻结，不按已执行子集反推全量；
3. 记录每项适用平台/后端/优化级别。

退出：边界矩阵和预期身份冻结，覆盖 F-3/F-12 必测项。

### B6-3B：Runtime/VM 资源与错误

1. lfloat 算术/转换/精确除法的 D/W/H、precision、指数、scale 边界；
2. cmath 15 项 NaN/Inf/域/非有限/有符号零/子正规/下溢；
3. `lfloat_divide`、`//`/`%`、非平凡底数幂的边界与错误优先级；
4. 失败无部分结果、输入不变、finally/release；
5. 记录 VM 与原生 O0/O3 的实际错误身份/span 与黄金。

退出：Runtime/VM 边界与错误身份稳定。

### B6-3C：C ABI/out/挂起槽/Fatal

1. 状态 0/1/2/3/6 的 out 保留/替换、挂起槽不变/设置；
2. 011 受控分配失败与非恢复 Fatal/8 的独立进程取证；
3. VM `Fault::Error`/catch 与 ABI 挂起槽隔离；
4. 保存每项实际返回码、out/挂起槽快照与日志。

退出：C ABI 状态/所有权/错误通道符合冻结契约。

### B6-3D：回归与三平台

1. B1 20 入口/资源 18/双精确/28 守卫、B4 cmath 回归；
2. Windows/Linux 实际执行；macOS 有 Runner 则执行，否则 `unverifiable`；
3. 逐边界逐项 pass/fail/unverifiable，失败保留；
4. 保存平台、工具、Runtime、执行文件、PID、OS 原始单位与日志。

退出：回归与平台覆盖逐项记录，未验证项不汇总为通过。

### B6-3E：证据与 B6-4 交接

1. 生成 B6-3 证据清单，内容寻址；
2. 汇总边界矩阵、错误身份、原子性、C ABI 与平台结果；
3. 形成 B6-4 入口：三平台与 CI 工程门禁；本批不启动 B6-4；
4. 历史证据不改写；B6-5/B7 仍后置。

退出：B6-3 证据完整；B6-4 入口明确；B6-5/B7 未开始。

## 四、证据矩阵

| 层 | 必测 |
| --- | --- |
| 资源 | D/W/H、precision、指数、系数、scale、上限+1、强消去、2/5、非终止 |
| cmath | 15 项 NaN/Inf/域/非有限/有符号零/子正规/下溢 |
| 错误 | 002/009/011/012/013/033/034 及 F-12 优先级/顺序 |
| 原子性 | 无部分结果、输入不变、finally/release、临时值 |
| C ABI | 状态 0/1/2/3/6、out/挂起槽、Fatal/8 隔离 |
| 回归 | B1 20/18/双精确/28 守卫、B4 cmath |
| 平台 | Windows/Linux 实执行、macOS 实际或 `unverifiable` |
| 身份 | 平台/工具/Runtime/执行文件/PID/源码前后摘要 |
| 边界 | 无产品/测试/ABI/registry/721 变更 |

## 五、退出条件

1. 边界矩阵冻结且覆盖 F-3/F-12 必测项；
2. Runtime/VM 边界与预期错误身份一致；失败无部分结果、输入不变；
3. C ABI 状态/out/挂起槽/Fatal 符合冻结契约；
4. B1/B4 回归通过；28/28 只作工作流自检；
5. 三平台实际或 `unverifiable` 明确，未验证不汇总为通过；
6. 无产品/测试/ABI/registry/721 变更，历史证据未改写；
7. B6-4 入口明确；B6-5/B7 未开始。

## 六、风险与禁止

- 把边界通过当成 B6 性能通过；
- 缩减边界场景、放宽预算或改输入取得绿色；
- 用 B5 固定矩阵或旧721代替 B6-3 当前快照；
- 覆盖 B6-1/B6-2/FIX.2 历史证据；
- 在 B6-3 内修产品、改 ABI/registry/教程；
- 把 macOS `unverifiable` 写成三平台通过。

## 七、交接

B6-4 以 `58176ff5` 的三平台边界回执为入口，执行三平台与 CI 工程门禁。B6-5 做整批收口和 B7 交接。B6-1 按受限状态引用，B6-2 性能绑定 `79ea0375`。[B6-3 汇总](assets/20o-b2-b/entry-20261118-b6-3/summary.json)

自审复核了固定收口、仓库门禁回执与三平台原始记录。macOS 五份原始产物的 2,306 个文件逐字节一致，八个审计反例按预期拒绝。[复核命令与回执](assets/20o-b2-b/entry-20261118-b6-3/README.md#复核与交接)

新 Agent 先读：

1. [B6 资源性能规划](20o-b2-b6-resource-performance-plan.md)；
2. [B6-2 实测与自审](assets/20o-b2-b/entry-20261118-b6-2/README.md)；
3. [F-3 资源/性能](20o-b2-freeze-c-resource-dependency-plan.md)；
4. [F-12 错误表](20o-b2-freeze-b3-error-table-plan.md)；
5. [A6 资源/性能历史](20o-b2-a6-resource-performance-plan.md)。

## 八、落点

```text
core/rust/crates/xiao-runtime/                         资源/错误/C ABI
core/rust/crates/xiao-vm/                              VM 错误/清理
core/rust/crates/xiao-driver/tests/                    边界/协议回归
core/rust/crates/xiao-driver/examples/                 受控测量入口（若边界需要）
docs/DevDocs/assets/20o-b2-b/entry-20261118-b6-3/      B6-3 证据
```

本页只做 B6-3；B6-4、B6-5 与 B7/B2-C 仍后置。
