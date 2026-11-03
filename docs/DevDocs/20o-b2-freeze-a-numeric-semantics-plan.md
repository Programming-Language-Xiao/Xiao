# 20O-B2-FREEZE-A. 数值语义裁定规划

> **这是 B2-FREEZE-A 的裁定规划，不写产品函数、依赖锁、intrinsic、ABI 或测试实现。**
> 基线为 `67944184`；B2-FREEZE 仍有 18 条已确认答复和 12 组待审项，`cmath` 仍为 planned，
> Runtime ABI 为 1.18、`CODEGEN_VERSION` 为 20。
>
> **下一步先收口 F-A 七组数值语义，不进入 B2-A/B 产品实现。** F-A 只把冻结草案里的
> “建议/待审”变成星崽确认的精确规则；F-B/F-C/F-D 之后另行收口。

## 一、入口与当前事实

| 项 | 当前状态 |
| --- | --- |
| 代码基线 | `67944184`；工作区干净、已推送 |
| B2 决策台账 | [30 条](assets/20o-b2-freeze-close/entry-20261103/decision-ledger.json)：18 confirmed、12 pending |
| 本批范围 | F-1、F-2、F-4、F-8、F-9、F-10、F-11 七组 |
| 冻结状态 | 未完成；`cmath` planned；无产品、依赖、intrinsic 或 ABI 实施 |
| 最近 CI | `41667290451` workspace gates 成功；本轮只改 DevDocs，维护回归未触发 |
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

### 2.1 建议起点（未生效）

以下是冻结草案中的建议，不是已确认规则：

- F-1：保留 7 种舍入名称；`precision` 表示最多有效数字；短结果不补零；只在最终结果舍入一次。
- F-2：拒绝 NaN/Inf 正常输入/输出；`sqrt(-0)=-0`；`round` 最近偶数；`pow` 特殊值按冻结表；`tan` 不设 epsilon 伪极点。
- F-4：首批不开放 `lfloat //` 和 `%`；`**` 只接受非负整数值指数，负数/小数指数报数值域错误。
- F-8：`float/sfloat → lfloat` 保留实际二进制值；反向最近偶数；非有限/越界拒绝；允许正确的子正规数和带符号零。
- F-9：`sqrt` 0 ULP；取整/abs/min/max 逐位；普通超越与 `pow` 非零结果 ≤4 ULP；独立高精度参考。
- F-10：非零显示尺度保留；规范身份去尾零；零和加减乘除/幂的结果 scale 按冻结表。
- F-11：ASCII 十进制语法；`float/sfloat → str` 沿用有限值表示；不开放直接 `str → float/sfloat`。

完整候选、反例和理由见 [冻结草案 §五～§八](20o-b2-freeze-draft.md)。只有星崽逐项确认后
才把 `decision-ledger.json` 的 F 项改为 `confirmed`；未答复的项保持 `pending`。

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
