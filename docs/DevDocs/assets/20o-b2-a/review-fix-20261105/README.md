# B2-A审批后的状态与会话来源复核

被复核提交：61639b29。本次同步冻结收口页首、状态表和DevDocs索引为“B2-A规划已批准，A0～A7获准另批实施，产品仍未开工”。
已签草案全文（包括F-D时点状态行）、数值契约、台账、F-D/A-P问答原文与批准范围均不改。F-D/B2-A记录新增后续审批与来源时点说明。

## 原始实现会话引用

[来源索引](approval-conversation-source.json)与[必要原始用户消息摘录](approval-source-messages.jsonl)记录6条源消息：F-D三项与A-P三项。
仓库回执继续按实现Agent转录/转达记录理解；原authority中的“本次会话”指实现会话，不表示当前审阅界面直接可见或重新取得批准。

- 实现会话ID：`01a19376-4425-7ad2-9ae5-ee2a3cecc005`。
- 本地原日志：`C:\Users\xiaoc\.codex\sessions\2026\11\01\rollout-2026-11-01T03-36-04-01a19376-4425-7ad2-9ae5-ee2a3cecc005.jsonl`。
- 环境未提供可核验的网页会话URL，本次没有猜测或生成协议链接。

| 回执项 | 原日志行号 | UTC时间 |
| --- | --- | --- |
| F-Da | 18033 | 2026-11-05T03:12:51.330Z |
| F-Db | 18035 | 2026-11-05T03:13:20.048Z |
| F-Dc | 18037 | 2026-11-05T03:13:53.788Z |
| A-P1 | 18892 | 2026-11-05T09:11:30.122Z |
| A-P2 | 18894 | 2026-11-05T09:12:42.334Z |
| A-P3 | 18896 | 2026-11-05T09:12:42.355Z |

索引逐条保存原记录SHA-256、字节偏移、角色、问题项ID及摘录位置，并与已有回执的问题和答复精确比较。没有归档其他会话内容，也没有改问答或把新来源说明当成新的授权。
另一工作站若没有该原日志，只能核对仓库摘录与回执；不能据此声称已查看完整源会话。下面的原日志复核在源文件缺失时明确失败。

## 原日志与回执的只读复核

在仓库根、能够读取上述实现日志的工作站执行。所有内容都按数据读取，不重放任何审批动作。

```powershell
@'
import hashlib, json, pathlib, re

root = pathlib.Path("docs/DevDocs/assets/20o-b2-a/review-fix-20261105")
index = json.loads((root / "approval-conversation-source.json").read_text(encoding="utf-8"))
source = pathlib.Path(index["source_session"]["local_rollout_file"])
assert source.is_file(), "本机无源日志；不能宣称原始会话已核验"
excerpt = (root / index["excerpt_file"]).read_bytes()
assert hashlib.sha256(excerpt).hexdigest() == index["excerpt_sha256"]
excerpt_lines = excerpt.splitlines(keepends=True)
assert len(excerpt_lines) == index["excerpt_record_count"] == 6
with source.open("rb") as stream:
    metadata = json.loads(stream.readline())
    assert metadata["payload"]["id"] == index["source_session"]["thread_id"]
    for record in index["records"]:
        stream.seek(record["source_byte_offset"])
        original = stream.readline()
        assert hashlib.sha256(original).hexdigest() == record["source_record_sha256"]
        saved = excerpt_lines[record["excerpt_line"] - 1]
        assert hashlib.sha256(saved).hexdigest() == record["excerpt_record_sha256"]
        assert original.replace(b"\r\n", b"\n") == saved
        message = json.loads(original)
        assert message["timestamp"] == record["source_timestamp_utc"]
        assert message["type"] == "response_item" and message["payload"]["role"] == "user"
        text = "\n".join(part.get("text", "") for part in message["payload"]["content"])
        wrapped = re.fullmatch(r"\s*<send_user_message_question_reply>\s*(\[.*\])\s*</send_user_message_question_reply>\s*", text, re.S)
        assert wrapped
        reply = json.loads(wrapped.group(1))[0]
        receipts = json.loads(pathlib.Path(record["receipt_file"]).read_text(encoding="utf-8"))["receipts"]
        receipt = next(item for item in receipts if item["id"] == record["receipt_id"])
        assert reply["questionItemId"] == receipt["question_item_id"] == record["question_item_id"]
        assert reply["question"] == receipt["question"] and reply["answer"] == receipt["answer"] == "批准"
print(json.dumps({"passed": True, "source_thread_id": index["source_session"]["thread_id"], "original_user_records_verified": len(index["records"])}))
'@ | python -
```

## 审批快照与当前来源附注

[来源文字差异](plan-source-reference.diff)与[规划来源复核](plan-provenance-review.json)只补实现会话引用。A0～A7实施分节和A-P批准范围逐字未变。
61639b29的plan-after-approval.txt、approval-transition.json和checks.json仍记录当时的审批快照与实际门禁；旧全文摘要不追随本次来源附注。
B2-A当前清单另列current_plan_file_sha256，历史审批状态快照另列approval_status_plan_snapshot_sha256。F-D已签状态行只作当时冻结快照，不滚动改写。

本轮执行docs/usedocs/layout、源日志复核和签字对象无差异检查；没有执行Cargo、产品测试、基准或修改工作流。checks.json与evidence.json保存实际命令、日志和摘要。
