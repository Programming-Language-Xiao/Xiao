# B1-IMP 三模块返工的三平台证据

工程期：20O-B1-IMP。目录职责：记录提交 `6d2643ff` 的真实 Actions 结果与精确日志摘录。
边界：math/statistics/fmt 共 12/20 入口，以及 aliases/string-index 两份辅助回归；
其余四库和 B1-CLOSE 尚未完成，七库教程及 registry 条目继续 planned。

[维护回归 41372968883](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/41372968883)
六个作业全部成功；[工作区门禁 41372968916](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/41372968916)
成功。测试代码/fixture 对应 `e7d60c17`，其后的 `6d2643ff` 只补文档和本机证据。

| 执行点 | 作业 | 精确结果 |
| --- | --- | --- |
| [默认 VM](vm-linux.log) | [125047168072](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/41372968883/job/125047168072) | 1 passed / 0 failed / 0 ignored，17.34 秒 |
| [Linux 原生](native-linux.log) | [125047167959](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/41372968883/job/125047167959) | 同上，40.34 秒 |
| [Windows 原生](native-windows.log) | [125047167960](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/41372968883/job/125047167960) | 同上，44.48 秒 |
| [macOS 原生](native-macos.log) | [125047168054](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/41372968883/job/125047168054) | 同上，31.32 秒 |

三平台每个产品 fixture 均保留位置及命名调用，O0/O3 各执行一次；数学/统计同名别名和
Unicode 读取也各执行 O0/O3。原有 B0/FIX-A/FIX-B 的 28 次普通原生执行与六次清理保留，
新增三模块六次和辅助四次，不把辅助场景计作产品入口数。精确目标核对黄金 stdout、空 stderr
和退出码 0，不用整个作业成功替代具体目标计数。

同一默认 Linux 作业另有 [9/9 契约回归](contracts-linux.log)、[2/2 默认性能测试](performance-default-linux.log)
和 [28/28 计数守卫](guards-linux.log)。性能测试未加 ignore，VM 10 秒和翻倍比≤3 的预算断言保留。
这里没有采集 Linux 的完整五次耗时样本；本机 debug/release 计时另见
[本机证据](../review-local-20261102/README.md)，也不代替全部 F10。

[机器清单](evidence.json) 记录 run/job、完整源日志的字节数/摘要、摘录来源行段和入库摘录摘要。
日志从 GitHub job logs API 下载，入库截取明确行段、移除 ANSI/行尾空白并统一 LF；摘录不伪装成完整源日志。
精确块按目标名、五份 IMP 样例及唯一汇总共同定位，包含 --exact 的调用依据，B1 恰好 1 passed。

新增 Unicode 样例的销毁差异仍列在 [返工阻塞](../../../20o-b1-imp-blockers.md)，本轮没有执行
该新增计数诊断的最终闭环；既有释放门禁全绿不能证明该差异解除。未修改 19/11/22 发布债务。
