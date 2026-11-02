# REM 七库功能的三平台基线

工程期：20O-B1-IMP-REM。职责：登记 f7a831e6 的完整七库功能与参数回归。
边界：本轮生成器仍为19，完整F10和后续生成器20释放前置不在这次结论内；七库仍planned。

[维护回归41411981994](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/41411981994)
六作业成功，[工作区门禁41411982034](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/41411982034)
成功。七模块20入口的完整产品、位置/命名、错误与边界、两组别名、辅助字符串用例全部执行；
默认VM与三平台原生都恰好1 passed / 0 failed / 0 ignored，原生各有O0/O3实跑。

| 执行点 | 作业与摘录 |
| --- | --- |
| 默认VM | [125166857817](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/41411981994/job/125166857817) / [日志](vm-linux.log) |
| Linux原生 | [125166857751](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/41411981994/job/125166857751) / [日志](native-linux.log) |
| Windows原生 | [125166857815](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/41411981994/job/125166857815) / [日志](native-windows.log) |
| macOS原生 | [125166857408](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/41411981994/job/125166857408) / [日志](native-macos.log) |

[清单](evidence.json)保存所有作业结论、完整源日志字节数/摘要、摘录行段与统一LF、移除ANSI/行尾空白后的摘要。
摘录逐模块检查VM恰好一次、原生恰好两个优化级别；完整原始日志可按清单中的只读命令下载。
