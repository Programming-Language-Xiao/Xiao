# 20O-B1-CLOSE. 七库验收与证据收口

状态：B1-CLOSE已完成；七库教程与registry七项均verified，状态提交106b99c7的完整门禁与证据已收齐（2026-11-03）。
本批按[收口规划](20o-b1-closeout-plan.md)执行；开工提交70440da2，产品与工具仍与44dc9d6e/df27ca75相同。
本页登记状态提交的实际run/job/attempt及退出对账。B1收口时的证据回填只更新总结与清单，当时七册教程、registry和产品仍等于已执行快照。
当前B2已新增cmath planned登记，registry整文件不再等于106b99c7；复跑B1须按[历史SourceRoot入口](assets/20o-b1-close/README.md#只读复跑)执行，
新验证器透传与完整来源实证见[复跑记录](assets/20o-b1-close/source-root-replay-20261103/README.md)，不把新登记改回旧指纹。

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
在B1收口时再次由原audit.ps1对当时源码独立核对：VM/Linux103条，三平台原生O0/O3各206条，共721条；
20入口/31负载、唯一规模与优化键、五次样本、中位数、翻倍比、内存及产品摘要均通过。
本批没有产品、工具或预算变化，按规划携带这批证据，不伪称为新测量，不使用身份修复前的旧721条。

F10结果适用于报告中的硬件与工具链。时间不包含编译、初始化、输入准备和终端I/O，
内存是OS历史驻留峰值减当前基线的保守上界；不把验收数值解释为任意硬件时延保证或API超时。

## 三、状态提交的最终门禁

状态升级提交已按原门禁通过：默认workspace、四个B1精确点、9项契约、17项资源、5项默认性能回归、
四路径28/28守卫及完整工作区门禁。四个B1目标分别要求恰好1 passed/0 failed/0 ignored，原生含O0/O3。
移除、真实.xar重放、普通包隔离、双路径、参数/别名碰撞和教程逐字一致性由现有契约目标执行。
生成器20六组释放向量由三平台现有目标逐对象对账，原22D断言保留。

状态升级快照为106b99c70e88a88afd0dd9a33f6a9c792e13324c。
维护回归[41498029910](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/41498029910)第3次尝试成功，
同提交工作区门禁[41498029872](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/41498029872)成功。
首轮仅macOS未分配到runner而自动取消、没有执行步骤，其余五作业成功；第二次在既有22C释放顺序断言失败，
该次B1原生精确目标尚未执行，[原始22C失败](assets/20o-b1-close/41498029910/macos-22c-attempt2-failure.json)和完整日志保留。
只重试同一提交的macOS，不修改测试、断言、顺序或预算，
[原始取消与注释](assets/20o-b1-close/41498029910/macos-capacity.json)保留。下表使用实际产生日志的作业，
不把最终尝试中复用结果的聚合ID当作重新执行。

| 路径 | 实际日志作业 / 尝试 | 证据 |
| --- | --- | --- |
| VM/Linux | [125457995455](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/41498029910/job/125457995455) / 1 | [实际输出](assets/20o-b1-close/41498029910/vm-linux-exact.log) |
| 原生Linux O0/O3 | [125457995362](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/41498029910/job/125457995362) / 1 | [实际输出](assets/20o-b1-close/41498029910/native-linux-exact.log) |
| 原生Windows O0/O3 | [125457995302](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/41498029910/job/125457995302) / 1 | [实际输出](assets/20o-b1-close/41498029910/native-windows-exact.log) |
| 原生macOS O0/O3 | [125478253499](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/41498029910/job/125478253499) / 3 | [实际输出](assets/20o-b1-close/41498029910/native-macos-exact.log) |
| 完整工作区门禁 | [125457995746](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/41498029872/job/125457995746) / 1 | [实际输出](assets/20o-b1-close/41498029910/workspace-gates-summary.log) |

四个B1点各恰好1 passed/0 failed/0 ignored；七产品完整示例均实际执行原生O0/O3，历史B0示例按原测试配置各执行一次。
默认9项契约、17项资源、5项性能回归和四路径28/28守卫通过；三平台六组释放向量各12条逐对象匹配。
完整workspace、fmt、clippy、docs/usedocs/layout、覆盖率、benchmark/Bun等均按既有门禁通过。
文档覆盖率90.27%，公共API文档覆盖率100%；未放宽断言、阈值或既有REPL隔离规则。

[最终证据目录](assets/20o-b1-close/41498029910/README.md)保存机器清单、实际作业归属、完整日志ZIP与源行/摘要。
清单同时绑定七册教程与registry的Git对象，避免把证据回填后的文档提交伪称为新的产品运行。


## 四、独立债务与限制

41450826013及41464439400全绿不关闭41444159791的macOS22D历史失败；
原始释放顺序失败、所属提交及源日志见[历史22D记录](20o-b1-rem-self-review.md#历史22d失败与最新维护回归)。
本次macOS的22C释放顺序失败也按原测试归属独立保留；后续重试绿色不核销这次失败或22体系债务。
19发布、11C/11B与22D/22债务继续按各自文档处理，本批不核销。

CSV仍为单记录子集，URL仍为组件编码，statistics仍只覆盖int求和/最小/最大；
全文CSV、完整URL解析、浮点/高精度/均值方差、通用私有helper和其它后续库均不因本次verified而开放。
后续若发现产品缺陷，应回到相应IMP/前置批次，不在CLOSE内修改函数体或预算。

## 五、退出对账

| 出口 | 结果 |
| --- | --- |
| 七册教程与七项registry同步verified | 已完成，stage包含20O-B1-CLOSE，产品示例与夹具逐字一致 |
| 状态提交的默认workspace、四精确点、完整工作区门禁 | 106b99c7实际执行通过，实际run/job/attempt与完整源日志已入库 |
| F10的721条同源证据 | 原报告、产品/工具、冻结表与测量协议身份复核通过；明确为携带已有测量 |
| 导出/移除、归档、普通包、错误/资源及教程执行 | 九项契约与17项资源及精确目标均通过 |
| 生命周期 | 生成器20六组向量三平台匹配，原22D断言保持 |
| 独立债务 | 历史22D失败及19/11/22债务继续独立登记，不在本批核销 |

B1七库在本批收口。历史planned记录、旧721条与所有已归档失败保持其原提交/阶段语境，
不改写成当时已经verified；后续库按20O/20P各自的探测与冻结流程另行启动。

## 六、后续自审核

890cdb81之后的[自审核](20o-b1-close-self-review.md)收紧示例/资源测试的覆盖口径，
补入可复跑的CLOSE只读审计入口及两个受控拒绝证据；产品、七册教程、registry、测量工具与冻结预算未变。
初次aggregate反例实际触发.Count属性异常，按预期原因拒绝的旧结论已撤回；
修复后按真实异常消息和类型重做验证，见[结构化勘误](assets/20o-b1-close/review-fix-20261103/README.md)。

## 七、下一阶段

B2 数值基础批的规划见[数值基础批规划](20o-b2-numeric-foundation-plan.md)。
B2 未开工、未冻结；B1 的 721 条 F10 在 B2 改变 Runtime/工具后只作历史，不能自动充当
B2 当前证据。
