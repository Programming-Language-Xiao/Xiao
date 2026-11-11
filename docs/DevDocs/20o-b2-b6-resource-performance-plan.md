# 20O-B2-B6. cmath/lfloat 资源、错误与三平台性能规划

> **B5 尚未完成；B6 仍未开始，B7 与 B2-C 后置。**
> 入口为 B5 内部 ABI 1.19/生成器 21 收口后的固定源码；B4 的 cmath 语言链路已内部可用。
> B6 只做资源、错误、原子性、三平台性能与 CI 证据；不新增数学语义、不改 ABI/registry/教程。
> 不跑旧 721；完整 721、最终教程/registry 和公共发布留 B2-C。

## 一、入口与范围

| 项 | 当前状态 |
| --- | --- |
| B4 | cmath 内部语言链路、30 条导入、双路径/归档（待完成） |
| B5 | ABI 1.19/生成器 21、旧/新拒绝、缓存（待完成） |
| 版本 | 目标内部 1.19/21、接口 1、intrinsic 1～48 |
| 本批目标 | cmath/lfloat 资源/错误/原子性、VM/原生三平台性能与 CI 证据 |
| 未做 | B7 收口、B2-C 721/教程/registry/最终公共发布 |
| 公共状态 | `stdlib.cmath`/UseDocs/registry 仍 planned；旧 721 未跑 |

B6 是 **内部资源/性能验收批**，不是 B2-C 最终发布。B6 的测量只绑定本批源码/产物；B2-C 才在新快照重跑 721 并做最终三平台公共收口。

### 1.1 要做

1. 复核 cmath/lfloat 的 002/009/011/012/013/033/034 边界和错误顺序；
2. 复核失败无部分结果、输入不变、finally/release、C ABI out 保留；
3. 复核 cmath Pure 不删除、float `**`/cmath.pow 共核、lfloat_divide 33 的资源；
4. 按 F-3 §6.2 测量 VM release、原生 O0/O3、五批独立进程、预热/校准、中位数、OS 峰值和翻倍比；
5. 覆盖 15 项 cmath、float `**`、lfloat_divide 的边界输入；
6. 补齐 Windows/Linux/macOS 实际执行或明确不可验证；
7. 保存源码、锁、Runtime、工具链、目标、PID、日志、产物摘要；
8. 形成 B7 收口入口。

### 1.2 明确不做

- 不新增 intrinsic 49+/C 符号，不改 1～48/ID 100；
- 不改 ABI 1.19/生成器 21、不重开 B5 版本矩阵；
- 不实现新数学/精度/舍入规则；
- 不升级 registry/教程；
- 不跑旧 721，不关闭 22C/22D、19/11/22；
- 不把 B6 内部性能写成任意硬件保证或 B2-C 最终成绩。

## 二、开工闸门

1. B5 审计通过，1.19/21/接口1、旧/新拒绝和缓存证据固定；
2. B4-5 内部 `language_available=true`，30 条导入、双路径/归档可复现；
3. F-3 资源/性能协议和 F-12 错误表无未登记变化；
4. 15 项 cmath/float pow/lfloat_divide 的 VM/LLVM/归档路径可复现；
5. B1 20/17/双精确/28 守卫和 B4 回归可复现；
6. 三平台目标/工具链可用或明确不可验证；
7. 唯一任务根、独立 target/Runtime/缓存已建立。

## 三、分批执行

### B6-1：资源、错误与原子性

1. 15 项 cmath：NaN/Inf、域 009、非有限结果、有符号零/子正规；
2. float `**`/cmath.pow 共核：特殊值、错误身份、结果位模式；
3. lfloat_divide 33：precision 1/32,768、0/超界、除零、短结果、scale/系数预算；
4. 002/009/011/012/013/033/034 按 F-12 顺序；
5. 失败无部分结果、输入不变、C ABI out 保留、挂起槽隔离；
6. finally/release/临时值清理；
7. 保存资源/错误/原子性证据。

退出：主要路径错误/资源/原子性证据完整；无未登记行为变化。

### B6-2：VM/原生性能测量

1. VM release、原生 O0/O3 分记；
2. 五批独立进程，每批预热一次；
3. 单次≥50ms则repeat=1；否则批次至少50ms、100ms停止增长阈值、每轮至多16倍、总上限2^24；
4. 时间先归一后取五批中位数；
5. OS驻留增量取保守峰值最大值；Windows/Linux/macOS各自单位换算；
6. 线性组≤3、超线性≤5；同路径/同优化/同输入族比较n与2n；
7. 覆盖 15 项 cmath、float `**`、lfloat_divide、双路径导入后的调用；
8. 不可验证项明确记录，不放宽预算。

退出：VM/原生性能有受控样本或不可验证记录；无预算掩盖。

### B6-3：错误/资源边界负载

1. 最大合法输入、上限+1、强消去、2/5因子、非终止、scale差；
2. cmath pow/超越、min/max、-0、子正规、下溢；
3. lfloat_divide precision 边界、work/result/scale；
4. C ABI 状态 1/2/3/6/8 与 out/挂起槽；
5. B1 20入口/资源17/双精确/28守卫、B4 cmath 回归；
6. 不跑旧721；旧1.19/21证据只作历史。

退出：边界负载/错误身份稳定；没有用松上界或预算通过掩盖超限。

### B6-4：三平台与 CI 证据

1. Windows/Linux/macOS 实际执行；缺平台明确不可验证；
2. 保存 CI run/job/attempt、源码、Runtime、工具链、目标、日志；
3. push 路径过滤未触发按事实写明；
4. 性能/资源结果按平台/优化级别分别记录；
5. 不把本机结果写成三平台通过；
6. 不修改工作流阈值/默认超时来补绿。

退出：三平台证据完整或不可验证项明确；CI 身份可核对。

### B6-5：收口与 B7 交接

1. 汇总资源/错误/原子性/性能/CI；
2. 分平台记录执行/未执行/失败样本；
3. 复核无 cmath 语义、ABI/registry/教程变化；
4. 复核旧721未跑、历史债务未核销；
5. 形成 B7 入口：整批收口、教程/registry 状态与 B2-C 交接。

退出：B6 内部证据完整；B7 未开始；B2-C 未开始。

## 四、证据矩阵

| 层 | 必测 |
| --- | --- |
| 错误 | 002/009/011/012/013/033/034、F-12 顺序 |
| 原子性 | 失败无部分结果、输入不变、out/挂起槽、finally/release |
| 资源 | D/W/H、precision、系数/scale/指数、上限+1、强消去 |
| 时间 | VM release、原生O0/O3、五批、预热/校准、中位数 |
| 内存 | OS峰值≤64MiB、三平台单位、不可验证标记 |
| 翻倍比 | 线性≤3、超线性≤5、同路径/优化/输入族 |
| 平台/CI | Windows/Linux/macOS、run/job/attempt、路径过滤 |
| 边界 | 无ABI/registry/教程/721/历史债务变化 |

## 五、退出条件

1. cmath/lfloat 主要错误/资源/原子性路径有证据；
2. VM/原生 O0/O3 按 F-3 执行或明确不可验证；
3. OS峰值/翻倍比/五批中位数/校准记录完整；
4. 三平台实执行或不可验证项明确，CI 身份可核对；
5. 没有用预算/阈值修改掩盖超限；
6. 无 cmath 语义、ABI/registry/教程变化；
7. 旧721未跑、历史债务未核销；
8. B7 未开始；B2-C 未开始。

## 六、风险与禁止

- 把CI绿色或旧报告当性能证据；
- 不做预热/校准或只取单次；
- 用五批中位数掩盖OS峰值超限；
- 复用旧可执行文件/Runtime/缓存；
- 为通过预算/时限改语义、删场景或放宽阈值；
- 把B1旧721或B5版本证据写成B6成绩；
- 升级registry/教程或关闭历史债务。

## 七、交接

B6完成后进入B7：整批收口、教程/registry边界和B2-C交接。B2-C才在最终1.19/21快照重跑721、三平台公共产物与教程/registry。

新 Agent 先读：

1. [B5版本矩阵](20o-b2-b5-version-matrix-plan.md)；
2. [B4-5收口](20o-b2-b4-5-closeout-plan.md)；
3. [F-3资源/性能](20o-b2-freeze-c-resource-dependency-plan.md)；
4. [F-12错误表](20o-b2-freeze-b3-error-table-plan.md)；
5. [A6资源/性能历史](20o-b2-a6-resource-performance-plan.md)。

## 八、落点

```text
core/rust/crates/xiao-runtime/                        资源/错误/C ABI
core/rust/crates/xiao-vm/                             VM性能
core/rust/crates/xiao-codegen-llvm/                   原生O0/O3
tests/benchmarks/                                     受控性能
docs/DevDocs/assets/20o-b2-b/entry-<date>-b6/         B6证据
```

本页只做B6；B7与B2-C仍后置。
