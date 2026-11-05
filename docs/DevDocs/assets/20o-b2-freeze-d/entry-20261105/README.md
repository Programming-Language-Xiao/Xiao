# F-D冻结签字记录

工程期：20O-B2-FREEZE-D。职责：按[F-D规划](../../../20o-b2-freeze-d-signature-plan.md)完成前置修正、固定30条签字基线、取得三项实际答复并登记状态切换。
工作入口459f9e5d，前阶段入口0a93f957，产品基线8dfbc89b；当前实际ABI1.18、生成器20、活动intrinsic最大32，cmath继续planned。

F-Da/F-Db/F-Dc已逐项收到明确“批准”，见[签字原文](decision-receipts.json)；F-D为signed，联合冻结已激活，产品仍未开工。
三项使用自由文本且无预选，没有从门禁或30/0推断签字，也未新增第31条数值裁定。

## 固定的签字对象

[签字基线](signature-baseline.json)固定以下对象的SHA-256：

- 草案签字前全文及正文；正文按Git规范UTF-8/LF计算，只移除唯一“状态：”行，其他字节全部保留。
- [签字前草案快照](draft-before-signature.txt)与[签字前台账](ledger-before-signature.json)；台账30项内容另作规范JSON摘要，以区别签字状态/来源字段的合法变化。
- [F-3/F-7五条实际答复](f-c-receipts.snapshot.json)、六份原始裁定来源文件及[30条来源对账](decision-coverage.json)。
- [38d6120d历史104条清单快照](coverage-38d6120d.snapshot.json)，仍按历史Git内容复跑，不更新原104条身份。

草案正文在提问前仅整理F-C退出时点的历史状态和一处已确认规则的措辞，见[前置差异](preflight-editorial.diff)；数值表、F-C资源分节及30项内容未变。
§8.6测试接线建议属于下一阶段实施规划，已登记在来源对账中，不作为未回答的数值裁定，也不授权本轮编写测试或产品。

[签字切换记录](signature-transition.json)与[状态行diff](status-line.diff)证明本次只改草案第3行；正文SHA-256仍为8bca48018be3bf2cf38f615d7cf35d55a0ff9745f73e3c196161abd854d286ff。任何后续正文变化都必须重新签字。F-C旧resource-performance-audit保持其历史Git blob，不跟随新状态行重写。
台账只更新已登记的状态/来源字段，30项内容摘要完全一致；[前后字段差异](ledger-state-changes.json)及ledger-before/after-signature.json分别保留全文和摘要。

## 前置修正与入口CI

F-C复核清单原reviewed_entry_evidence已改为current_entry_evidence并注明当前工作树时点；historical_entry_evidence另列38d6120d的Git blob与摘要。
F-C文档中的“最近CI”已改为“F-C收口CI”，并另列后续复核节点。

[0a93f957入口CI](ci-entry-0a93f957.json)保留workspace 41953275349成功及实际作业；maintenance没有触发。
原始API的push运行查询、维护工作流9个push.paths与该提交实际变更路径均归档，未命中任何路径；不把未触发写成通过或漏跑。
状态切换提交53e95a55已有真实CI结论，按下节独立归档，不与前置节点混用。
[未签基线提交CI](ci-preparation-22a865bc.json)另存22a865bc的workspace 41959843609成功与maintenance路径过滤事实；不将准备门禁当成状态切换提交的结果。

[只读源码事实](source-facts.json)与签字前检查记录核对30/0、ABI1.18/20、intrinsic32/100墓碑及cmath planned；没有产品、Cargo解析/构建或性能测量。
签字前检查记录见pre-signature-checks.json；本目录evidence.json保存当前证据摘要。

## 状态切换提交53e95a55的CI与退出

[状态切换CI](ci-signed-state-53e95a55.json)记录workspace 41962238521与maintenance 41962238445第1次尝试均成功，七个实际作业全部成功，四份原始API及摘要完整保存。
该提交只在UseDocs中改动cmath教程，真实命中docs/UseDocs/library/**而正常触发维护回归；0a/22a未命中路径、未触发维护的事实仍独立保留。
这仅证明状态切换提交的既有门禁，未交付B2数值产品、未解析或构建六包新依赖，也未生成新的721条F10成绩。

[历史边界](historical-boundaries.json)按真实Git差异确认B1文档/证据及19/11/22文档组未改，不核销历史债务。
F-D三项实际批准和签字后本地检查均已通过，联合冻结已激活；CI成绩绑定53e95a55。后续只保存DevDocs证据的提交单独判断触发情况，不沿用该成绩。

## 签字历史快照的只读复跑

以下在仓库根读取未签基线22a865bc和状态切换提交53e95a55；不依赖当前工作树状态，不改仓库文件，也不把哈希比对当作新的用户授权。
当前正文的防漂移检查另见post-signature-checks.json。任何后续正文变化都须重新签字，不能用本历史复跑冒充对新正文的批准。

```powershell
@'
import hashlib, json, subprocess

root = "docs/DevDocs/assets/20o-b2-freeze-d/entry-20261105/"
prepared = "22a865bc2c5e6dbc688cf2c99805462a805b5d50"
signed = "53e95a554a8fb6afd1c48af9227c38d3482a8c7c"
def raw(commit, path):
    return subprocess.check_output(["cmd", "/c", "git", "show", commit + ":" + path])
def sha(data):
    return hashlib.sha256(data).hexdigest()
def canonical(value):
    return json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(",", ":")).encode("utf-8")

baseline = json.loads(raw(prepared, root + "signature-baseline.json"))
assert sha(canonical({k: v for k, v in baseline.items() if k != "baseline_id"})) == baseline["baseline_id"]
before = raw(prepared, baseline["draft"]["file"]).splitlines(keepends=True)
after = raw(signed, baseline["draft"]["file"]).splitlines(keepends=True)
index = baseline["draft"]["status_line_number"] - 1
assert len(before) == len(after) and sum(a != b for a, b in zip(before, after)) == 1
assert after[index].decode("utf-8").rstrip("\n") == baseline["draft"]["allowed_status_line_after"]
body = b"".join(line for i, line in enumerate(after) if i != index)
assert sha(body) == baseline["draft"]["body_sha256"]
ledger = json.loads(raw(signed, baseline["ledger"]["file"]))
assert len(ledger["items"]) == 30 and sha(canonical(ledger["items"])) == baseline["ledger"]["decision_items_sha256"]
assert ledger["f_d_signature_status"] == "signed" and ledger["joint_freeze_activated"]
assert ledger["product_implementation_started"] is False
receipts = json.loads(raw(signed, root + "decision-receipts.json"))["receipts"]
assert [r["id"] for r in receipts] == ["F-Da", "F-Db", "F-Dc"]
assert all(r["answer"] == "批准" and r["valid_for_activation"] and not r["conditional"] for r in receipts)
assert all(r["baseline_id"] == baseline["baseline_id"] for r in receipts)
print(json.dumps({"passed": True, "signed_commit": signed, "only_status_line_changed": True, "body_sha256": sha(body), "actual_approvals": len(receipts)}))
'@ | python -
```

## B2-A交接边界

三个有效实际答复已齐备，台账f_d_signature_status已置为signed、joint_freeze_activated已置为true。
product_implementation_started始终保持false；下一入口仅为[B2-A实施规划交接](b2-a-planning-handoff.md)，不在F-D编写产品或启动B2-B/C。
cmath仍planned，B1旧721条只作历史、B2-C新快照完整重跑；22D/22C与19/11/22债务独立保留。
