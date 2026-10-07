# 10Q-I6：函数清理链重构后续任务

来源：[10Q 释放审计](10q-selector-case-strength-and-release-audit.md)。状态：10R 已实现并在 Windows 受控差分验证；实现、改后数据及边界用例见 [10R](10r-release-accounting-and-native-ci-gate.md)。下文保留修复前问题与验收输入。

最小复现为 `d19a_differential.rs` 中的 `nested-finally-drops`，不得删除或弱化比较。
现状 VM 的销毁对象顺序为 3→2→1，原生为 1→2，外层 finally 未执行。
仅将函数 return 改走 `emit_nonlocal_exit_from_depth` 并补 `collect_value_slots`
后，原生变成 2→3→1，仍不符合冻结顺序；该实验已撤回。

实现需统一 `dynamic/control.rs` 的函数返回与嵌套 finally 调度，处理 pending return、
finally 覆盖返回或抛错、作用域释放计划以及 `llvm.stackrestore` 的有效期；
`dynamic.rs` 的函数出口不可再依赖按名称扫描槽位来替代计划顺序。
禁止修改 VM 语义，禁止新增豁免字段。所有权值到槽位的映射需按函数作用域隔离，
不能将整个程序中的同名值直接映射为同一个槽。

验收需覆盖嵌套 finally 返回、返回堆值、finally 覆盖返回、finally 抛错、普通无值返回，
对比输出、错误、终止方式和完整释放序列。原用例严格通过后摘除 I6 的 Drops 登记。
这项重构不包括动态表方法 ABI、静态溢出或 CI 策略。
