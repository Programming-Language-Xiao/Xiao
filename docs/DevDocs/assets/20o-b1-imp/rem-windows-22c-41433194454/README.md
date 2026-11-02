# Windows既有22C释放顺序阻塞

工程期：20O-B1-IMP-REM-C。职责：保留14908c64首轮Windows作业的真实失败，避免只登记绿色结果。
边界：22C源码及断言未修改，不接管19/11/22债务；F10尚未执行，不计为通过。

维护回归41433194454的Windows作业125237347099在
queued_task_cancel_never_runs_body_and_wait_is_027失败，22c_vm_concurrency.rs:567期望
cleanup_order=[36,34]，实际[34,36]。B1原生精确目标此前已完成，后续F10因这项断言中止而未运行。

[失败摘录](failure.log)与[源日志清单](evidence.json)保留原始断言；与开工基线90ea5a5a相比，
该测试文件无修改。整次运行结束后只重试现有作业，重试结果另记，不能删除断言或把未执行写成绿色。
