# 2026-11-02 B1-IMP 返工证据

工程期：20O-B1-IMP。目录职责：保存修复后的本机成功日志和额外释放失败轨迹。
边界：仅 Windows、本机、三模块 12/20 草稿；不代表七库、三平台或 B1-CLOSE 验收完成。

| 记录 | 实际范围 |
| --- | --- |
| [原生精确日志](native-windows.log) | math/statistics/fmt 完整位置及命名调用、数学/统计别名、Unicode 下标均过 O0/O3；原有 B0/FIX-A/FIX-B 与清理例保留；恰好 1 passed，97.19 秒 |
| [VM 精确日志](vm-exact.log) | 同源产品及前置完整黄金，恰好 1 passed，35.74 秒 |
| [完整 workspace](workspace-default.log) | 1,555 passed / 0 failed / 49 环境门控 ignored；包含三模块 9 项契约、双路径逐形参负例和两个默认性能测试 |
| [debug 计时](vm-debug-performance.log) | 保留原 10 秒预算；最大输入扫描和左右 262,144 字节输出，ASCII 与 Unicode 填充翻倍比通过；预热一次、五次中位数 |
| [release 计时](vm-release-performance.log) | 最大扫描 0.9500146 秒，左右最大填充 0.2810241 / 0.300354 秒；与 debug 分记 |
| [守卫日志](guards.log) / [机器报告](guard-report.json) | 仓库脚本实际复跑四路径的 28/28 正反例；只证明精确计数脚本，不代表三平台产品执行 |
| [Bun 测试](bun-tests.log) | 301 pass / 5 skip / 0 fail |
| [20M 移除回归](20m-removal.log) | 缺少真实函数体时精确核对模块、函数与 span，没有缩弱移除断言 |
| [严格 Clippy](clippy.log) / [基准工作区检查](benchmark-check.log) | workspace all-targets -D warnings 和独立 benchmark check 通过，锁文件未改 |

代码前置为 `8bd9205b`、`5a8ab776`、`35becfb1`，最终测试提交为 `e7d60c17`。
docs/usedocs/layout、rustfmt 和覆盖率门禁均通过，公共 API 文档覆盖 5,782/5,782。
测试/fixture 的真实内容由
[机器清单](evidence.json) 的逐文件摘要标识；日志登记源字节数/摘要及入库规范化后的字节数/摘要。
debug 计时采于索引修复后、最后的类型导入补充前，fmt 产品与性能源码未改；最终 workspace、
原生与 release 日志使用机器清单中的最终源码。这里不混写原始日志摘要与摘录摘要。

最大输出原文为 16,384 个 🚀，fill 为 😀，width=65,536；左右独立黄金均为 262,144 字节。
Runtime 对象峰值最多 1,901,228 字节，包含字符串索引和拥有缓冲；不包含全部 VM 分配或进程
RSS，不能据此声称完整 F10 的峰值增量内存已通过。原生功能 O0/O3 也不替代 F10 原生计时。

另保留两份**失败诊断**：[完整越界样例](unicode-with-bounds.trace.tsv) 与
[只含成功读取](unicode-success.trace.tsv)。本机扩展 20C 诊断时分别为 native 14 / VM 16、
native 12 / VM 13；完整样例的文本与末次循环字符、成功样例的末次循环字符未出现 destroy。
这是生成器 19 之前的诊断轨迹，不是最终版本通过的生命周期证据；最终 B1 语义目标不统计
销毁数，不能代替该验收。原有 20C 释放场景与断言未修改，19/11/22 发布债务未处理。
完整触发源码保存在 tests/fixtures/20o-b1/imp/string-index/product.xiao，清理差异继续列为未收口。

复跑命令与当前剩余项见 [返工记录](../../../20o-b1-imp-blockers.md)。三平台证据取得前七库教程
与登记项继续 planned；其余四库、八个入口和编码别名碰撞也不能由本目录代替。
