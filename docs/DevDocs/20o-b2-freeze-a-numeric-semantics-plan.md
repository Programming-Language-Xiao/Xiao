# 20O-B2-FREEZE-A. 数值语义裁定规划

> **这是 B2-FREEZE-A 的裁定规划，不写产品函数、依赖锁、intrinsic、ABI 或测试实现。**
> 基线为 `67944184`；入口为18/12，F-A七项退出时台账为25/5；本页保留本阶段历史状态，后续总状态以实时台账为准。`cmath`仍planned，
> Runtime ABI 为 1.18、`CODEGEN_VERSION` 为 20。
>
> **F-A七项裁定与退出检查已完成，可交接F-B。** 本批只冻结规则，未交付数值产品或三平台产品证据。

## 一、入口与当前事实

| 项 | 当前状态 |
| --- | --- |
| 代码基线 | `67944184`；工作区干净、已推送 |
| B2 决策台账 | F-A退出时25/5；后续状态见[实时总台账](assets/20o-b2-freeze-close/entry-20261103/decision-ledger.json)，F-A七项全部confirmed |
| 本批范围 | F-1、F-2、F-4、F-8、F-9、F-10、F-11 七组 |
| 冻结状态 | 未完成；`cmath` planned；无产品、依赖、intrinsic 或 ABI 实施 |
| 入口 CI | 67944184的`41667290451`成功，实际作业126025900872；本轮门禁另记[收口证据](assets/20o-b2-freeze-a/entry-20261104/README.md) |
| 文档门禁 | `check:docs`、`check:usedocs`、`check:layout`、`git diff --check` 通过 |
| B1 | 历史 SourceRoot 复跑通过；默认当前 registry 拒绝为已解释状态 |

F-A 不依赖 F-B 的 ABI 编号或 F-C 的依赖闭包；但 F-A 的 rounding/scale 与 F-B 的
`lfloat_divide` 参数表、F-C 的资源公式必须保持同一语义，收口时要回写冻结草案而不是另立一套。

## 二、F-A 的七组裁定

| 编号 | 必须确定的规则 | 为什么必须在本批定 |
| --- | --- | --- |
| F-1 | 7 种 `rounding` 的精确语义、`precision` 有效数字、短结果不补零、单次最终舍入 | 决定 `lfloat_divide` 的数学结果和文本结果，不能留到 B2-B |
| F-2 | `cmath` 每函数定义域、负零、下溢、`round`/`pow`/`tan` 特殊值 | 决定 15 个公开函数的行为和错误边界 |
| F-4 | `lfloat` 的 `//`、`%`、`**` 是否首批开放，以及指数域 | 决定类型检查、Runtime 和教程的公开面 |
| F-8 | `float`/`sfloat` ↔ `lfloat` 的实际值保留、最近偶数、下溢、范围错误 | 决定跨数值族转换和跨后端结果 |
| F-9 | 每函数 ULP/绝对误差、独立参考向量、`cmath.pow` 与 `**` 共用内核 | 决定平台容差、黄金和两个后端的差分 |
| F-10 | 规范/显示 scale、输出格式、运算结果 scale、相等/哈希与预算 | 决定 `1.50`/`1.5`、零和显示尺度 |
| F-11 | `str → lfloat` 语法、`float/sfloat → str`、拒绝语法与往返 | 决定文本输入/输出和教程示例 |

### 2.1 已确认语义（2026-11-04）

以下七项均已收到直接答复，完整精表以冻结草案为唯一规则来源：

- F-1：保留 7 种舍入名称；`precision` 表示最多有效数字；短结果不补零；只在最终结果舍入一次。
- F-2：拒绝 NaN/Inf 正常输入/输出；`sqrt(-0)=-0`；`round` 最近偶数；`pow` 特殊值按冻结表；`tan` 不设 epsilon 伪极点。
- F-4：同时开放lfloat //和%，采用floor配对；按既有提升结果为lfloat即允许，整数对整数保留整数族，float/sfloat结果仍拒绝；商scale=0、非零余数取较大输入scale；幂只接受非负整数值指数。
- F-8：精确承接二进制实际值；反向最近偶数、舍入后非有限才拒绝；保留正确子正规/有符号零。新转换中的整数结果显示scale=0，lfloat零仍统一为0。
- F-9：sqrt 0 ULP，取整/abs/min/max逐位，普通超越与pow非零结果≤4 ULP；MPFR4.2.2/GMP6.3.0仅作离线区间参考，不能唯一确定时不生成黄金。
- F-10：非零显示尺度保留；规范身份去尾零；零和加减乘除/幂的结果 scale 按冻结表。
- F-11：ASCII 十进制语法；`float/sfloat → str` 沿用有限值表示；不开放直接 `str → float/sfloat`。

完整规则、反例和理由见[冻结草案§五～§八](20o-b2-freeze-draft.md)。七项答复及补充问题已原文保存，
F-A退出时仅据实际答复确认七项，其余五项当时pending；后续新增裁定与待审项按实时台账，不重写本阶段退出计数。

### 2.2 本轮裁定进度（2026-11-04）

F-A1、F-A2、F-A3均已确认，见[本轮记录](assets/20o-b2-freeze-a/entry-20261104/decision-receipts.json)。
F-4选择同时开放//和%、混合类型按既有提升；F-8选择舍入后有限性判溢出、整数结果显示scale=0。
已确认规则及设计算例回写[冻结草案](20o-b2-freeze-draft.md)，它们不是产品输出或新增测试实现。
本节保留F-A退出时点：25/5，退出门禁通过并交接F-B；后续F-B裁定另行登记，本节不表示当前总进度。

### 2.3 F-A退出对账

| 项 | 结果 |
| --- | --- |
| F-A1三组 | F-1/F-10/F-11确认，舍入/文本/显示及边界算例齐备 |
| F-A2两组 | F-2/F-9确认，15函数域、特殊值与误差/独立参考规则齐备 |
| F-A3两组 | F-4/F-8及混合类型补充均确认；允许/拒绝组合写入完整类型表 |
| 台账与证据 | F-A退出时25/5；12条实际答复、规则摘要与[门禁输出](assets/20o-b2-freeze-a/entry-20261104/README.md)入库；后续计数见实时总台账 |
| 产品边界 | 仅改文档并同步cmath planned教程契约；ABI1.18、生成器20；产品/依赖/测试/工作流无差异 |
| 下一步 | 退出检查通过后进入F-B；F-C仍须覆盖新增//与%的资源/性能验收 |

## 三、分批执行

### F-A1：舍入、显示与文本

范围：F-1、F-10、F-11。

1. 冻结 rounding/precision/短结果/最终舍入；
2. 冻结规范/显示 scale、零值、加减乘除/幂结果 scale；
3. 冻结 `str → lfloat` 语法、`float/sfloat → str` 和往返；
4. 用具体例子覆盖 `2.5`、`-2.5`、`9.99`、`1.50`、`0.00`、超大指数和预算边界；
5. 将三组规则回写冻结草案，更新 ledger 状态。

### F-A2：cmath 域与误差

范围：F-2、F-9。

1. 逐函数冻结定义域、负零、下溢、`round`/`pow`/`tan` 特殊值；
2. 冻结每函数 ULP/绝对误差、参考向量来源、零/符号/域精确核对；
3. 明确 `cmath.pow` 与 `float **` 共用数学内核，`sfloat` 不冒充已支持；
4. 记录平台误差边界和不可验证场景，不生成猜测黄金；
5. 将两组规则回写冻结草案，更新 ledger 状态。

### F-A3：`lfloat` 运算与二进制转换

范围：F-4、F-8。

1. 冻结 `lfloat //`、`%`、`**` 的首批开放范围；
2. 冻结负指数、非整数指数、零底数/零指数的错误；
3. 冻结 `float/sfloat ↔ lfloat` 的实际值、最近偶数、下溢、带符号零和范围错误；
4. 覆盖 `0.1`、子正规数、跨宽度比较和预算边界；
5. 将两组规则回写冻结草案，更新 ledger 状态。

### F-A4：F-A 收口

执行规划见[F-A 收口规划](20o-b2-freeze-a-closeout-plan.md)；当前只差 F-4 混合类型补问。

1. 核对 F-1、F-2、F-4、F-8、F-9、F-10、F-11 全部 confirmed；
2. 更新 `decision-ledger.json`：25 confirmed、剩余 5 pending（F-3、F-5、F-6、F-7、F-12）；
3. 更新冻结草案和本页状态，不修改活动 intrinsic/ABI/依赖；
4. 通过 docs/usedocs/layout 与 `git diff --check`；
5. 保存决策答复、命令和摘要；F-B 仍未开始。

## 四、验收与证据

| 检查 | 期望 |
| --- | --- |
| 七组决定 | 每组有唯一规则；没有“建议/实现时再看/上游默认”占位 |
| 语义一致 | F-1/F-10 与 `lfloat_divide` 参数、F-4/F-8 与类型检查、F-2/F-9 与 cmath 表互相一致 |
| 例子 | rounding、零、scale、文本、cmath 特殊值、转换边界都有可核对的输入/输出 |
| ledger | 25 confirmed、5 pending；F-A 七项不再 pending |
| 文档 | 冻结草案、收口规划、索引链接一致；docs/usedocs/layout 通过 |
| 证据 | 保存实际决策答复、命令、输出摘要和提交号；不伪造产品/三平台成绩 |
| 产品边界 | `core/rust`、`stdlib`、Cargo 文件、intrinsic/ABI、测试和工作流无差异 |

## 五、退出条件

1. F-A 七项全部 confirmed，且没有未说明的上游默认；
2. 舍入/scale/文本、cmath 域/误差、`lfloat` 运算/转换三组规则互相一致；
3. `decision-ledger` 从 18/12 变为 25/5；F-B/F-C/F-D 仍未完成；
4. 冻结草案、收口规划和索引同步；文档门禁通过；
5. 没有修改产品、依赖、intrinsic、ABI、测试或工作流；
6. B2-A/B 仍未开工；22D/22C、19/11/22 债务保持独立。

## 六、交接

新 Agent 先读：

1. [B2 冻结收口规划](20o-b2-freeze-closeout-plan.md)；
2. [B2 冻结草案](20o-b2-freeze-draft.md) §五～§八；
3. [30 条决策台账](assets/20o-b2-freeze-close/entry-20261103/decision-ledger.json)；
4. [依赖与 B1 取证](assets/20o-b2-freeze-close/entry-20261103/README.md)；
5. [B1 SourceRoot 入口](assets/20o-b1-close/README.md#只读复跑)。

不得：

- 把 F-A 的建议表直接标成 confirmed；
- 在 F-A 中新增 intrinsic、修改 ABI 或写 `lfloat_divide`/`cmath` 产品函数；
- 把 `cmath` planned 升为 verified；
- 用 F-A 收口代替 B2-A/B 的实现或三平台证据；
- 把 B1 历史 721 条 F10 写成 B2 当前性能。

## 七、落点

```text
docs/DevDocs/20o-b2-freeze-a-numeric-semantics-plan.md  本规划
docs/DevDocs/20o-b2-freeze-closeout-plan.md             F-A/F-B/F-C/F-D 总规划
docs/DevDocs/20o-b2-freeze-draft.md                     精确契约
docs/DevDocs/assets/20o-b2-freeze-close/entry-20261103/decision-ledger.json 30条台账
docs/DevDocs/README.md                                  阶段索引
docs/DevDocs/20-builtins-and-standard-library.md        阶段表
docs/DevDocs/20o-official-library-program.md            B2 批次链接
```

F-A 完成后下一入口是 F-B：原语、C ABI、版本矩阵与错误精表；仍不是 B2-A 代码。
