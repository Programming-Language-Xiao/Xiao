# F-D冻结签字记录

工程期：20O-B2-FREEZE-D。职责：按[F-D规划](../../../20o-b2-freeze-d-signature-plan.md)完成前置修正、固定30条签字基线、取得三项实际答复并登记状态切换。
工作入口459f9e5d，前阶段入口0a93f957，产品基线8dfbc89b；当前实际ABI1.18、生成器20、活动intrinsic最大32，cmath继续planned。

目前仅完成签字准备，F-D状态仍为not_started，联合冻结未激活，产品未开工。
F-Da/F-Db/F-Dc须逐项实际答复；不设预选、不从门禁或30/0推断签字，也不新增第31条数值裁定。

## 固定的签字对象

[签字基线](signature-baseline.json)固定以下对象的SHA-256：

- 草案签字前全文及正文；正文按Git规范UTF-8/LF计算，只移除唯一“状态：”行，其他字节全部保留。
- [签字前草案快照](draft-before-signature.txt)与[签字前台账](ledger-before-signature.json)；台账30项内容另作规范JSON摘要，以区别签字状态/来源字段的合法变化。
- [F-3/F-7五条实际答复](f-c-receipts.snapshot.json)、六份原始裁定来源文件及[30条来源对账](decision-coverage.json)。
- [38d6120d历史104条清单快照](coverage-38d6120d.snapshot.json)，仍按历史Git内容复跑，不更新原104条身份。

草案正文在提问前仅整理F-C退出时点的历史状态和一处已确认规则的措辞，见[前置差异](preflight-editorial.diff)；数值表、F-C资源分节及30项内容未变。
§8.6测试接线建议属于下一阶段实施规划，已登记在来源对账中，不作为未回答的数值裁定，也不授权本轮编写测试或产品。

签字成功后只允许改变草案页首状态行；任何正文变化都必须重新签字。F-C旧resource-performance-audit保持其历史Git blob，不跟随新状态行重写。
台账在签字后可更新已登记的状态/来源字段，但30项内容摘要必须一致，并保留前后全文、差异和摘要。

## 前置修正与入口CI

F-C复核清单原reviewed_entry_evidence已改为current_entry_evidence并注明当前工作树时点；historical_entry_evidence另列38d6120d的Git blob与摘要。
F-C文档中的“最近CI”已改为“F-C收口CI”，并另列后续复核节点。

[0a93f957入口CI](ci-entry-0a93f957.json)保留workspace 41953275349成功及实际作业；maintenance没有触发。
原始API的push运行查询、维护工作流9个push.paths与该提交实际变更路径均归档，未命中任何路径；不把未触发写成通过或漏跑。
本阶段尚无最终签字提交的CI结论，后续按真实提交另记触发情况。

[只读源码事实](source-facts.json)与签字前检查记录核对30/0、ABI1.18/20、intrinsic32/100墓碑及cmath planned；没有产品、Cargo解析/构建或性能测量。
签字前检查记录见pre-signature-checks.json；本目录evidence.json保存当前证据摘要。

## B2-A交接边界

只有三个有效实际答复齐备，才可将台账f_d_signature_status置为signed、joint_freeze_activated置为true。
product_implementation_started始终保持false；下一入口仅为[B2-A实施规划交接](b2-a-planning-handoff.md)，不在F-D编写产品或启动B2-B/C。
cmath仍planned，B1旧721条只作历史、B2-C新快照完整重跑；22D/22C与19/11/22债务独立保留。
