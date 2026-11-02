# 固定源码身份后的完整F10矩阵

工程期：20O-B1-IMP-REM。职责：替代身份修复前的旧721条最终性能证据，保存完整重跑与独立复核。
生产提交44dc9d6efa5e636ee6d83aa9db3722237aae7c43；测量、目录与源码守卫工具同df27ca75，
[Git对象逐项核对](tool-identity.log)完全一致。Runtime ABI仍1.18，生成器20。
边界：七库教程与七库registry条目继续planned；本目录不执行B1-CLOSE，不核销19/11/22债务。

维护回归[41464439400](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/41464439400)
第二次尝试成功，显式b1_f10=true；同提交工作区门禁
[41464421187](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/41464421187)成功。
首次macOS没有取得runner，未执行步骤；[原始容量取消及注释](macos-capacity.json)保留。
41464439400中只重试该macOS作业，没有重试或筛选实际执行的性能失败；前三份报告来自首次实际执行，macOS来自第二次。

| 路径 | 完整记录 | 最大单次中位数 | 最大翻倍比 | 最大驻留增量上界 |
| --- | --- | --- | --- | --- |
| VM/Linux | 103 | 2.031501秒 | 2.119525 | 327,680字节 |
| 原生Linux O0/O3 | 206 | 0.463889秒 | 2.404349 | 5,308,416字节 |
| 原生Windows O0/O3 | 206 | 0.720361秒 | 2.216421 | 5,967,872字节 |
| 原生macOS ARM64 O0/O3 | 206 | 0.408915秒 | 2.530658 | 5,423,104字节 |

四份报告共721条、20入口/31负载；标量负载记maximum，其余含最大输入和三档规模，原生分别记O0/O3。
VM≤10秒、原生≤2秒、翻倍比≤3、驻留增量上界≤64MiB；一次预热、五次采样、每批≥50ms，
有界校准到100ms，增长不超过16倍。编译、输入生成、黄金比较与终端I/O在窗口外。
内存方法是OS历史驻留峰值减当前基线的保守上界，包含算法暂存与输出；不是分配器精确峰值。
硬件/工具链适用范围以profile/compiler日志为准，不外推到未测硬件。

## 源码与产物身份

采样前用原子create_dir新建独占目录，并检查加载器的规范化stdlib根必须等于仓库stdlib。
七模块摘要在开始时固定，每项采样前后复核，报告只能使用固定摘要。完整运行成功意味着这些守卫
在全部项目执行时实际通过；本次独占目录见各profile日志的B1-PERF-ARTIFACTS行。
默认5项性能回归另覆盖旧exe目录碰撞、加载根覆盖和中途源码变化，未放宽任何产品预算。
因此本轮采用守卫后的完整重跑证明，不再以旧报告的product_sha256字段替代隔离证据。

[清单](evidence.json)保存生产提交/工具Git对象、七产品摘要、实际run/job/attempt、报告与ZIP摘要，
以及完整日志来源和摘录行号。[完整日志归档](ci-logs.zip)逐字节保存四作业日志，不依赖远端日志保留期；
摘录仅去ANSI、行尾空白并统一LF。本目录固定JSON/log为LF，避免检出时换行转换破坏摘要。
下载ZIP已对照API摘要，归档JSON与其内容逐字节一致。

## 精确点与默认回归

| 路径 | 实际产生日志的作业/尝试 | 证据 |
| --- | --- | --- |
| VM/Linux | 125342384994 / 1 | [精确点](vm-linux.log)、[profile](vm-linux-profile.log) |
| 原生Linux | 125342385470 / 1 | [精确点](native-linux.log)、[profile](native-linux-profile.log)、[编译器](native-linux-compiler.log)、[释放](native-linux-release.log) |
| 原生Windows | 125342385386 / 1 | [精确点](native-windows.log)、[profile](native-windows-profile.log)、[编译器](native-windows-compiler.log)、[释放](native-windows-release.log) |
| 原生macOS | 125360853661 / 2 | [精确点](native-macos.log)、[profile](native-macos-profile.log)、[编译器](native-macos-compiler.log)、[释放](native-macos-release.log) |

四个B1精确点各恰好1 passed / 0 failed / 0 ignored，七产品全部执行；原生各含O0/O3。
三平台各12条释放记录逐对象销毁顺序与VM匹配。既有22断言保留；历史22D失败仍按原运行登记。
同次默认执行[9项契约](vm-linux-20o_b1_imp_contract.log)、[17项资源](vm-linux-20o_b1_imp_resources.log)、
[5项性能回归](vm-linux-20o_b1_imp_vm_performance.log)，[四路径计数守卫](vm-linux-guards.log)28/28。

## 独立复核与历史边界

[audit.ps1](audit.ps1)强制四份准确平台报告、103/206唯一规模与优化键、20入口/31负载、
版本和产品摘要；独立重算五次中位数、翻倍比、每批时间与驻留增量，结果为[audit.json](audit.json)。
复跑须使用44dc9d6e的产品源码；当前工作树保持这些产品字节时可直接运行：

```powershell
pwsh -NoProfile -File docs/DevDocs/assets/20o-b1-imp/rem-identity-final-41464439400/audit.ps1
```

未来产品变化时加-RepositoryRoot指向44dc9d6e源码目录，不把未来摘要套入本批报告。
14908c64的[旧721条](../rem-final-41433194454/README.md)继续保留为身份修复前快照；
df27ca75的[首轮CSV翻倍比3.0913失败](../rem-identity-f10-41457598606/README.md)完整保留，
44dc9d6e仅复用CSV函数内不可变常量以减少重复分配，[本机定向记录](../rem-identity-csv-local-20261102/README.md)不代替本批矩阵。
41450826013功能回归全绿，以及本次全绿，均不关闭41444159791的[历史22D失败](../rem-self-review-local-20261102/README.md)。
