# A5第二轮自审：审计契约收紧

入口`3dd492d1`；A5产品固定`f21715d3`。本轮复核类型/IR、Runtime语言提升、VM/LLVM错误与临时值清理、完整示例和CI来源，未发现新的数值实现缺陷；修复的是证据审计的可复现漏检，不新增ABI/intrinsic，不启动A6/A7，不修改19/11/22及22C/22D债务。

## 发现与修复

旧审计只按checks.json提供的计数、精确入口和标记映射检查。这些表本身可删除或修改，令验证被跳过；构建命令若改为非Cargo程序，原先只检查Cargo命令的条件也会被跳过。单项本地日志附加第二份测试汇总同样未被拒绝。初轮[三个真实反例](counterexamples-before.json)均被旧工具误放行。

现由审计器固定17个执行点的角色、参数、版本/锁、8项计数与4个精确目标及2个标记组；格式检查输入按入口/产品Git差异核对完整Rust集合，工作目录须匹配被测源码。计数点要求唯一汇总，非精确目标还核对ignored数量。空命令、空路径在使用前明确拒绝，不再靠IndexError/TypeError退出。

[audit-before.py](audit-before.py)逐字保存入口3dd492d1中的旧工具，SHA-256为`cae56109aaccf8647e3d5006f02e25773faede49d349773cca793f79e742596d`。原A5目录self-review保留2026-11-06的9案与旧工具摘要，属于历史记录；本目录是修复后新的执行，不将旧9案冒充新工具已测。

## 实际验证

[14案对照](regression/results.json)对每个相同临时归档分别执行旧/新工具，包含真归档、缺失/改写计数表、缺失精确表/标记组、缺少--exact、空操作替代Runtime构建、cargo check替代build、仅docs子集、漏格式输入、重复汇总、空路径/命令和错误cwd。旧工具有10个负例被误放行；新工具14/14满足真实退出码与实际stderr的精确原因。

[原9案重跑](legacy-regression/negative-cases.json)也通过，涵盖标记丢失/重复、ignored、运行ID和复制映射ID替代原始作业ID、遗漏产品源码/Clippy/macOS。所有stdout/stderr分别保存，反例会更新日志与目录摘要，避免只因无关哈希错误退出。

两组实际输入清单为[input-evidence.json](input-evidence.json)，修复后工具SHA-256为`7d9628ee3eb72629b385e88072b129edf00a59697d9119f18583421488b62525`，与两份结果一致。原14/9案的完整临时路径、实际argv与摘要均在报告内；输入README原字节另存README-at-input.md.txt。

复跑（指定尚不存在的输出目录，不覆盖历史报告）：

```powershell
python -X utf8 docs/DevDocs/assets/20o-b2-a5/entry-20261106/audit.py
python -X utf8 docs/DevDocs/assets/20o-b2-a5/self-review-20261107/review.py --temporary-parent <已有临时父目录> --output-directory <新的外部输出目录>
```

[产品边界](boundary.json)覆盖完整产品/工具/UseDocs路径，差异为空，因此继续使用f21715d3已完成的三平台功能证据，不冒充重新运行数值产品或旧721条F10。入口文档回执3dd492d1的CI实际通过见[原始响应](entry-ci-3dd492d1.json)。本轮文档门禁与提交CI另行登记。
