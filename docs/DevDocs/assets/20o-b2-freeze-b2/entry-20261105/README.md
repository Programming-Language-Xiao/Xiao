# F-B2版本与产物矩阵裁定记录

工程期：20O-B2-FREEZE-B2。职责：按[本批规划](../../../20o-b2-freeze-b2-version-matrix-plan.md)收齐F-6版本、产物、缓存与C接口边界。
边界：仅修改文档与证据；当前实际ABI1.18、生成器20、活动intrinsic最大32，cmath继续planned。未生成或执行1.19产物。

入口为3a7654a9；[三条实际答复](decision-receipts.json)已确认F-6a/F-6b/F-6c，
[实时台账](../../20o-b2-freeze-close/entry-20261103/decision-ledger.json)为27 confirmed、3 pending，剩余F-3/F-7/F-12。
F-B3、F-C、B2-A/B未开始，本轮不采纳新的错误码、资源算法或依赖。

完整契约以[冻结草案§8.5](../../../20o-b2-freeze-draft.md)为准，[结构化矩阵](version-matrix.json)只记录后续实现目标：

- A的1.18/20仅内部验证，绑定实际源码、Runtime内容、缓存身份和输出路径，不发布或纳入最终兼容证据。
- 旧/新原始.xiaoc、.xar和原生元数据在跨越1.18/1.19精确边界时执行前拒绝；不改原件、不放宽版本、不新增迁移工具或格式/schema变更。
- B使用1.19/21、标准库接口1；缓存身份失配时重生成并链接；从B1源码在新版重跑20入口及721条F10，旧成绩仍为历史。
- 保留1.18的C符号、布局与调用约定；数值行为按已确认F-A升级，不承诺所有旧数值行为不变。

[源码核对](source-facts.json)保存8个只读文件的摘要、核对片段与行号；这是当前1.18/20源码事实，不是跨版本产物的真实拒绝测试。
[检查记录](checks.json)保存本轮文档门禁、状态对账及受保护源码命令，不能用来宣称1.19已实现或三平台产品已通过。

## b8462f82入口CI的明确登记

[CI摘要与来源命令](ci-baseline-b8462f82.json)保留：workspace 41847658578/作业126635303224成功；
maintenance 41847658731第1次尝试整体failure，macOS作业126635301882取消，其余五作业成功。
macOS的runner_id=0、没有步骤；[平台原始注释](macos-annotations-api.json)明确说明容量约束及多次获取托管runner失败。
注释中的multiple attempts指获取runner的尝试，不是另一个workflow run_attempt；本记录仍为attempt=1。

本轮按规划选择明确登记，没有触发重跑。该节点不标三平台全绿，macOS没有产品执行证据，也不把取消写成产品失败。
原始运行、作业、检查与注释API响应均归档并绑定摘要；整体failure、取消记录和19/11/22债务保持原归属。

可用以下只读入口重新核对同一API；实际采集命令及原始响应摘要仍以CI清单为准，不依赖临时采集脚本：

```powershell
gh api repos/Programming-Language-Xiao/Xiao/actions/runs/41847658731
gh api "repos/Programming-Language-Xiao/Xiao/actions/runs/41847658731/attempts/1/jobs?per_page=100"
gh api "repos/Programming-Language-Xiao/Xiao/check-runs/126635301882/annotations?per_page=100"
```
