# 20O-B2-B2. 显式精度除法内部实施记录

本批从 `faecd26d` 新建任务根 `D:\project\Xiao\.b2b-divide\20261108-faecd26d`，B0/B1 历史审计均在入口通过。B2 只接全局 `lfloat_divide` 与 intrinsic 33；当前内部版本仍为 ABI 1.18、生成器20、标准库接口1，34～48未激活，registry与stdlib均未改。B3～B7、B2-C以及最终版本发布后置。

实现为四个必填参数 `numerator`、`denominator`、`precision`、`rounding`，接受位置与命名调用。类型检查、VM和LLVM共用同一参数绑定计划；LLVM先按源码顺序求值，再按形参顺序传入C入口，并在实参求值失败和调用失败时释放临时拥有槽。没有默认值、重载、别名或新的隐式数值提升。

Runtime与C入口共用借用文本门面：先检查四参类型、precision和rounding，再按形参顺序接纳两个数值文本，随后检查除零、工作与结果预算。C入口调用期间保留源强引用，语义检查前不复制原始大文本；成功释放替换旧out，失败保持out，结构错误1/2/3不访问挂起槽，语义错误通过6传播，输出分配沿011。

## 数值底座修正

B1内核曾额外物化一个保护位，并在去尾零前检查结果scale。这会多消耗一位工作预算，也会拒绝规范化后仍合法的极端scale短结果。B2按已签F-3 §6.1.4改为 `h=p-1-K`：一次精确商余，以 `r` 与 `M-r` 比较中点，最终只舍入一次，去尾零后检查实际系数与scale。没有修改精度、舍入模式或资源数值。

测试包含七种正负舍入、`3/8`的单次舍入、`1/2`在32768位精度仍返回`0.5`、最大系数对齐到65536工作位、`1e-100000`的规范scale边界、结果超限和外部C原始载荷的复合错误顺序。

## 语言示例与维护入口

完整源码在[显式除法示例](../../tests/fixtures/docs/numeric/b2_explicit_divide.xiao)，逐字嵌入[转换教程](../UseDocs/language/basics/types/conversions.md)的planned段落。独立VM/原生精确测试均断言恰好1 passed；VM额外经过正式.xiaoc编码与删除源码后的真实.xar重放。两个既有维护精确入口也追加同一B2套件，原B1/A1/A5场景和28条守卫未删改。

本机Windows的O0/O3是本批执行证据，Linux/macOS尚无新作业运行号；不能当作三平台已验证或升级教程/registry。B1常量ULP向量仍仅为私有底座初检，F-9 MPFR/GMP正式参考由B3补齐。

## 验证记录

[独立证据目录](assets/20o-b2-b/entry-20261108-b2/README.md)保存每条命令的参数、cwd、PID、起止时间、前后源码清单、日志摘要与退出码。入口先冷构建独立Runtime，再执行B1契约9项、资源17项、VM/原生回归和28/28守卫。

| 最终代码 `37b92c53` 的验证 | 结果 |
| --- | --- |
| Runtime / intrinsic / Driver单元 | 259 / 7 / 64 passed；Driver保留既有1个ignored子进程入口 |
| 新入口类型与完整示例测试 | 2 passed / 1 ignored；ignored原生入口另用精确命令执行 |
| 新入口 VM / 原生 O0/O3 | 两个 `--exact` 命令各恰好1 passed |
| 维护 VM / 原生 O0/O3 | 两个既有 `--exact` 命令各恰好1 passed，包含B2追加标记 |
| B1契约 / 资源 / 计数守卫 | 9 / 17 / 28全部通过 |
| 其余工作区crate默认测试及文档测试 | `--workspace --exclude xiao-driver`通过；Driver单元和本批/B1目标另记 |
| 工程门禁 | workspace check、Clippy全目标、fmt与benchmark `--locked`通过 |
| 文档 | docs/usedocs/layout/覆盖率由总门禁检查，历史私有文档警告保留 |

上述最终执行记录均绑定同一份1037文件源码清单；精确命令、实际计数和Runtime摘要由只读审计复核。C静态库实际导出150个`xiao_runtime_*`符号，其中新增`xiao_runtime_lfloat_divide`，没有`xiao_runtime_cmath_*`。

开发期间的失败均保留：初版边界测试把应规范化的短结果误判为固定32768位；初版语言测试缺少ABI常量后缀；动态负例错误引用函数外的顶层值；Clippy发现共用支持模块被重复加载；文档门禁先遇到任务目录缺少TypeScript，再发现VM单文件行数超限。相应修复分别为独立数学黄金、更正测试常量、局部显式值、共用单一支持模块、复制相同5.9.3依赖，以及拆分数值intrinsic执行模块。历史日志不会被最终通过覆盖。

本批不运行旧721条F10或B2-A性能开关，不核销22C/22D、19/11/22债务。下一入口为[B3的15项cmath、intrinsic34～48、MPFR/GMP参考及共享pow](20o-b2-b3-cmath-plan.md)。

后续[自审记录](20o-b2-b2-self-review.md)修复五处证据审计遗漏并同步状态文案；独立数值2300组和C ABI别名3组通过。产品代码与原始执行证据仍保持`37b92c53`身份。
