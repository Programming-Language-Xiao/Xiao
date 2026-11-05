# 20O-B2-FREEZE-D. 冻结签字与收口规划

> **这是 F-D 的签字收口规划，不写产品函数、依赖锁、intrinsic、ABI、诊断表或测试实现。**
> 入口为`0a93f957`，产品基线为`8dfbc89b`；台账30 confirmed / 0 pending，F-D尚未开始。
> **30/0不等于联合冻结已经生效**；F-D必须由星崽实际答复签字，文档门禁和摘要校验不能代替。
> 本批只做前置修正、签字取证、状态切换和B2-A交接；B2-A/B产品实现仍未开工。

## 一、入口与事实

| 项 | 当前状态 |
| --- | --- |
| 代码入口 | `0a93f957`；已推送、工作区干净 |
| 产品基线 | `8dfbc89b`；ABI 1.18、生成器 20、活动 intrinsic 最大 32、`cmath` planned |
| 台账 | 30 confirmed / 0 pending；F-D为`not_started`；联合冻结未激活；产品未开工 |
| 最近CI | `0a93f957`的workspace `41953275349`成功；该提交只变更DevDocs及其assets，未命中maintenance的任何push.paths（包含UseDocs/library、core/rust等），因此未触发，不记为通过 |
| F-C复核 | [复核记录](assets/20o-b2-freeze-c/review-fix-20261105/README.md)已补104条构成、收口CI与UseDocs范围 |
| 前置修正 | F-C当前/历史清单已分开命名，CI时点措辞已修正；正文历史状态已整理，数值表/资源协议未变，签字准备见[F-D记录](assets/20o-b2-freeze-d/entry-20261105/README.md) |
| 版本边界 | 仍为ABI 1.18、生成器20；B2-B才升1.19/21 |

F-D不新增第31条裁定，也不重开F-1～F-12；它是30条精确契约的联合冻结签字闸门。
本页只规划签字和状态切换，不把“已确认”自动写成“已签字”。

## 二、F-D的三个签字对象

| 编号 | 需要星崽实际答复的内容 | 关键边界 |
| --- | --- | --- |
| F-Da | 是否批准30条已确认裁定作为B2联合冻结基线，并以当前草案/台账摘要为准 | 18条基础答复与12组精确项一致；无pending；不允许静默默认 |
| F-Db | 是否批准激活联合冻结，并授权下一阶段为B2-A实施规划 | B2-A仍保持ABI 1.18/生成器20、不新增intrinsic；ABI变更必须回冻结；B2-B才使用1.19/21 |
| F-Dc | 是否确认签字后的状态与证据边界 | `cmath`仍planned；B1 721条F10只作历史且B2-C完整重跑；22D/22C与19/11/22债务独立；签字不等于产品开工 |

三个对象都必须有实际答复、日期和来源记录；任一未答复、附条件或拒绝时，F-D保持未签字。

## 三、前置修正（D-A）

签字前先处理上一轮审核留下的两处证据口径，不改历史裁定：

1. [复核证据](assets/20o-b2-freeze-c/review-fix-20261105/evidence.json)原`reviewed_entry_evidence`指向复核后的入口清单，现已改名`current_entry_evidence`并注明当前时点，另补`historical_entry_evidence`的38d6120d Git身份。
   历史104条继续以`coverage-38d6120d.json`为准，不改原历史清单。
2. F-C文档的“最近CI”改为“F-C收口CI”，并记录`0a93f957`的workspace运行与maintenance未触发的路径原因；不把未运行写成通过。
3. 重新执行104条历史复跑、文档门禁和覆盖`docs/UseDocs`的产品边界检查；确认30/0、ABI/生成器/cmath状态未变。
4. 在提问前冻结签字基线摘要：草案正文（不含页首状态行）、台账、F-3/F-7答复原文及104条清单的摘要写入F-D证据；签字后正文不得静默替换。
5. 签字只允许改变`20o-b2-freeze-draft.md`的页首状态行；同时保存签字前正文摘要、签字后全文摘要和状态行diff，任何正文变化都必须重新签字。
6. F-C的`resource-performance-audit.json`保持其历史draft blob身份，不因签字后的状态行变更而重写；台账`review_document_sha256`改为签字后全文摘要，并另记已签正文摘要。

正文内F-C退出时“F-D未签字”的叙述须在提问前注明历史时点；基线固定后不再修改这些文字。
台账签字前全文及30项内容投影分别保存摘要；签字后只允许已登记的状态/来源字段变更，30项内容摘要必须一致，签字前后全文与差异均保留。

## 四、分批执行

### D-A：前置修正与基线冻结

1. 修正上一轮两处证据命名/时点问题；
2. 重跑104条复跑、docs/usedocs/layout、`git diff --check`和产品边界检查；
3. 记录30/0、ABI 1.18/20、intrinsic最大32、cmath planned的只读事实；
4. 生成F-D签字基线摘要，不改变任何裁定；
5. 仍不进入B2-A/B产品实现。

### D-B：实际签字提问

1. 向星崽逐项提出F-Da、F-Db、F-Dc；
2. 不得预选、暗示或把文档校验当成答复；
3. 保存问题原文、实际答复、日期、会话来源和问题项ID；
4. 附条件、部分确认或拒绝必须原样登记，不能改写成“已签字”；
5. 未取得三个对象的有效答复前，F-D保持`not_started`。

### D-C：状态切换与索引同步

签字成功后：

1. 台账`f_d_signature_status`改为`signed`，`joint_freeze_activated`改为`true`；
2. `product_implementation_started`保持`false`，B2-A只获得实施规划入口；
3. 只改冻结草案页首状态行，保存签字前正文摘要、签字后全文摘要和状态行diff；冻结收口规划、数值基础规划和索引同步更新；
4. 更新`cmath`教程的F-D状态说明，但仍标planned；
5. 不把`cmath`升为verified，不改registry、产品、Cargo、ABI或测试。

### D-D：证据与退出

1. 新建F-D证据目录，保存签字原文、基线摘要、门禁日志、CI运行、边界检查和哈希清单；
2. 记录实际CI：workspace与maintenance各自是否触发；不把路径过滤写成漏跑；
3. 复核产品边界无差异，历史104条/B1 721条/22D/22C/19/11/22保持独立；
4. 通过门禁后提交，形成B2-A实施规划的交接入口；
5. 不生成产品、不跑B2性能、不声称三平台产品成绩。

## 五、签字后的状态变化

| 项 | 签字前 | 签字后 |
| --- | --- | --- |
| 30条台账 | 30/0 | 30/0，不重开 |
| F-D | `not_started` | `signed` |
| 联合冻结 | 未激活 | 已激活 |
| 产品实现 | 未开工 | 仍未开工 |
| B2-A | 无入口 | 只获得实施规划入口 |
| `cmath` | planned | planned，不升verified |
| ABI/生成器 | 1.18/20 | 不变；B2-B才1.19/21 |
| B1 721条F10 | 历史证据 | 仍为历史，B2-C重跑 |
| 22D/22C、19/11/22 | 独立债务 | 仍独立 |

## 六、验收与证据

| 检查 | 期望 |
| --- | --- |
| 签字 | F-Da/F-Db/F-Dc均有实际答复、日期、来源；无预选或默示 |
| 台账 | 30 confirmed / 0 pending；`f_d_signature_status=signed`；`joint_freeze_activated=true`；`product_implementation_started=false` |
| 摘要 | 草案正文、台账、答复原文、104条清单的摘要与签字基线一致；签字后正文摘要不变，只有已登记的状态行diff |
| 覆盖 | 18条基础答复与12组精确项无冲突；无未登记的“待审/建议” |
| 历史 | 104条复跑仍通过；B1 721条、22D/22C、19/11/22保持历史/独立 |
| 文档 | 冻结草案、F-D规划、F-A/F-B/F-C、索引一致；docs/usedocs/layout通过 |
| 产品边界 | `core/rust`、`stdlib`、Cargo、intrinsic/ABI、诊断表、测试、工作流无差异 |
| CI | 记录实际workspace/maintenance运行；未触发时写明路径原因 |
| 证据 | 保存实际命令、输出、运行号、哈希；不伪造产品/性能/三平台成绩 |

## 七、退出条件

1. F-Da、F-Db、F-Dc全部有有效实际答复；
2. 台账30/0，F-D已签字，联合冻结已激活，产品仍未开工；
3. 签字基线摘要与证据一致，无静默替换；签字前后正文摘要一致，只有已登记的状态行变化；
4. 文档门禁通过，产品边界无差异；
5. `cmath`仍planned，B2-A/B/C边界未变；
6. B1 721条、22D/22C、19/11/22不被核销；
7. B2-A只获得实施规划入口，B2-A代码和B2-B/C仍须各自规划与验收。

## 八、风险与禁止

- 把30/0或文档门禁通过当成F-D签字；
- 把预选、默认或规划文本当成星崽的实际答复；
- 签字后顺手开始B2-A产品实现；
- 把`cmath`升为verified或提前改registry；
- 改ABI/生成器/intrinsic而不回B2-FREEZE；
- 重写历史104条、B1 721条或22D/22C证据；
- 把maintenance未触发写成漏跑，或把入口CI写成F-D产品成绩；
- 修改产品、Cargo、诊断表、测试或工作流。

## 九、交接

新Agent先读：

1. [B2冻结收口规划](20o-b2-freeze-closeout-plan.md)；
2. [B2冻结草案](20o-b2-freeze-draft.md)§二～§九；
3. [F-C复核记录](assets/20o-b2-freeze-c/review-fix-20261105/README.md)；
4. [30条决策台账](assets/20o-b2-freeze-close/entry-20261103/decision-ledger.json)；
5. [B2数值基础规划](20o-b2-numeric-foundation-plan.md)§九。

## 十、落点

```text
docs/DevDocs/20o-b2-freeze-d-signature-plan.md          本规划
docs/DevDocs/20o-b2-freeze-closeout-plan.md             F-A/F-B/F-C/F-D总规划
docs/DevDocs/20o-b2-freeze-draft.md                     精确契约
docs/DevDocs/20o-b2-numeric-foundation-plan.md          B2批次边界
docs/DevDocs/assets/20o-b2-freeze-c/review-fix-20261105/ F-D前置复核
docs/DevDocs/assets/20o-b2-freeze-d/entry-20261105/     F-D签字证据（待建）
```

F-D签字完成后的下一入口是B2-A实施规划，不是B2-A产品代码；B2-B/C仍按原边界后置。
