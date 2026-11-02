# 生成器20的三平台功能与释放证据

工程期：20O-B1-IMP-REM-C。职责：登记c5826020的产品与独立清理前置实跑。
边界：41415412610整体失败于macOS测量校准，不能写为完整维护回归通过；本目录仅登记已经实际通过的功能/释放执行点。

默认VM和三平台原生的B1精确目标均恰好1 passed / 0 failed / 0 ignored，七产品、两组别名、
历史探测和FIX-B清理均执行；原生各有O0/O3。[清单](evidence.json)记录完整源日志摘要和摘录行段。
工作区门禁 [41415405901](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/41415405901)成功。

| 执行点 | 作业 | 精确日志 | 释放日志 |
| --- | --- | --- | --- |
| 默认VM/Linux | [125178103729](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/41415412610/job/125178103729) | [VM](vm-linux.log) | 默认六向量语义黄金随workspace执行 |
| 原生Linux | [125178103821](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/41415412610/job/125178103821) | [原生](native-linux.log) | [12条](native-linux-release.log) |
| 原生Windows | [125178103651](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/41415412610/job/125178103651) | [原生](native-windows.log) | [12条](native-windows-release.log) |
| 原生macOS | [125178103866](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/41415412610/job/125178103866) | [原生](native-macos.log) | [12条](native-macos-release.log) |

既有20C差分入口在每个平台实际执行六份同源向量的O0/O3，逐对象销毁顺序均匹配VM；
没有放宽旧20C或22D断言，没有新增ignored目标。原22D三目标与22E也在同一源码下通过。
功能、释放与F10按实际执行分别记证，不因后续测量工具中止而抹掉已有记录，也不把已有记录外推为全批通过。
F10完整报告与macOS原始失败见 [相邻目录](../rem-f10-41415412610/README.md)，最终重跑另行登记。
