# A7第二轮最终审计器正反例

本轮复核新增外部旧产物路径反例：把本地产物路径改成任务根之外的旧缓存后，审计必须按实际原因拒绝。`cases.json`记录当前`audit.py`与`validate-audit.py`摘要，1个正例与8个破坏性反例全部通过；每个用例的stdout/stderr单独保存，拒绝以实际错误全文和退出码断言。

`validate-audit.py`现在先校验有效归档的`evidence.json`与磁盘原件一致，再复制到临时目录重建清单；避免清单缺文件被临时重建掩盖。首轮1正/7反与最终7反记录保持原值，见[首轮反例](../self-review-20261107/README.md)和[最终7反](../self-review-final-20261107/README.md)；[有效归档与复跑](../entry-20261107/README.md)未改写。
