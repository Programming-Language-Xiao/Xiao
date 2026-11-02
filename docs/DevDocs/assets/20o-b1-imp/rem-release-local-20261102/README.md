# REM-C 释放差异的失败证据

工程期：20O-B1-IMP-REM-C。职责：保存可复跑的实际失败、完整源及 VM/native O0/O3 trace。
边界：这是退出码 1 的诊断，不能登记为生命周期验收通过；生成器仍为 19、ABI 仍为 1.18。

入口为 core/rust/crates/xiao-driver/examples/b1_release_diagnostic.rs；原生工具链按 10D 准备。
[完整运行日志](diagnostic.log)、[机器清单](evidence.json) 和 [结果](results.json) 记录成功段
12/13、含越界段 14/16 的 native/VM 销毁数；O0/O3 一致。

两个完整源为 [success](success.xiao) / [with-bounds](with-bounds.xiao)，各有 VM 与两级原生
trace。success-ownership.json 只摘录已有作用域/释放计划，success-loop-end.ll.txt 摘录实际
LLVM 的缺口，完整 IR/LLVM 摘要和来源行号在机器清单。所有 trace 完整保存，不按计数裁剪。

归因与待裁定最小修复见 [独立前置草案](../../../20o-b1-rem-release-prerequisite.md)。
