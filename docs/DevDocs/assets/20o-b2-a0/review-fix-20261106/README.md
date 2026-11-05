# A0引用与文本身份补充复核

复核入口为`d88f68a718d781bd1aeea1ecf7e1b9a1317580d9`，A0被测源码仍为`d0e4a5fb9043aa6c93dce7892dbe1ef333d3b276`。
本次只补充引用说明、文本摘要与外部CI查询，不重新执行A0，不启动A1。
原[20261105归档](../entry-20261105/README.md)的全部文件、摘要、原始结果与日志保持原样。

## 下游引用

[reference-map.json](reference-map.json)从原始文件采集以下事实，并绑定其SHA-256：

| 用途 | 应引用对象 | 有效范围 |
| --- | --- | --- |
| A0有效收口结论 | [closeout.json](../entry-20261105/closeout.json) | `passed=true`，包含环境修复后的成功重试 |
| 初轮原始结果 | [result.json](../entry-20261105/result.json) | `passed=false`、`windows=failed`；不能写成通过或一次全绿 |
| A0源工作树文档门禁 | [repo-all-local-copy.json](../entry-20261105/repo-all-local-copy.json) | cwd为独立源码工作树，退出0 |
| 主工作区补充检查 | [workspace-doc-gate.json](../entry-20261105/workspace-doc-gate.json) | cwd为主仓库，退出0；只验证当时的收口文档，不能替代上一行 |

初轮依赖解析退出2、首次重试的`A0-LAYOUT-002`和实体复制后的成功均保留。当前结论是环境修复后有效收口，没有把初轮失败改写为通过。
本次没有生成新的绿色`result.json`；若后续明确要求原始执行本身全部通过，须在新任务目录按修正后的环境完整重跑，并作为新的运行身份单独登记。

## 两种文本摘要

[text-identities.json](text-identities.json)为原identity.json绑定的23个文件补列实际原始摘要和UTF-8/LF规范摘要，包含两份Cargo.toml、两份Cargo.lock、rust-toolchain.toml、.cargo/config.toml及其他既有身份对象。
规范化严格验证UTF-8，只把CRLF替换成LF；不删除BOM、不补末尾换行、不调整其他内容。

全部源工作树原始字节仍符合旧identity.json；全部规范字节同时与`d0e4a5fb`的Git blob及本次主工作区内容一致。
因此Cargo/锁文件的原始摘要差异属于行尾差异，不是内容或依赖变化。跨平台比较应使用`utf8_lf_sha256`，原始运行来源仍用`source_raw_sha256`。
采集命令、PID、固定提交的逐文件请求、Git对象ID和摘要均在同一JSON中；[review.py](review.py)保留采集与只读复核方法。

## CI查询的时间与范围

上次审阅反馈为网络失败、无法确认CI；该反馈不能写成成功证据。
本次重新查询成功，[ci-query.json](ci-query.json)记录实际命令、PID、时间和返回身份，[原始API响应](ci-response.json)确认`d88f68a7`的[Xiao workspace gates 42048065769](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/42048065769)第1次尝试已完成且成功。
该响应仅用于该提交的外部CI登记，不代表本次补注文档提交的CI状态，也不代替A0源工作树功能、文档门禁或其他平台执行。
原closeout.json中的`ci_used_as_local_evidence=false`保持不变。

## 复核方式

以下命令只读归档，不运行产品或CI：

```powershell
python -X utf8 docs/DevDocs/assets/20o-b2-a0/entry-20261105/verify-evidence.py
python -X utf8 docs/DevDocs/assets/20o-b2-a0/review-fix-20261106/review.py check
```

[checks.json](checks.json)记录本次文档门禁、两次只读复核和全路径改动边界；本次文档门禁的cwd仍为主工作区，单列为补充检查。
[evidence.json](evidence.json)列出本补充目录的全部文件摘要，原82项归档清单不改写。
Linux/macOS本地仍未验证，B1旧721条F10没有重跑，22D/22C与19/11/22历史债务未动，A1未开始。
