# B6-1重跑审计反例

本目录只保存主归档封存后的反例报告，脚本位于[重跑目录](../entry-20261118-b6-1-rerun/self_negative.py)。脚本在临时副本调用真实审计，验证基线通过以及七项拒绝：缺属性、删文件、错文件数、改清单摘要、伪造Windows完整性、冒进平台/阶段状态、同时改文件和重算清单。拒绝必须满足结构化错误全文、退出码、标准错误为空和只读检查；无关异常不计成功。

主归档提交后执行：

```powershell
python -B -X utf8 docs/DevDocs/assets/20o-b2-b/entry-20261118-b6-1-rerun/self_negative.py --record
```

不加`--record`只输出本次结果。记录模式只新建`negative-cases.json`和实际字节清单`evidence.json`，不覆盖报告，不写主归档。报告绑定主清单提交与摘要，以及审计/反例脚本摘要，并保存实际PID、命令、UTC时间、时长、标准输出与错误。

主归档最终docs门禁绑定本README和属性文件；报告在封存后产生，不宣称已被先前门禁读取。Windows始终只记功能通过、COFF导出完整性未验证；macOS缺项不被升级，B6-2未执行。
