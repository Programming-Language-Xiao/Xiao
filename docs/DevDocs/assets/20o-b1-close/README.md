# B1-CLOSE证据

工程期：20O-B1-CLOSE。职责：保存最终状态升级的门禁、源码身份与退出对账。
边界：不修改产品、ABI、生成器、预算或19/11/22债务；旧失败和旧报告保留原归属。

- [入口复核](preflight-20261103/README.md)：沿用固定源码身份的721条F10所需对象与摘要证明。
- [状态提交最终门禁](41498029910/README.md)：106b99c7的维护回归41498029910与工作区门禁41498029872、四精确点与完整日志。
- [后续自审](self-review-20261103/README.md)：原始归档正向审计及来源ID、缺失O3两个受控拒绝验证。

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
脚本只核对清单列出的文件，不代替完整Git树对照；本批完整树还应按以下只读命令复核：

```powershell
cmd /c git diff --exit-code 44dc9d6e 890cdb81 -- core/rust tests/fixtures .github/workflows tools/gates
```

当前入口不访问网络。线上结论仍以已归档run/job/attempt及对应GitHub源日志为依据。
