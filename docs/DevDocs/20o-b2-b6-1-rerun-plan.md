# 20O-B2-B6-1-RERUN. B6-1 重跑与收口规划

> **RERUN 已完成Windows/Linux限定范围（2026-11-18）；B6-1形成受限结论，B6-2性能未开始。** Linux官方包装原阻断已解除；Windows功能通过、COFF导出完整性未验证；macOS未验证，`stage_passed=false`，不记三平台通过。见[本批证据](assets/20o-b2-b/entry-20261118-b6-1-rerun/README.md)。
> 入口：FIX.2 产品 `43f18274`（组件推导修复；ABI1.19/生成器22/接口1），以及 FIX.2 证据与反例记录。
> B6-1 原证据 `entry-20261117-b6` 保持 `stage_passed=false`/`blocked` 历史身份，不得改写；本批必须在 B6-1 自己的任务根重跑，不能用 FIX.2 证据代替 B6-1 签收。
> 平台口径已冻结：Windows 功能通过但 COFF `UnverifiedCoffExports`，导出完整性未验证；Linux ELF `Complete`；macOS 未验证。三者不得合并成“三平台通过”。
> 不修改产品/测试/看门狗/预算/ABI/registry；不跑旧721。

## 一、入口与范围

| 项 | 当前状态 |
| --- | --- |
| FIX.2 | 产品 `43f18274`；组件推导修复，codegen 109 passed、driver 29 passed，双平台 cmath 包装矩阵通过 |
| FIX.2证据 | [实施证据](assets/20o-b2-b/entry-20261118-b4-component-fix/README.md)；[反例记录](assets/20o-b2-b/entry-20261118-b4-component-review/README.md)（8项） |
| B6-1原证据 | [原阻断记录](assets/20o-b2-b/entry-20261117-b6/README.md)：`stage_passed=false`、`blocked=true`；Windows 功能通过/COFF 未验证，Linux 官方包装被拒，macOS 未验证 |
| 版本 | ABI1.19/生成器22/接口1；B5 固定1.19/21矩阵与21→22增量分层引用 |
| 本批结果 | 自有任务根Windows/Linux包装与相关回归通过；Linux ELF Complete且声明=观察，Windows COFF未验证；B6-1为restricted，B6-2仅具备两平台受限入口，未启动 |
| 未做 | B6-2 性能、macOS 实机、三平台最终发布、registry/教程、旧721 |

B6-1 是 **资源/错误/原子性验收批**，不是性能批。本批只重跑与 FIX.2 修复相关的 B6-1 场景和回归；B6-2 的五批性能、OS 峰值与全转义 URL 最大负载仍后置。

### 1.1 要做

1. 新建 B6-1 重跑任务根（Windows/Linux），绑定 FIX.2 产品 `43f18274`、Rust1.96、原锁、各自 Runtime/工具/标准库；Windows 为 x86_64-pc-windows-msvc，Linux 为 Ubuntu WSL2/LLVM 21.1.8；
2. Linux 原阻断场景：真实官方 cmath 两别名 O0/O3，`ArtifactVerification` 必须为 ELF `Complete`，声明组件等于观察组件（含 `containers`），逐字黄金与 009 捕获通过；
3. Windows 同矩阵：功能通过，但 COFF 仍为 `UnverifiedCoffExports`，只能记“功能通过、导出完整性未验证”，不得伪造 `Complete`；
4. 相关回归：B6-1 Runtime/VM/归档、A6 资源/原子性 8/8、错误/原子性/显式除法/巨大商/输入保持；B5/FIX.1/B4 回归与 codegen/driver 目标；
5. 逐平台/后端/优化/别名保存命令、PID、源码/Runtime/工具/实际执行文件、符号表状态（COFF/ELF）、OS/时间与日志；
6. 更新 B6-1 状态：只有重跑通过的场景才能标记；原失败记录与 FIX.2 证据只读；macOS `unverifiable` 不得汇总为通过；
7. 证据：新建 `assets/20o-b2-b/entry-20261118-b6-1-rerun/`，日志按 SHA-256 内容寻址；B6-1/FIX.2/B5/URL/静态比较历史目录不改写；定稿后单次 docs 门禁；
8. B6-2 入口评估：B6-1 通过或形成明确受限结论后，按 B6 规划决定 B6-2 是否开工；macOS 不可用时的受限口径必须写明，不能自动写成三平台通过。

### 1.2 明确不做

- 不改产品、测试、看门狗、预算或输入；不绕过 `ArtifactVerification`；不把 Windows COFF 写成 `Complete`；
- 不改 ABI1.19、生成器22、接口1、intrinsic 1～48/ID100、registry/教程；
- 不覆盖 B6-1/FIX.2 历史证据，不用 FIX.2 证据代替 B6-1 自身签收；
- 不跑 B6-2 性能、旧721、三平台最终发布或 macOS 假装执行；
- 不把 Windows/Linux 成绩合并成“三平台通过”，不缩减场景或加 ignore 取得绿色。

## 二、开工闸门

1. FIX.2 主审计与 8 项反例通过；B6-1 原阻断证据可读；
2. 产品 `43f18274` 到当前 HEAD 的 `core/`、`stdlib/` 无产品差异；
3. Windows/Linux 新任务根、target/Runtime/标准库/工具身份可用；macOS 缺失按 `unverifiable` 记录；
4. B6-1 测试目标与精确命令可复现；原失败命令保留；
5. 历史证据只读；平台口径（COFF/ELF/macOS）已冻结；
6. 修复面只允许“重跑与证据”，产品/测试变更须退回 FIX.2/B4 或另立裁定。

## 三、分批执行

### RERUN-0：任务根与身份

1. 建立 Windows/Linux 独立任务根，记录 Rust/Cargo/LLVM/MSVC 或 clang、锁、Runtime、标准库根；
2. 复核产品树等于 `43f18274`，记录源码/工具/Runtime 摘要；
3. 复刻 B6-1 原测试命令与测试目标，保留原失败命令作为历史对照；
4. 保存环境与身份回执。

退出：两平台身份与环境可复现，产品未漂移。

### RERUN-1：Linux 原阻断场景

1. 真实官方 cmath 两别名 O0/O3，执行 `ArtifactVerification` 与黄金/009；
2. 记录 ELF 符号表状态、声明/观察组件、实际执行文件与日志；
3. 原 Linux 失败回执保留，不覆盖；任何新失败按实际记录，不放宽校验；
4. 相关 Runtime/VM/归档/资源/原子性回归同批执行。

退出：Linux 官方包装原阻断场景通过，ELF `Complete` 且声明=观察；或保留新失败并停止。

### RERUN-2：Windows 同矩阵与回归

1. Windows 同两别名 O0/O3 与黄金/009；
2. 明确记录 COFF `UnverifiedCoffExports`，只写功能通过、导出完整性未验证；
3. 相关回归与 FIX.1/B4/B5 固定历史只读复核；
4. 保存命令、PID、源码/Runtime/工具/执行文件与日志。

退出：Windows 功能矩阵通过，COFF 限定如实记录，不冒充完整验证。

### RERUN-3：状态更新与 B6-2 交接

1. 按平台汇总 pass/fail/unverifiable，更新 B6-1 自身状态；原失败与历史证据不改写；
2. 明确 B6-2 是否开工：Windows/Linux 通过而 macOS 缺失时，只能形成受限结论，开工需按 B6 规划或另裁；
3. 保存证据清单与单次最终 docs 门禁；
4. 本批不运行 B6-2 性能、不跑721。

退出：B6-1 状态按平台更新；B6-2 入口明确；B7/B2-C 仍后置。

## 四、证据矩阵

| 层 | 必测 |
| --- | --- |
| Linux | 两别名 O0/O3、ELF `Complete`、声明=观察、黄金/009、原失败保留 |
| Windows | 两别名 O0/O3、功能/黄金、COFF `UnverifiedCoffExports` 如实记录 |
| 回归 | B6-1 Runtime/VM/归档、A6 8/8、错误/原子性/除法/输入保持 |
| 身份 | 平台/工具/Runtime/执行文件/PID/源码前后摘要 |
| 状态 | 平台 pass/fail/unverifiable、B6-1 更新、B6-2 未擅自开工 |
| 边界 | 无产品/测试/ABI/registry/721 变更 |

## 五、退出条件

1. RERUN-0 两平台身份与产品漂移检查通过；
2. Linux 官方包装两别名 O0/O3 通过，ELF `Complete` 且声明=观察；
3. Windows 同矩阵功能通过，COFF 导出完整性未验证如实记录；
4. 相关 B6-1 回归通过；原失败与 FIX.2 证据未改写；
5. B6-1 状态按平台更新；macOS `unverifiable`，未写三平台通过；
6. B6-2 入口明确（通过或受限裁定），B7/B2-C 后置；
7. 无产品/测试/看门狗/预算/ABI/registry/721 变更。

## 六、风险与禁止

- 把 FIX.2 的双平台矩阵直接当成 B6-1 签收；
- 把 Windows COFF 写成 `Complete` 或把两平台合并成三平台通过；
- 覆盖 B6-1 原失败或改动历史证据；
- 为通过而改产品/测试/看门狗/预算/输入；
- 在 B6-1 重跑未完成时启动 B6-2 性能；
- 跑721、升级 registry/教程或关闭历史债务。

## 七、交接

RERUN 通过后，B6-1 按其自身证据更新状态；B6-2 是否开工按 B6 规划与 macOS `unverifiable` 口径决定。B6-2 五批性能、OS 峰值和 §2.1 全转义 URL 最大负载仍后置；B7/B2-C 未开始。

新 Agent 先读：

1. [B6-1 原阻断证据](assets/20o-b2-b/entry-20261117-b6/README.md)；
2. [FIX.2 实施证据](assets/20o-b2-b/entry-20261118-b4-component-fix/README.md)；
3. [FIX.2 反例记录](assets/20o-b2-b/entry-20261118-b4-component-review/README.md)；
4. [B6 资源性能规划](20o-b2-b6-resource-performance-plan.md)；
5. [官方库总纲](20o-official-library-program.md)。

## 八、落点

```text
core/rust/crates/xiao-driver/tests/20o_b2_b6_resources.rs      B6-1 测试目标
core/rust/crates/xiao-driver/tests/support/                    平台包装辅助
docs/DevDocs/assets/20o-b2-b/entry-20261118-b6-1-rerun/         RERUN 证据
```

本页只做 B6-1 重跑；B6-2 性能与后续批次仍后置。

## 九、实际退出记录（2026-11-18）

重跑固定产品`43f18274`，两平台新源码根各导出1,076个受保护文件；新target、新Runtime、任务stdlib和工具身份均逐命令绑定。Runtime各305/0/1，正式C ABI参考各17,243项；资源/VM/归档回归Windows42/0/3、Linux43/0/3（含Linux独立诊断），A6资源8/8，codegen109/0/15、额外driver17/0/3。官方包装两别名O0/O3、核心Cmath/共享pow、A5错误/011原子性和显式除法O0/O3通过。

Windows首轮Clippy因导出缺少协议编译夹具退出101；从固定Git对象补齐`tests/spec`原字节后，同一命令通过，失败回执不覆盖。Linux缺Clippy按实际组件清单登记；两平台fmt通过。详见[回执与边界](assets/20o-b2-b/entry-20261118-b6-1-rerun/README.md)。

`rerun_closed=true`只表示本批重跑与取证完成；B6-1仍以`restricted/stage_passed=false`交接，Windows导出完整性及macOS保留未验证。B6-2可按B6规划准备Windows/Linux受限测量，不能据此宣称三平台入口满足；本批未执行性能、远端CI或721，B7/B2-C仍后置。原B6-1阻断及FIX.2/B5/URL/静态比较历史字节不变。
