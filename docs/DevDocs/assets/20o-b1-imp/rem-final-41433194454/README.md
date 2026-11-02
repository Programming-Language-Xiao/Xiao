# REM最终功能、释放与F10矩阵

> 历史快照：本目录来自14908c64，早于df27ca75的目录/源码身份守卫。文件名保留原记录，
> 旧721条不作为修复后最终F10证据；摘要字段一致不能单独证明旧运行没有复用程序或覆盖加载根。
> 已采用df27ca75身份工具完整重跑；首轮41457598606失败，优化后另跑41464439400，见 [当前对账](../../../20o-b1-rem-evidence.md)。原始样本与摘要不改写。

工程期：20O-B1-IMP-REM-C。职责：保存14908c64的四份完整报告、三平台精确点与独立重算。
状态：维护回归41433194454第二次尝试成功，同提交工作区门禁41433190291成功。
边界：这是REM退出证据；七库教程/registry保持planned，B1-CLOSE独立后续，19/11/22债务不核销。

| 路径/CPU | 完整记录 | 最大中位数 | 最大翻倍比 | 最大驻留增量上界 |
| --- | --- | --- | --- | --- |
| VM/Linux，AMD EPYC 9V74 | 103 | 2.188167秒 | 2.596067 | 397,312字节 |
| 原生Linux O0/O3，AMD EPYC 9V45 | 206 | 0.270997秒 | 2.265885 | 5,304,320字节 |
| 原生Windows O0/O3，Xeon Platinum 8370C | 206 | 0.648620秒 | 2.106825 | 5,943,296字节 |
| 原生macOS ARM64 O0/O3，Apple M1 Virtual | 206 | 0.334972秒 | 2.563997 | 5,423,104字节 |

721条记录均覆盖相应20入口/31负载，保持VM≤10秒、原生≤2秒、翻倍比≤3、内存≤64MiB。
一次预热、五次采样逐批≥50ms；编译、输入生成、黄金比较与终端I/O在窗口外。
内存为OS历史驻留峰值减当前基线的保守上界，包含算法暂存与输出，不是分配器精确峰值。
原始JSON保留全部校准、样本、实际规模与产品摘要；代码生成器20、ABI65554（1.18）、Rust release。

Rust为1.96.0（LLVM22.1.2）；原生编译器分别为Ubuntu clang18.1.3、MSYS2 clang23.1.4、
Homebrew clang23.1.4。四份profile.log保存CPU/工具链/构建段，清单记录源行与摘要。

## 精确点与释放

| 路径 | 实际执行日志所属作业 | 证据 |
| --- | --- | --- |
| 默认VM | [125237346998](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/41433194454/job/125237346998) | [唯一精确通过](vm-linux.log) |
| Linux原生 | [125237347048](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/41433194454/job/125237347048) | [唯一精确通过](native-linux.log) / [12条释放](native-linux-release.log) |
| macOS原生 | [125237347577](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/41433194454/job/125237347577) | [唯一精确通过](native-macos.log) / [12条释放](native-macos-release.log) |
| Windows重试原生 | [125250341833](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/41433194454/job/125250341833) | [唯一精确通过](native-windows.log) / [12条释放](native-windows-release.log) |

四个B1目标均恰好1 passed / 0 failed / 0 ignored；七产品及历史/辅助用例全部执行，原生各含O0/O3。
三平台既有20C目标均执行六组同源向量，逐对象销毁顺序与VM一致；原22D/22E断言保留并通过。
首次Windows被未修改的22C释放顺序断言挡住，重试成功；[原失败](../rem-windows-22c-41433194454/README.md)保留。
只重试现有作业，未删除测试或吞掉失败；清单区分实际源日志作业与第二次尝试的聚合作业身份。

## 独立核对

[audit.ps1](audit.ps1)要求四份完整报告与准确平台矩阵，检查103/206条唯一规模/优化键、
20个公开入口、版本、五批采样、100ms有界校准和产品摘要，独立重算中位数/翻倍比与内存上界。
原提交的核对结果为 [audit.json](audit.json)。复跑历史报告时，须将 `RepositoryRoot` 指向已检出
14908c64的源码目录；当前CSV已优化，不能用当前产品摘要替代历史源码：

```powershell
pwsh -NoProfile -File docs/DevDocs/assets/20o-b1-imp/rem-final-41433194454/audit.ps1 -RepositoryRoot D:/path/to/14908c64-checkout
```

历史重放须对照生产源码14908c64，避免用未来产品修改覆盖原报告的摘要。源码与zip/report原始摘要、
完整源日志命令、摘录行段和规范化摘要均在 [清单](evidence.json)。较早的校准中止、3.1008超限与本机探索结果分别保留，不混作这次通过记录。
