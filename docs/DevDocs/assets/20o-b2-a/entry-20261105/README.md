# B2-A实施规划准备记录

工程期：20O-B2-A。入口为`04bca101`后的规划工作树，F-D已签字、联合冻结已激活、台账30/0；本目录保存只读事实、已审规划快照和三项实际批准。

产品仍未开工；当前ABI1.18、生成器20、活动intrinsic最大32，cmath继续planned。没有执行Cargo解析/构建、产品测试或性能测量。

[source-facts.json](source-facts.json)绑定12个产品/登记文件的内容摘要和匹配片段，说明现有Lint/Lfloat文本承载、定宽算子内核、TAC/VM/XCON/ABI恢复与活动错误常量。
[decision-receipts.json](decision-receipts.json)保存A-P1/A-P2/A-P3三条明确“批准”，问题项ID、日期和原文完整；[decision-requests.json](decision-requests.json)已无待答项。F-D签字没有代替这次实施规划审批，产品仍未开工。
F-D已签草案页首状态行是F-D签字时的冻结快照，不作为后续审批的滚动状态；该行及已签正文、摘要保持原样。B2-A后续审批独立记录，当前为规划已批准、A0～A7获准另批实施，产品仍未开工。
回执本身保留实现Agent的原始转录，不将其中“本次会话”解释为当前审阅会话直接可见的批准。[源会话索引及必要原始消息摘录](../review-fix-20261105/approval-conversation-source.json)补列实现会话ID、日志路径、行号、时间戳和逐记录SHA-256，问答与问题项ID逐项匹配；没有补造网页会话链接。

规划硬边界：不升ABI/生成器、不新增intrinsic/状态、保留Lint/Lfloat标签和XCON结构、不实现lfloat_divide/cmath、不把cmath升verified、不重跑B1旧721条F10。

未来实施批须先执行A0源码/Runtime/产物身份与B1功能基线，然后按A1～A7记录真实Cargo锁、错误目录、VM/原生、资源、性能和教程证据；本目录不是产品能力证据。

[签字与分批映射](signature-and-scope-audit.json)核对已签正文、30项台账投影及三项F-D批准未变，将F-1～F-12逐项映射到B2-A和后置B2-B；该只读核对本身不作为审批；三项批准以实际答复原文为准。
[前置CI](ci-predecessor-943db5b1.json)保留943db5b1的workspace 41981794413成功及maintenance未触发的真实原始响应，不将入口CI写成04bca101或本轮规划提交的成绩。
本轮发现的A4文本转换方向歧义已按已签F-11修正；033/034活动诊断、两后端汇合、无分配Eq/Hash、缓存身份、B1功能与受控性能分离均已补入规划，尚未改产品。

## 已审快照与审批后状态

[规划基线](planning-review-baseline.json)与reviewed-plan-before-approval.txt固定提问时f4a0b881的规划，SHA-256为3e76188c34ecc4bef6709652bef41903d812240abfe862b2c95645fa5c4587be。
[审批切换](approval-transition.json)、plan-after-approval.txt与planning-approval-status.diff仅更新审批状态和来源，A0～A7实施正文及三项批准范围未改；plan-audit.json保持提问时的历史Git blob，不拿旧全文摘要比较当前状态文字。

[待审规划提交CI](ci-reviewed-plan-f4a0b881.json)另列f4a0b881的workspace 41998981332第1次尝试成功及maintenance路径过滤未触发；它不是审批收口提交或B2-A产品验证。
本轮只有文档和只读证据，没有执行Cargo解析/构建、测试、基准或修改工作流；另批产品从A0开始。旧721条及19/11/22债务未动，已签正文、30项台账与F-D原文未动。

上述plan-after-approval.txt、approval-transition.json和checks.json记录61639b29审批收口时点；后续只补充来源说明，不追改这些实施内容快照。当前源会话引用补充及文档摘要见[本次复核](../review-fix-20261105/README.md)，不能把旧快照的全文摘要当成滚动当前正文摘要。
