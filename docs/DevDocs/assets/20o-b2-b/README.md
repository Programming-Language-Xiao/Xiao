# B2-B 实施证据

本目录保存B2-B独立任务根、源码与Runtime身份。B0/B1已交付；B2的lfloat_divide/intrinsic33在`37b92c53`实现，见[单入口证据](entry-20261108-b2/README.md)。当前内部1.18/20快照不作最终发布；B3～B7与B2-C后置。

B0/B1 的开工步骤、命令、计数和退出条件见[B2-B0/B1 开工规划](../../20o-b2-b0-b1-kickoff-plan.md)；B2 单入口见[B2 `lfloat_divide`/intrinsic 33](../../20o-b2-b2-lfloat-divide-plan.md)与[实施记录](../../20o-b2-b2-lfloat-divide-progress.md)；已完成的[B3-0/B3-1](../../20o-b2-b3-0-b3-1-kickoff-plan.md)与本机证据见下；下一子批见[B3-2/B3-5 开工规划](../../20o-b2-b3-2-b3-5-kickoff-plan.md)、[B3-2A pow切换规划](../../20o-b2-b3-2a-pow-switch-plan.md)与[B3-3A/B C ABI/intrinsic规划](../../20o-b2-b3-3a-b-c-abi-intrinsics-plan.md)。

证据必须绑定实际源码、锁文件、Runtime、工具链/目标、PID 和独立输出路径；不复用 A7 的 Runtime、缓存或可执行文件，不覆盖 A0～A7 归档。

B3-2A共享Runtime pow与B3-3A/B C ABI/intrinsic已在独立任务根重新验收，见[本批证据](entry-20261109-b3-cabi/README.md)。产品89d782e1，活动最大48、ABI1.18/20；无stdlib包装，证据仅为Windows本机，B3-4及B4后置。先前包装/LLVM失败分支保留其未收口身份。
