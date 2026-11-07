# A6第二轮自审证据

入口`36ec83ef`，产品继续固定`e7bf7a49ba3ed55679b69c97ac1bd5ce7f5777b6`。本轮复查实现与归档审计；改动只涉及审计器、反例与复跑说明，没有重跑或改写历史性能样本。

## 复现与修复

| 问题 | 旧版复现 | 修复后的校验 |
| --- | --- | --- |
| 默认Windows/Linux实际作业互换仍通过 | before/swapped-default-platforms | 按角色核对实际作业名称，再核对原始执行来源 |
| 维护回归可冒充工作区门禁 | before/maintenance-as-workspace | 核对工作流名称、文件路径和触发方式 |
| 删除字节码/原生可执行文件的摘要仍通过 | before/missing-executable-hashes | 要求后端对应的完整且唯一产物集合，并校验文件字节 |
| 原始进程可改指向其他可执行文件 | before/foreign-process-executable | 关联实际可执行文件、参数、优化级别、输出路径和启动时间 |
| 自造合规增长次数、重算中位数仍通过 | before/fabricated-repeat | 从原始时长恢复schema1控制器的实际次数序列 |
| README反例命令写入原归档，破坏后续清单 | 输出路径检查 | 输出必须在原归档之外；新命令使用独占临时目录 |

`before-tools.json`及before-*保留旧工具字节与摘要，`before/`记录误接受的真实输出。`after/`以同一反例核对修订版的实际错误全文。每条结论都包含退出码和stdout/stderr，未用源码文本搜索充当拒绝证据。

## 实际验证

`run-check.py`逐条保存argv、cwd、PID、退出码和原始输出，并在执行前后确认产品输入相对固定快照无差异。`historical-bytes.json`逐项核对原归档732个文件，只允许四个工具及README发生变更；原始样本、ZIP、CI/历史日志和来源清单保持原字节。

- 六份历史报告继续通过新校验：本机VM77/原生154，CI VM77和三平台各154，全部五批原始样本重算。
- 同一组4项缺口反例已按指定原因被拒绝；重复次数反例纳入11项报告负例。
- 另有5项原始进程字段反例及1项归档内输出拒绝。
- 完整归档复跑包含1个正例与9个反例；输出写在本目录，原归档清单保持不变。

实际文档门禁已通过，完整归档正例与9个反例均通过；所有命令与stdout/stderr保存于本目录。本轮提交CI在运行后单独登记。原始`entry-20261107`仍使用自己的evidence.json；本目录清单独立计算，不把新日志倒写进首次自审的结果。

第二轮修复提交`bbf9feaa76e1fd5083bf7924a363a31baf4c69a9`的独立工作区CI`42340653748`已成功；`ci-completion.json`保存实际运行、作业日志和产品输入前后摘要。该CI只验证审计器与文档改动。

## 复跑

在仓库根执行原归档只读审计：

```powershell
python -X utf8 docs/DevDocs/assets/20o-b2-a6/entry-20261107/audit.py
```

复跑完整正反例时使用原归档之外的新目录：

```powershell
$a6ReviewOutput = Join-Path ([System.IO.Path]::GetTempPath()) ("xiao-a6-audit-" + [guid]::NewGuid().ToString("N"))
python -X utf8 docs/DevDocs/assets/20o-b2-a6/entry-20261107/validate-audit.py --output $a6ReviewOutput
```

版本、预算和状态不变：ABI1.18、生成器20、intrinsic最大32；A7未开始，新增教程/cmath继续planned。22C/22D及19/11/22债务独立保留，旧721条F10未运行。
