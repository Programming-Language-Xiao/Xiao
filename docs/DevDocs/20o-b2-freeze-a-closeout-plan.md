# 20O-B2-FREEZE-A-CLOSE. F-A 收口与混合类型补问规划

> **这是 F-A 的收口规划，不写产品函数、依赖锁、intrinsic、ABI 或测试实现。**
> 基线为 `b636f312`；F-A 已有 6 组完整确认，F-4 主体规则已确认，只有混合类型准入待答；
> 决策台账为 24 confirmed / 6 pending，`cmath` 仍为 planned，Runtime ABI 仍为 1.18、生成器 20。
>
> **本阶段只做 F-4 类型补问和 F-A 退出对账，不进入 F-B，更不进入 B2-A/B 产品实现。**

## 一、入口与事实

| 项 | 当前状态 |
| --- | --- |
| 代码基线 | `b636f312`；工作区干净、已推送 |
| F-A 精表 | F-1、F-2、F-8、F-9、F-10、F-11 已完整确认；F-4 主体规则已确认 |
| 唯一缺口 | `lfloat //` / `%` 的混合类型准入尚未选定 |
| 决策台账 | [24/6](assets/20o-b2-freeze-close/entry-20261103/decision-ledger.json)；目标 25/5 |
| 最近 CI | `41694599345` maintenance、`41694599210` workspace，均成功 |
| 文档门禁 | `check:docs`、`check:usedocs`、`check:layout`、`git diff --check` 通过 |
| 产品/ABI | 无产品、依赖、intrinsic 或 ABI 变化；`cmath` 仍 planned |

F-4 的数学与显示规则已经确认：首批同时开放 `//` 和 `%`，采用 floor 配对，返回 lfloat，
商 scale=0、非零余数 scale=max(s_a,s_b)，`**` 只接受非负整数值指数。当前只差
“哪些混合实参组合允许进入这两条 `lfloat` 路径”。

## 二、唯一待答问题

### F-4 混合类型准入

现有 `int`/`sint` 的 `//`/`%` 是整数族；现有浮点组合整体拒绝。新增 `lfloat` 分支后，三选一：

| 方案 | 规则 | 结果 |
| --- | --- | --- |
| A（建议） | 沿用既有数值提升：提升结果为 `lfloat` 就允许，例如 `lfloat // int`、`lint // float`、`lfloat % int`；两个整数仍返回整数族；只提升到 `float/sfloat` 的组合仍拒绝；`bool` 不新增转换 | 与既有提升矩阵一致，覆盖面最大 |
| B | 至少一个实参显式是 `lfloat` 才允许；例如 `lfloat // int` 允许，`lint // float` 不自动允许 | 更保守，避免隐式跨族 |
| C | 两个实参都必须是 `lfloat` | 最小公开面，但实用性最低 |

建议选 A；只有星崽实际答复后才把 F-4 写成 confirmed。不能把 B2 的“建议起点”直接当成裁定。

需要同时记录：

- `lfloat // int`、`lint // lfloat`、`lfloat % int` 等是否允许；
- `float/sfloat` 与 `lfloat` 混合时是否按既有提升进入 lfloat；
- 整数对整数是否继续走既有整数结果；
- 只含 `float/sfloat` 的组合是否继续拒绝；
- `bool` 是否继续不参与数值转换；
- 类型检查、Runtime 和教程的示例必须共用这套边界。

## 三、执行步骤

### A-1：收答复

1. 保存 F-4 混合类型答案及来源；
2. 把 F-4 从 `pending` 改为 `confirmed`，保持其余五项 pending；
3. 更新 `decision-receipts.json`/`decision-requests.json` 或对应收口台账；
4. 不把 A/B/C 的推荐项预选为回答。

### A-2：回写契约

1. 在冻结草案 F-4 类型表写明允许/拒绝组合；
2. 在 F-A 规划和收口规划更新状态；
3. 在 `cmath` planned 教程只同步实际确认的契约，不升级 verified；
4. 检查 F-4 数学规则、F-8 转换规则和类型边界没有冲突；
5. 不新增 intrinsic、ABI、产品或依赖改动。

### A-3：F-A 退出

1. 七组 F-A 全部 confirmed；
2. 台账变为 **25/5**，剩余 F-3、F-5、F-6、F-7、F-12；
3. 更新冻结草案、F-A 规划、收口规划和索引；
4. 通过 `check:docs`、`check:usedocs`、`check:layout`、`git diff --check`；
5. 保存决策来源、命令和摘要，F-B 仍未开始。

## 四、验收与证据

| 检查 | 期望 |
| --- | --- |
| F-4 答复 | 一个明确方案，不是“建议”或“实现时再看” |
| 类型边界 | 允许/拒绝组合与既有提升矩阵、F-8 转换和教程一致 |
| 台账 | 24/6 → 25/5；F-4 不再 pending |
| F-A | F-1/F-2/F-4/F-8/F-9/F-10/F-11 全部 confirmed |
| 文档 | 冻结草案、F-A 规划、收口规划、索引一致；文档门禁通过 |
| 产品边界 | `core/rust`、`stdlib`、Cargo 文件、intrinsic/ABI、测试、工作流无差异 |
| 证据 | 保存实际答复、命令、输出摘要和提交号；不伪造产品/三平台成绩 |

## 五、退出条件

1. F-4 混合类型准入有唯一裁定；
2. F-A 七组全部 confirmed，台账为 25/5；
3. 类型边界与既有提升/F-8/F-10 精表一致；
4. 文档门禁通过，证据可复跑；
5. 没有修改产品、依赖、ABI、测试或工作流；
6. F-B 仍未开始；B2-A/B 仍未开工；22D/22C、19/11/22 债务保持独立。

## 六、交接

新 Agent 先读：

1. [F-A 裁定规划](20o-b2-freeze-a-numeric-semantics-plan.md)；
2. [B2 冻结收口规划](20o-b2-freeze-closeout-plan.md)；
3. [B2 冻结草案](20o-b2-freeze-draft.md) F-4 段；
4. [F-A 实际答复](assets/20o-b2-freeze-a/entry-20261104/decision-receipts.json)；
5. [F-4 待答问题](assets/20o-b2-freeze-a/entry-20261104/decision-requests.json)。

不得：

- 把 A/B/C 任一建议直接写成已确认；
- 在 F-A 收口时新增 intrinsic、ABI 或产品函数；
- 把 `cmath` planned 升为 verified；
- 用 F-A 收口代替 B2-A/B 的实现或三平台证据；
- 把 B1 历史 721 条 F10 写成 B2 当前性能。

## 七、落点

```text
docs/DevDocs/20o-b2-freeze-a-closeout-plan.md  本规划
docs/DevDocs/20o-b2-freeze-a-numeric-semantics-plan.md F-A 计划
docs/DevDocs/20o-b2-freeze-closeout-plan.md    F-A/F-B/F-C/F-D
docs/DevDocs/20o-b2-freeze-draft.md            精确契约
docs/DevDocs/assets/20o-b2-freeze-a/entry-20261104/ 答复与门禁
docs/DevDocs/assets/20o-b2-freeze-close/entry-20261103/decision-ledger.json 30条台账
```

F-A 收口后下一入口是 F-B：原语、C ABI、版本矩阵与错误精表；仍不是 B2-A 代码。
