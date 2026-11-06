# A4 自审证据

工程期：20O-B2-A4-SELF-REVIEW。入口900c3df8，数值产品仍为1838fae1；只修改A4的audit.py及其清单，再添加自审工具/文档。历史运行、源码清单、原始日志和22C/22D不改。

## 实际结果

| 检查 | 记录 |
| --- | --- |
| 原审计复现 | [before-final-cases.json](before-final-cases.json)：正常1例通过，12个反例错误放行；不是说坏副本被验证正确 |
| 修正后 | [after-final-cases.json](after-final-cases.json)：正常1例通过，12个反例按指定原因拒绝；逐例日志核对实际SystemExit，普通Exception不算成功 |
| 真实A4归档 | [当前审计命令](review-a4-positive-audit.json)通过，989项产品输入和16项历史有效检查仍可复核 |
| 独立数值对照 | [实际执行](review-conversion-differential.json)、[原始结果](review-conversion-differential.log)：6,046输入、12,092比较、0差异 |
| 来源 | [新构建](review-build-runtime.json)、[对照编译](review-probe-compile.json)、[产物摘要](probe-identity.json)、[输入摘要](vectors.json) |

before/after为开发期记录；带final的两份报告由相同最终脚本重跑，并绑定脚本SHA。所有反例只改任务目录的归档副本。没有通过重算原始日志、改产品预算或移除场景取得成功。

反例覆盖重复17/21名单、遗漏门禁/CI、错误规范摘要/文件数/命令源码摘要、平台run_id错误、旧作业号、归档外日志、守卫明细失败及嵌套evidence.json。原工具完整源码保留为[original-audit.py.txt](original-audit.py.txt)。

## 可复跑入口

在仓库根目录执行以下命令，副本和输出默认位于新的系统临时目录，不改原归档；结果目录由脚本输出：

```powershell
python -X utf8 docs/DevDocs/assets/20o-b2-a4-self-review/entry-20261106/audit-cases.py --audit docs/DevDocs/assets/20o-b2-a4/entry-20261106/audit.py --repository-root . --label replay --mode after
```

用original-audit.py.txt及--mode before可复现旧脚本放行；复用work-root时必须换label，禁止覆盖旧输出。普通A4归档仍使用原来的只读入口：

```powershell
python -X utf8 docs/DevDocs/assets/20o-b2-a4/entry-20261106/audit.py
```

数值对照使用[完整Rust源](conversion-probe.rs.txt)、[向量生成器](generate-vectors.py)、[原始向量](conversion-vectors.txt)和[运行脚本](run-probe.py)。还原到独立scripts/source/environment目录结构，以新target构建同一产品树；原文`-0`按F-8规范为lfloat零，非零负值下溢仍保留负零。
标准库f32解析直接作为binary32参考，不经过f64中转；测试过程未添加产品依赖。该对照是Windows本地Runtime证据，不是新语言三平台或性能验收。

[入口](bootstrap.json)、[环境](environment.json)、[run-step](run-step.py)记录实际cwd/PID/argv/起止/退出码和前后源码树；新的原始CRLF树与A4不同，989项UTF-8/LF规范树相同。TypeScript5.9.3实体复制在源码根内，避免归档外junction。
最终文档门禁见result.json的checks列表；原产品CI保持历史身份，自审修复提交的CI在获得真实结果后单独登记，不能把原1838fae1的绿色当作自审CI。

修复提交6ebdd5b1的[workspace CI 42207132845](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/42207132845)已通过；[run](ci-fix-run.json)和[jobs](ci-fix-jobs.json)绑定实际修复提交。本回执另行提交，不声称回填文档已经包含在该运行中。

本目录字节、前后反例和独立对照结果可只读复核：

```powershell
python -X utf8 docs/DevDocs/assets/20o-b2-a4-self-review/entry-20261106/verify.py
```
