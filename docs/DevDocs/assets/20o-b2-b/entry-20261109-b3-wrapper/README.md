# B3-4 独立任务与基线修补

本任务从 `f0cb6253` 建独立工作树，按[B3-4规划](../../../20o-b2-b3-4-stdlib-wrapper-plan.md)实施源码包装；当前记录先修复既有B2测试的过时断言。

[原基线失败](baseline/b2-full.log)中，`lfloat_divide_requires_the_frozen_public_contract`仍断言34～48不存在，但B3-3已将其登记为内部active。上一批只执行两个精确入口，没有执行这条普通测试。修补保留33号全部源级参数负例，并精确核对34～48的Cmath/Float/Pure/None、49未知与100墓碑；[完整B2测试](baseline/b2-full-fixed.log)现为2 passed / 1 ignored。没有安装VM/LLVM执行器或扩大源级能力。

本任务target、Runtime、Home、临时目录全新且独立，Runtime ABI1.18、生成器20、标准库接口1。B3-3的[历史审计](baseline/b3-audit.log)与真实Runtime存档通过，旧代码和失败样本均保留。

后续包装源码与测试仍在独立分支，不自动改变主分支的模块可用性。本机源码证据不构成语言全链路或发布，cmath继续planned；旧721、22C/22D及19/11/22债务保持原归属。
