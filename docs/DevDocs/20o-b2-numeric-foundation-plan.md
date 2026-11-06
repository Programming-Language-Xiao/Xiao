# 20O-B2. 数值基础批规划：`lint`/`lfloat` 与 `cmath`

> **这是 B2 的规划，不写产品函数、intrinsic、ABI 或测试实现。**
> 规划入口基线为 `87bac439`；B1-CLOSE 已收口，Runtime ABI 为 1.18、`CODEGEN_VERSION` 为 20。
> B2 的目标是把 `lint`/`lfloat` 从“十进制文本占位”升级为可运算、可转换的数值基础，
> 并交付 `cmath` 浮点原语；**联合冻结已由F-D实际签字激活，B2-A实施规划已由A-P1/A-P2/A-P3实际批准，A1依赖与值身份已退出；A2/A3内核已完成本批验收；A4转换内核已完成本批验收，A5语言全链路已完成本批验收，A6资源/性能实施中，A7及B2-B/C后置**。
> 星崽已逐项确认的精确表与签字状态见[冻结草案](20o-b2-freeze-draft.md)；本页建议不自动变为已冻结契约。
> F-A/F-B/F-C裁定30/0，F-D已签字；B2-A规划已获三项实际批准，A0 Windows本地基线已通过，A1已退出，A2/A3内核实施进度见[实施记录](20o-b2-a2-a3-arithmetic-progress.md)（2026-11-06）。
>
> 本批不包含 `decimal`、`fractions`、浮点 `statistics`、确定性 `random`（B5）、
> `time`/`args`/`fs`/`env`/`log`（B3）、JSON/TOML（B4）、`regex`/`compression`（B6）、
> 安全/网络/数据库（B7–B9）。
>
> 开工进度见本页 §四；退出对账另建 B2-CLOSE 文档。B1 的 721 条 F10 在 B2 改变
> Runtime/工具后不能自动充当当前 B2 证据。

## 一、入口与前置

### 1.1 当前基线

| 项 | 事实 |
| --- | --- |
| B1-CLOSE | 七库 20/20 产品与教程/registry 均 `verified`；最终 B1 证据见 [收口记录](20o-b1-closeout.md) |
| 规划入口基线 | 87bac439，规划开工465e213d仅改文档；入口工作区门禁41585868754成功；当前签字状态以F-D证据与Git为准 |
| Runtime ABI | 1.18；`CODEGEN_VERSION` 20；标准库接口版本 1 |
| 数值现状 | `lint`/`lfloat` 以十进制文本保存；一元负号与部分转换可用，`NumericPair` 仍只接受定宽数值，算术、比较和多数转换明确报错 |
| cmath 现状 | 没有浮点数学 intrinsic；仅有 `float`/`sfloat` 的基础算术和 `**` |
| B1 遗留 | 721 条 F10 只在 B1 产品/工具身份下有效；22D/22C 历史失败及 19/11/22 债务仍独立 |

B2 不把“B1 已通过”自动解释成“数值基础已具备”。B2 的入口是**冻结**，不是直接写函数体。

### 1.2 已清理的交接口径

B1自审后的两项P3已在本次冻结准备中处理，原始诊断与B1结论保持：

1. 保留cbd7fdaf的原CI观察，另补[87bac439节点](assets/20o-b1-close/review-fix-20261103/ci-87bac439.json)：
   工作区门禁41585868754、实际作业125751076106成功；明确该节点没有改变B1被测产品快照。
2. `validate-audit.ps1` 生成的 `source-line-false-hit-formatted.log`/`.error.json`
   含随机临时路径，重新运行会得到不同哈希；`validation.json`、两个反例和实际异常消息稳定。
   本次选择标为仅诊断，从复跑摘要项移出，原文保留。planned登记前的两次历史比较为10份稳定产物一致、2份诊断不同，
   见[入口实证](assets/20o-b2-freeze-prep/entry-20261103/README.md)。当前registry已扩展，默认命令会拒绝；
   验证器现已增加SourceRoot透传，当前须按[B1入口](assets/20o-b1-close/README.md#只读复跑)导出106b99c7，见[本轮复跑](assets/20o-b1-close/source-root-replay-20261103/README.md)。

这两项是证据可读性问题，不改产品、ABI 或 B1 结论。

### 1.3 产品实施闸门（由B2-FREEZE逐项落实）

开始B2-A/B产品前必须确认以下条件；未确认项允许在文档冻结中准备和审阅，不能进入实现：

1. B1 的完整基线、20 入口功能回归和 B2 入口门禁仍成功；
2. 依赖库、版本、许可证、跨平台构建方式和锁文件策略已冻结；
3. `lint`/`lfloat` 的精确值表示、规范化、算术、转换、错误、限额和 ABI 方案已冻结；
4. `cmath` 的函数集合、类型、舍入、NaN/Inf/域错误、性能和 ABI 方案已冻结；
5. 明确 B1 F10 是历史证据，B2-CLOSE 之前不得把它写成当前 B2 性能。

任何一项未确认，B2 只停留在文档冻结，不写产品代码。

## 二、本批范围

### 2.1 B2-A：数值运行时基础

目标：让 `lint`/`lfloat` 真正参与语言运算，而不是字符串占位。

- 引入经冻结的任意精度整数/十进制底层，保持 `RuntimeValue` 和 ABI 边界不暴露 Rust 布局；
- 统一规范十进制文本的解析、输出、零值、符号、scale、指数和相等/哈希规则；
- 实现已确认的高精度运算、比较和相等：lint包含`+ - * / // % **`，lfloat的`// % **`开放域及混合类型已由F-4确认；整体冻结已由F-D激活；
- 补齐 `int`/`sint`/`lint`/`lfloat`/`float`/`sfloat`/`str` 的显式转换矩阵；
- 打通类型检查、IR、字节码/TAC、VM、LLVM 动态 Runtime 路径和序列化/并发值承载；
- 保持 B1 七库回归，不改变 B1 产品源码、错误表和预算。

B2-A 预期**不新增 intrinsic、不改变 `XiaoValue` 标签或 C ABI**；若实现需要改变 ABI，
必须先回到 B2-FREEZE 重新裁定。

### 2.2 B2-B：显式精度接口、浮点原语与 `cmath`

目标：交付星崽确认的全局lfloat_divide显式精度接口与cmath首批15项float函数。
语言/仅精确有限；非终止除法走显式precision接口。B2-A不因此新增intrinsic。

- 冻结 intrinsic 名称、稳定 ID（从 33 起的未占用编号）、签名、错误表、能力和效果；
- 在 Runtime、VM、LLVM 和 C ABI 四处接线，加入 `.xiaoc`/`.xar`/原生元数据版本拒绝；
- `stdlib/cmath.xiao` 只做公开包装，遵守 B1 的内联/无顶层 helper 导出约束；
- 覆盖舍入、NaN/Inf、域错误、极值、负零和平台一致性；
- 不把 `lfloat` 的超越函数混入首批，除非 B2-FREEZE 明确授权。

### 2.3 B2-C：回归与收口

- 新 `cmath` 教程、registry 条目和 stdlib README 状态；
- B1 七库 20 入口在新 Runtime/ABI 下重新执行；
- B1旧721条F10保留历史身份；星崽已确认B2-C必须在新Runtime/ABI快照完整重跑721条，不能沿用旧成绩；
- 三平台精确目标、完整 workspace/clippy/fmt/docs/usedocs/layout/覆盖率；
- ABI 版本升级后的 `.xiaoc`/`.xar`/原生元数据拒绝证据；
- 最终证据目录、实际 run/job/attempt 和独立失败记录。

### 2.4 明确不做

- 不在 B2 写 `decimal`/`fractions`/浮点 `statistics`/`random`；
- 不开放公开 `dynamic`、新容器模型或新错误成员访问；
- 不修改 22D/22C 释放顺序、断言或历史失败；
- 不认领 19 发布、11C L4、11B I5、22E 登记差异；
- 不因 B2 通过而把未测的 `lfloat` 超越函数、任意精度 `decimal` 或 `fractions` 标成已验证。

## 三、B2-FREEZE 必须裁定的事项

以下保留首轮建议起点及需要落实的契约字段，当前逐条裁定以[冻结草案](20o-b2-freeze-draft.md#二逐条裁定台账)为准。
首轮方向答复时，参数、舍入、误差、错误/资源与ABI精表尚未联合生效；这些精表现已全部确认，并由F-D实际签字激活联合冻结。下表保留历史对照，不能从旧建议推导新的裁定。

| 编号 | 决策 | 建议起点 | 未定即不能开工的原因 |
| --- | --- | --- | --- |
| B2-D1 | 任意精度依赖 | 纯 Rust `num-bigint` + `bigdecimal`；`num-rational` 只留给 B5。锁定版本、许可证和 `Cargo.lock`。 | 选错库会改变值表示、平台构建和许可证；`rug` 等 C/GMP 方案需要单独审计 |
| B2-D2 | 值表示与 ABI | 保留现有 `Lint`/`Lfloat` 文本标签作为 ABI 边界；内部解析为冻结的 BigInt/BigDecimal，不暴露 Rust 布局。 | 直接把 Rust 对象放进 `XiaoValue` 会改变 1.18 ABI 和跨语言所有权 |
| B2-D3 | `lint` 算术 | 加减乘精确；`//`/`%` 与现有 `int` 的 floor 语义一致；除零稳定错误；`0**0=1`；负指数拒绝；比较不用 f64。 | 这些规则决定错误身份和跨后端输出，不能各后端自行解释 |
| B2-D4 | `lfloat` 算术 | 加减乘精确；除法必须有明确 precision/rounding/scale；`//`/`%` 是否开放需裁定；`**` 的指数域需裁定；相等/哈希按数值规范而非文本字节。 | 任意精度十进制除法不能无上下文地“算到底”；文本相等会产生 `1.5` 与 `1.50` 两套值 |
| B2-D5 | 显式转换 | `int`/`sint` ↔ `lint` 精确或范围错误；`lint`/`lfloat` ↔ `str` 往返；`float`/`sfloat` ↔ `lfloat` 必须有舍入/溢出规则；窄化不隐式。 | 不冻结会把近似、截断和溢出混成同一种静默行为 |
| B2-D6 | `cmath` 函数集 | 固定 `float` 首批：`sqrt`、`floor`、`ceil`、`round`、`trunc`、`abs`、`min`、`max`、`pow`、`sin`、`cos`、`tan`、`exp`、`ln`、`log10`；是否同时提供 `sfloat`、返回值类型与名称需确认。 | D-4 只确认“含超越函数”，没有确认精确签名；没有签名不能分配 intrinsic ID |
| B2-D7 | NaN/Inf/域错误 | 建议沿用现有“非有限结果报稳定算术错误”的方向，不把 NaN/Inf 作为正常返回值；`sqrt(-1)`、`ln(0)`、`tan` 极点等逐项冻结。 | 当前 Runtime 的 `finite_float64` 会拒绝非有限结果；cmath 若返回 NaN 会改变语言契约 |
| B2-D8 | ABI/生成器拆分 | B2-A 不升 ABI；B2-B 新增 intrinsic 时升到 ABI 1.19、`CODEGEN_VERSION` 21，并更新 `.xiaoc`/`.xar`/原生元数据拒绝；标准库接口版本是否保持 1 在冻结时确认。 | 一次升级可避免每个函数 bump，但必须明确旧产物是拒绝还是重生成 |
| B2-D9 | 资源与性能 | 冻结十进制位数、scale、指数、单次结果大小、运算次数和 `cmath` 最大输入；沿用 B1 的 F9/F10 风格，最大合法输入+上限+1。 | 任意精度和指数可制造内存/时间放大；没有上限不能进入公开 API |
| B2-D10 | 错误与能力 | F-12已确认纯计算无新能力；复用002/009/011/012/013，规划033预算与034非终止；类别、触发条件、优先级和状态映射见冻结草案§5.2。 | 活动诊断表未改；不能把规划契约当成已实现 |
| B2-D11 | 平台一致性 | 平台顺序 Windows → Linux → macOS；`cmath` 明确采用宿主 libm 还是确定性实现，并冻结是精确输出还是 ULP 容差。 | 不同 libm 的超越函数末位可能不同；不冻结会把平台差异误报成产品缺陷 |
| B2-D12 | 教程与 registry | B2-FREEZE 只新增 `cmath` 的 planned 登记；B2-C 取得三平台和完整示例证据后才升 `verified`。 | 没有产品就不能先写 verified，不能用探测通过冒充产品教程 |

### 3.1 B2-D4 的特别约束

`lfloat` 不是 `f64` 的别名，也不是“字符串上的加减”。必须明确：

- 规范文本是值身份的一部分还是仅显示形式；若是显示形式，`1.5`、`1.50`、`15e-1` 必须能被统一比较；
- 除法结果的 scale/precision 和舍入模式不能从宿主 `f64` 反推；
- 结果大小、指数展开和反复运算必须有界，禁止把 `1e1000000` 直接展开成巨型字符串；
- `lfloat` 参与 `/`、`**` 和混合宽度比较时，类型检查与 Runtime 必须使用同一套规则。

### 3.2 B2-D6 的边界

`cmath` 是浮点库，不是高精度十进制库。首批建议只接受 `float`，不把 `lfloat` 的符号计算、
任意精度超越函数或 `decimal` 混入 B2；若后续要支持 `sfloat`，必须单独确认返回类型、精度和
平台差异。`pow` 与 `**` 的关系也应明确：是包装、别名，还是只保留一个入口。

## 四、分批执行

### B2-FREEZE：只改文档

当前30条裁定均已确认，F-D已按[冻结收口规划](20o-b2-freeze-closeout-plan.md)实际签字；联合冻结已激活；B2-A规划已批准，A0 Windows本地基线已通过，A1依赖与值身份已完成本批验收，A2/A3内核已完成本批验收；A4转换内核已完成本批验收，A5语言全链路已完成本批验收，A6资源/性能实施中、A7后置。
本轮不分配活动编号或写实现，契约台账见
[冻结草案](20o-b2-freeze-draft.md#二逐条裁定台账)。

1. 逐条裁定 §三，记录在独立冻结文档或本页新章节；
2. 给出精确 API 表：名称、参数名、类型、返回、错误、能力、效果、限制；
3. 说明依赖选择、版本、许可证、跨平台构建、`Cargo.lock` 和可复现性；
4. 冻结 ABI/生成器/产物版本拒绝方案；
5. 写清 B1 F10 的历史边界和 B2 的回归范围；
6. 通过 docs/usedocs/layout 门禁；**不写产品代码**。

### B2-A：`lint`/`lfloat` 基础

执行规划见[B2-A数值运行时基础实施规划](20o-b2-a-implementation-plan.md)；[A0工作快照与基线记录](20o-b2-a0-baseline-plan.md)已保存Windows契约9/9、资源17/17、VM/原生精确点、workspace与28/28守卫的实际通过；A1规划见[A1依赖、适配层与值身份实施规划](20o-b2-a1-dependency-value-identity-plan.md)；A2/A3规划见[A2/A3算术实施规划](20o-b2-a2-a3-arithmetic-plan.md)；A4规划见[A4转换矩阵实施规划](20o-b2-a4-conversions-plan.md)；A5规划见[A5语言全链路实施规划](20o-b2-a5-language-integration-plan.md)；A6规划见[A6资源与性能实施规划](20o-b2-a6-resource-performance-plan.md)。A0的Linux/macOS本地缺口保留；A1实施与平台证据独立登记；A2/A3内核已完成本批验收；A4转换内核已完成本批验收，A5语言全链路已完成本批验收，A6资源/性能实施中、A7未开始。

建议顺序：

1. 依赖与值规范化：`Lint`/`Lfloat` 的解析、输出、零值、scale、指数和相等/哈希；
2. `lint` 加减乘除/整除/取模/幂/比较/一元负号；
3. lfloat加减乘、精确有限/、比较/一元负号及冻结规则；显式lfloat_divide归B2-B；
4. 转换矩阵：整数族、浮点族、`str` 与 `lfloat` 的关系；
5. 类型检查、IR、TAC/字节码、VM、LLVM 动态 Runtime、序列化/并发承载；
6. 资源上限、失败原子性和错误身份；
7. B1 七库功能回归；不声称 B1 F10 是当前测量。

B2-A 的退出条件：每个运算符和转换都有 VM/native 与三平台证据；规范值不出现双重表示；
超限、除零、精度和溢出路径稳定；未新增未登记 intrinsic 或 ABI。

### B2-B：`lfloat_divide`、`cmath` 与 ABI

1. 按冻结表分配lfloat_divide与cmath的intrinsic ID（从33起，不占用100墓碑；文档候选33～48不先写活动表）；
2. 实现 Runtime 原语、VM 分派、LLVM 降低和 ABI 符号；
3. 更新 ABI、生成器、`.xiaoc`/`.xar`/原生元数据拒绝；
4. 新增 `stdlib/cmath.xiao` 包装和教程 fixture；
5. 覆盖定义域、负零、NaN/Inf 冻结规则、舍入、极值和跨平台；
6. 三平台原生与 VM 差分；F9/F10；
7. 不把旧 B1 F10 冒充新测量。

### B2-C：收口

1. 新 `cmath` 教程、registry、stdlib README；
2. B1 20入口功能回归，并按已确认D9在新Runtime/ABI快照完整重跑721条F10；旧报告保持历史标记；
3. 完整 workspace/clippy/fmt/docs/usedocs/layout/覆盖率/benchmark/Bun；
4. 三平台精确目标、ABI 版本拒绝、释放/归档/包隔离；
5. 证据目录保存实际 run/job/attempt、原始日志、摘要和失败样本；
6. 22D/22C 历史失败、19/11/22 债务继续独立，不用全绿核销。

## 五、测试与证据矩阵

| 层次 | 必测 |
| --- | --- |
| 依赖与构建 | 锁定版本、许可证、三平台构建、`--locked`、可复现产物、无 C/GMP 隐藏依赖 |
| 规范化 | 前导零、正负零、scale/trailing zero、指数、下划线、超长输入、相等/哈希一致 |
| `lint` 运算 | 加减乘、整除/取模符号、零除、幂、负指数、极值、超大数、递增/递减、比较 |
| `lfloat` 运算 | 加减乘、冻结精度除法、scale/round、`0.0`、负零、巨大指数、非有限/超范围 |
| 转换 | 所有冻结方向；越界、截断、舍入、溢出、非有限和往返 |
| 跨宽度 | `int`/`sint`/`lint`/`float`/`sfloat`/`lfloat` 混合比较与显式转换，不得静默近似 |
| `cmath` | 每个函数的正常值、定义域、负零、极值、舍入、NaN/Inf 策略；`pow` 与 `**` 一致 |
| 后端 | VM 与原生 O0/O3 逐字 stdout/退出码/stderr；动态与静态路径 |
| 平台 | Windows、Linux、macOS；三平台比较输出或冻结容差，不把单平台写成三平台结论 |
| 回归 | B1 20 入口、替换/移除、归档、普通包隔离、错误 `error_code` 读取 |
| 性能/资源 | 最大合法输入、上限+1、翻倍比、VM/原生时限、增量内存；cmath 输入限制 |
| 产物/ABI | `.xiaoc`/`.xar`/原生元数据版本拒绝；1.18 旧产物与 B2 产物行为明确 |
| 文档 | 每位公开入口有完整示例、实际导入、错误路径和限制；registry/教程一致 |

**B1 721 条 F10 的处置**：B2-A 改变 Runtime/依赖，B2-B 若改 ABI 还会改生成器；因此
B1旧721条只保留历史身份，不得在B2证据中写成“当前通过”。B2-C必须重新执行B1的20入口
功能和精确目标，并按已确认D9在新快照完整重跑721条；仅标旧报告为历史不能替代这次重跑。

## 六、退出条件

1. §三全部裁定有唯一契约，未确认项不在产品中静默选择；
2. 依赖/表示/规范化/转换/错误/限额/ABI 冻结，并有许可证与三平台构建证据；
3. B2-A 的 `lint`/`lfloat` 运算、比较、转换、序列化和两后端全部通过；
4. B2-B 的 `cmath` 函数、VM/LLVM、ABI、版本拒绝和三平台证据齐全；
5. B1的20入口在新Runtime/ABI下回归，721条F10在新快照完整重跑；旧报告保留历史身份；
6. 教程、registry、stdlib 说明与真实产品一致，未提前标 verified；
7. 22D/22C 历史失败、19/11/22 债务保持独立，不因 B2 全绿核销；
8. 未修改 B1 产品函数、冻结预算、22D 断言语义或工作流阈值。

## 七、交接

新 Agent 按以下顺序阅读：

1. [20O 总纲](20o-official-library-program.md) §二–§五；
2. [20P Day-1 架构](20p-day1-library-architecture.md) §五、§六、§八；
3. [20 标准库路线](20-builtins-and-standard-library.md) §二/§六/§七；
4. [B1 冻结契约](20o-b1-freeze-draft.md) 与 [B1-CLOSE](20o-b1-closeout.md)；
5. [02 类型系统](02-type-system.md) 的数值/转换边界；
6. [10 原生后端](10-native-backend.md)、[09R 机型研究](09r-bytecode-machine-research.md) 的 `lint`/`lfloat` 与 ObjHandle 说明；
7. 本页 §三的未决裁定与 §五测试矩阵。

实现时不得：

- 把 `Lint`/`Lfloat` 文本直接交给 `f64` 或字符串比较来“凑”算术；
- 在 B2-A 新增 intrinsic 或改变 1.18 ABI 而不先回到冻结；
- 用隐式窄化、宿主舍入或平台差异掩盖转换/精度错误；
- 把 B1 721 条或 B1 状态提交的运行号写成 B2 当前证据；
- 修改 22D/22C 测试断言、释放顺序、B1 产品函数或冻结预算；
- 用顶层 helper、顶层 import 或未登记导出绕过 `cmath` 的公开面约束。

## 八、落点

```text
docs/DevDocs/20o-b2-numeric-foundation-plan.md   本规划
docs/DevDocs/20o-official-library-program.md     总纲状态与 B2 链接
docs/DevDocs/20p-day1-library-architecture.md    B2 架构边界
docs/DevDocs/20-builtins-and-standard-library.md 阶段表
docs/DevDocs/README.md                           阶段索引
docs/DevDocs/20o-b1-closeout.md                  B1→B2 交接
core/rust/crates/xiao-runtime/src/value/ops.rs   高精度运算（B2-A）
core/rust/crates/xiao-runtime/src/value         值规范化
core/rust/crates/xiao-types/src/numeric.rs       类型/常量/错误
core/rust/crates/xiao-types/src/conversion.rs    转换矩阵
core/rust/crates/xiao-bytecode/src/lower         TAC 降低
core/rust/crates/xiao-vm/src                      VM 分派
core/rust/crates/xiao-codegen-llvm/src           原生/ABI 降低
core/rust/crates/xiao-intrinsics                   intrinsic 表（B2-B）
core/rust/crates/xiao-runtime-abi                 ABI（B2-B）
stdlib/cmath.xiao                                 cmath 包装（B2-B）
docs/UseDocs/library/cmath.md                     cmath 教程（B2-C）
docs/module-registry.json                         cmath 登记（B2-FREEZE/C）
```

## 九、裁定收口与F-D闸门

现有18条答复及本次实际答复已记入[裁定台账](20o-b2-freeze-draft.md#二逐条裁定台账)，不重复询问。
F-A/F-B/F-C已收齐，台账30/0；F-D三项实际批准齐备，联合冻结已激活；F-D退出时产品未开工，后续A1已按A-P1完成本批验收。F-D的签字、状态切换与B2-A交接见[F-D冻结签字规划](20o-b2-freeze-d-signature-plan.md)。
F-3已覆盖lfloat //与%的接纳/工作量及独立预算；A1已接入F-7依赖与身份层；算术内核证据见[A2/A3实施记录](20o-b2-a2-a3-arithmetic-progress.md)，转换内核进度见[A4实施记录](20o-b2-a4-conversions-progress.md)，完整语言、性能及整批验收留A5～A7。F-D当时只打开B2-A实施规划；该规划后续已获三项实际批准，A0 Windows本地基线已通过，A1已退出；A2/A3内核已完成本批验收；A4转换内核已完成本批验收，A5语言全链路已完成本批验收，A6资源/性能实施中、A7未开始。
