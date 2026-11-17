# 20O-B2-B4-3-FIX.2. 静态 Runtime 组件登记修复与 B6-1 重跑规划

> **FIX.2 已完成（2026-11-18），产品提交 `43f18274175ba09ac84e759999c40cde72eae1c5`；B6-1 自有任务根重跑待执行，B6-2 性能未开始。** 详见[FIX.2收口证据](assets/20o-b2-b/entry-20261118-b4-component-fix/README.md)。
> FIX-0复现确认：Linux真实 `std.cmath`/`stdlib.cmath` 包装 O0/O3 原先被 `ArtifactVerification` 拒绝；IR与ELF含 `xiao_runtime_array_new`/`array_push`，组件声明漏 `containers`。Windows同语料通过，但COFF状态为 `UnverifiedCoffExports`，不能证明完整符号表一致。
> 根因：既有 Runtime 组件遍历跟随 `br`，未跟随 Cmath 状态路径发出的 LLVM `switch`；FIX.2将 `br`/`switch`/`indirectbr` 可达性与共享符号映射用于静态、动态模块组件登记，保留未调用函数和未注册回调裁剪。
> 只修 codegen-llvm 的组件推导/登记与必要测试；不改 cmath 数学、资源上限、ABI1.19/生成器22/接口1、registry/教程；不跑旧721。

## 一、入口与范围

| 项 | 当前状态 |
| --- | --- |
| B6-1 | `stage_passed=false`、`status=blocked`；Windows 通过，Linux 官方包装原生被拒，macOS `unverifiable` |
| 产品基线 | `67319d61`；B6-1 测试提交 `a067bd90`（仅新增 `20o_b2_b6_resources.rs`）；ABI1.19/生成器22/接口1 |
| 现场证据 | [B6-1记录](assets/20o-b2-b/entry-20261117-b6/README.md)、`result-final-r5.json`；Linux 为 Ubuntu 26.04 WSL2、LLVM 21.1.8，Windows 为 x86_64-pc-windows-msvc |
| 相关实现 | 静态模块组件字段 `core/rust/crates/xiao-codegen-llvm/src/ir.rs:506`；动态推导 `dynamic/runtime_abi.rs::reachable_runtime_components`；符号到组件的映射 `artifact.rs::add_components_for_symbol`（已含 `xiao_runtime_array_ → containers`） |
| 本批目标 | 修正静态 Runtime 组件推导/登记，解释 Windows/Linux 差异，恢复 Linux 官方包装 O0/O3，交回 B6-1 重跑 |
| 未做 | B6-2 性能、三平台最终发布、cmath 语义、ABI/生成器变更、registry/教程、旧721 |

FIX.2 是 **codegen 元数据修复批**，不是 B6 资源/性能批。B6-1 的阻断状态在修复并重跑前保持不变；B6 不在本批内绕过校验。

### 1.1 要做

1. 在 Windows 与 Linux 用 B6-1 的同一份真实包装语料、两个别名、O0/O3 直接复现，保存实际命令、PID、源码/Runtime/工具身份、IR 文本、声明组件、ELF/PE 观察组件与完整日志；
2. 解释平台差异：为什么同一包装在 Windows 通过、在 Linux 被拒；对比两平台的组件推导来源、验证入口、优化/目标 lowering 与链接产物，不能只记录“Linux 多一个 containers”；
3. 冻结修复面：静态 `runtime_components` 必须从实际发射的 Runtime 符号推导，复用既有符号→组件映射或动态路径的 `reachable_runtime_components`；不得只对 Linux 或只对 `containers` 做硬编码补丁；
4. 覆盖组件族：`value`、`rc`、`weak`、`containers`、`tables`、`cmath` 等既有多组；保证去重、稳定顺序和与文本符号一致，并明确“声明集合”与“观察集合”的验收关系；
5. 增加回归：代表性程序（cmath 包装、数组/集合/字典/表、lfloat、字符串、归档）逐项断言发射文本中的 Runtime 符号族都有对应声明组件；同时防止无条件把所有组件都声明成“全开”来掩盖真实依赖；
6. Windows/Linux 真实 cmath 包装两别名 O0/O3 通过 `ArtifactVerification` 并逐字执行黄金；B6-1 阻断场景由 B6-1 在其自身证据目录重跑，FIX.2 只提供修复快照与定向回归；
7. 版本边界：只改组件推导/登记时生成器保持22；若必须修改发射的 LLVM 文本或 `CODEGEN_VERSION`，停下另行裁定，不擅自升版本、不改写 B5/22 历史证据；
8. 证据：新建 `assets/20o-b2-b/entry-20261118-b4-component-fix/`，日志内容寻址，保存平台/工具/Runtime/实际执行文件/PID；历史 B6-1、B5、URL、静态比较目录不改写。

### 1.2 明确不做

- 不关闭、绕过或放宽 `ArtifactVerification`；不只在 Linux 侧补 `containers`；
- 不改 cmath 数学/资源上限/错误表，不改 ABI1.19、生成器22、接口1、intrinsic 1～48/ID100；
- 不改 registry/教程，不跑旧721，不启动 B6-2 性能；
- 不把 B6-1 的历史失败改写成通过，不用独立诊断的链接成功冒充产品路径；
- 不为让测试变绿而缩减场景、改看门狗/阈值/输入或加 ignore；
- 不在 B6 批内修本问题，也不在 FIX.2 内顺手改 B6 性能/资源实现。

## 二、开工闸门

1. B6-1 阻断证据、Linux 失败回执与精确复现命令可读；
2. 当前产品为 `67319d61`，`core/`、`stdlib/` 相对基线只有 B6-1 测试文件差异；ABI1.19/生成器22/接口1；
3. Windows 与 Linux 的平台、工具链、Runtime 身份可复现；
4. 唯一任务根、独立 target/Runtime/缓存已建立；
5. 组件推导/验证代码和既有单测可读；
6. 修复面限定在 codegen-llvm 组件推导/登记与必要测试，生成器/ABI 变更未获裁定前不得实施。

## 三、分批执行

### FIX-0：复现与平台归因

1. 两个平台各跑真实包装两别名 O0/O3，保存声明组件、IR Runtime 符号、ELF/PE 观察组件和验证结果；
2. 对比 Windows 与 Linux 的组件来源和验证入口，给出直接证据解释为何一方通过；
3. 若 Windows 独立复现也失败，或 Linux 失败无法复现，停止并按实际结果重定归因；
4. 保存源码/工具/Runtime/执行文件摘要与原始日志。

退出：平台差异有直接复现证据，根因指向具体组件推导/登记路径。

### FIX-1：组件推导修复

1. 让静态模块的 `runtime_components` 从实际发射文本推导，复用既有符号→组件映射，保持去重和稳定顺序；
2. 覆盖 `value/rc/weak/containers/tables/cmath` 等既有多组，不声明未使用组件；
3. 增加映射/模块级单元测试，证明“文本符号集合 → 声明组件集合”一致；
4. 检查生成器指纹与冻结版本：若只改组件元数据则保持22；若文本变化，停下另裁。

退出：静态模块组件声明与发射符号一致，单测通过。

### FIX-2：平台回归矩阵

1. Windows/Linux 真实 cmath 包装两别名 O0/O3 通过产物校验并执行黄金；
2. 数组/集合/字典/表/lfloat/字符串/归档代表性程序通过，确认没有漏声明或全量误声明；
3. VM/归档与 B6-1 既有测试保持通过；
4. 保存逐平台、逐后端、逐优化级别的命令、PID、日志和产物身份。

退出：两平台包装矩阵通过，未以放宽校验换绿。

### FIX-3：证据与 B6-1 交接

1. 归档 FIX.2 证据，内容寻址，单次最终 docs 门禁；
2. 交回 B6-1：B6-1 用其自身任务根重跑 Linux 包装阻断场景与相关回归，FIX.2 不代替 B6-1 签收；
3. B6-1 通过后才允许进入 B6-2；本批不运行性能五批、不跑721。

退出：FIX.2 收口；B6-1 可重跑；B6-2 未开始。

## 四、证据矩阵

| 层 | 必测 |
| --- | --- |
| 复现 | Windows/Linux 两别名 O0/O3、声明/观察组件、IR 符号、ELF/PE、完整日志 |
| 归因 | 平台差异解释、组件来源、验证入口、优化/目标差异 |
| 推导 | 文本符号→组件映射、去重/顺序、未使用组件不声明 |
| 回归 | cmath 包装、容器族、lfloat、字符串、归档、VM/原生 |
| 版本 | 生成器22/ABI1.19/接口1未变；文本变化须另裁 |
| 交接 | B6-1 重跑入口、历史证据不改写、B6-2 未开始 |

## 五、退出条件

1. FIX-0 有平台差异的直接复现与归因；
2. 静态组件声明由发射符号推导，映射/模块测试通过；
3. Windows/Linux 真实 cmath 包装两别名 O0/O3 通过产物校验并执行黄金；
4. 容器/表/lfloat/字符串/归档代表性回归通过，无全量误声明；
5. ABI1.19/生成器22/接口1/registry/教程未变，旧721未跑；
6. B6-1 已交回重跑，B6-2 未开始；
7. 失败/不可验证项保留，不以放宽校验或缩减场景签收。

## 六、风险与禁止

- 只补 Linux/`containers` 硬编码，不解释 Windows 差异；
- 关闭或放宽 ArtifactVerification；
- 无条件声明全部组件掩盖依赖；
- 为通过而改生成器/ABI 或改写历史证据；
- 在 B6 批内绕过校验或把独立诊断写成产品路径通过；
- 跑721、升级 registry/教程或启动 B6-2。

## 七、交接

FIX.2 退出后由 B6-1 重跑其 Linux 包装阻断场景与相关回归；B6-1 通过后才进入 B6-2 性能。B5 固定1.19/21矩阵、21→22增量、URL优化与静态比较证据保持只读；B7/B2-C仍后置。

新 Agent 先读：

1. [B6-1阻断证据](assets/20o-b2-b/entry-20261117-b6/README.md)；
2. [B6资源性能规划](20o-b2-b6-resource-performance-plan.md)；
3. [B4-3 LLVM规划](20o-b2-b4-3-llvm-plan.md)；
4. [B4-3-FIX.1路由修复](20o-b2-b4-3-fix1-numeric-range-routing-plan.md)；
5. [官方库总纲](20o-official-library-program.md)。

## 八、落点

```text
core/rust/crates/xiao-codegen-llvm/src/ir.rs                 静态runtime_components推导
core/rust/crates/xiao-codegen-llvm/src/artifact.rs           符号→组件映射/验证测试
core/rust/crates/xiao-codegen-llvm/src/dynamic/runtime_abi.rs 既有推导复用
core/rust/crates/xiao-driver/tests/                          平台包装/组件回归
docs/DevDocs/assets/20o-b2-b/entry-20261118-b4-component-fix/ FIX.2证据
```

FIX.2 已按本页范围完成并收口；B6-1 仍需在自身任务根重跑，B6-2 与后续批次仍后置。
