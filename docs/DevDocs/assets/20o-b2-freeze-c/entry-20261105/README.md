# F-C依赖、资源与性能裁定记录

工程期：20O-B2-FREEZE-C。职责：按[本批规划](../../../20o-b2-freeze-c-resource-dependency-plan.md)收齐F-7依赖闭包/构建及F-3资源/工作量/性能规则。
产品基线8dfbc89b，实际工作入口35ccdb30；当前实际ABI1.18、生成器20、活动intrinsic最大32，cmath继续planned。

[五条答复原文](decision-receipts.json)已确认F-3三项资源/工作量/性能精确表与F-7两项依赖/构建策略；
F-C以30 confirmed、0 pending退出；后续总进度见[实时台账](../../20o-b2-freeze-close/entry-20261103/decision-ledger.json)。
F-D仍待单独签字，B2-A/B未开始；五项均有实际答复，不以预选或文档校验替代确认。

唯一精确契约见[冻结草案§四、§6.1、§6.2](../../../20o-b2-freeze-draft.md)。
[来源复核](dependency-review.json)绑定六个真实归档、上游清单快照、bigdecimal构建脚本和既有11份许可证摘要。
原始归档仍在记录的临时路径，复核使用上游URL和已存SHA-256；新增清单文本用于审阅，不是Cargo项目或新锁文件。
没有执行Cargo解析、修改产品Cargo文件或运行构建；Rust1.96.0与workspace声明1.85分别登记，不推断MSRV兼容。

[源码事实](source-facts.json)保存7个只读文件的内容摘要、匹配片段和行号，未改实际ABI/生成器、intrinsic或工作流。
[数学算例](formula-witnesses.json)使用Python标准整数/Fraction独立核对17个边界，包含表达式和结果规模；
xiao_product_executed与performance_measurement_performed均为false。这些算例不声称产品可接纳、实际错误路径通过或F10达标。
每个表达式可用Python整数复算；涉及超过4,300位的十进制输出时，仅在该复算进程使用sys.set_int_max_str_digits(0)，不改变系统或Xiao配置。
[资源/采样静态对账](resource-performance-audit.json)绑定冻结草案Git blob、七个接纳分节、性能表与五项答复，后续文档变化按该blob复核历史快照。
[检查记录](checks.json)与[摘要清单](evidence.json)保存本轮门禁和来源核对；不替代真实Cargo构建或性能测量。

## 8dfbc89b入口CI

[入口CI原始记录及采集命令](ci-baseline-8dfbc89b.json)保存maintenance 41880682303第1次尝试六作业全部成功、workspace 41880682319成功。
这证明入口维护回归和工作区门禁；不是F-C六包的Cargo解析/构建、高精度产品或新721条F10成绩。
b8462f82与59a82934的macOS容量取消仍在各自证据目录保留，入口绿色不核销独立19/11/22历史债务。

可用以下只读命令核对同一提交与实际作业：

```powershell
gh api repos/Programming-Language-Xiao/Xiao/actions/runs/41880682303
gh api "repos/Programming-Language-Xiao/Xiao/actions/runs/41880682303/attempts/1/jobs?per_page=100"
gh api repos/Programming-Language-Xiao/Xiao/actions/runs/41880682319
```
