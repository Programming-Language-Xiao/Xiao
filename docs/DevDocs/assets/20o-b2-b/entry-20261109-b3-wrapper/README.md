# B3-4 独立任务与基线修补

本目录随 `b62505ae` 纳入当前工作分支，仅保存从 `f0cb6253` 建立独立工作树后的基线与既有B2测试失败/修补证据。当前主工作区尚无 `stdlib/cmath.xiao` 包装和B3-4审计脚本，B3-4产品实施尚未开始，本目录不构成[B3-4](../../../20o-b2-b3-4-stdlib-wrapper-plan.md)收口。

[原基线失败](baseline/b2-full.log)中，`lfloat_divide_requires_the_frozen_public_contract`仍断言34～48不存在，但B3-3已将其登记为内部active。上一批只执行两个精确入口，没有执行这条普通测试。修补保留33号全部源级参数负例，并精确核对34～48的Cmath/Float/Pure/None、49未知与100墓碑；[完整B2测试](baseline/b2-full-fixed.log)现为2 passed / 1 ignored。此次修补未为34～48增加VM/LLVM绑定或扩大源级能力。

这里的测试结果只证明当前快照回归。B2“只开33”的历史证据属于固定代码 `37b92c53` 的[原始测试与归档](../entry-20261108-b2/README.md)，不能用本目录的修补后测试替代。34～48内部active也不证明 `std.cmath` / `stdlib.cmath` 已可执行导入。

本任务target、Runtime、Home、临时目录全新且独立，Runtime ABI1.18、生成器20、标准库接口1。B3-3的[历史审计](baseline/b3-audit.log)与真实Runtime存档通过，旧代码和失败样本均保留。

后续包装源码与测试仍在独立分支，未计入当前工作分支交付。当前VM/LLVM仍未绑定34～48，cmath继续planned；旧721、22C/22D及19/11/22债务保持原归属。
