# F-B3错误精表裁定记录

工程期：20O-B2-FREEZE-B3。职责：按[本批规划](../../../20o-b2-freeze-b3-error-table-plan.md)收齐F-12错误身份、优先级和F-5状态映射。
边界：只写文档与来源核验，当前实际ABI1.18、生成器20、活动intrinsic最大32；cmath继续planned。

实际工作入口为537ce4ec，产品基线为59a82934。[三条答复原文](decision-receipts.json)已确认F-12a/F-12b/F-12c，
F-B3以28 confirmed、2 pending退出，剩余F-3/F-7留F-C；后续进度见[实时台账](../../20o-b2-freeze-close/entry-20261103/decision-ledger.json)。
三项均有实际答复，不从预选推断确认。F-B3退出时整体未联合冻结、F-C与B2-A/B未开始；本目录保留阶段退出事实，后续总进度见实时台账。

完整已确认契约以[冻结草案§5.2](../../../20o-b2-freeze-draft.md)为准；033/034仍是规划身份，未占用活动诊断表。
本轮确认034在工作/结果预算前的判定阶段、文本内语法与数值预算顺序，以及013也经C ABI状态6传播。
F-3接纳算法和性能、F-7依赖仍留F-C，不把B1旧721条F10写成当前成绩。

[源码事实](source-facts.json)绑定7个只读文件及核对片段；保留现有002/009/011/012/013和C ABI状态，
并核对033/034未进入活动错误定义。本批没有生成B2产品、调用新C符号或执行错误路径。
[静态对账](error-table-audit.json)保存8行错误、7阶段优先级、7项状态6错误映射及21个契约算例。
算例仅是已确认规则的预期，product_executed全部为false；文档和结构校验不能代替实现阶段的错误路径测试。
对账保存冻结草案的Git blob与SHA256；后续F-C若更新整页，应按已存blob核对历史快照，不要求旧报告追随新文档。
[检查记录](checks.json)和[摘要清单](evidence.json)绑定本轮文档门禁、来源核验与产品边界；
F-12三条实际答复和这些检查共同构成F-B3退出证据，不构成F-C或产品实施的完成证据。

## 59a82934入口CI的明确登记

[CI摘要与完整来源命令](ci-baseline-59a82934.json)记录workspace 41874054683成功；
maintenance 41874054753第1次尝试整体failure，macOS作业126728553452为cancelled，其余五作业成功。
macOS的runner_id=0、无步骤；[原始注释](macos-annotations-api.json)说明托管runner未获取及macOS arm64容量约束。
注释的multiple attempts指获取runner尝试，不能写成workflow多次重跑；此处run_attempt仍为1。

本轮明确登记，没有派发重跑；保留整体失败和取消，不写成三平台全绿或产品失败，也不核销独立19/11/22历史债务。
原始API响应及采集argv完整保留；以下只读命令可重新核对：

```powershell
gh api repos/Programming-Language-Xiao/Xiao/actions/runs/41874054753
gh api "repos/Programming-Language-Xiao/Xiao/actions/runs/41874054753/attempts/1/jobs?per_page=100"
gh api "repos/Programming-Language-Xiao/Xiao/check-runs/126728553452/annotations?per_page=100"
```
