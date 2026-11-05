# B2-A实施规划准备记录

工程期：20O-B2-A。入口为`04bca101`后的规划工作树，F-D已签字、联合冻结已激活、台账30/0；本目录只保存只读事实与待审规划问题。

产品仍未开工；当前ABI1.18、生成器20、活动intrinsic最大32，cmath继续planned。没有执行Cargo解析/构建、产品测试或性能测量。

[source-facts.json](source-facts.json)绑定12个产品/登记文件的内容摘要和匹配片段，说明现有Lint/Lfloat文本承载、定宽算子内核、TAC/VM/XCON/ABI恢复与活动错误常量。
[decision-requests.json](decision-requests.json)保存A-P1/A-P2/A-P3三项自由文本规划问题；未收到实际批准前，B2-A产品代码不开工。F-D签字不替代本规划的实施批确认。

规划硬边界：不升ABI/生成器、不新增intrinsic/状态、保留Lint/Lfloat标签和XCON结构、不实现lfloat_divide/cmath、不把cmath升verified、不重跑B1旧721条F10。

未来实施批须先执行A0源码/Runtime/产物身份与B1功能基线，然后按A1～A7记录真实Cargo锁、错误目录、VM/原生、资源、性能和教程证据；本目录不是产品能力证据。

[签字与分批映射](signature-and-scope-audit.json)核对已签正文、30项台账投影及三项F-D批准未变，将F-1～F-12逐项映射到B2-A和后置B2-B；它不构成A-P1～A-P3批准。
[前置CI](ci-predecessor-943db5b1.json)保留943db5b1的workspace 41981794413成功及maintenance未触发的真实原始响应，不将入口CI写成04bca101或本轮规划提交的成绩。
本轮发现的A4文本转换方向歧义已按已签F-11修正；033/034活动诊断、两后端汇合、无分配Eq/Hash、缓存身份、B1功能与受控性能分离均已补入规划，尚未改产品。
