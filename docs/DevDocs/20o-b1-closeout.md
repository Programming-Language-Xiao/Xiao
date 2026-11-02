# 20O-B1-CLOSE. 七库验收与证据收口

状态：教程与registry七项已同步升为verified，最终状态提交的维护回归和工作区门禁待取得（2026-11-03）。
本批按[收口规划](20o-b1-closeout-plan.md)执行；开工提交70440da2，产品与工具仍与44dc9d6e/df27ca75相同。
状态升级依据已有完整产品、教程、三平台与F10证据；本页在新提交门禁完成后登记实际run/job/attempt并做最终退出对账。

## 一、交付与边界

| 模块 | 入口数 | 教程 | registry状态 |
| --- | --- | --- | --- |
| math | 6 | [整数计算](../UseDocs/library/math.md) | verified |
| hex | 2 | [十六进制](../UseDocs/library/hex.md) | verified |
| base64 | 2 | [Base64](../UseDocs/library/base64.md) | verified |
| url | 2 | [组件转义](../UseDocs/library/url.md) | verified |
| csv | 2 | [单记录CSV](../UseDocs/library/csv.md) | verified |
| fmt | 3 | [文本与整数格式](../UseDocs/library/fmt.md) | verified |
| statistics | 3 | [整数统计](../UseDocs/library/statistics.md) | verified |

七册教程以真实产品导入、稳定错误及独立黄金作为主示例，B0探测原文保留在附录；
两份完整源码继续由既有CI逐字核对并执行。七项stage加入20O-B1-CLOSE，源码、夹具、测试和证据路径同步。
Runtime ABI保持1.18，生成器保持20；不修改七库函数体、测试断言、工作流、预算或CI阈值。
error_code的独立verified状态不变，标准库骨架登记仍为draft。

## 二、入口与F10身份复核

[入口清单](assets/20o-b1-close/preflight-20261103/README.md)核对了七产品摘要、工具Git对象、
core/rust、产品夹具、工作流和守卫目录，与实际测量提交相同。原始完整日志ZIP摘要已核对。
运行41464439400第二次尝试与工作区门禁41464421187已在线确认success。

固定源码身份的[四份原始报告](assets/20o-b1-imp/rem-identity-final-41464439400/README.md)
再次由原audit.ps1对当前源码独立核对：VM/Linux103条，三平台原生O0/O3各206条，共721条；
20入口/31负载、唯一规模与优化键、五次样本、中位数、翻倍比、内存及产品摘要均通过。
本批没有产品、工具或预算变化，按规划携带这批证据，不伪称为新测量，不使用身份修复前的旧721条。

F10结果适用于报告中的硬件与工具链。时间不包含编译、初始化、输入准备和终端I/O，
内存是OS历史驻留峰值减当前基线的保守上界；不把验收数值解释为任意硬件时延保证或API超时。

## 三、状态提交的最终门禁

状态升级提交仍须通过：默认workspace、四个B1精确点、9项契约、17项资源、5项默认性能回归、
四路径28/28守卫及完整工作区门禁。四个B1目标分别要求恰好1 passed/0 failed/0 ignored，原生含O0/O3。
移除、真实.xar重放、普通包隔离、双路径、参数/别名碰撞和教程逐字一致性由现有契约目标执行。
生成器20六组释放向量由三平台现有目标逐对象对账，原22D断言保留。

当前尚未取得本次状态提交的最终运行号；完成前不把本页写成B1-CLOSE已退出。
最终run/job/attempt、源日志摘要与摘录归档到[CLOSE证据目录](assets/20o-b1-close/README.md)。

## 四、独立债务与限制

41450826013及41464439400全绿不关闭41444159791的macOS22D历史失败；
原始释放顺序失败、所属提交及源日志见[历史22D记录](20o-b1-rem-self-review.md#历史22d失败与最新维护回归)。
19发布、11C/11B与22D/22债务继续按各自文档处理，本批不核销。

CSV仍为单记录子集，URL仍为组件编码，statistics仍只覆盖int求和/最小/最大；
全文CSV、完整URL解析、浮点/高精度/均值方差、通用私有helper和其它后续库均不因本次verified而开放。
若最终回归暴露产品缺陷，按规划停止CLOSE并回到相应IMP/前置批次。

## 五、退出对账

七库教程与registry已同步升级、产品/F10身份已复核；最终同提交门禁和证据收集尚待完成。
全部完成后在本页登记实际运行号并同步阶段索引，不把历史planned快照改写成当时已验收。
