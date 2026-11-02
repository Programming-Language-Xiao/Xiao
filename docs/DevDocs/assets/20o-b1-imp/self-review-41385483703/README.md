# B1 本轮自审后的三平台验证

工程期：20O-B1-IMP。职责：保存自审修复提交 `e811e414` 的真实 CI 结果和精确日志摘录。
边界：math/statistics/fmt 三模块、12/20 入口；其余四库、完整 F10 和生命周期差异仍待收口。

[维护回归 41385483703](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/41385483703)
六作业全部成功，[工作区门禁 41385483801](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/41385483801)
通过。实际代码/测试提交为 `0b37e8ed` / `a9c8ca97`；`e811e414` 只补文档和本机记录。

| 执行点 | 作业 | 唯一目标结果 |
| --- | --- | --- |
| [默认 VM](vm-linux.log) | [125085266081](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/41385483703/job/125085266081) | 1 passed / 0 failed / 0 ignored，18.25 秒 |
| [Linux 原生](native-linux.log) | [125085266506](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/41385483703/job/125085266506) | 同上，51.05 秒 |
| [Windows 原生](native-windows.log) | [125085266281](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/41385483703/job/125085266281) | 同上，54.59 秒 |
| [macOS 原生](native-macos.log) | [125085266361](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/41385483703/job/125085266361) | 同上，36.77 秒 |

三个产品与 aliases/string-index 两份辅助样例均实跑，原生各执行 O0/O3。
fmt 完整产品样例包含新增的 MIN 全部 2～36 进制 35 个命名调用向量；原有位置、命名、
错误、资源用例保留。辅助回归和进制向量不计作新的产品模块或公开入口。

同一 Linux 默认作业的 [九项契约回归](contracts-linux.log) 覆盖仅删目标模块函数和共用清单
完整性；[两项默认性能测试](performance-linux.log) 和 [28/28 守卫](guards-linux.log) 通过。
没有增加 ignored 目标或放宽性能预算，功能回归不代替完整 F10 或释放验证。

[机器清单](evidence.json) 记录全部作业状态、完整源日志的字节数/摘要、摘录行段和规范化摘要。
摘录从已完成 job logs API 下载，按目标名、五份 IMP 案例和唯一通过汇总定位，保留 --exact
的调用依据；移除 ANSI/行尾空白并统一 LF，不将摘录摘要混作完整源摘要。
修复细节与剩余范围见 [自审记录](../../../20o-b1-self-review.md)。
