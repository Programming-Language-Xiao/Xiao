# 20O-B2-FREEZE. 数值底座冻结草案与裁定台账

状态：30条裁定全部confirmed、0 pending；F-A/F-B/F-C已完成，F-D仍待单独签字，整体尚未联合生效（2026-11-05）。
逐项顺序见[冻结收口规划](20o-b2-freeze-closeout-plan.md)；F-A的七项裁定及退出见[数值语义规划](20o-b2-freeze-a-numeric-semantics-plan.md)与[F-A收口](20o-b2-freeze-a-closeout-plan.md)，F-5已由[F-B1收口](20o-b2-freeze-b1-closeout-plan.md)完成，F-6见[版本矩阵规划](20o-b2-freeze-b2-version-matrix-plan.md)，F-12见[错误精表规划](20o-b2-freeze-b3-error-table-plan.md)；F-C见[依赖、资源与性能裁定规划](20o-b2-freeze-c-resource-dependency-plan.md)。本页保存精确台账。
来源为[数值基础规划](20o-b2-numeric-foundation-plan.md)与本次会话[逐条答复记录](assets/20o-b2-freeze-prep/entry-20261103/decision-receipts.json)；本页的“建议/待审”不等于已确认。
F-A各项直接答复另存[本轮裁定原文](assets/20o-b2-freeze-a/entry-20261104/decision-receipts.json)，不覆盖既有18条记录。
F-5a/F-5b/F-5c的三条直接确认另见[F-B1答复原文](assets/20o-b2-freeze-b/entry-20261104/decision-receipts.json)。
F-6a/F-6b/F-6c的三条直接确认另见[F-B2答复原文](assets/20o-b2-freeze-b2/entry-20261105/decision-receipts.json)。
F-12a/F-12b/F-12c的三条直接确认另见[F-B3答复原文](assets/20o-b2-freeze-b3/entry-20261105/decision-receipts.json)。
F-3a/F-3b/F-3c与F-7a/F-7b的五条直接确认见[F-C答复原文](assets/20o-b2-freeze-c/entry-20261105/decision-receipts.json)；资源、工作量、性能协议和依赖策略均已确认。
F-C退出时精确裁定已收齐、F-D尚未签字；本文正文在F-D提问前固定，当前签字状态只以页首状态行、台账和[独立签字记录](assets/20o-b2-freeze-d/entry-20261105/README.md)为准。
F-D只可能开放B2-A实施规划；产品、锁文件、构建与性能实测仍留后续实施批。
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

| 编号 | 已确认内容 | 后续实施与验收 |
| --- | --- | --- |
| B2-D1 | 纯Rust num-bigint + bigdecimal；精确版本=0.4.8/=0.4.11、仅std与双许可留档；num-rational留B5，不引入C/GMP | F-7已确认六包版本/特性/来源与锁定策略；实际解析、锁文件和构建留实施批次 |
| B2-D2 | 保留Lint/Lfloat文本标签与C ABI边界；非零显示精度、规范数值身份和无共享缓存已确认；F-10已确认系数/scale、零和结果显示表 | 产品值承载与序列化在实现批验证，不把文档确认当作能力已交付 |
| B2-D3 | lint精确运算、floor整除/余数、除零、非负幂已确认；跨宽度提升后按F-8精确承接二进制值，不经f64近似大整数比较；F-12错误表已确认 | F-3已确认接纳/工作量；实际产品与性能验证留实施批 |
| B2-D4 | 四参lfloat_divide、精确有限/、七种舍入及结果scale已确认；F-4确认首批同时开放lfloat //和%，以及非负整数值指数的**；F-5原语/ABI及F-12错误精表已确认 | F-3已确认接纳/工作量；实际产品与性能验证留实施批 |
| B2-D5 | 码点兼容、整数截断、十进制文本已确认；F-8确认实际二进制值承接、最近偶数、舍入后有限性及转换显示尺度 | 实现和三平台验证留B2后续批次 |
| B2-D6 | cmath首批15项float参数/结果；sfloat后置；F-2确认逐函数行为、F-9确认误差和共享内核；F-5确认原语参数名/编号/符号及ABI契约 | 产品接线与三平台证据留实现批；文档确认不等于活动能力 |
| B2-D7 | 拒绝NaN/Inf；F-2已确认定义域、负零、IEEE下溢、最近偶数及pow/tan边界 | 产品实现与三平台差分留B2后续批次 |
| B2-D8 | B2-A保持ABI1.18且不新增intrinsic；B2-B目标ABI1.19/生成器21、标准库接口1；F-5原语/所有权及F-6中间快照/版本拒绝/缓存/C二进制兼容矩阵已确认；A若需突破先回冻结 | 实际接线与版本拒绝执行证据留实现批，不把规划契约当成活动能力 |
| B2-D9 | B2-C必须完整重跑721条F10；资源数值已确认；F-10补齐显示/规范系数与scale计量；F-12错误顺序已确认 | F-3已确认接纳/工作量/性能协议；实际测量留实施批 |
| B2-D10 | 纯计算无新能力；F-12确认规划033/034、旧002/009/011/012/013边界、完整优先级及状态6映射 | 活动诊断表和错误路径留实施批，不把已确认契约当成已实现 |
| B2-D11 | Rust libm方向已确认；F-9确认逐函数误差、独立离线参考和pow共享内核 | F-7已确认libm0.2.16与闭包；实际向量生成与平台证据留实现批 |
| B2-D12 | 已确认冻结时cmath planned教程/registry，lint/lfloat更新语言说明；B2-C证据全过才verified，不新增大数标准库 | 教程落点、完整示例与状态升级点 |

D4显式接口已与D8落在B2-B，不能在B2-A里新增intrinsic。余下补充裁定仍逐条记录。
收口阶段将既有18条与F-1～F-12逐项列入[30条台账](assets/20o-b2-freeze-close/entry-20261103/decision-ledger.json)。
F-1～F-12与既有18条全部confirmed，总计30/0。F-C退出时F-D签字闸门尚未通过；后续是否激活以实际签字记录及页首状态为准，文档裁定不等于活动产品能力。

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
这些是源码事实；F-12已另行确认§5.2的新场景映射，不改变上述活动定义。

活动intrinsic到32，100为墓碑；新入口的名称/数量与函数签名确定后才列规划编号，不改活动表。
ABI布局、既有借用/拥有与失败输出规则按[Runtime ABI](../../core/rust/crates/xiao-runtime-abi/src/lib.rs)保持。

## 四、依赖表与冻结闭包（F-7已确认，尚未解析）

[上游资料与索引快照](assets/20o-b2-freeze-prep/entry-20261103/candidate-dependencies.json)仅证明候选来源，不是锁定或三平台构建已通过。

| 直接依赖 | 已确认精确版本 | 上游许可证声明 | 已确认特性 | 来源 |
| --- | --- | --- | --- | --- |
| num-bigint | =0.4.8 | MIT OR Apache-2.0，声明Rust1.60 | default-features=false；std | num-bigint-0.4.8，65330056 |
| bigdecimal | =0.4.11 | MIT/Apache-2.0；未声明rust-version | default-features=false；std | v0.4.11，374b2a21 |

bigdecimal要求num-bigint ^0.4，0.5.1不能直接替代并声称共用同一类型。num-integer、num-traits、libm及autocfg的精确闭包已由F-7确认，实际锁定留实施批。
rand、serde与serde-json不是本批默认所需特性；序列化边界不得直接采用库的私有表示。
收口阶段已实际下载六包归档，SHA-256均与原索引公布值一致，并保存11份上游许可证原文；
见[归档与许可核验](assets/20o-b2-freeze-close/entry-20261103/dependency-archives.json)。
这是来源核验，四个传递/数学依赖的版本已由F-7确认；未做Cargo解析、锁定或三平台构建，Cargo.toml/Cargo.lock未改。

完整正常/构建闭包已由F-7a确认如下，不包含上游开发测试依赖；归档/清单/11份许可复核见[F-C来源记录](assets/20o-b2-freeze-c/entry-20261105/dependency-review.json)。

| 包 | 已确认版本 | 角色/特性 | 上游许可证声明 |
| --- | --- | --- | --- |
| num-bigint | 0.4.8（已确认） | 直接，仅std；无rand/serde | MIT OR Apache-2.0 |
| bigdecimal | 0.4.11（已确认） | 直接，仅std；无serde-json | MIT/Apache-2.0 |
| libm | 0.2.16（已确认） | bigdecimal传递依赖及cmath直接依赖；default/arch，无unstable | MIT |
| num-integer | 0.1.47（已确认） | 传递，std/i128 | MIT OR Apache-2.0 |
| num-traits | 0.2.19（已确认） | 传递，std/i128，不额外启用libm特性 | MIT OR Apache-2.0 |
| autocfg | 1.5.1（已确认） | 构建依赖 | Apache-2.0 OR MIT |

libm发布源已迁入rust-lang/compiler-builtins/libm，0.2.16声明Rust1.63；不能误用旧独立仓库的早期标签。
F-7b确认：直接依赖精确约束、Cargo.lock锁定六包版本及校验值，维护构建用--locked；实际解析新增包、版本或特性超出该表时回裁定，不静默接受漂移。
F-7b确认：受控构建子进程清理继承的RUST_BIGDECIMAL_*变量，Xiao数值语义始终显式配置，不使用上游默认上下文/Display代替语言规则。
不改全局系统环境、并行/优化阈值。构建记录绑定工具链/目标、环境清理、锁文件及产物摘要；未实施前不称已锁定或已构建。

bigdecimal的build.rs允许环境改变默认精度（100）、舍入（HalfEven）、指数显示阈值（5/15）与整数填零上限（1000）。
Xiao必须使用已冻结的显式策略；不能调用默认上下文或默认Display后把这些环境值当语言契约。
F-7b确认沿用固定Rust1.96.0及现有三平台目标；workspace的rust-version=1.85声明不改，不推断未实测MSRV兼容。上游bigdecimal未声明rust-version不能作为兼容性证据；所有实际构建证据留对应实现批次。

## 五、已确认接口与后续实施落图

D4已确认全局自由函数：

```text
lfloat_divide(lfloat numerator, lfloat denominator, int precision, str rounding) -> lfloat
```

四参数必填，支持位置/命名，precision为有效数字，纳入B2-B与cmath一起升级ABI1.19/生成器21。
语言/只返回精确有限十进制；非终止结果要求显式接口，不采用上游100位或全局精度状态。
F-1已确认7种舍入模式、短结果不补零和单次最终舍入：

| 已确认rounding文本 | 精确含义 |
| --- | --- |
| half_even | 舍入到最近值，正好中点时保留末位为偶数 |
| half_up | 舍入到最近值，正好中点时远离零 |
| half_down | 舍入到最近值，正好中点时靠近零 |
| toward_zero | 向零舍入 |
| away_from_zero | 有被舍弃的非零部分时远离零 |
| floor | 向负无穷舍入 |
| ceiling | 向正无穷舍入 |

已确认有效数字规则：precision表示最多有效数字；精确结果较短时不补零，不制造固定小数位数。
先求精确商余，再只在最终结果按指定模式舍入一次；不继承上游默认精度或舍入。
例如precision=1时，2.5在half_even/half_up/half_down下分别为数学值2/3/2，
-2.5分别为-2/-3/-2；9.99在precision=2、half_even下为数学值10。显示文本另由F-10确定。
上游Context提供invert/multiply而非通用divide；不得先舍入倒数再乘造成双重舍入，精确商余可作实现底座。

F-A1已确认规则的设计算例（不是Xiao产品执行结果）：以下precision=1，单元格为数学值，显示格式按F-10。

| rounding | 2.5 | -2.5 | 2.1 | -2.1 |
| --- | --- | --- | --- | --- |
| half_even | 2 | -2 | 2 | -2 |
| half_up | 3 | -3 | 2 | -2 |
| half_down | 2 | -2 | 2 | -2 |
| toward_zero | 2 | -2 | 2 | -2 |
| away_from_zero | 3 | -3 | 3 | -3 |
| floor | 2 | -3 | 2 | -3 |
| ceiling | 3 | -2 | 3 | -2 |

其余F-1算例：9.99按2位half_even得到数学值10；1/2按32,768位仍是0.5，不补零；
3/8按2位half_even应为0.38。若先把1/8舍入为0.12再乘3，将得到0.36，说明先舍入倒数再乘不满足已确认的单次舍入规则。
precision=0或32,769超出已确认范围；F-12确认此类参数域错误为TypeError/012，优先级见§5.2。

F-4已确认首批同时开放lfloat的`//`和`%`，旧“不开放”建议不再适用。
精确规则为q=floor(a/b)、r=a-b*q，两者返回lfloat；非零余数满足|r|<|b|且符号随b。
商显示scale=0，非零余数显示scale=max(s_a,s_b)，零统一为0；除零沿用ArithmeticError/X06-RUNTIME-013。
两个算子独立核对自身工作与结果预算；%不要求先物化完整商，不能仅因//的结果超预算就自动否定可有界计算的余数。
F-3已确认它们的资源接纳公式与性能分组（§6.1/§6.2）；F-12已确认完整错误优先级（§5.2）。
F-4混合类型准入已由星崽直接答复“采用推荐规则”：沿用既有数值提升，提升结果为lfloat即允许；两个整数仍返回整数族。
仅提升到float/sfloat的组合继续拒绝，bool、str等不新增数值转换。下表同时适用于//和%，列出返回类型；“拒绝”是静态类型边界，不是运行后降级。

| 左实参 \ 右实参 | sint | int | lint | sfloat | float | lfloat |
| --- | --- | --- | --- | --- | --- | --- |
| sint | sint | int | lint | 拒绝 | 拒绝 | lfloat |
| int | int | int | lint | 拒绝 | 拒绝 | lfloat |
| lint | lint | lint | lint | lfloat | lfloat | lfloat |
| sfloat | 拒绝 | 拒绝 | lfloat | 拒绝 | 拒绝 | lfloat |
| float | 拒绝 | 拒绝 | lfloat | 拒绝 | 拒绝 | lfloat |
| lfloat | lfloat | lfloat | lfloat | lfloat | lfloat | lfloat |

例如lfloat//int、lint//float、sfloat%lint进入lfloat精确路径；int//sint保持int；float//int仍拒绝。
这是B2后续实现的类型契约，不宣称当前尚未修改的类型检查器已接受这些新组合。

F-4也已确认`**`只接受非负整数值指数：1.0合法，负数和非整数拒绝；0**0=1、零的正幂为0。
非平凡底数指数上限1,000,000且受结果预算；0/±1快路径仍先检查输入合法性与预算，不引入高精度超越运算。

| F-4已确认语义的设计算例 | q=a//b | r=a%b |
| --- | --- | --- |
| a=5.5，b=2 | 2 | 1.5 |
| a=-5.5，b=2 | -3 | 0.5 |
| a=5.5，b=-2 | -3 | -0.5 |
| a=-5.5，b=-2 | 2 | -1.5 |
| a=1.50，b=1.00 | 1 | 0.50 |
| a=20.00，b=2 | 10 | 0 |

这些是约定输出而非产品执行结果。两算子均为精确运算，不调用显式precision/rounding接口，不通过f64计算floor或余数。

### 5.1 原语与符号精表（F-5已确认，仍不写活动表）

cmath的15项名称及float结果已确认；F-5进一步确认参数名、cmath_底层名称、规划编号和C符号。
源码模块只导出15个公开包装，底层原语按现有os_模式另作全局登记，不假装已有私有可见性。

| 规划ID（F-5确认） | 源级入口 | 参数名 | 返回/效果 | C ABI符号 |
| --- | --- | --- | --- | --- |
| 33 | 全局lfloat_divide | lfloat numerator, lfloat denominator, int precision, str rounding（源级已确认） | lfloat / Allocates | xiao_runtime_lfloat_divide |
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

全部无新宿主能力；Pure描述计算副作用，不排除正常错误传播，不等于LLVM的无错或readnone承诺。上述命名、效果和能力由F-5确认。
未证明不会报错时，优化器不得仅因结果未使用而删除调用。参数全部必填、沿用现有位置/命名与数值转换规则，不增加默认参数、重载或方法别名。
ID100墓碑和1～32均保持。这里的intrinsic ID与X06-RUNTIME错误编号是不同空间。

F-5确认的C ABI统一借读XiaoValue输入、返回i32状态，输出使用已初始化的拥有值槽：

```c
int32_t xiao_runtime_lfloat_divide(const XiaoValue *numerator, const XiaoValue *denominator,
    const XiaoValue *precision, const XiaoValue *rounding, XiaoValue *out);
int32_t xiao_runtime_cmath_sqrt(const XiaoValue *value, XiaoValue *out);
int32_t xiao_runtime_cmath_floor(const XiaoValue *value, XiaoValue *out);
int32_t xiao_runtime_cmath_ceil(const XiaoValue *value, XiaoValue *out);
int32_t xiao_runtime_cmath_round(const XiaoValue *value, XiaoValue *out);
int32_t xiao_runtime_cmath_trunc(const XiaoValue *value, XiaoValue *out);
int32_t xiao_runtime_cmath_abs(const XiaoValue *value, XiaoValue *out);
int32_t xiao_runtime_cmath_min(const XiaoValue *left, const XiaoValue *right, XiaoValue *out);
int32_t xiao_runtime_cmath_max(const XiaoValue *left, const XiaoValue *right, XiaoValue *out);
int32_t xiao_runtime_cmath_pow(const XiaoValue *base, const XiaoValue *exponent, XiaoValue *out);
int32_t xiao_runtime_cmath_sin(const XiaoValue *value, XiaoValue *out);
int32_t xiao_runtime_cmath_cos(const XiaoValue *value, XiaoValue *out);
int32_t xiao_runtime_cmath_tan(const XiaoValue *value, XiaoValue *out);
int32_t xiao_runtime_cmath_exp(const XiaoValue *value, XiaoValue *out);
int32_t xiao_runtime_cmath_ln(const XiaoValue *value, XiaoValue *out);
int32_t xiao_runtime_cmath_log10(const XiaoValue *value, XiaoValue *out);
```

上面列齐16个已确认的规划C符号，无额外Cmath入口。输入强引用覆盖调用，输入与out不重叠；成功释放out旧拥有值并替换结果，失败保留out。
语言侧按既有规则完成允许的类型转换；这些C ABI入口按降低后的目标标签检查参数，不自行新增数值提升或窄化。
状态表（F-5确认；F-12已统一具体错误身份，见§5.2）：

| 状态 | 触发条件 | out与挂起槽 |
| --- | --- | --- |
| 0 | 成功 | 释放并替换旧out；不清空既有挂起错误 |
| 1 | 必需输入/out指针为空 | 保持可写out原值，不触碰挂起槽 |
| 2 | 未知XiaoValue标签等ABI结构非法 | 保持out，不触碰挂起槽 |
| 3 | 句柄无效或种类错配 | 保持out，不触碰挂起槽 |
| 6 | 已知有效标签的实参类型不符，或参数/数学域/预算/受控分配失败 | 保持out，设置对应的正常可恢复错误 |
| 8/宿主致命终止 | 既有Fatal边界下的不可恢复故障 | 不承诺可恢复，不能伪装成普通状态6 |

有效标签但参数类型错误属于002/状态6，与未知标签的状态2分开；所有保持out的保证都要求调用方满足已初始化、拥有、无重叠的前置条件。
任意悬空指针不在安全识别承诺内。上述只冻结规划契约，未新增符号、改变布局或写入活动表。

### 5.2 错误精表（F-12已确认，活动诊断表未改）

F-12a/F-12b/F-12c已收到三条实际确认；下列为后续实现的唯一错误契约，不是活动错误路径的执行成绩。

| 场景 | 已确认身份 | 常量 / message_id与边界 |
| --- | --- | --- |
| 内部动态调用/实参类型不符 | TypeError / X06-RUNTIME-002 | TYPE_MISMATCH_CODE / runtime.type_mismatch；有效标签的错误类型不等于ABI未知标签 |
| 数值文本不符合§8.1语法；precision不在1～32,768；rounding不为§五的七个精确名称 | TypeError / X06-RUNTIME-012 | INVALID_VALUE_CODE / runtime.invalid_value；沿用既有类别，不新造ValueError |
| lint/lfloat的 /、//、%或lfloat_divide分母为零 | ArithmeticError / X06-RUNTIME-013 | DIVISION_BY_ZERO_CODE / runtime.division_by_zero；在调用底层除法前检查；cmath.pow零的负幂按009 |
| lint负指数、lfloat非整数值/负指数；cmath非有限输入、函数域或非有限结果；定宽转换溢出 | ArithmeticError / X06-RUNTIME-009 | NUMERIC_OVERFLOW_CODE / runtime.numeric_overflow；域和转换边界依F-2/F-4/F-8，不将正常子正规数或有符号零当溢出 |
| 原文字节、输入系数/指数/scale、工作或最终结果预算超限 | ResourceError / X06-RUNTIME-033（规划） | NUMERIC_LIMIT_CODE / runtime.numeric_limit；已确认数值与F-3接纳算法保持；不是实际分配失败 |
| 合法非零分母下lint/lfloat语言 / 的精确商为非终止十进制 | ArithmeticError / X06-RUNTIME-034（规划） | NON_TERMINATING_DIVISION_CODE / runtime.non_terminating_division；显式lfloat_divide按precision舍入，不因1/3触发034；//和%不使用034 |
| 受控输出分配失败 | ResourceError / X06-RUNTIME-011 | ALLOCATION_CODE / runtime.allocation；源值/别名不改，无部分结果；不是预算拒绝 |
| 无法安全建错的真实OOM | 既有Fatal / X07-FATAL-003 | FATAL_OUT_OF_MEMORY_CODE / fatal.out_of_memory；第三方内部OOM不承诺可恢复，沿用既有终止/报告边界 |

033/034只冻结规划身份，不占用活动诊断表；源级错误码、intrinsic ID和C ABI状态是三个编号空间。

已确认入口内部优先级如下：

| 次序 | 检查 | 失败身份 |
| --- | --- | --- |
| 1 | 动态调用/类型；静态非法类型仍走既有类型诊断 | 002 |
| 2 | lfloat_divide先precision、再rounding参数域 | 012 |
| 3 | 按形参顺序逐个完成输入预算与合法性；数值文本内部先原文字节上限，再从左到右语法，再系数/显式指数/显示与规范scale预算 | 原文或数值预算033；语法012 |
| 4 | 除零或F-2/F-4/F-8的数学域、非有限、转换溢出检查 | 013或009，按操作适用 |
| 5 | 仅精确语言 / 判断非终止十进制 | 034 |
| 6 | 工作与最终结果预算 | 033 |
| 7 | 受控输出分配 | 011 |

此顺序由F-12b确认，接纳算法与性能协议另由F-3实际答复确认（§6.1/§6.2）；预算失败不返回部分结果。
这是实参已求值后的入口内部顺序，不重排现有表达式求值；构造实参时已经发生的解析错误先按该构造入口传播。
契约示例（产品尚未执行）：precision=0且分母=0先报012；原文超过65,536字节即报033，不被后面的坏字符改成012；
长度合格的文本即使含超预算长系数，若语法非法仍先012；语法合格的32,769位系数才报033。
合法输入1/3报034，1/0报013；显式lfloat_divide(1,3,3,"half_even")约定为0.333；cmath.ln(0)为009。
真实OOM不按可恢复检查排序延后处理；无法安全建错时仍沿既有Fatal边界。不能把可捕获的数值错误提前折叠成不可捕获编译错误。

F-12c已确认映射：002/009/011/012/013/033/034均走F-5的状态6，设置对应可恢复挂起错误且保留out；
1/2/3结构错误保持out且不碰挂起槽；成功0释放替换旧out、不清既有挂起错误；8/Fatal只沿既有不可恢复边界。
输入借读且强引用覆盖调用，out必须已初始化、拥有且与输入不重叠。保持out的保证不扩展为识别任意悬空指针。
以上均为已确认的规划契约，不代表新C符号或错误路径已有实现，也不改变B1七库既有错误表。

## 六、资源数值与性能契约（F-3已确认，尚未实测）

资源数值、F-A系数/scale计量、F-12错误顺序及F-3接纳/工作量/性能协议均已确认；不是已有性能成绩。

| 维度 | 已确认数值/规则 | 接纳含义 |
| --- | --- | --- |
| 数值文本（数值已确认） | 最多65,536 UTF-8字节，解析语法按已确认§8.1的ASCII十进制 | 原文仅预检，不保证规范值或结果必然接纳 |
| 系数/整数位数（已确认） | 输入及最终结果最多32,768十进制位 | 不含符号，去前导零并保留显示尾零；零算1位，详§8.1 |
| 规范/显示scale与显式指数（已确认） | 绝对值各最多100,000 | F-10补充确认显示scale也独立受限，不能仅看原文字段 |
| 中间系数（数值已确认） | 最多65,536十进制位的工作预算 | 分配/对齐/幂前估算；不能先展开巨型指数再拒绝 |
| 显式除法precision（已确认） | 1～32,768有效数字 | F-1/F-10已确认舍入、零和输出规则，不使用库默认100 |
| 幂指数（已确认） | 非平凡底数上限1,000,000，另受结果预算限制；0/±1保留快路径 | F-4确认lfloat非负整数值指数、F-12确认域与预算顺序；接纳算法已由F-3确认，见§6.1 |
| 单次算法上界（已确认） | 幂最多64次大数乘法；商余及其它工作模型见§6.1 | 不把指数值当循环次数；不能省略内部大数工作量 |
| 测量预算（已确认） | 单次VM≤10秒，原生≤2秒，驻留增量上界≤64MiB | 受控验收，不是API超时或任意硬件保证 |
| 翻倍预算（已确认） | 线性操作≤3；乘/除/gcd/幂/模幂等超线性负载≤5 | 必须按运算及实际输出规模分组，不盲用B1的线性模型 |

scale、指数、系数与结果预算共同决定接纳；例如小系数加极大指数并不允许展开成超预算整数。
固定输入组合还要覆盖balanced乘法最大结果、密集长除法、2/5因子有限除法、非终止除法、不同scale对齐和强消去，不能只测乘1或小指数。
第三方大数临时分配不自动受Xiao对象计数器管理；OS驻留测量、受控输出分配失败与真实OOM通路必须分开冻结/验证，不能承诺未证的可恢复性。

F-3c已确认的测量协议以§6.2为准：VM/Runtime使用release构建，原生Xiao O0/O3分记；五批各在独立进程中先预热一次再采正式样本。
时间按repeat归一化后取五批中位数；OS驻留增量取各批保守峰值的最大值，不用中位数掩盖超限。当前没有B2功能或性能样本。

### 6.1 资源接纳与工作量精表（F-3a/F-3b已确认）

本节已由F-3a/F-3b实际答复确认；已确认资源数值、F-10显示规则及F-12错误顺序保持不变。
记D=32,768、W=65,536、H=100,000。数值写为c×10^(-s)，d为去前导零、保留显示尾零的系数位数，d(0)=1；
规范键另去尾零并调整scale。D约束输入/最终系数，W约束显式物化的整数工作值，显示/规范scale及原文显式指数各受H约束。
上游算法的内部暂存块不冒充语言系数；其内存须纳入§6.2的OS峰值，不能只靠Xiao对象计数器声称受控。

#### 6.1.1 输入、输出与共同顺序（F-3a）

1. 数值文本先检查原始UTF-8字节≤65,536，再完整按§8.1语法从左到右检查，最后检查系数、显式指数、显示/规范scale。
   解析中可用有界计数记录预算溢出，但不能抢在后续语法错误之前返回033，也不能先构造无限大小系数或指数。
2. 每个非零输入显示系数d≤D；零仍先检查原文、语法与显式指数，再按F-10以c=0、s=0验收系数及scale；不将已去掉的前导零当系数位数。
   lint沿用整数文本边界，str→lint仍为码点转换，不新增直接整数数字解析入口。
3. 实参求值、类型、precision/rounding、逐形参输入、除零/域、非终止、工作/结果、输出分配严格沿§5.2；不重新裁定错误优先级。
4. 每个工作值在构造前核对已知位数或可证明的上界≤W；最终系数必须按实际值核对d≤D，不能用松的结果上界拒绝强消去等合法结果。
   预算超限为033；失败无部分结果、输入不变，C ABI失败保留out。受控输出分配失败仍为011，真实OOM沿Fatal边界。
5. 结果显示和规范scale各满足abs≤H，完整文本按§8.2选择普通/指数格式；不得为过预算自行舍入、删显示尾零或展开巨型指数。
   零、n=0/n=1及0/±1快路径沿F-10/F-4，均先完成适用的输入和域检查。
6. 比较、相等、哈希用规范系数/scale及虚拟补零比较，不为了scale差展开巨型整数；定宽窄化先按F-8判定范围，不能先构造无界整数。
   lfloat→lint的整数输出也受D约束；可证明结果超限时在物化前拒绝，不能改变既有向零截断或009/033边界。

#### 6.1.2 加减、乘法（F-3a）

两非零操作数加减令S=max(s_a,s_b)，L_a=d_a+S-s_a，L_b=d_b+S-s_b。
先查max(L_a,L_b)+1≤W再对齐；实际最终系数≤D，结果scale沿F-10。
零不抬高另一操作数的显示scale，精确消去为规范0；不能把最终位数的上界当成实际位数。

非零乘法先查d_a+d_b≤W，再按实际乘积核对D；显示scale=s_a+s_b，规范scale也独立检查。
显式工作模型为一次有界大整数加减或乘法；内部乘法实现可优化，但不改变接纳边界或F-10保精度规则。

#### 6.1.3 精确语言除法（F-3a）

合法非零分母下，先对两正系数按gcd约分，记约分后的分子A、分母B，符号另存，不为scale差展开10的幂。
若B含2/5之外的因子，按F-12先报034；这一判定在工作/结果预算阶段之前，不能用预估展开预算抢报033。
若B=2^u×5^v，令k=max(u,v)，构造系数C0=A×2^(k-u)×5^(k-v)，原始scale s0=s_a-s_b+k。
C0的构造前工作上界为d_A+abs(u-v)≤W；各次整数工作值也受W约束。

去掉C0尾零得到规范系数C、规范scale t；最终显示scale取S=max(s_a-s_b,t)，显示系数为C×10^(S-t)。
先按精确补零长度核对D及显示/规范scale再物化输出；1.00/2仍为0.50，不能为减少工作或输出位数丢显示精度。
零分子在输入及非零分母检查后返回规范0。

工作模型：gcd与2/5因子判定只处理不超过D位的正整数；若用逐步欧几里得与逐因子参考路径，
余数步骤保守上界为5D+2，成功剥除2/5因子次数上界为4D，另有两次终止检查。
允许批量因子提取或更快gcd；这些是逻辑工作上界，不是现有性能成绩，也不允许改变034的判定位置。

#### 6.1.4 显式精度除法（F-3a）

非零分子系数绝对值为A，非零分母系数绝对值为B。以整数/虚拟补零比较精确确定K=floor(log10(A/B))，
K只可能为d_A-d_B或d_A-d_B-1，不能用浮点log在边界决定舍入。
令p=precision、h=p-1-K，N=A×10^max(h,0)，M=B×10^max(-h,0)。
先查d_A+max(h,0)≤W、d_B+max(-h,0)≤W，再做一次整数商余；s_a-s_b只计入结果scale，不展开到N/M。

按F-1对最终商余舍入一次，原始结果scale为s_a-s_b+h；进位后按F-10规范化，再检查实际系数与scale。
不把先舍入倒数再乘作为替代路径；精确短结果不补零。零分子仍先查类型、precision/rounding、输入及非零分母，再返回规范0。
工作模型是一组有界缩放、一次商余及一次最终舍入；商的进位或余数比较的显式整数也须≤W。

#### 6.1.5 幂（F-3a）

先按F-4检查指数为非负整数值；0/±1允许已确认快路径，不将超大但合法指数先窄化为定宽整数。
n=0为规范1、n=1保留输入尺度，其余单位/零快路径按F-10规范化。
非平凡底数先精确比较n≤1,000,000，再用平方求幂，最多64次大数乘法；每次乘法先查系数位数和≤W，不计算最后多余平方。
非零非平凡结果的显示/规范scale分别为输入对应scale乘n，可证明超限时在系数构造前拒绝；最终系数仍按实际值检查D。
不能仅凭d_base×n的松上界拒绝可表示的幂；允许用可证明的结果位数下界提前拒绝必然超限结果。

#### 6.1.6 lfloat //（F-3b）

令A=abs(c_a)、B=abs(c_b)、k=s_b-s_a，abs(k)≤200,000；零分子和零分母仍按既有顺序处理。
沿§6.1.4的精确比较确定K=floor(log10(A/B))，再令ell=K+k，得到绝对比值的十进制数量级。

- ell<0时，非零比值绝对值小于1；同号返回0、异号返回-1，不物化巨型对齐分母。
- ell≥0且ell+1>D时，商一定超最终预算，先报033。
- 其余情况下，k≥0构造N=A×10^k、M=B；k<0构造N=A、M=B×10^(-k)。
  先核对两长度≤W，再做商余，同号取截断商，异号且余数非零时增加绝对值1并取负号。
- 对修正后的实际商再次检查D，防止负数floor进位增加一位；结果为lfloat、显示scale=0，零规范化。

工作模型为精确数量级比较，加上至多一组有界对齐和一次整数商余；不要求先计算完整小数商或以float决定floor。

#### 6.1.7 lfloat %（F-3b）

沿用A、B、k，结果显示scale为S=max(s_a,s_b)，商和余数各自独立验收，不沿用//的超限结果。

- k≥0时，仅计算z=((A mod B)×(10^k mod B)) mod B，使用平方求模；包含最终混合乘法最多64次模乘。
  每次两个余数的乘积位数上界≤2d_B≤W，不构造A×10^k或完整商；B=1可在输入检查后直接得到零余数。
- k<0时，按虚拟补零比较A与T=B×10^(-k)。若A<T且操作数同号，余数系数就是原c_a；无需构造T。
  异号且A<T时，余数绝对系数为T-A；若d_B-k>D+1，则可证明该差仍超过D，在构造T前报033。
  其余情况先检查对齐长度≤W，再仅求A mod T或T-A，按实际余数检查D；d_B-k=D+1的边界必须实际核对，不能一概拒绝。
- 对尚未修正的非负截断余数z，除数为负时结果取负；同号时绝对余数取z，异号且z非零时取模数减z，零保持规范0。
  上述T-A分支已经完成异号修正，不能再做一次模数减z。
  对最终实际系数、S和规范scale分别验收；不得为保留一个小余数先构造超大商。

整数lint的//、%是k=0的整数配对，保留既有返回族、floor和余数符号；不将该表扩展为float/sfloat或bool的新算子准入。
例如1e100000 // 1e-100000因200,001位商报033，而同输入的%可经有界模运算得到0；
-1e-100000 // 1e100000得到-1，%的巨大非零显示系数独立按预算拒绝。
这些是已确认契约的设计算例，不是产品执行结果。

### 6.2 性能与采样精表（F-3c已确认，尚未实测）

以下B2受控验收协议由F-3c单独确认，不从B1旧预算推导；它不是已有成绩或任意硬件时延保证，B1既有预算不在本批改变。

| 项 | 已确认精确协议 |
| --- | --- |
| 构建 | VM和Runtime使用release Rust构建；原生Xiao O0/O3独立记录，不混成绩；编译、链接与参考向量生成不计入运算时限 |
| 目标 | 每次调用归一化中位数：VM≤10秒、原生≤2秒；OS驻留增量保守峰值≤64MiB |
| 时间区间 | 固定输入与产物准备完成后计完整语言调用，包含调用期间的解析/分配/输出构造；按操作分别测试转换入口，不能借外移工作缩小被测语义 |
| 采样 | 五批分别在独立测量进程中执行，每批先预热一次再采正式样本；最大输入不能多批叠进同一60秒看门狗调用 |
| 校准 | 单次≥50ms则repeat=1；否则批次至少50ms，以100ms为停止增长阈值，每次repeat最多增16倍，上限2^24；上限仍不足50ms则不可验证 |
| 校准失败 | 实际过慢样本如实保留，不以100ms阈值丢样；时限/看门狗失败算未达标，不删除、换负载或加ignore |
| 时间统计 | 每批保留repeat及总耗时，先除repeat得单次，再取五批中位数；各优化级别分别计算 |
| 内存 | 基线在同PID的Runtime/VM初始化后、构造输入/预热前读取当前驻留；覆盖输入、输出及上游暂存，以进程保守高水位减基线，取max(0,差值)，五批取最大值 |
| 平台内存来源 | Windows WorkingSetSize/PeakWorkingSetSize；Linux当前VmRSS及VmHWM或等价getrusage；macOS resident_size及getrusage ru_maxrss；原始单位与换算均记录，Linux通常KiB、macOS字节，不可混用 |
| 不可验证 | 无可靠OS峰值、时长校准不足或缺实际平台记录时标不可验证；Xiao对象计数只作补充，不能代替上游内存；不通过放宽预算取得通过 |
| 翻倍比 | 线性组≤3；乘/除/gcd/幂/模幂等超线性组≤5；只比较相同路径、同优化、同输入族的n与2n，双方均须合法且测得，最大合法规模另测绝对预算 |
| 记录身份 | 绑定实际源码、Cargo.lock、Runtime内容、工具链/目标、产物/缓存指纹、PID与独立输出路径，逐批保存原始样本；不能复用旧可执行文件或覆盖不同任务产物 |

线性组包括语法/规范化/显示、比较/哈希和按对齐长度计量的加减；乘法按平衡系数规模，密集除法按系数/precision，
幂按实际结果增长且底数非0/±1，模幂按模数系数规模且固定scale差分组；不以简单乘1、小指数或零余数替代主负载。
须覆盖最大合法系数、2/5因子有限除法、非终止、不同scale对齐、强消去、//负数floor进位及%非零余数，以及上限+1拒绝。
B1 CSV字段/数组和填充最大输出仍按B1原契约及已确认的新版721条重跑，不给B2新增未裁定容器限额。

## 七、cmath行为、误差与二进制转换（F-2/F-8/F-9已确认）

| 函数 | 已确认数学/特殊值规则 | 已确认数值验收 |
| --- | --- | --- |
| sqrt | x<0报009；sqrt(-0)为-0 | 正确舍入，0 ULP |
| floor | 向负无穷取整；输入±0保留原符号；负极小数为-1 | 逐位精确 |
| ceil | 向正无穷取整；输入±0保留原符号；负极小数可为-0 | 逐位精确 |
| round | 最近偶数；±0.5分别为±0；-1.5为-2、2.5为2；输入±0保留 | 逐位精确 |
| trunc | 向零截断；输入±0保留；负极小数为-0 | 逐位精确 |
| abs | abs(-0)=+0，其余取非负值 | 逐位精确 |
| min | 两实参均有限；异号零平局取-0，同号零保留 | 逐位精确 |
| max | 两实参均有限；异号零平局取+0，同号零保留 | 逐位精确 |
| sin | 弧度；sin(±0)=±0 | 普通非零结果≤4 ULP |
| cos | 弧度；cos(±0)=+1 | 普通非零结果≤4 ULP |
| tan | 弧度；tan(±0)=±0；不设epsilon伪极点 | 普通非零结果≤4 ULP |
| exp | 非有限结果报009，IEEE下溢可到+0 | 普通非零结果≤4 ULP |
| ln | x≤0报009；ln(1)=+0 | 普通非零结果≤4 ULP |
| log10 | x≤0报009；log10(1)=+0 | 普通非零结果≤4 ULP |
| pow | 0**0=1；负底仅整数指数；0的负幂/非有限结果报009 | 普通非零结果≤4 ULP，特殊值精确 |

F-2已确认：全部入口先拒绝NaN/Inf输入，不能用pow(x,0)或min/max的另一实参掩盖非法输入。
pow(x,±0)=1；有限负底只有整数值指数合法；-0的正奇整数幂为-0，其它正指数为+0；±0的负幂报009。
任意非有限结果报009；正常IEEE下溢允许子正规数和有符号零，符号必须正确。所有取整函数仍返回已确认的float。
上游同名round不保证最近偶数，不能直接根据同名函数推导Xiao行为。

| F-A2特殊值设计算例（产品尚未执行） | 约定结果 |
| --- | --- |
| sqrt(-0)、sin(-0)、tan(-0) | -0 |
| round(-0.5)、round(0.5)、round(2.5) | 分别为-0、+0、2 |
| min(-0,+0)、max(-0,+0)、max(-0,-0) | 分别为-0、+0、-0 |
| ln(1)、log10(1) | +0 |
| sqrt(-1)、ln(0)、log10(-1) | 数学域错误 |
| pow(-0,3)、pow(-0,2.5)、pow(-0,-1) | 分别为-0、+0、数学域错误 |
| pow(NaN,0) | 先拒绝非有限输入，不能以零指数返回1 |
| tan输入位模式0x3ff921fb54442d18 | 输入是π/2的有限近似值；不按epsilon极点拒绝，以独立参考及容差验收 |

参考输入必须按实际float位模式解释，不能把π/2的近似值当成精确实数极点；不能用被测libm生成同算法黄金。
参考值按独立高精度数据正确舍入到float，生成工具/版本/来源仍需留档。零/非零分类和符号先精确核对，不能用4 ULP放过错误的flush-to-zero或符号。
子正规非零结果同样按最多4个相邻float间距；参考零必须返回正确有符号零，错误身份不使用数值容差。
F-9已确认cmath.pow与float的**静态/动态路径接入同一数学内核，避免两个实现各自给域/末位规则；sfloat既有路径单独保持，不冒充已获cmath支持。

F-9已确认参考：仅离线使用MPFR 4.2.2/GMP 6.3.0生成向量，产品保持已确认的纯Rust底座。
输入按二进制64位模式精确装入；从256位起分别向下/向上求值，两端都最近偶数转换为同一binary64结果才接受参考值。
否则逐次加倍到4096位；仍不能唯一确定时标不可验证，不生成猜测黄金。向量登记输入/输出位模式或稳定错误、生成器来源、版本和摘要。
零、符号、域与错误按精确表判断；sqrt用0 ULP，其余取整/abs/min/max逐位，普通超越与pow非零结果≤4 ULP。
工具版本、区间判据和上限均由F-9答复确认；本轮没有生成B2产品执行证据或引入C/GMP产品依赖。

F-8已确认：整数族到lfloat精确，float/sfloat到lfloat保留二进制实际值，不通过最短显示文本；
lfloat/lint显式窄化float/sfloat按最近偶数，以舍入结果非有限判溢出，允许按IEEE舍入为带符号零。
float/sfloat到lfloat的规范数值身份精确，二进制的±0在lfloat中统一为0。
新转换的整数结果显示scale=0；非整数二进制值采用精确规范系数。显式文本解析与既有lfloat同类型转换仍保留F-10的显示身份。
例如float的0.1承接其实际二进制值，未承诺与文本解析的十进制0.1相等；反向允许正确的子正规数和带符号零。
小数显式转lint/int/sint向零截断已确认，定宽结果检查范围，lint受资源预算；str到整数继续是码点。
其余定宽旧规则保持。跨类型比较按已有提升后的值做精确比较；同种lfloat的Eq/Hash规范化数值，其他结构类型规则不自行扩大。

## 八、文本、显示与结果scale（F-4/F-8/F-10/F-11已确认）

### 8.1 输入与资源计量

F-11已确认str到lfloat的ASCII十进制语法：可带正负号、可前导零、支持.5与5.、e/E及指数正负号，
下划线仅允许在同一数字段的相邻数字之间；拒绝空白、NaN/Inf、非ASCII数字与其它前后缀。
源码数值字面量仍按既有词法规则处理，不把这个字符串解析规则扩展成新语法。

F-11已确认字符串语法的精确写法如下，digit只指ASCII的0～9，sign只指ASCII的+或-：

```text
digits   = digit { [ "_" ] digit } ;
sign     = "+" | "-" ;
mantissa = digits [ "." [ digits ] ] | "." digits ;
decimal  = [ sign ] mantissa [ ( "e" | "E" ) [ sign ] digits ] ;
```

不在解析前剥除空白、BOM或NUL；指数中的下划线同样只能在数字之间。该语法不改变str到整数族的码点契约。

设去符号与小数点/下划线后的整数系数为c，去前导零但保留有意义尾零，显示尺度s=小数位数-显式指数。
零统一为c=0、s=0，输出“0”。非零数值身份再去掉c的尾零，同时相应减少scale，作为Eq/Hash的规范键。
系数位数按|c|的十进制位计，去前导零、保留尾零，零为1位；这明确D9数值如何作用于显示精度。

F-10已确认显示scale s与数值身份的规范scale都独立满足|值|≤100,000；
显式指数也≤100,000，原文≤65,536字节，输入/最终系数≤32,768位，中间工作系数≤65,536位。
“显示scale也受限”由本次F-10答复单独确认，不倒推为早期资源数值答复已涵盖。

预算共同决定接纳，不能把任何单项上限写成运算必然成功。例如很多尾零要求保留时，乘法结果的显示系数可能超限，即使规范数值很简单仍应报资源错误，不可偷偷丢精度绕过。

### 8.2 确定显示与往返

F-10已确认保留非零c/s所表示的精度：s≥0且普通十进制输出≤65,536字节时用普通格式；
否则采用整数系数c乘10的-s次方的指数形式，不强制科学记法只留一位整数。表示scale和指数同界后，输出指数也在界内。
符号省略正号、指数用小写e且省正号，省无意义前导零；零仍为“0”。例如1.50保持1.50，1e100000保持紧凑形式。
这样不会把合法“大系数+指数”强制改成超过输入指数上限的文本，也不展开巨型指数。
普通格式字节数（非零，含负号记号b取0或1）：s=0时b+d；0<s<d时b+d+1；s≥d时b+s+2。
s<0或上述普通格式超过65,536字节时输出整数系数加e及-s。

F-A1显示算例（已确认规则，产品尚未执行；c/s表示显示身份，键表示规范数值身份）：

| 输入文本 | 显示c/s | 规范键 | 约定输出 |
| --- | --- | --- | --- |
| 1.50 | 150 / 2 | 15 / 1 | 1.50 |
| 1.5 | 15 / 1 | 15 / 1 | 1.5 |
| 15e-1 | 15 / 1 | 15 / 1 | 1.5 |
| +0001.50_0e+00 | 1500 / 3 | 15 / 1 | 1.500 |
| 0.00、-0.00 | 0 / 0 | 0 / 0 | 0 |
| 10 | 10 / 0 | 1 / -1 | 10 |
| 1e100000 | 1 / -100000 | 1 / -100000 | 1e100000 |
| 1e-65534 | 1 / 65534 | 1 / 65534 | 普通格式为0.后接65,533个0和1，共65,536字节 |
| 1e-65535 | 1 / 65535 | 1 / 65535 | 1e-65535；不展开成65,537字节 |

这里前三行及1.500数值相等、哈希相同，显示精度可以不同；10和1e1也数值相等但显示身份不同。
按已确认F-1/F-10，9.99舍入后采用规范系数1、scale=-1，故显示为1e1；不能把数学值10误写为强制输出“10”。
100e100000的显式指数虽在界内，去尾零后的规范scale=-100002仍超已确认预算，不能因为任一单项合格就接纳。
65,536个字符0组成的原文在长度界内且系数为规范零；65,537个0仍须先按原文预算拒绝，不能用零快路径绕过。

### 8.3 运算结果的显示scale

| 操作 | 已确认结果显示规则（含F-4补充） |
| --- | --- |
| 加/减 | 两非零操作数取max(s_left,s_right)；零操作数不提高另一项显示尺度；精确消去为规范0 |
| 乘 | 非零结果为s_left+s_right；零结果规范0；不得为通过结果预算自行舍入/删尾零 |
| 语言精确/ | 先拒绝零分母；非零有限结果取max(s_left-s_right,规范结果scale)；零分子结果为规范0 |
| lfloat //（F-4补充确认） | 返回lfloat整数值，显示scale=0，零仍规范化 |
| lfloat %（F-4补充确认） | 返回lfloat；非零余数显示scale=max(s_left,s_right)，零规范化；独立按自身预算计算 |
| 显式lfloat_divide | precision为最多有效数字；一次最终舍入后使用规范结果，不人为补齐precision |
| 非负整数幂 | n=0返回规范1；n=1保留原尺度；其它0/±1快路径返回规范结果；非平凡底数为s_base*n并受预算限制 |
| 比较/相等/哈希 | 以规范数值比较，不让显示精度改变数值身份 |

例如语言1.00/2保留0.50，显式precision接口可返回0.5；数值相等、哈希相同。
零操作数的例外已由F-10确认，避免0+1e100000仅因把零的scale=0纳入max而迫使展开并超限。
幂的0/±1快路径仍先检查两个输入；n=1保留输入尺度，其它快路径返回规范值，例如1.00**2为规范1；指数开放域已按F-4确认。
规范与显示按本表执行，不采用上游默认Display或环境变量的输出。

| F-A1运算算例（产品待实现） | 约定显示结果 | 对应规则 |
| --- | --- | --- |
| 1.50 + 1.5 | 3.00 | 非零加法取较大显示scale |
| 1.50 - 1.5 | 0 | 精确消去为规范零 |
| 0.00 + 1e100000 | 1e100000 | 零不抬高另一项的scale |
| 1.50 * 2.0 | 3.000 | 乘法scale相加 |
| 1.00 / 2 | 0.50 | 精确除法保必要尺度 |
| 显式除法1.00/2，precision=10、half_even | 0.5 | 短结果不补零 |
| 1.00 ** 1 | 1.00 | n=1保留输入尺度 |
| 1.00 ** 2 | 1 | 单位底数快路径给规范值 |
| 2.0 ** 2 | 4.00 | 非平凡底数scale乘指数 |
| 1e-100000 * 1e-100000 | 资源预算拒绝 | 结果规范scale超100,000，不静默截断 |

这些F-A算例不作为后续F-B/F-C的裁定来源；F-5/F-6/F-12已由单独答复确认，F-3资源/工作量/性能与F-7依赖已由独立实际答复确认，详见§四、§六。

### 8.4 转换矩阵（F-8/F-11已确认）

六种数值标量之间明确列出方向：同类型保持值；整数族到lint/lfloat精确；
浮点到整数显式向零截断，定宽检查范围；float/sfloat到lfloat精确保留二进制实际值；
lint/lfloat到float/sfloat显式最近偶数，非有限来源拒绝、舍入后非有限才判溢出。既有定宽规则不扩大。

F-11已确认新增float/sfloat到str时沿用已有print/display_text的有限值表示（包括负零符号），产品尚未实施；
直接str到float/sfloat继续不开放，十进制解析统一经lfloat后显式窄化。
str到int/sint/lint继续是码点，str到lfloat是新十进制解析，lfloat到str按已确认的8.2。
F-8已直接确认“正确舍入结果非有限才拒绝”，与现有[float到sfloat实现](../../core/rust/crates/xiao-runtime/src/value/ops.rs)一致；
这来自本次答复，不是从旧实现擅自外推。数学值略大于最大有限值但仍舍入回最大值时允许。
F-8显示补充已确认：整数及整数值二进制输入统一生成scale=0，非整数二进制值使用精确规范系数；这不重开数值与舍入规则。

| F-8转换设计算例（产品尚未执行） | 约定结果 |
| --- | --- |
| 整数10或值为10的float/sfloat转lfloat | 显示10；显示系数10、scale=0，规范键仍为1/-1 |
| str文本10.00解析为lfloat | 保留10.00；不套用新数值转换的scale=0规则 |
| float的0.1转lfloat | 精确值0.1000000000000000055511151231257827021181583404541015625 |
| sfloat的0.1转lfloat | 精确值0.100000001490116119384765625 |
| lfloat非零值-1e-100000窄化到float/sfloat | 最近偶数得到-0；输入仍先通过资源预算 |

| F-8已确认的舍入边界 | 结果 |
| --- | --- |
| float目标：MAX=(2^53-1)*2^971；溢出中点T=2^1024-2^970 | MAX<x<T仍舍入为MAX；x≥T舍入为Inf并拒绝；负数对称 |
| sfloat目标：MAX=(2^24-1)*2^104；溢出中点T=2^128-2^103 | 同样按舍入后的有限性判断 |
| float目标：精确输入±2^-1075 | 最近偶数得到±0，保留非零来源的符号 |
| float目标：精确输入3*2^-1075 | 中点取偶，结果为2^-1073，即第二个正子正规数 |
| float/sfloat的±0到lfloat再回二进制 | lfloat零规范为0，不能宣称负零位模式往返不变 |

F-11设计算例（已确认规则，不是实际转换执行证据）：

| 输入/方向 | 约定行为（产品待实现） |
| --- | --- |
| str文本.5、5.、1.e2到lfloat | 接受；分别显示0.5、5、1e2 |
| str文本+1_234.50_0e-2到lfloat | 接受；显示12.34500 |
| 空串、+、-、.、1e、1e+ | 拒绝不完整文本 |
| _1、1_、1__0、1_.0、1e+_1 | 拒绝分隔符位置 |
| 首尾/内部空白、BOM或NUL | 拒绝，不预先清理 |
| NaN、Inf、0x10、全角数字、Unicode负号 | 拒绝，不能替代ASCII十进制 |
| float/sfloat的0.1到str | 拟新增转换沿现有display_text显示0.1；不表示其实际二进制值恰为十进制0.1 |
| float/sfloat的-0到str | 拟保留现有显示为-0 |
| str文本0.1直接到float/sfloat | 不开放该方向；显式经lfloat再窄化 |
| str文本42到lint | 仍按已确认码点契约拒绝多个码点；数值恢复可经lfloat再到lint |

### 8.5 版本与产物矩阵（F-6已确认，当前版本常量未改）

A保持ABI1.18/生成器20作为中间验证快照，B2完成交付的最终新产物统一1.19/21。
A快照仅作本批内部验证，不发布为最终B2产物。
F-6已确认的目标生产规则：B2最终.xiaoc头、.xar索引与指令流均记录精确1.19；原生元数据记录精确1.19、生成器21及对应标准库ABI身份，标准库接口版本保持1。
不通过手改旧产物头、放宽旧产物max版本或忽略指令流版本来宣称兼容；不在本轮改变现有容器格式/索引schema版本。
源码核对发现，现有.xiaoc头与.xar索引均把ABI最小/最大值写成当前精确版本，指令流也精确检查runtime_abi_version；
见[xiaoc编码](../../core/rust/crates/xiao-bytecode/src/xiaoc.rs)、[归档索引](../../core/rust/crates/xiao-driver/src/archive.rs)和[指令流校验](../../core/rust/crates/xiao-bytecode/src/encode/validate.rs)。
F-6已确认按精确版本边界拒绝旧原件，从B1源码在1.19/21重新生成回归；原先未确认的“旧原件直接兼容重放”建议不采用，也不另加迁移工具。

| 对象 | 目标 | 已确认的目标行为 |
| --- | --- | --- |
| B1原始1.18 .xiaoc/.xar | B2-B 1.19 | 执行前按既有头/索引/指令流版本检查拒绝，原件不改写 |
| B2最终1.19 .xiaoc/.xar | 旧1.18 | 执行前拒绝，不能只靠执行到未知原语才失败 |
| 原始1.18原生元数据 | B2-B 1.19验证器 | 按精确ABI范围与标准库身份拒绝；不改写原件来伪装新产物 |
| 新1.19原生元数据 | 旧1.18验证器 | 在用户入口执行前按版本/标准库身份拒绝 |
| B1源码重新生成的1.19产物 | B2-B 1.19 | 重新执行20入口及新快照721条，不继承旧成绩 |
| 生成器20的原生对象缓存 | 生成器21 | 缓存身份不匹配，重新生成并链接 |
| A的1.18/20中间产物 | 本批A工具快照 | 仅内部验证；不承诺旧Runtime具备新的高精度语义 |

保留既有1.18 C符号、布局与调用约定，新增C入口按加法扩展；不把这种二进制接口兼容误写为容器接受超出max版本的许可。
数值行为按已确认F-A升级，包括lfloat的规范身份及float **共用内核；不承诺所有旧数值行为不变。这不是新增数学裁定，也不改写F-5的所有权/状态表。
原生元数据的精确ABI检查见[native_metadata.rs](../../core/rust/crates/xiao-codegen-llvm/src/native_metadata.rs)；
缓存指纹已包含生成器、ABI、优化计划和Runtime静态库内容，见[toolchain.rs](../../core/rust/crates/xiao-codegen-llvm/src/toolchain.rs)。
A的版本号不变也不等于可以混用旧Runtime：内部取证仍绑定实际源码、Runtime内容与输出路径；B的21号生成器必须重新生成并链接匹配的产物。
本节均为后续实现与取证的已确认合同：当前仓库实际仍为ABI1.18/生成器20，本轮未生成1.19产物、修改旧头或执行跨版本产品。

### 8.6 实施执行点提案

建议复用现有VM/原生两个逻辑精确执行入口，追加独立B2语料，保留全部B1场景和各自标记，
四个实际路径仍恰好1 passed，原28/28守卫保持；不是只在ignored批量里偷跑新样例。
cmath产品fixture按D20放tests/fixtures/docs/cmath/，数值运算/转换/错误语料放独立B2目录，
默认契约测试另核对真实导出、移除、归档和普通包隔离。实施阶段才写这些测试，当前不创建伪执行证据。

## 九、精确项对账与F-D待办

1. F-1已确认七种舍入、最多有效数字、短结果不补零与单次最终舍入（§五）。
2. F-2已确认cmath域、负零、下溢、角度与round/pow/tan精细规则（§七）。
3. F-3已确认接纳公式、工作量及//、%路径，性能/OS内存/采样与身份协议（§六）；尚未生成测量成绩。
4. F-4已确认同时开放lfloat //与%、floor配对、显示尺度、非负整数值幂和既有提升到lfloat的类型准入（§五）。
5. F-5已确认原语参数名、规划ID33～48、效果/能力与C ABI所有权/状态精表（§5.1），活动表未改。
6. F-6已确认中间快照、旧/新原件版本拒绝、缓存重生成与C二进制兼容边界（§8.5），版本常量未改。
7. F-7已确认六包精确版本/特性/许可/来源、锁文件与受控构建环境策略（§四）；Cargo文件和实际构建未实施。
8. F-8已确认二进制值精确承接、反向舍入/下溢、舍入后有限性和默认显示尺度（§七/§8.4）。
9. F-9已确认每函数误差、独立参考与cmath.pow/float `**`共享内核（§七）。
10. F-10已确认系数/显示scale/规范scale定义、输出和运算结果scale（§8.1～§8.3）。
11. F-11已确认文本语法与数值到str的转换补充（§8.1/§8.4）。
12. F-12已确认规划033/034、旧错误边界、全场景优先级与F-5状态映射（§5.2），活动诊断表未改。

十二组与既有18条均已确认，总计30/0；F-C退出时F-D仍待单独签字。F-D只允许切换页首状态行，不因签字改写本页已确认正文。
实际构建、序列化承载和教程完整示例留实施批次逐项验证；cmath教程与registry当前仅planned。

F-C裁定已收口；F-D当前状态见页首与独立签字证据，B2-A/B产品仍未开工。19/11/22及历史22C/22D失败仍独立保留。
