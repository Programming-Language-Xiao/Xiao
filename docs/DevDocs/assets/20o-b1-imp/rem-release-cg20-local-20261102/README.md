# REM-C 生成器20清理前置的本机证据

工程期：20O-B1-IMP-REM-C。职责：保存星崽授权的独立修复后同源VM/原生O0/O3完整trace。
边界：ABI仍为1.18，不改IR释放计划或22D规则/断言；这是本机证据，不替代三平台验收。

[诊断日志](diagnostic.log)、[全部结果](results.json)与[机器清单](evidence.json)记录六组向量、
12条原生记录；逐字stdout、退出码、空stderr和逐对象销毁顺序全部匹配。完整源和三种后端
trace均保存于本目录；源自 tests/fixtures/20o-b1/imp/release-edges/ 的固定清单。

原始失败继续保留在 [生成器19基线](../rem-release-local-20261102/README.md)，不覆盖失败样本。
success由12/13对齐13/13，with-bounds由14/16对齐16/16；嵌套跳转7/7、临时return/空循环3/3、
索引错误6/6、循环体错误3/3，O0/O3均一致。完整strong_release事件不要求VM/native逐条相同，
逐对象destroy顺序实际相同；不把计数相等冒充完整原始事件完全相等。

清单区分原始字节摘要与统一LF后的仓库副本摘要，记录生成器源码摘要，结果可由显式诊断复跑。
归因和授权边界见 [独立前置记录](../../../20o-b1-rem-release-prerequisite.md)。
