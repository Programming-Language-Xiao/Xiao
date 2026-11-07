# A7回归与B2-A内部收口原件

本批已完成内部1.18/20收口。验证快照5749d6e0，产品仍为A6的e7bf7a49；B2-B/C未开始，最终数值教程/cmath planned，registry未改，旧721未跑。

bootstrap.json保存独立工作树与入口bf625d43。environment.json固定全新target、Runtime、Xiao缓存、临时目录和仓库stdlib，Rust1.96.0、Cargo --locked，清理RUST_BIGDECIMAL_*。run-step.py保存每条argv/cwd/PID、起止/退出码、原始日志和前后源码清单；执行中输入变化直接拒绝。

checks.json登记18项当前回归/工程门禁和7项历史原件审计。stage-and-boundary.json逐阶段绑定有效closeout与实际源码，单列A0原始失败与B2-A从A0至A6的128个合法改动路径；A7仅更新5个语言/fixture说明输入文档及DevDocs。local-artifacts.json保存本轮新Runtime、工具/目标和独占目录内实际原生产物摘要，没有借用A6可执行文件。

维护回归42348812742及工作区42348812756均是验证快照push实际触发，第一次尝试全绿。ci/records登记各平台原始作业ID、run/attempt和日志摘要；两种性能开关均为0。A6性能只作为已固定历史原件再次只读验证，不称A7新测量，不替代B2-C最终成绩。

独立债务见independent-debts.json，22C/22D原件仍closed=false，19/11/22归属不变。初轮UseDocs旧标题锚点失败及修复后的重跑分别保留；A0的result.json依然passed=false，有效结论只来自其closeout.json。

## 只读复跑

```powershell
python -X utf8 docs/DevDocs/assets/20o-b2-a7/entry-20261107/audit.py
```

audit.py对固定Git对象恢复完整被测输入，不要求未来HEAD的语言文档继续保持旧字节。它核对实际Cargo参数、计数/标记、真实20项导出、阶段原件、工作流/平台角色、性能开关关闭和Runtime/产物身份。

正反例只写归档之外的独占目录，不修改有效原件：

```powershell
$a7ReviewOutput = Join-Path ([System.IO.Path]::GetTempPath()) ("xiao-a7-audit-" + [guid]::NewGuid().ToString("N"))
python -X utf8 docs/DevDocs/assets/20o-b2-a7/entry-20261107/validate-audit.py --output $a7ReviewOutput
```

原始字节清单为evidence.json，不递归计自身；文件数以实际清单为准。原始日志保留CRLF/尾部空行，不能为样式检查修改测量/执行原件。归档正反例和后续文档提交CI分别登记，不用产品CI冒充文档收口提交。
