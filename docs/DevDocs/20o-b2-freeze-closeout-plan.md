# 20O-B2-FREEZE-CLOSE. 数值冻结收口规划

> **这是 B2-FREEZE 的收口规划，不写产品函数、依赖锁、intrinsic、ABI 或测试实现。**
> 基线为 `0011f4c5`；B1-CLOSE已完成；18条入口答复加六个完整F-A项，当前24 confirmed、6 pending；F-4待混合类型补充，
> `cmath` 仍为 planned，Runtime ABI 仍为 1.18、`CODEGEN_VERSION` 仍为 20。
>
> **F-A尚差F-4类型补充；收齐后才能进F-B，不进入B2-A/B产品实现。** 只有全部精确项变成
> 可执行的精确契约，才允许把冻结状态改为完成并开始 B2-A/B。

## 一、入口与当前事实

| 项 | 当前状态 |
| --- | --- |
| 代码基线 | `0011f4c5`；工作区干净、已推送 |
| B1-CLOSE | 七库 20/20、教程/registry verified；B1 旧 721 条 F10 已标历史 |
| B1 复跑 | 默认当前 registry 会拒绝；`audit.ps1`/`validate-audit.ps1` 的 `-SourceRoot` 106b99c7 入口已取证并通过 |
| B2-FREEZE | F-A六项完整确认，F-4混合类型待答，台账24/6；F-B/F-C/F-D未开始；cmath planned，无产品/依赖/ABI实施 |
| 最近 CI | `41644476014` workspace gates 成功；最新提交只改 DevDocs/审计脚本，维护回归未触发 |
| 文档门禁 | `check:docs`、`check:usedocs`、`check:layout`、`git diff --check` 通过 |

B1 的当前默认审计拒绝不是新缺陷：它是整份 registry 快照校验遇到 B2 planned 条目的预期结果；
B1 历史复跑必须按 [B1 SourceRoot 入口](assets/20o-b1-close/README.md#只读复跑) 执行。
本阶段不重开 B1，也不把历史 721 条写成当前 B2 性能。

## 二、本批范围

### 2.1 要做

1. 把 [冻结草案 §九](20o-b2-freeze-draft.md#九仍须逐项完成) 的 12 组待审项逐条变成精确契约；
2. 补齐依赖闭包、版本/许可证/校验值、构建环境和锁文件策略；
3. 冻结 `lint`/`lfloat` 的值身份、规范化、算术、转换、错误、资源和显示规则；
4. 冻结 `lfloat_divide` 与 `cmath` 的参数、ID、C ABI、效果、错误和版本边界；
5. 冻结性能目标、算力模型、测量协议和参考向量；
6. 通过文档门禁并形成 B2-FREEZE 完成记录，明确交接给 B2-A/B。

### 2.2 不做

- 不写 `stdlib/cmath.xiao` 或任何 B2 产品函数；
- 不修改 `Cargo.toml`、`Cargo.lock`、`intrinsics.json`、Runtime ABI 或生成器；
- 不把 `cmath`/新数值路径升为 `verified`；
- 不重跑或改写 B1 的 721 条 F10；B2-C 才按 D9 在新 Runtime/ABI 快照完整重跑；
- 不关闭 22D/22C 历史失败或 19/11/22 债务；
- 不用“待审/建议”文本冒充已冻结契约。

## 三、十二组待审项对账

| 编号 | 来源 | 本阶段必须产出 | 归属 |
| --- | --- | --- | --- |
| F-1 | D4 舍入/精度 | 7 种 `rounding` 的精确语义、有效数字规则、短结果不补零、单次最终舍入 | F-A |
| F-2 | D7 cmath 域 | 每个函数的定义域、负零、下溢、`round`/`pow`/`tan` 特殊值 | F-A |
| F-3 | D9 性能 | 工作量模型、VM/原生/内存预算、翻倍比、预热与采样协议 | F-C |
| F-4 | D4 `lfloat // % **` | 首批是否开放、整数/非整数指数域、错误与资源边界 | F-A |
| F-5 | D6/D8 原语精表 | 参数名、规划 ID 33–48、效果、能力、C ABI 符号和调用约定 | F-B |
| F-6 | D8 版本兼容 | B2-A 1.18/20 中间快照、B2-B 1.19/21、旧 `.xiaoc`/`.xar`/原生对象处理 | F-B |
| F-7 | D1 依赖闭包 | `num-bigint`/`bigdecimal` 传递闭包、精确版本、校验值、许可证、构建环境 | F-C |
| F-8 | D5 二进制转换 | `float`/`sfloat` ↔ `lfloat` 的实际值保留、最近偶数、下溢与范围错误 | F-A |
| F-9 | D11 误差/内核 | 每函数 ULP/绝对误差、独立参考向量、`cmath.pow` 与 `**` 共用内核 | F-A |
| F-10 | D2/D9 scale | 规范/显示 scale、输出格式、运算结果 scale、相等/哈希与预算 | F-A |
| F-11 | D5 文本/输出 | `str → lfloat` 语法、`float/sfloat → str`、拒绝语法与往返规则 | F-A |
| F-12 | D10 错误精表 | 新增 033/034（若保留）、触发条件、类别、优先级和与旧错误的边界 | F-B |

对账要求：每一项都必须有星崽答复或明确的“不纳入本批”裁定；不能以“建议值”或
“沿用上游默认”代替。已确认的 18 条不重问；只问仍未冻结的精确字段。

## 四、分批执行

### F-A：数值语义、显示与误差

执行规划见[F-A 数值语义裁定规划](20o-b2-freeze-a-numeric-semantics-plan.md)，当前收口见
[F-A 收口规划](20o-b2-freeze-a-closeout-plan.md)；本批只裁定，不写产品。

范围：F-1、F-2、F-4、F-8、F-9、F-10、F-11。当前六项完整确认，F-4主体答复保留而混合类型补充待答，见[F-A退出对账](20o-b2-freeze-a-numeric-semantics-plan.md#23-f-a退出对账)。

1. 冻结 `lfloat_divide` 的 rounding/precision/scale 和“只精确有限”的边界；
2. 冻结 `lfloat` 的显示身份与规范身份、运算结果 scale、相等/哈希；
3. 冻结 `lfloat //`、`%`、`**` 是否首批开放及各自指数域；
4. 冻结浮点/定点互转的最近偶数、下溢、带符号零、范围错误；
5. 冻结 `cmath` 每函数定义域、特殊值、ULP/绝对误差和参考向量来源；
6. 冻结 `str → lfloat` 语法与 `float/sfloat → str`；
7. 产出精确语义表，替换冻结草案中的“建议/候选”。

退出：F-A 七项均有唯一规则；不存在“按上游默认”“实现时再看”的占位。

### F-B：原语、ABI 与错误

执行规划见[F-B 原语、ABI、版本与错误裁定规划](20o-b2-freeze-b-contract-plan.md)；F-B 须等 F-A 25/5 后才开始。

范围：F-5、F-6、F-12。

1. 冻结 `lfloat_divide` 与 15 个 `cmath` 原语的参数名、ID、效果和能力；
2. 冻结 C ABI 符号、输入强引用/输出所有权、失败时 out 槽和状态码；
3. 冻结 B2-A 保持 1.18/20、B2-B 升 1.19/21 的产物兼容/拒绝矩阵；
4. 冻结错误 033/034 是否保留、错误类别/优先级和旧错误复用边界；
5. 活动 intrinsic 表仍不修改；规划 ID 只存在于冻结文档。

退出：F-B 三项均有精确表和示例；没有未命名符号、未定 ID 或未定错误优先级。

### F-C：依赖、资源与性能

范围：F-3、F-7。

1. 记录 `num-bigint`/`bigdecimal` 的精确闭包、Cargo 特性、许可证、校验值和 Rust 版本；
2. 确认 `bigdecimal` 构建环境变量不影响 Xiao 语义；
3. 冻结原文/系数/scale/指数/中间值/precision/幂及新增lfloat //、%的资源验收公式和错误顺序；不能遗漏F-4扩大后的范围；
4. 冻结工作量模型、VM/原生/内存预算、翻倍比、采样协议和失败边界；
5. 记录 `libm` 版本与 `cmath` 参考向量来源；不把候选版本写成已构建。

退出：F-C 两项都有可执行的检查定义；未锁依赖、未跑构建的事实仍明确标注。

### F-D：收口与冻结签字

1. 逐条核对 12 项与 18 条已确认答复，形成 30 条完整台账；
2. 删除或明确登记所有“待审/建议”状态；
3. 更新 `20o-b2-freeze-draft.md` 状态、`20o-b2-numeric-foundation-plan.md` 和索引；
4. 通过 `check:docs`、`check:usedocs`、`check:layout`、`git diff --check`；
5. 记录本阶段 run/gate；不改产品、依赖、ABI 或测试。

退出：B2-FREEZE 可以标记完成，B2-A/B 才获得实施入口。

## 五、裁定状态与剩余待审

以下记录已确认的F-A与仍待审的五项；完整规则和待审原因见[冻结草案](20o-b2-freeze-draft.md)§五～§八。

| 编号 | 状态或待审议题 | 已确认规则或待审建议 |
| --- | --- | --- |
| F-1 | 已确认 | 七种模式、最多有效数字、短结果不补零、单次最终舍入 |
| F-2 | 已确认 | 15函数定义域、负零、IEEE下溢、最近偶数及pow/tan边界 |
| F-3 | 性能/工作量/采样 | 沿用 F10 预热+5 批中位数；短操作≥50ms；超线性负载分组预算 |
| F-4 | 主体确认，类型准入待补 | 同时开放lfloat //与%、floor配对、显示尺度及非负整数值幂已确认；混合实参准入未选定 |
| F-5 | 原语参数名/ID/ABI | `lfloat_divide` 33；`cmath` 34–48；统一借读输入、拥有 out |
| F-6 | 版本/旧产物 | B2-A 保持 1.18/20；B2-B 升 1.19/21；旧产物按矩阵兼容重放或拒绝 |
| F-7 | 依赖闭包/构建 | 锁定直接+传递版本和校验值；按上游声明保留许可（libm为MIT，其余五包双许可）；构建环境不得改变语言语义 |
| F-8 | 已确认 | 实际二进制值、最近偶数、舍入后有限性、子正规/零；新转换整数结果显示scale=0 |
| F-9 | 已确认 | 每函数容差、离线MPFR区间参考、float pow共享内核；无唯一参考则不可验证 |
| F-10 | 已确认 | 系数/规范身份、显示与结果scale、零例外、输出预算与格式 |
| F-11 | 已确认 | 精确ASCII十进制语法、拒绝边界与数值文本输出；不开放直接str到float/sfloat |
| F-12 | 错误 033/034 | 保留规划编号但先确认；033 资源、034 非终止；与 009/011/013 优先级明确 |

F-A六项完整确认，F-4主体答复保留但类型补问未答；F-3/F-5/F-6/F-7/F-12也仍待审。F-A未退出，整体未联合冻结。

## 六、验收与证据

| 检查 | 期望 |
| --- | --- |
| 决策完整性 | 当前24 confirmed、6 pending；最终30条均有直接答复/裁定，无静默默认 |
| 冻结表一致性 | 名称、参数、ID、C ABI、效果、错误、资源、性能互相对账 |
| 依赖记录 | 直接/传递版本、许可证、校验值、构建环境和锁文件策略明确；不声称已构建 |
| ABI 记录 | 1.18/20 与 1.19/21 的产物行为、拒绝/兼容矩阵明确；活动表未改 |
| B1 回归 | 历史 SourceRoot 审计仍通过；默认当前 registry 拒绝保持已解释状态 |
| 文档门禁 | docs/usedocs/layout、链接和索引通过 |
| 证据 | 保存实际命令、输出摘要、决策答复和提交号；不伪造三平台或性能成绩 |
| 产品边界 | `core/rust`、`stdlib`、依赖锁、intrinsic、ABI、测试和工作流无差异 |

## 七、退出条件

1. 12 组待审项全部冻结，且与 18 条已确认答复无冲突；
2. 依赖、值表示、精度/舍入、显示/转换、`cmath` 域、错误、资源和性能都有精确表；
3. `lfloat_divide`/`cmath` 的原语 ID、C ABI 和版本兼容矩阵明确；
4. 所有“待审/建议”状态被替换或明确延期，不能进入 B2-A 后才临时决定；
5. 文档门禁通过，证据带实际命令/输出；
6. 没有修改产品、依赖、ABI、测试或工作流；
7. B2-A/B 仍未开工；22D/22C、19/11/22 债务保持独立。

## 八、交接

新 Agent 先读：

1. [B2 数值基础规划](20o-b2-numeric-foundation-plan.md)；
2. [B2 冻结草案](20o-b2-freeze-draft.md) §二～§九；
3. [决策答复记录](assets/20o-b2-freeze-prep/entry-20261103/decision-receipts.json)；
4. [B1 SourceRoot 入口](assets/20o-b1-close/README.md#只读复跑) 与 [复跑实证](assets/20o-b1-close/source-root-replay-20261103/README.md)；
5. [B2 入口资料](assets/20o-b2-freeze-prep/entry-20261103/README.md)。

实现/收口时不得：

- 把候选版本、建议回归模式或建议错误码直接写成已冻结；
- 在未确认的依赖闭包上修改 `Cargo.toml`/`Cargo.lock`；
- 在 B2-A 新增 intrinsic 或改变 1.18 ABI；
- 把 `cmath` planned 页升为 `verified`，或把 B1 历史 721 条写成 B2 当前性能；
- 用默认 `audit.ps1`/`validate-audit.ps1` 的当前 registry 拒绝来否定 B1 历史证据。

## 九、落点

```text
docs/DevDocs/20o-b2-freeze-closeout-plan.md   本规划
docs/DevDocs/20o-b2-freeze-draft.md           精确契约与逐条台账
docs/DevDocs/20o-b2-numeric-foundation-plan.md 批次边界
docs/DevDocs/README.md                        阶段索引
docs/DevDocs/20-builtins-and-standard-library.md 阶段表
docs/DevDocs/20o-official-library-program.md  B2 批次链接
docs/DevDocs/assets/20o-b2-freeze-prep/entry-20261103/ 决策与依赖证据
docs/DevDocs/assets/20o-b1-close/source-root-replay-20261103/ B1 复跑来源
```

本阶段只把文档收紧；完成后下一入口是 B2-A 的实现规划，不是 B2-A 代码本身。
