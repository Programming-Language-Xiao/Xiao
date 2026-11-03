# 20O-B2-FREEZE. 数值底座冻结草案与裁定台账

状态：文档冻结进行中，十二项方向及多项补充已由星崽确认，剩余精确表尚未联合生效（2026-11-03）。
逐项收口顺序、批次和退出条件见[冻结收口规划](20o-b2-freeze-closeout-plan.md)；本页保存精确台账。
来源为[数值基础规划](20o-b2-numeric-foundation-plan.md)与本次会话[逐条答复记录](assets/20o-b2-freeze-prep/entry-20261103/decision-receipts.json)；本页的“建议/待审”不等于已确认。
未决项、依赖闭包、资源接纳定义、性能、误差与ABI精确接线全部收口前，不进入B2-A/B产品实施。
当前代码仍为ABI1.18、生成器20、标准库接口版本1；没有新增产品、依赖、intrinsic、ABI或测试实现。
按D12只新增cmath planned教程/登记，code/tests为空；它不是已支持的源码模块。

## 一、入口与已清理交接

代码基线87bac439，规划开工提交465e213d。代码、stdlib源码、夹具、工作流、守卫及并行配置未变。
[入口实证](assets/20o-b2-freeze-prep/entry-20261103/README.md)保存：

- 87bac439的工作区门禁41585868754、实际作业125751076106成功，并[补入B1证据链](assets/20o-b1-close/review-fix-20261103/ci-87bac439.json)；该节点不改B1被测产品快照。
- 本机Windows现有数值/转换检查18项、B1契约9项（覆盖20入口）、VM精确目标恰好1 passed；它们验证现状，不是B2新运算已实现的证据。
- planned登记前，同环境两次运行旧审计验证器，10份结果逐字节一致；两份含随机路径的输出标为仅诊断。这是历史比较，不能代替当前HEAD复跑。

新增planned登记后，当前registry整文件不再等于106快照，两个默认命令均会拒绝。
验证器现已增加SourceRoot并透传全部审计分支；按[明确入口](assets/20o-b1-close/README.md#只读复跑)只读导出106b99c7，
本轮正向与反例实证、实际来源命令及输出见[复跑记录](assets/20o-b1-close/source-root-replay-20261103/README.md)。
旧[历史摘要](assets/20o-b2-freeze-prep/entry-20261103/historical-source-audit.json)保留来源不完整的历史身份，不改旧指纹或重写成新执行。

保留B1的七库verified状态和原始失败。B2将改变Runtime/ABI，届时旧721条只能作为B1历史；星崽已要求B2-C在新快照完整重跑721条。

## 二、逐条裁定台账

| 编号 | 已确认内容 | 仍须精确冻结 |
| --- | --- | --- |
| B2-D1 | 纯Rust num-bigint + bigdecimal；精确版本=0.4.8/=0.4.11、仅std与双许可留档；num-rational留B5，不引入C/GMP | 传递闭包、锁文件和构建策略；实际构建留实施批次 |
| B2-D2 | 保留Lint/Lfloat文本标签与C ABI边界；内部使用高精度实现，不暴露Rust布局 | 已补充确认：保留非零显示尺度，数值相等/哈希按值；每次操作解析并释放，不加跨调用共享缓存；结果scale与具体显示规则仍待 |
| B2-D3 | lint加减乘、//、%、**精确；整除向下、余数随除数；除零报稳定错误；负指数拒绝、0**0=1；/返回lfloat并遵守D4；比较不经过f64 | 指数类型及已确认D9数值的接纳定义、跨宽度矩阵及错误精表 |
| B2-D4 | 首批增加全局lfloat_divide四参显式精度接口，必填且支持位置/命名，precision为有效数字且范围1～32,768；归B2-B；语言/仅精确有限，非终止走显式接口 | 舍入模式、短结果不补零等细则、结果scale、lfloat //、%、**开放域与ABI精表 |
| B2-D5 | 文本兼容部分：保留str到int/sint/lint的码点转换，新增str到lfloat十进制解析；lint数值文本恢复可显式经lfloat到lint | 已确认小数显式转lint/int/sint向零截断、定宽检查范围；二进制互转、下溢、输入语法与格式仍待 |
| B2-D6 | cmath首批15项，参数/结果为float；floor/ceil/round/trunc也返回float；sfloat后置 | 参数名/调用规则、原语名称/编号、ABI符号、舍入与边界精表 |
| B2-D7 | 拒绝NaN/Inf作为正常输入或结果 | 各函数域、负零、下溢、舍入、pow特殊值与tan近极点规则 |
| B2-D8 | B2-A保持ABI1.18且不新增intrinsic；B2-B集中升ABI1.19、生成器21，标准库接口版本1；A若需突破先回冻结 | 显式精度接口已补充确认纳入B；中间快照/旧产物兼容与拒绝、规划编号及符号表仍待 |
| B2-D9 | B2-C必须完整重跑721条F10；原文/系数/scale/指数/中间值/precision/幂的资源数值已确认，见§六 | 接纳定义、错误顺序、运算工作量与性能预算 |
| B2-D10 | 已确认纯计算无新能力；除零013、溢出/非有限009、实际受控分配011优先复用 | 全场景稳定身份/类别/优先级、必要的新错误编号 |
| B2-D11 | 已确认Rust libm；普通超越结果按逐函数ULP/绝对误差，零/符号/域/错误精确核对 | 精确依赖、每函数误差、参考向量、精确特殊值与平台证据 |
| B2-D12 | 已确认冻结时cmath planned教程/registry，lint/lfloat更新语言说明；B2-C证据全过才verified，不新增大数标准库 | 教程落点、完整示例与状态升级点 |

D4显式接口已与D8落在B2-B，不能在B2-A里新增intrinsic。余下补充裁定仍逐条记录。

## 三、源码事实与兼容约束

### 3.1 数值分层

类型层[numeric.rs](../../core/rust/crates/xiao-types/src/numeric.rs)已经规定整数真除法返回相应浮点宽度：
int/int返回float，lint/lint返回lfloat。Runtime的[NumericPair](../../core/rust/crates/xiao-runtime/src/value/ops.rs)只接收同宽定宽值，
后端负责先插入转换。因此“Runtime不直接接收整数/”不能写成“语言层禁止整数/”。
高精度算术目前未实现，不能用定宽内核的拒绝路径冒充B2契约。

既有//和%是floor配对，余数符号跟随除数；bool±整数的奇偶规则与整数族/浮点族的宽度提升另有既有契约，B2不得悄悄丢掉lint边界。
lint/lfloat在[09R](09r-bytecode-machine-research.md)中归ObjHandle，不等于公开dynamic；不得为新算术退回无类型槽。

### 3.2 文本与数值身份

[02转换规则](02-type-system.md)与Runtime当前代码规定：

- str到int/sint/lint要求一个Unicode标量；多码点“42”不是数值解析，不能改成42。
- int/sint/lint到str输出十进制文本；整数族到lint精确，lint窄化定宽检查范围。
- lfloat多数转换仍未实现；其一元负号已规范零为“0”，非零仍保留原文本精度。
- 当前Lint/Lfloat的PartialEq与Hash按各自文本实现；数值等价、显示形式及类型标签之间的关系必须同批冻结。

星崽已确认保留非零显示尺度、数值等价与相同哈希；不得继续按“1.5”与“1.50”的原始文本哈希。
合法性与预算必须先在值边界成立；PartialEq/Hash不能因临时数值分配失败而panic或退回默认值。可采用无分配规范视图，避免引入未授权共享数值缓存。

### 3.3 已有错误与版本

诊断本体在[xiao-diagnostics](../../core/rust/crates/xiao-diagnostics/src/lib.rs)，Runtime仅重导出。
现有009为数值溢出/非有限算术错误，013为整数除零，011为受控分配失败，012为invalid_value且当前类别为TypeError。
这些是源码事实，不自动把新精度或数学域问题都归到同一码；错误方案仍按D10确认。

活动intrinsic到32，100为墓碑；新入口的名称/数量与函数签名确定后才列规划编号，不改活动表。
ABI布局、既有借用/拥有与失败输出规则按[Runtime ABI](../../core/rust/crates/xiao-runtime-abi/src/lib.rs)保持。

## 四、依赖表与候选闭包

[上游资料与索引快照](assets/20o-b2-freeze-prep/entry-20261103/candidate-dependencies.json)仅证明候选来源，不是锁定或三平台构建已通过。

| 直接依赖 | 已确认精确版本 | 上游许可证声明 | 已确认特性 | 来源 |
| --- | --- | --- | --- | --- |
| num-bigint | =0.4.8 | MIT OR Apache-2.0，声明Rust1.60 | default-features=false；std | num-bigint-0.4.8，65330056 |
| bigdecimal | =0.4.11 | MIT/Apache-2.0；未声明rust-version | default-features=false；std | v0.4.11，374b2a21 |

bigdecimal要求num-bigint ^0.4，0.5.1不能直接替代并声称共用同一类型。num-integer、num-traits、libm及autocfg的精确闭包仍须列清并锁定。
rand、serde与serde-json不是本批默认所需特性；序列化边界不得直接采用库的私有表示。
尚未下载核验crate归档或修改Cargo.lock，不宣称三平台构建已验证；发布索引的cksum明确是上游公布值。

完整正常/构建闭包的版本候选如下，除已确认的两个直接依赖外，其余仍待确认；不包含上游开发测试依赖。

| 包 | 版本提案 | 角色/特性 | 上游许可证声明 |
| --- | --- | --- | --- |
| num-bigint | 0.4.8（已确认） | 直接，仅std；无rand/serde | MIT OR Apache-2.0 |
| bigdecimal | 0.4.11（已确认） | 直接，仅std；无serde-json | MIT/Apache-2.0 |
| libm | 0.2.16（待确认） | bigdecimal传递依赖及cmath直接依赖；default/arch，无unstable | MIT |
| num-integer | 0.1.47（待确认） | 传递，std/i128 | MIT OR Apache-2.0 |
| num-traits | 0.2.19（待确认） | 传递，std/i128，不额外启用libm特性 | MIT OR Apache-2.0 |
| autocfg | 1.5.1（待确认） | 构建依赖 | Apache-2.0 OR MIT |

libm发布源已迁入rust-lang/compiler-builtins/libm，0.2.16声明Rust1.63；不能误用旧独立仓库的早期标签。
建议直接依赖精确约束、Cargo.lock锁定此闭包并用--locked验证；实际解析若超出该表回冻结，不静默接受新版本。
建议构建子进程清理继承的RUST_BIGDECIMAL_*变量，使锁定上游默认只影响构建字节；Xiao数值语义仍始终显式配置。
不改全局系统环境、并行/优化阈值。此构建策略仍待确认。

bigdecimal的build.rs允许环境改变默认精度（100）、舍入（HalfEven）、指数显示阈值（5/15）与整数填零上限（1000）。
Xiao必须使用已冻结的显式策略；不能调用默认上下文或默认Display后把这些环境值当语言契约。
本仓库仍固定Rust1.96.0，所有实际构建证据留对应实现批次。

## 五、接口与批次落图（待补充裁定）

D4已确认全局自由函数：

```text
lfloat_divide(lfloat numerator, lfloat denominator, int precision, str rounding) -> lfloat
```

四参数必填，支持位置/命名，precision为有效数字，纳入B2-B与cmath一起升级ABI1.19/生成器21。
语言/只返回精确有限十进制；非终止结果要求显式接口，不采用上游100位或全局精度状态。
7种舍入模式、短结果不补零和单次最终舍入已补问，尚待确认：

| 候选rounding文本 | 待审含义 |
| --- | --- |
| half_even | 舍入到最近值，正好中点时保留末位为偶数 |
| half_up | 舍入到最近值，正好中点时远离零 |
| half_down | 舍入到最近值，正好中点时靠近零 |
| toward_zero | 向零舍入 |
| away_from_zero | 有被舍弃的非零部分时远离零 |
| floor | 向负无穷舍入 |
| ceiling | 向正无穷舍入 |

待审有效数字规则：precision表示最多有效数字；精确结果较短时不补零，不制造固定小数位数。
先求精确商余，再只在最终结果按指定模式舍入一次。上述仍是D4补充提案，不因名称与上游相同而自动生效。
上游Context提供invert/multiply而非通用divide；不得先舍入倒数再乘造成双重舍入，精确商余可作实现底座。

其余lfloat运算的补充提案仍待答：首批不开放`//`和`%`；`**`只接受非负整数值指数，
负数/小数指数报冻结的数值域错误，不在B2引入高精度超越运算。加减乘保持精确，不用precision接口替代。

### 5.1 原语与符号精表提案（未确认，未写活动表）

cmath的15项名称及float结果已确认；本表的参数名、cmath_底层名称、规划编号和C符号仍待统一确认。
源码模块只导出15个公开包装，底层原语按现有os_模式另作全局登记，不假装已有私有可见性。

| 规划ID（待审） | 源级入口 | 待审参数名 | 返回/效果 | 待审C ABI符号 |
| --- | --- | --- | --- | --- |
| 33 | 全局lfloat_divide | numerator, denominator, precision, rounding | lfloat / Allocates | xiao_runtime_lfloat_divide |
| 34 | std.cmath.sqrt / 原语cmath_sqrt | float value | float / Pure | xiao_runtime_cmath_sqrt |
| 35 | std.cmath.floor / 原语cmath_floor | float value | float / Pure | xiao_runtime_cmath_floor |
| 36 | std.cmath.ceil / 原语cmath_ceil | float value | float / Pure | xiao_runtime_cmath_ceil |
| 37 | std.cmath.round / 原语cmath_round | float value | float / Pure | xiao_runtime_cmath_round |
| 38 | std.cmath.trunc / 原语cmath_trunc | float value | float / Pure | xiao_runtime_cmath_trunc |
| 39 | std.cmath.abs / 原语cmath_abs | float value | float / Pure | xiao_runtime_cmath_abs |
| 40 | std.cmath.min / 原语cmath_min | float left, float right | float / Pure | xiao_runtime_cmath_min |
| 41 | std.cmath.max / 原语cmath_max | float left, float right | float / Pure | xiao_runtime_cmath_max |
| 42 | std.cmath.pow / 原语cmath_pow | float base, float exponent | float / Pure | xiao_runtime_cmath_pow |
| 43 | std.cmath.sin / 原语cmath_sin | float value | float / Pure | xiao_runtime_cmath_sin |
| 44 | std.cmath.cos / 原语cmath_cos | float value | float / Pure | xiao_runtime_cmath_cos |
| 45 | std.cmath.tan / 原语cmath_tan | float value | float / Pure | xiao_runtime_cmath_tan |
| 46 | std.cmath.exp / 原语cmath_exp | float value | float / Pure | xiao_runtime_cmath_exp |
| 47 | std.cmath.ln / 原语cmath_ln | float value | float / Pure | xiao_runtime_cmath_ln |
| 48 | std.cmath.log10 / 原语cmath_log10 | float value | float / Pure | xiao_runtime_cmath_log10 |

全部无新宿主能力；Pure描述计算副作用，不排除正常错误传播。参数全部必填、沿用现有位置/命名与数值转换规则，不增加默认参数、重载或方法别名。
ID100墓碑和1～32均保持。这里的intrinsic ID与X06-RUNTIME错误编号是不同空间。

候选C ABI统一借读XiaoValue输入、返回i32状态，输出使用已初始化的拥有值槽：

```c
int32_t xiao_runtime_cmath_sqrt(const XiaoValue *value, XiaoValue *out);
int32_t xiao_runtime_cmath_min(const XiaoValue *left, const XiaoValue *right, XiaoValue *out);
int32_t xiao_runtime_lfloat_divide(const XiaoValue *numerator, const XiaoValue *denominator,
    const XiaoValue *precision, const XiaoValue *rounding, XiaoValue *out);
```

其余一元/二元按同形映射。输入强引用覆盖调用，输入与out不重叠；成功释放out旧拥有值并替换结果，失败保留out。
候选状态：0成功；1空指针、3无效句柄不触碰挂起槽；源级类型/参数/域/预算及受控分配错误设置正常挂起错误并返回6。
任意悬空指针不在安全识别承诺内。上述是待审声明，未新增符号或改变布局。

### 5.2 错误精表提案（未确认）

| 场景 | 待审身份 | 约束 |
| --- | --- | --- |
| 内部动态实参类型不符 | TypeError / X06-RUNTIME-002 | 正常类型错误，不混入数值域错误 |
| 数值文本语法、precision范围或rounding名称非法 | TypeError / X06-RUNTIME-012 | 沿用现有invalid_value类别；语法与名称精表仍待确认 |
| 分母为零 | ArithmeticError / X06-RUNTIME-013 | 不调用底层除法制造panic |
| 负整数幂/非法高精度指数、cmath域/非有限结果、定宽转换溢出 | ArithmeticError / X06-RUNTIME-009 | 各操作触发条件仍须域精表 |
| 数值接纳、结果或中间工作预算超限 | ResourceError / X06-RUNTIME-033（规划） | 与实际分配失败分开；未写诊断表 |
| 高精度语言/出现非终止十进制 | ArithmeticError / X06-RUNTIME-034（规划） | 引导显式precision接口；未写诊断表 |
| 受控输出分配失败 | ResourceError / X06-RUNTIME-011 | 源值/别名不改，无部分结果 |
| 无法安全建错的真实OOM | 沿用Fatal边界 | 第三方内部OOM不承诺可恢复；报告路径与现有Fatal规则另行复核 |

候选优先级：调用/类型→precision与rounding参数域→按参数顺序的输入预算与合法性→除零/运算域→工作/结果预算→受控结果分配。
文本内部按左到右首次违规处理；原文字节上限先于内容解析。此顺序与新033/034均需星崽确认，不影响B1七库既有错误表。

## 六、资源数值与性能待审表

资源数值已由星崽确认；前导零/规范化/接纳公式与错误顺序仍待审。表中运算工作量和性能数值尚未确认，不是已有成绩。

| 维度 | 数值或待审提案 | 接纳含义 |
| --- | --- | --- |
| 数值文本（数值已确认） | 最多65,536 UTF-8字节，解析语法候选仅ASCII十进制 | 原文仅预检，不保证规范值或结果必然接纳 |
| 系数/整数位数（数值已确认） | 输入及最终结果最多32,768十进制位 | 不含符号；零算1位；实际定义须覆盖前导零和规范化 |
| scale与显式指数（数值已确认） | 绝对值各最多100,000 | 规范scale也须独立检查，不能仅看原文字段 |
| 中间系数（数值已确认） | 最多65,536十进制位的工作预算 | 分配/对齐/幂前估算；不能先展开巨型指数再拒绝 |
| 显式除法precision（数值已确认） | 1～32,768有效数字 | 舍入模式、零值和规范输出另冻；不使用库默认100 |
| 幂指数（数值已确认） | 非平凡底数上限1,000,000，另受结果预算限制；0/±1保留快路径 | lint负指数已确认拒绝；lfloat指数开放域、快路径及检查顺序仍待审 |
| 单次算法上界（待确认） | 幂候选最多64次大数乘法；其它按输入/输出规模界定 | 不把指数值当循环次数；不能省略内部大数工作量 |
| 测量预算（待确认） | 单次VM≤10秒，原生≤2秒，驻留增量上界≤64MiB | 受控验收，不是API超时或任意硬件保证 |
| 翻倍预算（待确认） | 线性操作≤3；乘/除/幂等超线性负载候选≤5 | 必须按运算及实际输出规模分组，不盲用B1的线性模型 |

scale、指数、系数与结果预算共同决定接纳；例如小系数加极大指数并不允许展开成超预算整数。
固定输入组合还要覆盖balanced乘法最大结果、密集长除法、2/5因子有限除法、非终止除法、不同scale对齐和强消去，不能只测乘1或小指数。
第三方大数临时分配不自动受Xiao对象计数器管理；OS驻留测量、受控输出分配失败与真实OOM通路必须分开冻结/验证，不能承诺未证的可恢复性。

待审测量协议：每组一次预热、五批计时取中位数；短操作每批累计至少50毫秒，校准增长的累计计时最多100毫秒、单次增加重复数最多16倍。
记录每批调用数与总耗时，再换算单次耗时；原生O0/O3分别留样，不把编译耗时计入运算时限。
这些协议和上表性能数值仍待答复；当前没有B2功能或性能样本。

## 七、cmath精细行为与转换提案（待确认）

| 函数组 | 候选数学/特殊值规则 | 候选数值验收 |
| --- | --- | --- |
| sqrt | x<0报009；sqrt(-0)为-0 | 正确舍入，0 ULP |
| floor/ceil/trunc/round | 有限输入；round最近偶数；保留应有负零 | 逐位精确 |
| abs/min/max | abs(-0)=+0；min零取-0，max零取+0 | 逐位精确 |
| sin/cos/tan | 弧度；有限输入；tan不设epsilon伪极点 | 普通非零结果≤4 ULP |
| exp | 非有限结果报009，IEEE下溢可到+0 | 普通非零结果≤4 ULP |
| ln/log10 | x≤0报009；1的结果为+0 | 普通非零结果≤4 ULP |
| pow | 0**0=1；负底仅整数指数；0的负幂/非有限结果报009 | 普通非零结果≤4 ULP，特殊值精确 |

参考输入必须按实际float位模式解释，不能把π/2的近似值当成精确实数极点；不能用被测libm生成同算法黄金。
参考值按独立高精度数据正确舍入到float，生成工具/版本/来源仍需留档。零/非零分类和符号先精确核对，不能用4 ULP放过错误的flush-to-zero或符号。
子正规非零结果同样按最多4个相邻float间距；参考零必须返回正确有符号零，错误身份不使用数值容差。
候选将cmath.pow与float的**静态/动态路径接入同一数学内核，避免两个实现各自给域/末位规则；sfloat既有路径单独保持，不冒充已获cmath支持。

数值转换候选：整数族到lfloat精确，float/sfloat到lfloat保留二进制实际值，不通过最短显示文本；
lfloat/lint显式窄化float/sfloat按最近偶数，超范围/非有限报009，允许按IEEE舍入为带符号零。
小数显式转lint/int/sint向零截断已确认，定宽结果检查范围，lint受资源预算；str到整数继续是码点。
其余定宽旧规则保持。跨类型比较按已有提升后的值做精确比较；同种lfloat的Eq/Hash规范化数值，其他结构类型规则不自行扩大。

## 八、文本、显示与结果scale定义提案（待确认）

### 8.1 输入与资源计量

str到lfloat的候选语法为ASCII十进制：可带正负号、可前导零、支持.5与5.、e/E及指数正负号，
下划线仅允许在同一数字段的相邻数字之间；拒绝空白、NaN/Inf、非ASCII数字与其它前后缀。
源码数值字面量仍按既有词法规则处理，不把这个字符串解析规则扩展成新语法。

设去符号与小数点/下划线后的整数系数为c，去前导零但保留有意义尾零，显示尺度s=小数位数-显式指数。
零统一为c=0、s=0，输出“0”。非零数值身份再去掉c的尾零，同时相应减少scale，作为Eq/Hash的规范键。
系数位数按|c|的十进制位计，去前导零、保留尾零，零为1位；这明确D9数值如何作用于显示精度。

为保证可表示和往返，建议显示scale s与数值身份的规范scale都独立满足|值|≤100,000；
显式指数也≤100,000，原文≤65,536字节，输入/最终系数≤32,768位，中间工作系数≤65,536位。
“显示scale也受限”是本节补充提案，尚未视为原数值答复已涵盖。

预算共同决定接纳，不能把任何单项上限写成运算必然成功。例如很多尾零要求保留时，乘法结果的显示系数可能超限，即使规范数值很简单仍应报资源错误，不可偷偷丢精度绕过。

### 8.2 确定显示与往返

建议始终保留非零c/s所表示的精度：s≥0且普通十进制输出≤65,536字节时用普通格式；
否则采用整数系数c乘10的-s次方的指数形式，不强制科学记法只留一位整数。表示scale和指数同界后，输出指数也在界内。
符号省略正号、指数用小写e且省正号，省无意义前导零；零仍为“0”。例如1.50保持1.50，1e100000保持紧凑形式。
这样不会把合法“大系数+指数”强制改成超过输入指数上限的文本，也不展开巨型指数。

### 8.3 运算结果的显示scale

| 操作 | 候选结果显示规则 |
| --- | --- |
| 加/减 | 以max(s_left,s_right)保留显示尺度，精确对齐和结果预算共同检查 |
| 乘 | s_left+s_right；不得为通过结果预算自行舍入/删尾零 |
| 语言精确/ | 精确有限结果；取max(s_left-s_right,规范结果scale)，补到必要的显示尺度 |
| 显式lfloat_divide | precision为最多有效数字；一次最终舍入后使用规范结果，不人为补齐precision |
| 非负整数幂 | n=0返回规范1；n=1保留原尺度；其它0/±1快路径返回规范结果；非平凡底数为s_base*n并受预算限制 |
| 比较/相等/哈希 | 以规范数值比较，不让显示精度改变数值身份 |

例如语言1.00/2保留0.50，显式precision接口可返回0.5；数值相等、哈希相同。
所有规范与显示规则须先确认，不能把上游默认Display或环境变量的输出当作答案。

### 8.4 转换矩阵补充提案

六种数值标量之间明确列出方向：同类型保持值；整数族到lint/lfloat精确；
浮点到整数显式向零截断，定宽检查范围；float/sfloat到lfloat精确保留二进制实际值；
lint/lfloat到float/sfloat显式最近偶数，非有限/越界拒绝。既有定宽规则不扩大。

建议新增float/sfloat到str时沿用已有print/display_text的有限值表示（包括负零符号）；
直接str到float/sfloat继续不开放，十进制解析统一经lfloat后显式窄化。
str到int/sint/lint继续是码点，str到lfloat是新十进制解析，lfloat到str按8.2。这个补充仍待确认。

### 8.5 实施与精确执行点提案

A保持ABI1.18/生成器20作为中间验证快照，B2完成交付的最终新产物统一1.19/21。
旧1.18的B1 .xiaoc/.xar做兼容重放，原生对象缓存重生成，新产物在旧Runtime上执行前拒绝；方案仍待补充裁定。

建议复用现有VM/原生两个逻辑精确执行入口，追加独立B2语料，保留全部B1场景和各自标记，
四个实际路径仍恰好1 passed，原28/28守卫保持；不是只在ignored批量里偷跑新样例。
cmath产品fixture按D20放tests/fixtures/docs/cmath/，数值运算/转换/错误语料放独立B2目录，
默认契约测试另核对真实导出、移除、归档和普通包隔离。实施阶段才写这些测试，当前不创建伪执行证据。

## 九、仍须逐项完成

1. D4七种舍入、最多有效数字、短结果不补零与单次最终舍入（§五）。
2. D7的cmath域、负零、下溢、角度与round/pow/tan精细规则（§七）。
3. D9性能目标、工作量与测量协议；已确认的资源数值不在待答范围（§六）。
4. D4的lfloat `// % **`开放域（§五）。
5. D6/D8原语参数名、规划ID33～48、效果与C ABI精表（§5.1）。
6. D8中间快照、旧产物兼容及新产物拒绝方案（§8.5）。
7. D1其余四包精确版本/许可闭包、锁文件及构建环境策略（§四）。
8. D5二进制浮点与lfloat的精确承接、反向舍入与下溢（§七/§8.4）。
9. D11每函数误差、独立参考与cmath.pow/float `**`共享内核（§七）。
10. D2/D9系数/显示scale/规范scale定义、输出和运算结果scale（§8.1～§8.3）。
11. D5文本语法与数值到str的转换补充（§8.1/§8.4）。
12. D10新033/034及全场景错误身份/优先级（§5.2）。

这十二组补充问题已提出，尚未收到答复；不重复询问已确认的十八条答复，也不把待审表静默采纳。
实际构建、序列化承载和教程完整示例留实施批次逐项验证；cmath教程与registry当前仅planned。

以上未收口，B2-A/B保持未开工；19/11/22及历史22C/22D失败仍独立保留。
