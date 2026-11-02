# B1-CLOSE状态升级的完整门禁

工程期：20O-B1-CLOSE。职责：保存106b99c7七库教程/registry升级提交的真实三平台门禁与身份对账。
边界：不修改产品、测试断言、ABI、生成器或预算；F10按对象身份携带原721条，不冒充新测量。

维护回归[41498029910](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/41498029910)与
工作区门禁[41498029872](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/41498029872)均成功。
维护回归最终尝试全绿；首轮仅macOS因runner容量自动取消、未执行步骤，其余五作业成功。
[首轮容量取消](macos-capacity.json)与原始注释保留。
第二次macOS在既有22C释放顺序断言失败，B1原生精确点未执行；[失败清单](macos-22c-attempt2-failure.json)、
[失败摘录](macos-22c-attempt2-failure.log)与[完整日志](macos-22c-attempt2-full.zip)独立保留。
只重试同一提交的macOS，未改测试或预算；后续绿色不关闭22C/22D或22体系债务。
实际提交、尝试、job、runner标签/编译器、源日志命令、字节数/摘要及摘录行号见[清单](evidence.json)。
[完整日志ZIP](ci-logs.zip)逐字节保存四个B1执行作业与工作区门禁，摘录去ANSI/行尾空白并固定LF。

| 路径 | 实际执行点 | 证据 |
| --- | --- | --- |
| VM/Linux | b1_capabilities_have_vm_golden_outputs | [精确输出](vm-linux-exact.log) |
| 原生Linux | b1_capabilities_match_native_golden_outputs，O0/O3 | [精确输出](native-linux-exact.log)、[释放](native-linux-release.log)、[编译器](native-linux-compiler.log) |
| 原生Windows | 同上 | [精确输出](native-windows-exact.log)、[释放](native-windows-release.log)、[编译器](native-windows-compiler.log) |
| 原生macOS | 同上 | [精确输出](native-macos-exact.log)、[释放](native-macos-release.log)、[编译器](native-macos-compiler.log) |

四点各恰好1 passed/0 failed/0 ignored，七产品示例分别经过原生O0/O3；七历史B0示例按原测试配置各执行一次，不外推为O0/O3双级证据。
默认[9项契约](vm-linux-20o_b1_imp_contract.log)、[17项资源](vm-linux-20o_b1_imp_resources.log)、
[5项性能回归](vm-linux-20o_b1_imp_vm_performance.log)及[28/28守卫](vm-linux-guards.log)通过。
契约项包括真实导出、双路径/逐形参负例、别名碰撞、逐函数移除、真实归档、普通包隔离与教程逐字一致性。
三平台六组同源释放向量各12条逐对象销毁顺序匹配，原22D断言未改。
[工作区摘要](workspace-gates-summary.log)及完整日志保存fmt、clippy、docs/usedocs/layout、覆盖率、benchmark/Bun等门禁。

七产品、Rust/夹具、工作流和守卫对象与44dc9d6e相同；测量工具与df27ca75相同。
[入口复核](../preflight-20261103/README.md)和[原721条](../../20o-b1-imp/rem-identity-final-41464439400/README.md)
证明当前产品/工具/冻结预算身份，原始报告与旧失败不改写。七册教程和registry的Git对象绑定106b99c7，
后续证据回填不改这些已经执行的页面与登记。

历史41444159791的22D失败及19/11/22债务仍独立保留，不因本次全绿关闭。
本批退出与后续范围见[收口记录](../../../20o-b1-closeout.md)。
