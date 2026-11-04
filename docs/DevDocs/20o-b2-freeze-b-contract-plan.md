# 20O-B2-FREEZE-B. 原语、ABI、版本与错误裁定规划

> **这是 F-B 的裁定规划，不写产品函数、依赖锁、intrinsic、ABI 或测试实现。**
> 原规划入口为`bcfa9631`；F-4已答复采用推荐类型边界，F-A七项确认，台账25 confirmed/5 pending。
> **F-A退出检查已通过，F-B可开始文档裁定**；本页的候选表不会自动变为裁定。
>
> `cmath` 仍为 planned，Runtime ABI 仍为 1.18、`CODEGEN_VERSION` 仍为 20；本阶段不改活动表。

## 一、入口与前置

| 项 | 当前状态 |
| --- | --- |
| 代码基线 | `bcfa9631`；已推送 |
| F-A | 七项完整确认；混合类型按既有提升进入lfloat，退出检查见F-A证据 |
| 决策台账 | 25 confirmed/5 pending（F-3、F-5、F-6、F-7、F-12） |
| F-B 范围 | F-5、F-6、F-12 |
| 版本 | Runtime ABI 1.18、生成器 20、标准库接口版本 1 |
| 产品状态 | 无 `cmath`/高精度产品、无新 intrinsic、无 ABI 变化 |
| 入口远端CI | bcfa9631的[41777990988](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/41777990988)第1次尝试成功，实际作业[126402709805](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/41777990988/job/126402709805)；不是F-B产品或裁定证据 |

### 1.1 开工闸门

F-B 开工前必须：

1. F-4 混合类型准入已答复；
2. F-A 七项全部 confirmed，台账达到 **25/5**；
3. F-A 文档门禁与证据通过；
4. 不把 F-A 未确认类型边界带入 F-5/F-6/F-12；
5. 仍不进入 B2-A/B 产品实现。

## 二、F-B 的三个裁定对象

| 编号 | 必须冻结的内容 | 关键边界 |
| --- | --- | --- |
| F-5 | `lfloat_divide` 与 15 项 `cmath` 的参数名、规划 ID 33–48、效果、能力、C ABI 符号/调用约定 | 只做规划表；不写活动 `intrinsics.json`；不改 ABI 符号 |
| F-6 | B2-A 1.18/20 中间快照、B2-B 1.19/21、旧 `.xiaoc`/`.xar`/原生元数据/缓存处理 | 明确拒绝还是重新生成；不得让旧原件静默以新语义执行 |
| F-12 | 错误 033/034（若保留）、类别、触发条件、优先级，以及与 009/011/013 的边界 | 错误身份必须和数学域、资源、分配失败分开 |

## 三、F-B 分批执行

### F-B1：原语与 C ABI（F-5）

1. 冻结 16 个规划入口：`lfloat_divide` + 15 个 `cmath`；
2. 逐项冻结参数名、类型、返回、效果、能力和调用规则；
3. 冻结 C ABI 符号、输入借用/强引用、out 槽所有权、失败状态和挂起错误；
4. 确认 `cmath` 公开包装只导出 15 项，底层原语按既有 `os_` 模式登记，不新增私有可见性语法；
5. 确认 `Pure` 不等于 `readnone`/无错；失败调用不能被优化器因结果未使用删除；
6. **不写活动 intrinsic 表，不分配已占用 ID，不改 ABI 版本**。

退出：F-5 有唯一参数/ID/符号/状态表；没有未命名符号、未定 ID 或未定所有权。

### F-B2：版本与产物矩阵（F-6）

1. 按源码事实确认 `.xiaoc` 头、`.xar` 索引和指令流都记录/校验 ABI 精确范围；
2. 冻结 B2-A 1.18/20 仅作内部验证快照；
3. 冻结 B2-B 1.19/21 的旧原件拒绝、重新生成和原生缓存失效行为；
4. 明确旧 1.18 C 符号/布局是否保留，以及新 C 符号的加法兼容；
5. 记录 `.xiaoc`、`.xar`、原生元数据和缓存的拒绝/重生成矩阵；
6. **不实际生成 B2 产物，不修改版本常量或工作流**。

退出：F-6 的四种对象（A 中间、B1 旧原件、B2 新原件、原生缓存）各有明确行为；不存在“兼容重放”占位。

### F-B3：错误精表（F-12）

1. 决定是否保留规划编号 033（资源）与 034（非终止除法）；
2. 逐项冻结类别、触发条件、优先级和与 009/011/013 的边界；
3. 明确 033 不等于实际分配失败 011，034 不等于除零 013；
4. 冻结文本预算、precision/rounding 参数域、输入合法性、除零/域、工作/结果预算、输出分配的顺序；
5. 明确这些错误在 F-5 的 C ABI 状态和 out 槽行为；
6. **不写活动诊断表，不改现有 B1 七库错误表**。

退出：F-12 的全场景错误表与 F-5 状态表、F-1/F-2/F-4/F-8 的语义一致。

### F-B4：F-B 收口

1. 核对 F-5、F-6、F-12 全部 confirmed；
2. 台账从25/5变为**28/2**，剩余F-3、F-7留给F-C；总数仍为30条；
3. 更新冻结草案、F-A/F-B 规划和索引；
4. 通过 docs/usedocs/layout 与 `git diff --check`；
5. 不修改产品、依赖、ABI、测试或工作流；F-C 仍未开始。

## 四、需要星崽确认的规则

| 编号 | 需要确认 | 当前建议（未生效） |
| --- | --- | --- |
| F-5a | 15项cmath的参数名/调用规则及与lfloat_divide对账 | lfloat_divide已确认的四参数源级签名不重问；cmath采用候选参数名，位置/命名调用、无默认参数 |
| F-5b | 规划 ID 33–48 与 C ABI 符号 | 33=`lfloat_divide`，34–48=`cmath`；统一借读输入、拥有 out |
| F-5c | ABI 状态/所有权 | 0 成功、1/2/3 结构/句柄错误、6 可恢复语义错误、8/Fatal 不可恢复 |
| F-6a | 旧 1.18 `.xiaoc`/`.xar` 在 1.19 下 | 执行前拒绝，原件不改写；从 B1 源码重新生成 |
| F-6b | B2-A 中间 1.18/20 产物 | 仅内部验证，不发布；不假定旧 Runtime 具备新语义 |
| F-6c | 原生对象缓存 | 生成器 21 身份不匹配，重新生成并链接 |
| F-12a | 033/034 是否保留 | 保留规划编号，不占活动表 |
| F-12b | 错误优先级 | 调用/类型 → 参数域 → 输入预算/合法性 → 除零/域 → 工作/结果预算 → 受控输出分配 |
| F-12c | 033/034 与 009/011/013 | 033 资源≠011 分配；034 非终止≠013 除零；域=009 |

这些是建议，不是已生效规则；必须逐项收到星崽答复，才更新 ledger。

## 五、验收与证据

| 检查 | 期望 |
| --- | --- |
| 前置 | F-A 已退出、25/5；F-4 类型边界已确认 |
| F-5 | 16 项参数/ID/效果/C ABI/所有权唯一；活动表未改 |
| F-6 | 旧/新 `.xiaoc`、`.xar`、原生缓存四类行为明确；版本常量未改 |
| F-12 | 新错误与旧错误优先级一致；活动诊断表未改 |
| 台账 | 25/5 → 28/2；F-5/F-6/F-12不再pending，剩余恰为F-3/F-7两项 |
| 文档 | 冻结草案、F-A/F-B 规划、索引一致；docs/usedocs/layout 通过 |
| 产品边界 | `core/rust`、`stdlib`、Cargo 文件、intrinsic/ABI、测试、工作流无差异 |
| 证据 | 保存实际答复、命令和摘要；不伪造产品/三平台成绩 |

## 六、退出条件

1. F-5、F-6、F-12 全部 confirmed；
2. 原语/C ABI/版本矩阵/错误优先级有唯一精确表；
3. 台账为28/2，剩余F-3、F-7归F-C；
4. 活动 intrinsic/ABI/诊断表未改；
5. 文档门禁通过，产品边界无差异；
6. F-C 仍未开始；B2-A/B 仍未开工；22D/22C、19/11/22 债务保持独立。

## 七、交接

新 Agent 先读：

1. [B2 冻结收口规划](20o-b2-freeze-closeout-plan.md)；
2. [F-A 裁定规划](20o-b2-freeze-a-numeric-semantics-plan.md) 与 [F-A 收口](20o-b2-freeze-a-closeout-plan.md)；
3. [B2 冻结草案](20o-b2-freeze-draft.md) §四～§九；
4. [30 条决策台账](assets/20o-b2-freeze-close/entry-20261103/decision-ledger.json)；
5. [F-A 证据](assets/20o-b2-freeze-a/entry-20261104/README.md) 与 [依赖归档](assets/20o-b2-freeze-close/entry-20261103/dependency-archives.json)。

不得：

- 在 F-A 25/5 前开始 F-B；
- 把 F-5 规划 ID 写进活动 `intrinsics.json` 或提前改 ABI；
- 把 F-6 的“拒绝旧原件”与“兼容重放”混写；
- 把 033/034 当成活动诊断或把 034 混同 013；
- 写 `lfloat_divide`/`cmath` 产品函数或把 `cmath` 升为 verified；
- 把 B1 历史 721 条 F10 写成 B2 当前性能。

## 八、落点

```text
docs/DevDocs/20o-b2-freeze-b-contract-plan.md  本规划
docs/DevDocs/20o-b2-freeze-closeout-plan.md    F-A/F-B/F-C/F-D
docs/DevDocs/20o-b2-freeze-a-closeout-plan.md  F-A 收口
docs/DevDocs/20o-b2-freeze-draft.md            精确契约
docs/DevDocs/assets/20o-b2-freeze-a/entry-20261104/ F-A 答复
docs/DevDocs/assets/20o-b2-freeze-close/entry-20261103/decision-ledger.json 30条台账
```

F-B 收口后下一入口是 F-C：依赖闭包、资源、性能与测量；仍不是 B2-A 代码。
