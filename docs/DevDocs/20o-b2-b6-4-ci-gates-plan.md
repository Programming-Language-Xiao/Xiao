# 20O-B2-B6-4. 三平台与 CI 工程门禁实施规划

> **B6-4 尚未开始；B6-3 边界功能通过，B6-5 后置。**
> 入口：B6-3 固定 `58176ff5` 的三平台边界证据（765 次语言执行、C ABI 11 项、B1 9/18、B4 回归、28/28 计数守卫）；B6-2 性能绑定 `79ea0375` 及其 source-boundary 证明；B6-1 仍为 `restricted/stage_passed=false`。
> B6-4 只做三平台执行、CI 与工程门禁证据；不重跑 B6-2 性能、不重跑 B6-3 全边界、不跑 721、不改产品/工作流阈值/ABI/registry。
> Windows COFF 导出完整性未验证、Linux Clippy 未执行；B6-4 不得写成三平台完整性或 Clippy 全通过。

## 一、入口与范围

| 项 | 当前状态 |
| --- | --- |
| B6-3 | 固定 `58176ff5`，`boundary_passed=true`；三平台 VM/O0/O3 各 85 条、C ABI 11 项、B1 9/18、B4 两别名、28/28 守卫 |
| B6-2 | 固定 `79ea0375`，`perf_pass=true`；source-boundary 证明后续仅有 Rustdoc/加载器移位，新增性能样本 0 |
| B6-1 | `restricted/stage_passed=false`；Windows COFF `UnverifiedCoffExports`、Linux Clippy 未执行 |
| 当前产品 | `58176ff5`，ABI1.19/生成器22/接口1；到当前 HEAD 的 `core/`、`stdlib/` 无产品差异 |
| 本批目标 | 三平台实际执行或 `unverifiable`、CI run/job/attempt、workspace/Clippy/fmt/lock/repo-check、B1 28/28 与平台身份 |
| 未做 | B6-2 性能重跑、B6-3 全边界重跑、B6-5 收口/B7 交接、registry/教程、旧721 |

B6-4 是 **工程门禁与 CI 证据批**，不是性能或边界验收批。B6-2 的 `perf_pass` 与 B6-3 的 `boundary_passed` 只按各自快照引用；B6-4 不重测性能、不把工程门禁绿色写成性能或三平台完整性通过。

### 1.1 要做

1. 固定 B6-4 源码与身份：产品 `58176ff5`、各平台 Runtime/toolchain/target/锁；B6-2 `79ea0375` 与其 source-boundary 证明、B6-3 `58176ff5` 边界回执分别引用；
2. 在固定源码上运行工程门禁：完整 workspace 测试、Clippy、fmt、`check:lock`、`repo-check all`（仅既有 A0 警告）；Linux 本地 Clippy 缺失时如实记 `not_run`/`unverifiable`，不伪造；
3. 三平台执行与 CI：Windows/Linux/macOS 实际执行或明确 `unverifiable`；保存 CI run/job/attempt、维护工作流/脚本摘要、源码/Runtime/tool/target、日志与平台身份；
4. B1 28/28 计数守卫：4 项真实输出回放 + 24 项派生负例，是工作流自检，不代替产品测试；保存脚本/工作流摘要；
5. push 路径过滤：未触发时按事实写明，不把“有 CI 记录”当成覆盖证明；
6. 逐平台/后端/优化级别记录结果；失败与不可验证项保留，不得汇总为通过；
7. 证据：新建 `assets/20o-b2-b/entry-20261118-b6-4/`，日志按 SHA-256 内容寻址；B6-1/B6-2/B6-3 历史目录不改写；定稿后单次 docs 门禁；
8. B6-5 交接：汇总 CI/工程门禁/平台证据，列出失败/不可验证项，不启动 B6-5 或 721。

### 1.2 明确不做

- 不改产品/测试/工作流阈值/默认超时/预算/输入；不把 A6 的 40 分钟时限自动继承到 B6；
- 不改 ABI1.19、生成器22、接口1、intrinsic 1～48/ID100、registry/教程；
- 不重跑 B6-2 性能、不重跑 B6-3 全边界、不跑旧721、不启动 B6-5；
- 不覆盖 B6-1/B6-2/B6-3 历史证据，不把 B6-1 `restricted` 写成全通过；
- 不把工程门禁绿色写成三平台完整性、Clippy 全通过或性能通过；
- 不在 B6-4 内修产品；发现缺陷记录失败并退回所属阶段或另立修复批。

## 二、开工闸门

1. B6-3 审计与 B6-2 source-boundary 证明通过；B6-1 restricted 口径可读；
2. 当前产品 `58176ff5` 到 HEAD 的 `core/`、`stdlib/` 无产品差异；ABI1.19/生成器22/接口1；
3. 三平台目标/工具链/CI Runner/权限与 run/job/attempt 可取得；macOS 缺失按 `unverifiable`；
4. workspace/Clippy/fmt/`check:lock`/`repo-check` 入口可复现；Linux Clippy 缺失已登记；
5. 唯一任务根、独立 target/Runtime/缓存已建立；历史证据只读；
6. 不改工作流阈值/默认超时；若需要调整，先请星崽另行裁定。

## 三、分批执行

### B6-4A：固定源码与工程门禁

1. 固定 `58176ff5`、锁、工具链、目标与 Runtime 身份；复核无产品漂移；
2. 运行完整 workspace 测试、Clippy、fmt、`check:lock`、`repo-check all`；
3. 记录每项命令、PID、前后源码/锁摘要、工具与 Runtime 身份、原始日志；
4. 保存失败/重试；Linux Clippy 缺失如实标注。

退出：工程门禁逐项有回执，失败/缺项明确。

### B6-4B：三平台执行与 CI

1. Windows/Linux/macOS 实际执行或明确 `unverifiable`；macOS 用固定 Runner 时保存 run/job/attempt；
2. 保存工作流/脚本/源码/Runtime/tool/target 摘要与平台身份；
3. 逐平台结果分别记录，不合并成三平台通过；
4. push 路径过滤按事实记录。

退出：三平台身份与结果可复核，缺平台/失败保留。

### B6-4C：B1 守卫与平台完整性

1. 运行 28 项计数守卫：4 真实输出 + 24 派生负例；保存脚本/工作流摘要；
2. 核对 Windows COFF、Linux ELF、macOS Mach-O 的实际验证等级，不升级未验证项；
3. 保存平台/工具/Runtime/执行文件摘要与日志。

退出：守卫与平台完整性口径准确，未验证项不写成通过。

### B6-4D：证据与 B6-5 交接

1. 生成 B6-4 证据清单，内容寻址；
2. 汇总工程门禁、CI、平台结果、失败/不可验证项；
3. 形成 B6-5 入口：B6 整批收口；本批不启动 B6-5；
4. 历史证据不改写；B7/B2-C 仍后置。

退出：B6-4 证据完整；B6-5 入口明确。

## 四、证据矩阵

| 层 | 必测 |
| --- | --- |
| 工程门禁 | workspace、Clippy、fmt、`check:lock`、`repo-check`；Linux Clippy 缺项如实登记 |
| CI | Windows/Linux/macOS run/job/attempt、工作流/脚本摘要、push 路径过滤事实 |
| 平台 | 实际执行或 `unverifiable`；COFF/ELF/Mach-O 验证等级单列 |
| 守卫 | 28/28 = 4 真实 + 24 派生；工作流自检不代替产品测试 |
| 身份 | 源码/锁/Runtime/tool/target/执行文件/PID/日志摘要 |
| 边界 | 无产品/工作流阈值/ABI/registry/721 变更 |

## 五、退出条件

1. B6-4A 工程门禁逐项有回执，失败/缺项明确；
2. 三平台实际执行或 `unverifiable` 明确，CI 身份可核对；
3. 28/28 守卫与平台验证等级记录准确；
4. 无产品/测试/工作流阈值/ABI/registry/721 变更；
5. 历史 B6-1/B6-2/B6-3 证据未改写；
6. 未把工程门禁、Windows COFF 或 Linux Clippy 缺项写成全通过；
7. B6-5 入口明确；B6-5/B7/B2-C 未开始。

## 六、风险与禁止

- 把工程门禁绿色写成三平台完整性/性能通过；
- 修改工作流阈值/默认超时或继承 A6 时限补绿；
- 把 B6-1/B6-2/B6-3 快照混成同一测量身份；
- 忽略 Linux Clippy/Windows COFF 缺项；
- 覆盖历史证据或启动 B6-5/721；
- 在 B6-4 内修产品。

## 七、交接

B6-4 退出后进入 B6-5：整批收口、教程/registry 边界与 B7 交接。B6-1 仍按 restricted 引用；B6-2 性能绑定 `79ea0375`；B6-3 边界绑定 `58176ff5`；B6-4 使用自身身份，不混用。

新 Agent 先读：

1. [B6 资源性能规划](20o-b2-b6-resource-performance-plan.md)；
2. [B6-3 实测](assets/20o-b2-b/entry-20261118-b6-3/README.md)；
3. [B6-2 实测与自审](assets/20o-b2-b/entry-20261118-b6-2/README.md)；
4. [B6-1 macOS 专项](assets/20o-b2-b/entry-20261118-b6-1-macos/README.md)；
5. [官方库总纲](20o-official-library-program.md)。

## 八、落点

```text
core/rust/crates/xiao-*/                               工程门禁
.github/workflows/                                     CI run/job/attempt
tools/gates/                                            维护工作流/守卫脚本
docs/DevDocs/assets/20o-b2-b/entry-20261118-b6-4/      B6-4 证据
```

本页只做 B6-4；B6-5、B7 与 B2-C 仍后置。
