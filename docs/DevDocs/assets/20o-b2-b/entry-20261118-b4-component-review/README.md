# FIX.2审计反例与口径复核

本批补充FIX.2的证据拒绝验证，并同步B6/B7的Windows限定与生成器22状态。固定产品仍为`43f18274175ba09ac84e759999c40cde72eae1c5`；不重跑产品或性能，不更改B6-1历史结论。

`self_negative.py`调用相邻FIX.2目录的真实`audit.py`，在临时副本中验证：基线通过；删除`.gitattributes`、删除已登记文件、改文件数、改清单摘要、伪造B6状态、把Windows验证等级伪造为`Complete`、改文件后重算清单均拒绝。后面三项重算临时清单，分别验证语义拒绝和固定Git对象拒绝。每项检查退出码、结构化实际错误全文、标准错误为空，以及审计前后证据字节不变；任何无关异常都不算预期拒绝。

封存后执行并记录：

```powershell
python -B -X utf8 docs/DevDocs/assets/20o-b2-b/entry-20261118-b4-component-review/self_negative.py --record
```

不加`--record`只打印本次验证结果。记录模式只新建`negative-cases.json`和本目录`evidence.json`，不覆盖已有回执。报告包含实际审计命令、PID、UTC时间、时长、标准输出/错误、固定清单提交和审计脚本摘要；清单按本目录实际字节生成，包含`.gitattributes`。

FIX.2最终docs门禁绑定本README、脚本及属性文件；反例报告和补充清单在封存提交后的审计中产生，单独归档，不宣称由先前门禁读取。主证据目录的清单不包含本目录的后置运行结果，避免报告与自身摘要递归绑定。Windows仍为COFF `UnverifiedCoffExports`：功能通过、导出完整性未验证；Linux ELF `Complete`的完整组件校验与之分开记录。
