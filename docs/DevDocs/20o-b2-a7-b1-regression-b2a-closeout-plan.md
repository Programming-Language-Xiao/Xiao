# 20O-B2-A7. B1回归与B2-A整批收口规划

> **这是 A7 的文档/回归收口规划；不写产品代码、不新增数值能力或改变ABI。**
> 入口为A6代码快照`e7bf7a49`及A6有效收口；A7已按A-P1～A-P3完成内部1.18/20文档/回归收口，实际结论见[A7收口记录](20o-b2-a7-b2a-closeout.md)，B2-B/C后置。
> A7只核对B1功能回归、A1～A6证据、语言说明/教程/registry边界和B2-A退出；不实现`lfloat_divide`/`cmath`。
> A0的`result.json`失败身份仍保留；A0只以`closeout.json`作为有效收口。

## 一、入口与范围

| 项 | 当前状态 |
| --- | --- |
| A0～A6 | 基线、依赖/身份、算术、转换、语言全链路、资源/性能均已退出 |
| A6代码快照 | `e7bf7a49ba3ed55679b69c97ac1bd5ce7f5777b6` |
| 版本 | ABI 1.18、`CODEGEN_VERSION` 20、活动intrinsic最大32、标准库接口1 |
| 状态 | `cmath` planned；B2-A整批未收口；B2-B/C未开始 |
| B1 | 七库20/20与教程/registry verified；旧721条F10只作历史 |
| 历史债务 | 22C/22D、19/11/22独立，不由A7核销 |
| 后续 | A7收口；B2-B再规划1.19/21与cmath；B2-C最终721/教程/registry |

A7是**B2-A内部1.18/20的收口**，不是B2最终发布。不得把A7写成B2-B/C完成、ABI1.19/21完成、cmath交付或721条当前成绩。

### 1.1 要做

1. 执行B1七库20入口功能回归，保持B1产品源码/错误表/预算不变；
2. 核对A1～A6的证据、源码快照、CI和平台边界；
3. 更新语言参考中lint/lfloat的B2-A内部状态，`cmath`仍planned；
4. 按实际证据决定新增教程/registry状态，只覆盖B2-A已验证范围；
5. 记录B2-A合法产品改动与版本/ABI/intrinsic边界；
6. 记录实际CI、路径过滤未触发和独立债务；
7. 形成B2-B规划入口，B2-B/C仍未开工。

### 1.2 明确不做

- 不运行或改写B1旧721条F10；B2-C才在新快照完整重跑；
- 不实现`lfloat_divide`、`cmath`或B2-B的intrinsic/ABI1.19/21；
- 不新增ABI/intrinsic、不改C ABI符号/布局/状态、XCON或依赖锁；
- 不把B1历史成绩、A6性能或A5语言证据拼成B2-C最终成绩；
- 不关闭22C/22D、19/11/22或B1历史债务；
- 不用“B2-A内部验证”冒充最终发布/三平台公共产品。

## 二、开工闸门

A7执行前必须确认：

1. A6有效收口、代码快照和性能证据一致；A7未开始；
2. A1～A6的证据和B1功能基线可复现；
3. 当前ABI1.18/生成器20/intrinsic最大32/cmath planned未变；
4. B1产品源码/错误表/预算未被A7修改；
5. 语言参考/教程/registry的现有verified范围不被A7自动扩大；
6. 旧721条F10仍标历史，22C/22D及19/11/22债务仍独立；
7. 若需改ABI/intrinsic/cmath/721或公共发布范围，停止并交B2-B/C规划。

## 三、分批执行

### A7-1：B1功能回归

1. 执行B1七库20入口功能矩阵，覆盖std/stdlib双路径、导出/移除、真实归档、普通包隔离和教程一致性；
2. 执行B1资源17项、VM精确1项、原生精确1项和28/28守卫；
3. 保持B1产品源码、错误表、预算、调度/释放断言不变；
4. 旧721条F10只作历史，不启用`b1_f10`；
5. 失败样本保留原归属；22C/22D/19/11/22不因全绿关闭。

退出：B1七库20入口功能回归通过；没有把旧721条写成A7/B2-A成绩。

### A7-2：A1～A6证据对账

1. 核对A1依赖/身份、A2/A3算术、A4转换、A5全链路、A6资源/性能的代码快照与有效收口；
2. 核对A6三平台性能与A5三平台语言证据的适用范围，不合并成B2-C最终成绩；
3. 核对A6合法产品改动与ABI/intrinsic/registry/cmath边界；
4. 核对A0原始失败结果仍保留、未改写；
5. 保存源码、锁、Runtime、工具链、目标、PID和输出路径。

退出：A1～A6证据链可对账；没有跨阶段覆盖或冒充。

### A7-3：语言说明、教程与registry边界

1. 更新语言参考中lint/lfloat的B2-A内部能力、错误/资源/转换边界；
2. `cmath`、`lfloat_divide`和B2-B/C仍标planned/未交付；
3. 新增数值教程/示例只升到B2-A已验证范围；不得把A7内部1.18/20写成最终公共发布；
4. registry只在有实际模块身份和证据时更新；不得新增未交付模块或提前verified；
5. 页面既有verified范围不自动扩展到B2-A新增段落；
6. B2-C再按1.19/21最终快照复核教程/registry和721条。

退出：语言说明/教程/registry状态与实际证据一致；cmath仍planned。

### A7-4：CI、门禁与证据

1. 运行docs/usedocs/layout、格式、B1回归、A1～A6相关包和workspace门禁；
2. 记录实际CI run/job/attempt；push路径过滤未触发按事实写明；
3. 记录三平台实际执行或未验证平台；
4. 保存失败样本、历史22C/22D和19/11/22归属；
5. 生成B2-A收口证据和B2-B交接入口。

退出：A7门禁与证据完整；B2-B/C未开始。

## 四、证据矩阵

| 项 | 必存证据 |
| --- | --- |
| B1功能 | 七库20入口、双路径、导出/移除/归档/普通包/教程；实际命令和日志 |
| B1资源/精确 | 17项、VM/原生精确各1、28/28守卫 |
| A1～A6 | 各阶段有效收口、代码快照、阶段证据及A0失败身份 |
| 语言说明 | lint/lfloat内部状态、cmath/lfloat_divide仍planned |
| 教程/registry | 只含B2-A已验证范围；未提升未交付模块 |
| 版本/ABI | 1.18/20、intrinsic最大32、C符号/状态/XCON未变 |
| CI | workspace/维护实际run/job/attempt；路径过滤和未验证平台标明 |
| 历史 | 721条F10历史、22C/22D、19/11/22独立 |
| 边界 | 无B2-B/C产品、无cmath、无ABI1.19/21 |

## 五、退出条件

1. B1七库20入口功能回归通过；旧721条F10未运行；
2. A1～A6证据链、代码快照和有效收口一致；
3. 语言说明/教程/registry只提升B2-A已验证范围，cmath仍planned；
4. ABI1.18/生成器20/intrinsic最大32/C符号与XCON未变；
5. B2-A合法产品改动已登记，未写成“产品源码无差异”；
6. 22C/22D、19/11/22及B1历史债务保持独立；
7. A7证据完整；B2-B/C仍未开工；
8. 下一入口是B2-B规划（1.19/21、lfloat_divide、cmath），不是B2-C最终收口。

## 六、风险与禁止

- 用A5/A6证据冒充B2-C最终721/发布；
- 把A7内部1.18/20写成B2-B/C完成；
- 把新增教程/registry提前升verified；
- 运行旧721条F10或改写B1历史；
- 关闭22C/22D、19/11/22债务；
- 为收口修改B1产品、预算、断言或工作流阈值；
- 把A6性能或A5语言证据拼成B2-C最终成绩。

## 七、交接

A7完成后B2-A整批可在1.18/20内部范围收口；下一入口是[B2-B实施规划](20o-b2-b-implementation-plan.md)：`lfloat_divide`、15项`cmath`、intrinsic 33～48和ABI1.19/生成器21。B2-C再做最终三平台、完整721条和教程/registry收口。

新Agent先读：

1. [B2-A实施规划](20o-b2-a-implementation-plan.md)；
2. [A6资源/性能记录](20o-b2-a6-resource-performance-progress.md)与[A6自审](20o-b2-a6-self-review.md)；
3. [A5语言全链路记录](20o-b2-a5-language-integration-progress.md)；
4. [B1收口记录](20o-b1-closeout.md)；
5. [冻结草案](20o-b2-freeze-draft.md)§五～§九。

## 八、落点

```text
docs/DevDocs/20o-b2-a7-b1-regression-b2a-closeout-plan.md 本规划
docs/DevDocs/20o-b2-a-implementation-plan.md               A0～A7总规划
docs/UseDocs/language/basics/p1-expressions/operators.md   lint/lfloat语言说明
docs/UseDocs/language/basics/types/conversions.md          转换说明
docs/module-registry.json                                  只在有证据时更新
docs/DevDocs/assets/20o-b2-a7/entry-<date>/               A7证据（待建）
```

本页保留A7规划与入口时点要求；实际执行及B2-A内部退出结论另记。B2-B/C仍是后续批次。
