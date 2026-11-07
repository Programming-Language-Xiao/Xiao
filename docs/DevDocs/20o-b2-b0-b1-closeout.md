# 20O-B2-B0/B1. B0基线收口记录

B0只读基线已完成，B1私有内核已在代码提交35872782完成。B0实际被测HEAD为d08b8460f1941fb82f4ce36b38359fc2ba448f33；规划页旧e596cea8仅是一版DevDocs规划提交，已在证据中单列。产品基线仍为A6的e7bf7a49ba3ed55679b69c97ac1bd5ce7f5777b6。

独立任务根固定全新target、Runtime、Xiao缓存、临时目录及仓库stdlib。B1契约9、资源17、B2-A资源3+8、语言12/1 ignored、Driver64/1 ignored、B1/A1 VM/原生精确入口、workspace/benchmark/clippy/fmt、docs/usedocs/layout/覆盖率和原28/28守卫全部通过（文档变更后的最终门禁记录为b0-docs-final）；精确入口均使用--exact且恰好1 passed/0 failed/0 ignored。

ABI1.18、生成器20、活动intrinsic最大32、33～48未激活、ID100墓碑、stdlib.cmath planned、B1七库verified。B0 的 Cargo.lock 和产品树相对 A7 未变；B1 在 `35872782` 的七个允许路径中补齐 libm 依赖及私有内核。b1_f10=0、b2_a_perf=0，旧721条F10未运行，历史债务保持独立。Runtime255项、Clippy与fmt通过；独立证据见[B1目录](assets/20o-b2-b/entry-20261108-b1/README.md)。

原始证据保存命令、PID、日志、前后源码摘要、守卫报告和closeout；B0审计脚本继续固定历史基线，B1审计脚本固定代码提交35872782与七个允许路径。下一入口为B2-B公开接线；公开入口、C ABI、ABI1.19/21和cmath仍后置。

B1 当前的常量 ULP 向量仅为私有底座初检，不构成 F-9 正式误差证据。B3 仍须提供 MPFR 4.2.2/GMP 6.3.0 离线生成的独立向量、生成器版本和摘要。
