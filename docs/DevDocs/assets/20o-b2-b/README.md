# B2-B 实施证据

当前入口为 B6-5 内部收口。B7 已具备开工条件，B6 整体验收仍受限，B2-C 待 B7 收口。[B6-5 归档与交接](entry-20261120-b6-5/README.md)

本目录保存 B2-B 的独立任务根、源码与 Runtime 身份。B0/B1 当时使用内部 1.18/20 快照，B2 的 `lfloat_divide` 与 intrinsic 33 在 `37b92c53` 接通。B6-5 使用内部 1.19/22 快照。[B2 单入口证据](entry-20261108-b2/README.md)

B0/B1 的开工步骤、命令、计数和退出条件见[B2-B0/B1 开工规划](../../20o-b2-b0-b1-kickoff-plan.md)；B2 单入口见[B2 `lfloat_divide`/intrinsic 33](../../20o-b2-b2-lfloat-divide-plan.md)与[实施记录](../../20o-b2-b2-lfloat-divide-progress.md)；已完成的[B3-0/B3-1](../../20o-b2-b3-0-b3-1-kickoff-plan.md)与本机证据见下；下一子批见[B3-2/B3-5 开工规划](../../20o-b2-b3-2-b3-5-kickoff-plan.md)、[B3-2A pow切换规划](../../20o-b2-b3-2a-pow-switch-plan.md)、[B3-3A/B C ABI/intrinsic规划](../../20o-b2-b3-3a-b-c-abi-intrinsics-plan.md)、[B3-4 stdlib包装规划](../../20o-b2-b3-4-stdlib-wrapper-plan.md)与[收口规划](../../20o-b2-b3-4-closeout-plan.md)；语言全链路见[B4规划](../../20o-b2-b4-language-integration-plan.md)、[B4-0/B4-1开工规划](../../20o-b2-b4-0-b4-1-kickoff-plan.md)、[B4-2 TAC/VM规划](../../20o-b2-b4-2-tac-vm-plan.md)、[B4-2A/B执行规划](../../20o-b2-b4-2a-b-execution-plan.md)、[B4-3 LLVM规划](../../20o-b2-b4-3-llvm-plan.md)、[B4-4双路径/归档规划](../../20o-b2-b4-4-dual-path-archive-plan.md)、[B4-5收口规划](../../20o-b2-b4-5-closeout-plan.md)、[B5版本矩阵规划](../../20o-b2-b5-version-matrix-plan.md)、[B6资源性能规划](../../20o-b2-b6-resource-performance-plan.md)、[B7收口规划](../../20o-b2-b7-closeout-plan.md)与[B2-C最终公共收口规划](../../20o-b2-c-final-closeout-plan.md)。

证据必须绑定实际源码、锁文件、Runtime、工具链/目标、PID 和独立输出路径；不复用 A7 的 Runtime、缓存或可执行文件，不覆盖 A0～A7 归档。

B3-2A 的共享 Runtime pow 与 B3-3A/B 的 C ABI 和 intrinsic 已在独立任务根重新验收。B3-3 固定产品 `89d782e1`，活动 ID 最大为 48，版本为 1.18/20。B3-4 的 planned 源码包装随后单独收口。早期包装和 LLVM 失败保留在对应批次。[B3-3 证据](entry-20261109-b3-cabi/README.md) · [B3-4 收口](entry-20261110-b3-wrapper-final/README.md)
