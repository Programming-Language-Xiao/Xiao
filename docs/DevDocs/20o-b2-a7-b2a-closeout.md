# 20O-B2-A7. B2-A内部收口记录

> 本批在ABI1.18、生成器20、标准库接口1范围完成文档与B1功能回归收口。验证快照`5749d6e0f277ba9aebf2bf8468e0c79a082e42a6`，产品继续固定A6的`e7bf7a49ba3ed55679b69c97ac1bd5ce7f5777b6`。B2-B/C未开始，cmath/lfloat_divide未交付，最终教程/registry与721条F10留B2-C。

按[A7规划](20o-b2-a7-b1-regression-b2a-closeout-plan.md)和A-P1～A-P3执行。原始命令、源码/Runtime/工具/目标身份、实际CI作业与复跑入口见[证据目录](assets/20o-b2-a7/entry-20261107/README.md)。本页结论仅覆盖B2-A内部验证，不是B2最终公共发布。

## 当前回归

本机在唯一工作树、全新target/Runtime/缓存/临时目录构建，stdlib固定为本仓库。每条命令保存argv、cwd、PID、时间、退出码、原始日志和前后源码清单，Cargo固定1.96.0与--locked，清理RUST_BIGDECIMAL_*。

| 执行点 | 结果与范围 |
| --- | --- |
| B1契约 | 9项，七库20入口；公开std/stdlib双路径、形参/未导入拒绝、导出/逐函数移除、真实.xar、普通包隔离、同名函数碰撞与教程一致性 |
| B1资源 | 17项，沿原输入/输出限额与错误顺序 |
| B1 VM/原生精确入口 | 各--exact、恰好1 passed/0 failed/0 ignored；保留全部B0/FIX/IMP/A1/A5场景；原生含O0/O3 |
| A1完整示例 | 两后端独立精确入口，身份、接纳错误和finally保留主错误三份完整源码逐字核对 |
| B2-A资源/语言 | 默认资源8项、校准协议3项、语言12项；完整六类型转换/承载、错误与原子性 |
| 包与工程门禁 | Runtime/types/IR/bytecode/VM/LLVM/diagnostics、driver单元、workspace/benchmark锁检查、Clippy、完整Rust格式、docs/UseDocs/layout/覆盖率与原28/28守卫 |

VM与原生精确日志保存了每个既有标记，不能仅用绿色汇总代替20入口执行。A5完整语料仍通过既有精确入口在三平台执行。A7没有启用b1_f10或b2_a_perf；没有重新测量A6，更没有运行旧721条。

## A0～A6对账

| 阶段 | 有效源码快照 | 结论与适用范围 |
| --- | --- | --- |
| A0 | d0e4a5fb | Windows基线；result.json仍为失败，经过工作树内相同TypeScript实体复制后，仅closeout.json作为有效收口；Linux/macOS本地缺口保留 |
| A1 | 39b2c1ea | 六包精确依赖、文本适配/身份、033/034与三平台完整示例；Rust映射不冒充语言数值字典键 |
| A2/A3 | 5ef17bb2 | 精确算术内核与资源/错误顺序；当时完整语言接线仍后置 |
| A4 | 1838fae1 | 显式转换内核与边界；不把内核测试冒充后来的语言测试 |
| A5 | f21715d3 | 类型/IR/TAC/VM/LLVM和序列化/并发全链路、两后端/三平台完整语料 |
| A6 | e7bf7a49 | 三平台release VM/原生O0/O3资源/性能；二进制gcd和线性文本路径缺陷已修复，历次失败保留 |

A0、A1、A2/A3、A4、A5、A6及A6第二轮审计共7个入口均重新只读验证通过，历史文件/源码/作业和性能原件可对账。A6六份报告继续按原始样本重算，成绩仍绑定其工具和硬件；这些复核不是当前测量，也不是B2-C最终成绩。

[阶段与边界清单](assets/20o-b2-a7/entry-20261107/stage-and-boundary.json)逐路径登记从A0到A6的128项合法改动，包括依赖/锁、诊断、共享数值接纳/身份、算术与转换、语言管线、测试夹具和既有CI接线。不能把整个B2-A写成“产品源码无差异”。A7自身仅改变UseDocs与fixture说明5个输入文档路径，加上DevDocs/证据，不改产品、测试、工作流、锁或registry。

## 语言说明与状态

[算术](../UseDocs/language/basics/p1-expressions/operators.md)、[转换](../UseDocs/language/basics/types/conversions.md)和[Runtime边界](../UseDocs/language/memory/runtime/README.md)现在区分内部验收与最终公共发布。更新了精确算术/商余/幂、二进制精确提升、向零截断/最近偶数、ASCII十进制文本语法、显示尺度及D/W/H和002/009/011/012/013/033/034边界。

完整示例源码与黄金未变，继续逐字进入原有VM/原生精确门禁。原页面verified只覆盖其既有范围；新增数值范围已经B2-A内部验收，最终发布仍planned。不虚构stdlib.lint/lfloat或新增能力位；registry整文件保持原值，B1七项仍verified，cmath仍planned。

首次UseDocs门禁发现库索引仍链接旧“规划中”标题，已修正锚点与过时冻结说明；失败日志保留，后续门禁实际通过。

## CI与独立债务

验证快照同时命中tests/fixtures/**、docs/UseDocs/library/**与docs/**过滤，维护回归和工作区push均真实触发。工作区运行[42348812756](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/42348812756)与三平台维护回归[42348812742](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/42348812742)均在第一次尝试全部成功；实际run/job/attempt和原始作业映射已归档，无本轮新增22C/22D失败。没有修改默认20分钟、A6手动40分钟、B1手动60分钟或函数阈值。

22C/22D、19/11/22与B1历史失败独立保留。A6原始Bool/头文件/看门狗/性能失败与22C排队取消、22D释放顺序日志仍在原目录，绿色不关闭这些债务；A0result.json失败身份未改写。本轮没有修改断言、调度或释放顺序，也不因第一次全绿关闭原有债务。

[本批收口自审](20o-b2-a7-self-review.md)包含实际1正/8反、状态纠正和历史边界核对。

## B2-A退出与下一入口

B2-A在内部1.18/20范围收口：精确值身份、算术、转换、语言管线、资源/错误/性能与B1功能均有阶段证据。本批没有改变ABI、生成器、intrinsic或XCON格式，不发布同版本内部产物。

下一入口是[B2-B实施规划](20o-b2-b-implementation-plan.md)，先读[数值基础总规划](20o-b2-numeric-foundation-plan.md)、已签[冻结正文](20o-b2-freeze-draft.md)和F-5/F-6/F-12/F-3精表，再拟定lfloat_divide与15项cmath的类型/IR/VM/LLVM/Runtime、intrinsic33～48、ABI1.19和生成器21实施/拒绝矩阵。不得在A7收口中直接写这些产品。

B2-C只在最终新1.19/21快照重跑完整B1 721条F10，并收齐最终三平台公共产物、教程和registry状态。A7的内部验收及A5/A6历史证据都不能替代它。
