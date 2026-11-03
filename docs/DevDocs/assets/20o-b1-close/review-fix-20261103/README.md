# CLOSE审计反例的结构化校验勘误

工程期：20O-B1-CLOSE复核。职责：修复空来源集合与旧格式化输出假命中，保存重做验证和自审提交CI观察。
边界：原41498029910成功日志、容量取消、22C失败及F10测量不改；本目录均为审计工具的本机受控验证。

`audit.ps1`在if表达式外捕获来源数组，空结果进入明确的Require检查。
`validate-audit.ps1`的子进程catch真实ErrorRecord，按退出码、异常类型及`Exception.Message`精确匹配；
格式化错误和源码行只作为诊断留档，不参与判断。脚本、结果摘要及归档范围见[清单](evidence.json)。

| 验证 | 实际结果 | 证据 |
| --- | --- | --- |
| 原始归档 | 正向审计通过，沿用721条F10离线复核 | [结果](audit-positive.json) |
| 聚合作业ID替换 | 退出1；RuntimeException；实际消息为“实际日志作业被聚合作业ID替代或缺失” | [真实异常](aggregate-job-id.error.json)、[stderr](aggregate-job-id.log) |
| CSV O3缺失且校验和自洽 | 退出1；RuntimeException；实际消息为“产品模块或优化级别缺失/重复” | [真实异常](missing-csv-o3.error.json)、[stderr](missing-csv-o3.log) |
| 源码回显假命中控制 | 格式化诊断含预期文字，实际为Count属性错误，新验证器不接受 | [控制结果](validation.json)、[真实异常](source-line-false-hit.error.json)、[格式化诊断](source-line-false-hit-formatted.log) |

[反例清单](negative-cases.json)同时保存预期与实际消息、实际类型和FullyQualifiedErrorId。
复跑命令见[证据目录](../README.md#反例复跑)，默认输出新的临时目录。

旧[aggregate日志](../self-review-20261103/aggregate-job-id.log)原样保留，旧清单已明确撤回该项误判。
[CI观察](ci-observation.json)补入cbd7fdaf自己的维护回归41580498113和工作区门禁41580498128，均成功；
它们与初次890cdb81观察分开，也不作为旧反例判断正确的证据。

## B2交接补登

87bac439自己的[工作区门禁节点](ci-87bac439.json)也已补登：41585868754、实际作业125751076106成功。
该提交只修文档与审计入口，没有改变B1被测产品快照；它不是B2功能或性能证据。

`source-line-false-hit-formatted.log`与`source-line-false-hit.error.json`含每次新建的随机临时路径，
已在清单`diagnostic_files`中标记为仅诊断，从`files`摘要校验项移出，原始内容不改。
通过判定使用稳定的真实异常字段与`validation.json`，不要求格式化诊断逐字节复现。
同环境两次复跑的10份稳定产物与2份诊断差异见[入口比较](../../20o-b2-freeze-prep/entry-20261103/replay-comparison.json)。
