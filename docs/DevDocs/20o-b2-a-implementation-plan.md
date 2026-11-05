# 20O-B2-A. `lint`/`lfloat` 数值运行时基础实施规划

> **这是 B2-A 的实施规划，不写产品函数、不执行 Cargo 解析或构建、不改 Cargo/Cargo.lock、intrinsic、ABI、诊断表或测试实现。**
> 规划入口为`04bca101`，其前置为`943db5b1`；F-D签字基线为`22a865bc`/`53e95a55`；已签草案正文摘要为`8bca48018be3bf2cf38f615d7cf35d55a0ff9745f73e3c196161abd854d286ff`。
> 当前30 confirmed / 0 pending，F-D已签字、联合冻结已激活；A-P1/A-P2/A-P3已由星崽实际批准。**本次止于规划交付，A0～A7获准另批实施，产品仍未开工**。
> B2-B的`lfloat_divide`/`cmath`/ABI 1.19/21与B2-C的完整721条F10重跑不在本批。

## 一、入口与不变量

| 项 | 当前状态 |
| --- | --- |
| 签字基线 | F-D三项实际批准；联合冻结已激活；台账30/0 |
| 版本 | Runtime ABI 1.18、`CODEGEN_VERSION` 20、活动intrinsic最大32、`cmath` planned |
| 规划审批/产品 | 三项实际批准已齐备；本次未运行A0或写产品代码，产品仍未开工 |
| 依赖 | F-7六包已确认，尚未Cargo解析、锁定或构建 |
| B1 | 七库20/20与教程/registry verified；旧721条F10只作历史，B2-C完整重跑 |
| 历史债务 | 22D/22C、19/11/22保持独立，不由B2-A核销 |
| 前置CI | 943db5b1的workspace 41981794413成功；maintenance未命中push.paths而未触发，见[原始记录](assets/20o-b2-a/entry-20261105/ci-predecessor-943db5b1.json)；不算本轮规划或B2-A产品测试 |
| 待审规划CI | f4a0b881的workspace 41998981332第1次尝试成功；maintenance未命中push.paths而未触发，见[实际记录](assets/20o-b2-a/entry-20261105/ci-reviewed-plan-f4a0b881.json)；不代表产品验证 |

F-D签字回执的问题项ID、日期和答复见[F-D证据](assets/20o-b2-freeze-d/entry-20261105/README.md)；原始会话引用未随证据归档，本规划不复述或改写签字内容。
本轮只读核对与待审规划记录见[B2-A准备资料](assets/20o-b2-a/entry-20261105/README.md)，未执行Cargo解析/构建或新增产品测试。

B2-A的硬边界：

1. 保持`Lint`/`Lfloat`现有文本标签作为ABI边界，不暴露Rust布局，不新增`XiaoValue`标签；
2. 不新增intrinsic，不分配33～48，不改C ABI符号、布局、调用约定或状态编号；
3. 不升ABI/生成器；B2-B才使用1.19/21；
4. 不实现`lfloat_divide`、15项`cmath`或`stdlib/cmath.xiao`；
5. 不把`cmath`升为verified，不改`docs/module-registry.json`状态；
6. 不修改B1产品函数、冻结预算、22D/22C断言或既有工作流阈值；后续接入B2-A验收的具体执行点须经§3.2批准。

### 1.1 已核对的代码事实与缺口

以下仅是源码事实，不表示高精度能力已交付，内容摘要与片段见准备资料的source-facts.json。

| 落点 | 当前事实 | 后续实施必须解决 |
| --- | --- | --- |
| Runtime value/mod.rs | Lint/Lfloat为String，Eq/Hash按原文，display_text返回原文 | 保留显示精度，规范数值身份共用；1.50/1.5同值同哈希，类型标签规则不扩大 |
| Runtime value/ops.rs | NumericPair只有定宽整数/浮点；部分lint转换仍经i128 | 新增精确内核，不能让大lint因宿主i128失败或近似为f64；bool±lint的既有奇偶规则也须覆盖 |
| 类型与常量降低 | 现有提升规则存在；大数字面量及两后端常量仍为文本 | 沿已签F-4/F-8更新类型准入并保持字面量精度；数值错误仍在正常可恢复路径传播 |
| Runtime ABI恢复与VM常量恢复 | 从文本直接恢复Lint/Lfloat | 共享合法性/资源校验，不能让非法值进入不可失败的Eq/Hash；保留旧ABI结构错误边界 |
| XCON与并发转换 | XCON文本标签只验证UTF-8，并发恢复直接包装文本 | 保留XCON格式与结构错误；恢复为数值时使用同一边界，不能借此改22类协议、排序或断言 |
| 诊断与消息目录 | 009/011/012/013已存在；033/034仍未进入活动定义 | 在经批准的实施A1落实F-12的两个已冻身份及消息目录，再由算术路径使用；不新占其他编号 |
| 原生缓存 | 指纹包含ABI、生成器和Runtime静态库内容 | B2-A同为1.18/20也须绑定新Runtime及独立产物路径，不能复用B1旧对象或当作最终B2交付 |

## 二、范围

### 2.1 要做

1. 引入`num-bigint 0.4.8`、`bigdecimal 0.4.11`及F-7冻结闭包，建立内部适配层；
2. 统一`Lint`/`Lfloat`的解析、输出、零、符号、scale、指数、规范身份、相等与哈希；
3. 实现`lint`的加减乘除、整除、取模、幂、比较和一元负号；
4. 实现`lfloat`的加减乘、精确有限`/`、`//`、`%`、`**`、比较和一元负号；
5. 实现已确认的整数族、浮点族、`str`与`lfloat`转换矩阵；
6. 打通类型检查、IR、TAC/字节码、VM、LLVM动态Runtime、序列化/并发值承载；
7. 落实F-3资源/工作量、F-12错误顺序、失败原子性和F-3性能测量；
8. 完成B1七库功能回归，不声称旧721条F10是B2当前成绩。

### 2.2 明确不做

- 不写`lfloat_divide`、`cmath`、B2-B的intrinsic/ABI；
- 不启动B2-C的721条F10重跑、教程/registry升级或三平台最终收口；
- 不写`decimal`/`fractions`/浮点`statistics`/`random`；
- 不开放公开`dynamic`、新容器模型或新错误成员访问；
- 不新增B1产品源码、错误表或预算变更；
- 不用"先实现再看"的占位替代已冻结规则。

## 三、实施开工闸门

产品代码开工前必须全部满足：

1. 星崽确认本B2-A规划的批次、证据和退出边界；F-D签字只授权规划，不自动授权写代码；
2. 已签草案正文摘要、30条台账和F-D回执仍一致，无未登记的正文变化；
3. F-7六包版本/许可证/锁策略已确认；实际落地与Cargo操作随本规划的另批实施批准，不能把来源核对当成已经解析或构建；
4. B1七库功能基线与当前workspace门禁可复现；旧721条F10仍标历史；
5. 实际ABI1.18/生成器20/活动intrinsic最大32未被改动；
6. 文档门禁和产品边界检查通过；
7. 若需要未冻结语义、F-12之外的新错误、新ABI或新intrinsic，先回B2-FREEZE；033/034属于已签规划身份，后续按A1落实，不再临时改名或混用旧码。

### 3.1 本轮规划与后续产品的分界

本轮允许读源码、修订规划、核对签字摘要/既有CI和执行docs/usedocs/layout；不执行Cargo解析、构建、基准或新增产品测试。
产品批的A0先验证新工作快照、B1功能基线与workspace gates，随后才落依赖和产品；这些实际命令在本轮只作为未来执行点列出。
允许修改的产品文件、诊断目录、工作流接线与教程落点都须按§3.2取得实际批准后另批实施。
F-D签字正文和30项内容投影不改；规划批准也不使尚未执行的测试、构建、性能或三平台成绩变为完成。

### 3.2 已批准的实施规划（A-P1/A-P2/A-P3）

| 项 | 已批准范围 | 不变边界 |
| --- | --- | --- |
| A-P1 | 按A0→A1→A2/A3→A4→A5→A6→A7分批；A1落实F-7依赖/锁与F-12的033/034诊断/消息目录、文本适配和身份 | 只批准另批产品范围；保持ABI1.18/生成器20/无新intrinsic，不改变30条契约，不在本轮运行Cargo或写产品 |
| A-P2 | §5.1复用B1已有VM/三平台原生精确目标，追加独立B2-A语料和标记；新增B2-A受控release性能开关与语言教程示例 | 保留所有B1场景、28/28守卫和原阈值；B2-A性能独立启动，不触发整批B1 721条，不新增标准库身份 |
| A-P3 | §5.2按F-3完成B2-A自身三平台功能、资源和性能证据；所有1.18/20产物仅内部验证，A7收口后才交B2-B规划 | 不重定义版本拒绝矩阵；cmath仍planned；B2-C仍负责最终1.19/21产物、完整721条与最终教程/registry收口 |

三项均收到无附加条件的明确“批准”，见[答复原文](assets/20o-b2-a/entry-20261105/decision-receipts.json)。批准依据为f4a0b881的规划快照，正文摘要3e76188c34ecc4bef6709652bef41903d812240abfe862b2c95645fa5c4587be；后续只同步审批状态与来源，不改变已审实施内容。
本次交付止于规划和证据；另批实施从A0真实基线开始，当前不运行Cargo、不写产品/测试或工作流。

## 四、分批执行

### A0：工作快照与基线（获准后执行）

1. 记录源码提交、已签草案正文、30项投影、工具链与现有锁文件摘要，确认1.18/20、intrinsic最大32、cmath planned；
2. 在依赖变更前执行既有B1契约、资源及VM/原生精确点和workspace gates，分别记录实际平台；只读CI不足以冒充本地复跑；
3. B1旧721条只作历史；本步不启用b1_f10，也不复跑大批F10；
4. 以唯一任务目录隔离Runtime/缓存/输出，明确XIAO_STDLIB_ROOT来自仓库；不复用旧原生可执行文件。

退出：源码、Runtime、产物根与真实功能基线可对账；失败先按本批范围定位，19/11/22债务保留独立归属。

### A1：依赖、适配层与值身份

1. 按F-7添加直接依赖`num-bigint=0.4.8`、`bigdecimal=0.4.11`，仅`std`，关闭不用的默认特性；
2. 锁定`libm 0.2.16`、`num-integer 0.1.47`、`num-traits 0.2.19`、`autocfg 1.5.1`传递/构建依赖；
3. 更新`Cargo.lock`，维护构建使用`--locked`；新增包/版本/特性超出F-7冻结表时回裁定；
4. 构建子进程清理继承的`RUST_BIGDECIMAL_*`，Xiao数值语义显式配置，不使用上游默认精度/舍入/Display；
5. 在xiao-diagnostics定义已冻NUMERIC_LIMIT_CODE/033、NON_TERMINATING_DIVISION_CODE/034与对应类别；Runtime门面及xiao-i18n接入runtime.numeric_limit、runtime.non_terminating_division；沿F-12，不新增C函数、状态或编号；
6. 建立`Lint`/`Lfloat`内部适配层，保持文本标签边界，不把BigInt/BigDecimal布局放进`XiaoValue`；每次操作精确解析为临时大数，结束释放，不加跨调用共享缓存；
7. 落实已确认规范/显示身份：D=32,768、W=65,536、H=100,000；零、符号、scale、指数、相等和哈希按F-10/F-3；
8. 类型、ABI、VM常量及XCON→Runtime入口恢复遵守同一数值合法性/预算；不可失败的Eq/Hash使用无分配规范视图，不临时大数分配、不panic、不失败后退回文本或默认值；
9. 单元与确定性生成用例覆盖前导零、尾零、零、负零、强消去、scale差、规范键以及集合/字典的去重与查询；不为属性测试引入闭包之外的rand/proptest依赖。

退出：同一数值只有一种规范身份；解析/输出往返稳定；未新增ABI标签或intrinsic。

### A2：`lint`算术与比较

1. 实现`+ - * / // % **`、比较和一元负号，逐项映射冻结草案§五/§六；
2. `//`/`%`使用floor配对和既有余数符号；除零013、负指数009；
3. `0**0=1`；非负整数值指数、指数上限和结果预算按F-4/F-3；
4. 混合类型沿用既有数值提升；两个整数仍返回整数族，不新增隐式窄化；
5. 比较、相等和哈希不使用`f64`或显示文本字节；
6. 失败无部分结果、输入不变；错误顺序按F-12。
7. bool±lint沿既有奇偶契约，只读符号/最低有效位；最大合法lint不得因i128解析上限失效。不扩大bool与浮点、反向算子或其他隐式转换。

退出：Runtime内核的每个运算符/比较具有边界+1和错误身份证据；没有未登记的类型提升。完整VM/native语言差分随A5接线一起验收，不能将A2内核通过标作两后端已完成。

### A3：`lfloat`算术与精确有限除法

1. 实现加减乘、比较、一元负号和规范身份；
2. 语言`/`只返回精确有限十进制；非终止按F-12报034，不实现显式`lfloat_divide`；
3. 精确除法先按gcd约分，再处理2/5因子；工作中值≤W，最终系数≤D，scale≤H；
4. 实现F-4开放域`//`、`%`、`**`：floor配对、`%`不物化完整商、非负整数值指数、0/±1快路径；
5. 混合类型按既有提升到lfloat；仅提升到float/sfloat的组合继续拒绝；
6. 显示scale、规范scale和结果scale按F-10；不为通过预算丢显示精度；
7. 强消去、`1.00/2`、负数floor进位、`1e100000 // 1e-100000`和`%`独立预算设计算例必须核对。

退出：Runtime内核有限/非终止/除零/负指数/超限路径稳定，`//`/`%`不产生巨大中间商；VM/native一致性随A5完整语言路径验收。

### A4：转换矩阵

1. `int`/`sint` ↔ `lint`：精确或009范围错误；
2. `lint`/`lfloat`→`str`按已冻十进制输出；`str`→`int`/`sint`/`lint`保留单码点契约；仅`str`→`lfloat`按F-11解析数值文本，恢复lint数值须显式经`lfloat`→`lint`，不新增直接数字字符串到lint入口；
3. `float`/`sfloat` ↔ `lfloat`：实际二进制值、最近偶数、下溢/溢出和带符号零按F-8；
4. 转换后整数结果默认显示scale=0；不开放直接`str → float/sfloat`；
5. 不隐式窄化，不用宿主`f64`决定十进制舍入；
6. 每个转换的失败原子性、错误身份和输入不变。
7. 按F-11补齐float/sfloat→str的现有有限显示与负零；lfloat→整数向零截断并分别核对定宽范围/大整数资源，数字10的默认lfloat显示scale=0；
8. 同类型lfloat保持显示精度；二进制±0转lfloat规范为0，不能承诺再转回保持负零位模式。静态非法类型与运行时值错误分开验收。

退出：转换矩阵逐项有内核和边界证据，无静默舍入或双重表示；VM/native和三平台的完整执行证据在A5/A7收齐，不倒写成A4之前已有。

### A5：类型、IR、TAC/字节码、VM、LLVM与承载

接线分析与A1～A4同步推进；A1的合法值入口与无分配Eq/Hash须同批落地，不能留下“入口仍可绕过校验、Eq/Hash却假定合法”的中间状态。
不把A1～A4尚无的完整语言证据写成通过；A5汇合后才核对源程序、归档与两后端全路径。

1. 类型检查与IR使用同一数值提升规则；
2. TAC/字节码编码保持现有标签/版本，不增加intrinsic id；
3. VM与LLVM动态Runtime共用语义内核；不能另写一套近似实现；
4. 序列化/并发值承载按现有文本标签或已冻结边界，不暴露Rust布局；
5. 源码位置、错误传播和资源释放路径与既有语言契约一致；
6. 不允许回退到`f64`、字符串比较或宿主libm来"凑"结果。
7. AST/IR/TAC保留原始大数文本；后端沿既有入口恢复及调用共享Runtime内核。不要把能catch的009/033/034折叠成不可catch的编译错误；
8. ABI无失败构造器、无效数值载荷及Eq/Hash前置不变量须一起审查；只用既有返回/挂起错误通路。若必须改签名、状态或XCON协议才能安全接线，停下回冻结，不自建替代接口；
9. float **与cmath共用libm内核属于B2-B，本批不提前重写定宽float幂；仅实现已冻高精度路径和现有必要转换。

退出：两后端和序列化承载一致；未改ABI/生成器/intrinsic；无双重表示。

### A6：资源、错误、原子性与性能

1. 按F-3共同接纳公式检查文本、系数、scale、指数、中间值、结果和工作量；
2. 超限使用上限+1；失败无部分结果、输入不变；预算检查先于结果构造；
3. 错误002/009/011/012/013/033/034按F-12七阶段顺序；B2-A不新增C ABI状态映射；
4. `033`资源与`011`分配失败分开；`034`非终止与`013`除零分开；
5. 按F-3 §6.2测量VM release、原生O0/O3、五批独立进程、中位数、OS峰值和翻倍比；
6. 性能是受控验收，不是任意硬件时延保证；未测得或不可验证必须如实记录。

退出：资源/错误/性能证据完整；没有用松上界拒绝合法强消去；没有用预算通过掩盖超限。

### A7：B1回归与B2-A收口

1. 执行B1七库20入口功能回归，保持产品源码/错误表/预算不变；
2. 旧721条F10只作历史；不在B2-A声称B2当前F10；
3. 22D/22C、19/11/22债务保持独立；
4. 更新语言参考中`lint`/`lfloat`状态，`cmath`仍planned；
5. 证据目录保存源码身份、`Cargo.lock`、Runtime内容、工具链/目标、PID和输出路径；
6. 记录实际CI运行；push路径过滤导致的未触发按事实写明。

退出：B2-A所有范围有证据，B2-B/C仍未开工；已批准文件之外无差异，版本/ABI/intrinsic边界无变化，不能把本批合法产品改动写成“产品源码无差异”。

## 五、测试与证据矩阵

| 层次 | 必测 |
| --- | --- |
| 值身份 | 解析/输出往返、规范/显示scale、零/负零、相等/哈希、强消去 |
| `lint` | 加减乘除、`//`/`%`符号、`**`指数域、比较、一元负号、除零/溢出 |
| `lfloat` | 加减乘、有限/非终止除法、`//`/`%`、`**`、混合类型、scale |
| 转换 | 整数族、浮点族、str、最近偶数、下溢/溢出、整数结果scale=0 |
| 资源/错误 | 上限+1、失败原子性、输入不变、F-12顺序、033/034边界 |
| 管线 | 类型检查、IR、TAC/字节码、VM、LLVM、序列化/并发承载 |
| 平台 | Windows→Linux→macOS；VM release与native O0/O3差分 |
| B1回归 | 七库20入口功能；旧721条F10不冒充当前成绩 |
| 性能 | F-3 §6.2五批独立进程、中位数、OS峰值、翻倍比 |

所有证据必须绑定实际源码、锁文件、Runtime内容、工具链/目标和产物身份；不能复用旧可执行文件或覆盖不同任务产物。

### 5.1 已批准的未来执行点与教程（A-P2，尚未创建）

| 执行点 | 后续实现落点与通过条件 |
| --- | --- |
| 普通功能回归 | 拟新增xiao-driver测试目标20o_b2_a_contract及20o_b2_a_resources，覆盖类型/转换、规范身份、F-12错误顺序、资源+1、失败原子性、旧定宽规则与序列化承载；不在默认debug测试中跑最大输入基准 |
| VM精确 | 复用20o_b1_probe::b1_capabilities_have_vm_golden_outputs，追加独立b2-a语料目录和B2-A标记，B1原有probe/fix/imp场景均保留；命令仍--exact，断言恰好1 passed/0 failed/0 ignored |
| 原生精确 | 复用20o_b1_probe::b1_capabilities_match_native_golden_outputs，三平台均--ignored并--exact，assert恰好1 passed，内部O0/O3各跑独立语料与黄金；不能遇到不支持形态删例 |
| 守卫 | 维护原四路径28/28计数守卫；追加B2-A缺标记/空语料等独立检查时单列名称与数量，不把28/28误称已经覆盖新增B2-A语义 |
| B2-A受控性能 | 拟增加workflow_dispatch布尔输入b2_a_perf，默认false，在既有VM/三平台原生作业调用独立release测量入口；与b1_f10互斥，并保留现有默认工作流时限、并发策略和B1阈值。若新测量需修改作业时限，先请星崽另行裁定 |
| 性能进程 | 各负载/规模/优化级别的五批独立进程，每批预热一次；完整语言调用计时，OS峰值覆盖输入/输出及上游暂存。校准、失败、不可验证与真实PID/目录按F-3§6.2；不可用普通ignored汇总代替可定位执行点 |
| B1源码回归 | 20o_b1_imp_contract、20o_b1_imp_resources及现有精确目标逐项对账20入口、导出/逐函数移除、真实.xar、普通包隔离；B1产品摘要不变，运行的Runtime必须是本次新源码 |
| lint/lfloat教程 | 更新既有language/basics/types/conversions.md、p1-expressions/operators.md及memory/runtime说明；完整示例拟放tests/fixtures/docs/numeric/，与教程代码逐字一致，经同一VM/原生精确点执行；不新建虚构stdlib.lint/lfloat模块 |
| 文档状态 | 既有verified页面只对原有规则背书；新增B2-A段落及示例未验证前明确planned，三平台证据齐全后才标相应新增范围可用。cmath仍planned，最终整批教程/registry升级留B2-C |

未来拟执行的功能入口（本轮未执行）：

```powershell
cargo test --locked --manifest-path core/rust/Cargo.toml -p xiao-driver --test 20o_b2_a_contract --test 20o_b2_a_resources
cargo test --locked --manifest-path core/rust/Cargo.toml -p xiao-driver --test 20o_b1_probe -- b1_capabilities_have_vm_golden_outputs --exact --nocapture
cargo test --locked --manifest-path core/rust/Cargo.toml -p xiao-driver --test 20o_b1_probe -- --ignored b1_capabilities_match_native_golden_outputs --exact --nocapture
```

新测试目标、fixture、工作流输入和测量工具在另批产品实施时才创建；本轮不借这些路径伪造执行结果。若精确入口抽取/标记规则需变更，同批更新守卫与正反例并公开构成。

### 5.2 已批准的负载与收口证据（A-P3，尚未执行）

| 验收组 | 必须保留的负载与身份 |
| --- | --- |
| 解析/输出/身份 | F-11完整语法、原文65,536与+1、D/W/H各边界、0与65,536个前导零、1.50/1.5同值同哈希及集合/字典实际操作；不能用哈希值相同代替相等性验证 |
| 加减/乘/幂 | 对齐后的强消去、最大合法非平凡乘法、预算拒绝、n=0/1与0/±1快路径、64次工作界、2**108852等真实结果规模；不只测乘1或小指数 |
| 语言精确除法 | 2/5因子大分母、gcd约分、1.00/2显示0.50、1/3的034及1/0的013；同一输入域下的工作/最终预算优先级，非终止不能抢报033 |
| 商余 | 四符号floor配对、负数修正进位、极大scale差、%的非零余数、模数对齐D+1的可接纳边界、//拒绝而%可为0；不得为测余数物化商 |
| 转换 | str到整数的单码点与数字42拒绝、数字文本经lfloat恢复lint、float/sfloat实际二进制0.1、窄化中点/子正规数/带符号零及舍入后溢出边界 |
| 外部恢复 | 同一文本经源码字面量、ABI/VM常量与XCON/并发恢复的身份一致；无效载荷在共享边界拒绝，XCON协议结构错误仍沿旧身份；不修改22D排序/断言 |
| 语言错误与释放 | 源级静态非法类型与可catch值错误分开；009/011/012/013/033/034均核对code、位置与恢复后继续执行，输入/out/强引用按既有契约；未测的真实OOM边界不宣称可恢复 |
| 性能 | 线性组按扫描/对齐长度，超线性组按平衡系数、gcd/有限长除法、实际幂结果或模数规模分组；合法n/2n与最大规模另测，原生O0/O3独立留样；无OS峰值不得用Xiao对象计数补成达标 |
| 内部产物 | A的ABI1.18/生成器20只作内部快照，记录Runtime静态库内容、优化级别、独立输出路径和源码/锁摘要；版本号相同不能证明B1旧Runtime支持B2-A |

A7退出需要所有B2-A适用路径的三平台功能、资源、两后端和性能证据，docs/usedocs/layout、fmt/Clippy、workspace与锁文件/覆盖率门禁均按仓库要求完成。
Windows→Linux→macOS按已有平台顺序登记真实作业和日志；失败先修或回裁定，不把超限/缺测删掉或写为通过。
只有最终B2-C才针对新1.19/21快照完整重跑B1的721条F10并合并B2-B能力，A7不得拿旧报告替代。

### 5.3 拟修改落点与不可变边界

| 批次 | 预计修改（获准后） | 不改变 |
| --- | --- | --- |
| A1～A4内核/转换 | core/rust/Cargo.toml、Cargo.lock、xiao-runtime/Cargo.toml、xiao-runtime/value及新增内部数值模块、xiao-diagnostics、xiao-i18n和相应README/有意义测试 | XiaoValue布局/标签、C符号/状态、数值公开签名、F-D已签正文与30项台账内容 |
| A5管线 | xiao-types/numeric与conversion、IR/bytecode常量与降低、VM执行/恢复、LLVM dynamic、Runtime ABI及XCON↔Runtime值边界 | 常量/容器/XCON格式、TAC/字节码版本、ABI1.18/生成器20、intrinsic1～32及100墓碑 |
| A6/A7验收 | xiao-driver既有精确目标及独立B2-A测试/测量工具、tests/fixtures/docs/numeric、语言教程、维护回归B2-A开关接线 | B1产品/预算/原始失败、B1 28/28守卫原路径、22D/22C与19/11/22发布债务、工作流既有阈值 |

实际文件超出表、引入新依赖/编号、无状态构造器无法安全保留现有通路，或三平台性能要求需变更时，先展示具体差异再请星崽裁定，不按“实现细节”绕过边界。

## 六、退出条件

1. `lint`/`lfloat`的运算、比较、转换、序列化和两后端全部有证据；
2. 规范值无双重表示；相等/哈希与显示身份一致；
3. 除零、数值文本/指数域、非终止、溢出、资源超限和分配失败路径稳定；显式precision/rounding参数入口仍留B2-B，不在B2-A偷偷增加；
4. 未新增intrinsic、未改ABI/生成器、未改C ABI符号；
5. B1七库20入口功能回归通过；旧721条F10仍标历史；
6. F-3性能测量按协议执行并如实记录；
7. `cmath`仍planned，B2-B/C未开工；
8. 22D/22C、19/11/22债务保持独立。

## 七、风险与禁止

- 用`f64`、字符串比较或宿主libm替代精确十进制；
- 把`bigdecimal`默认精度/舍入/Display当成Xiao契约；
- 让`1.5`/`1.50`成为两个值，或用文本字节做相等/哈希；
- 先物化完整商/巨大对齐值再做预算检查；
- 在B2-A新增intrinsic或改ABI“顺手解决”B2-B；
- 把`cmath`提前升verified或改registry；
- 修改B1产品、22D/22C断言或历史证据；
- 把旧721条F10写成B2当前性能；
- 把未触发CI写成通过，或把文档门禁写成产品成绩。

## 八、交接

B2-A收口后下一入口是B2-B实施规划：`lfloat_divide`、15项`cmath`、intrinsic 33～48和ABI1.19/生成器21。B2-C才做新快照721条F10、教程/registry和最终三平台收口。

新Agent先读：

1. [B2数值基础规划](20o-b2-numeric-foundation-plan.md)；
2. [冻结草案](20o-b2-freeze-draft.md)§二～§九；
3. [F-D签字记录](assets/20o-b2-freeze-d/entry-20261105/README.md)；
4. [30条决策台账](assets/20o-b2-freeze-close/entry-20261103/decision-ledger.json)；
5. [F-C依赖/资源/性能证据](assets/20o-b2-freeze-c/entry-20261105/README.md)。

## 九、落点

```text
docs/DevDocs/20o-b2-a-implementation-plan.md          本规划
docs/DevDocs/20o-b2-numeric-foundation-plan.md        B2总边界
docs/DevDocs/20o-b2-freeze-draft.md                   已签精确契约
core/rust/Cargo.toml / Cargo.lock                     B2-A实施时修改
core/rust/crates/xiao-runtime/...                     B2-A实施落点
core/rust/crates/xiao-types / xiao-ir / xiao-vm / xiao-codegen-llvm  B2-A接线
tests/...                                             B2-A回归/边界证据
docs/DevDocs/assets/20o-b2-a/entry-<date>/            B2-A证据
```

本规划A-P1/A-P2/A-P3已实际批准；本次没有执行Cargo、写产品或新测试。下一入口为按本规划另批执行A0～A7，B2-B/C仍按已签边界后置。
