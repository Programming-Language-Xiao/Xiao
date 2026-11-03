# B1-CLOSE证据

工程期：20O-B1-CLOSE。职责：保存最终状态升级的门禁、源码身份与退出对账。
边界：不修改产品、ABI、生成器、预算或19/11/22债务；旧失败和旧报告保留原归属。

- [入口复核](preflight-20261103/README.md)：沿用固定源码身份的721条F10所需对象与摘要证明。
- [状态提交最终门禁](41498029910/README.md)：106b99c7的维护回归41498029910与工作区门禁41498029872、四精确点与完整日志。
- [初次自审（含勘误）](self-review-20261103/README.md)：保留原始输出，aggregate按原因拒绝的旧判定已撤回。
- [结构化复核勘误](review-fix-20261103/README.md)：两个反例的实际异常原因、源码回显控制及cbd7fdaf自己的CI状态。

状态与最终运行号见[收口记录](../../20o-b1-closeout.md)。

## 只读复跑

从仓库根运行（PowerShell 7）：

```powershell
pwsh -NoProfile -File docs/DevDocs/assets/20o-b1-close/audit.ps1
```

[audit.ps1](audit.ps1)校验五份成功原始日志、十五处摘录及源行、实际作业与聚合ID的区别、
四个精确点、产品O0/O3、六组释放向量、默认回归和28/28守卫；容量取消与22C失败也须完整保留。
它还核对七产品、九个测量/诊断工具文件、七册教程与registry的文件/内容Git对象摘要，
重算五项冻结摘要，并调用原F10离线审计入口重算四报告721条；不执行新的测量或改写原记录。

默认使用当前源码；未来源码变化时，`-SourceRoot`可指向含106b99c7源码/教程的检出目录，
`-RepositoryRoot`仍指向保存本批归档的仓库。`-ReportDirectory`用于选择本次41498029910归档的副本。
脚本只核对清单列出的文件；本批受保护代码、夹具、门禁、配置和stdlib源码还应按以下只读命令对照：

```powershell
cmd /c git diff --exit-code 44dc9d6e 890cdb81 -- core/rust tests/fixtures .github/workflows tools/gates .cargo/config.toml tests/benchmarks "stdlib/*.xiao"
```

这里对照归档提交890cdb81；核对当前工作树时省去第二个提交参数。路径范围与自审清单中的`unchanged_code_command`一致。

## 反例复跑

```powershell
pwsh -NoProfile -File docs/DevDocs/assets/20o-b1-close/validate-audit.ps1
```

[validate-audit.ps1](validate-audit.ps1)在临时副本中验证聚合ID替换、CSV O3缺失，并复现“源码回显含预期文字、真实异常却不同”的误判条件。
子进程记录真实`ErrorRecord.Exception.Message`、异常类型和`FullyQualifiedErrorId`；
通过条件要求退出1、`RuntimeException`类型及实际异常消息精确匹配，格式化诊断和源码行不参与匹配。
命令返回新建的输出目录；可用`-OutputDirectory`指定空目录，原始归档保持不变。
旧aggregate判定的撤回与修订结果见[勘误证据](review-fix-20261103/README.md)。

当前入口不访问网络。线上结论仍以已归档run/job/attempt及对应GitHub源日志为依据。
