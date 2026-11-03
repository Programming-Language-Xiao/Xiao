# 当前分支的B1历史SourceRoot复跑

工程期：B2-FREEZE交接修复。职责：修复validate-audit的SourceRoot透传，重做当前分支的B1历史审计与反例验证。
边界：只读复核历史B1归档，不重新测量721条性能，不改产品、ABI、生成器或19/11/22债务。

取证入口为4be03c8f加本次验证器修复的工作树；完整脚本摘要、实际目录、命令参数和输出文件见[来源清单](provenance.json)。
数据源码固定为106b99c70e88a88afd0dd9a33f6a9c792e13324c，使用git archive导出，ZIP注释中的提交身份也已核对。
每次新建含空格的独占临时目录，不切换或覆盖工作树；本机绝对路径只标识实际采集，复跑时按[公开命令](../README.md#只读复跑)新建目录。

| 检查 | 实际结果 | 证据 |
| --- | --- | --- |
| 当前源码、不传SourceRoot的audit | 退出1；实际异常为“状态文件已偏离被测快照：docs/module-registry.json” | [结构化异常](default-audit.stdout.log) |
| 当前源码、不传SourceRoot的validator | 退出1；外层“真实归档审计未通过”，子审计仍为registry快照差异 | [外层异常](default-validator.stdout.log)、[子进程异常](default-current-source/positive.error.json) |
| 历史SourceRoot正向审计 | passed=true；721条F10、36条释放、8个状态文件、9个工具文件 | [完整stdout](audit-historical.stdout.log) |
| 历史SourceRoot反例验证，第一次 | 聚合ID替换与CSV O3缺失均按真实异常类型/消息拒绝；源码回显控制不被误接受 | [摘要](run-1/validation.json)、[反例](run-1/negative-cases.json) |
| 同一SourceRoot第二次验证 | 正向与反例全部通过，10份稳定结果字节一致 | [摘要](run-2/validation.json)、[比较](comparison.json) |
| README公开命令逐字复跑 | 在同一PowerShell会话执行准备、审计与反例命令，均退出0 | [命令与来源](readme-command-proof.json)、[完整stdout](readme-command.stdout.log) |

`SourceRoot`已透传至正向、聚合ID替换、CSV O3缺失以及源码回显控制的子进程。
反例判定要求退出1、RuntimeException及实际Exception.Message精确相等，不搜索格式化源码。
两个默认拒绝同样核对真实异常类型和消息，不以“非零退出”代替原因验证。

每次两份源码回显诊断包含反例临时路径，保留原文、标为仅诊断，不要求跨次字节相同。
本次10份稳定结果只比较同一源码、工具、Windows/PowerShell及区域条件下的两次运行；不声称跨机器诊断路径相同。
原始41498029910归档全部文件在复跑前后的摘要一致，见[原始文件摘要](original-report-digests.json)，当前cmath planned登记也保持。

此前[B2入口比较](../../20o-b2-freeze-prep/entry-20261103/replay-comparison.json)发生于planned登记前，
旧[审计摘要](../../20o-b2-freeze-prep/entry-20261103/historical-source-audit.json)未完整保存来源命令；两者保留历史身份，由本目录提供新的当前入口实证。
目录内文本进程输出仅做UTF-8、LF和ANSI清理；检查对象、结果与异常正文未删改。721条均为历史离线复核，performed_measurement=false。
