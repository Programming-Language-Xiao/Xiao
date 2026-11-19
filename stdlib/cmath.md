# `cmath` 模块源码（planned）

工程阶段：20O-B2-B4-4内部接线。规范源码为 [cmath.xiao](cmath.xiao)，只包含15个公开包装，每个函数仅转发到已确认的 `cmath_*` 原语。参数全必填，沿既有位置/命名规则，无默认值、重载、别名或隐式转换。

类型/IR/VM/LLVM与双路径、归档的内部接线已落地；`std.cmath` 与 `stdlib.cmath` 使用同一固定官方加载器。registry与[教程](../docs/UseDocs/library/cmath.md)继续planned，B4-4独立取证、B4-5整批语言收口及B2-C发布验收分别登记，不能据此声称三平台或最终发布通过。

| 包装签名 | 唯一调用的原语 |
| --- | --- |
| `sqrt(float value) -> float` | `cmath_sqrt(value)` |
| `floor(float value) -> float` | `cmath_floor(value)` |
| `ceil(float value) -> float` | `cmath_ceil(value)` |
| `round(float value) -> float` | `cmath_round(value)` |
| `trunc(float value) -> float` | `cmath_trunc(value)` |
| `abs(float value) -> float` | `cmath_abs(value)` |
| `min(float left, float right) -> float` | `cmath_min(left, right)` |
| `max(float left, float right) -> float` | `cmath_max(left, right)` |
| `pow(float base, float exponent) -> float` | `cmath_pow(base, exponent)` |
| `sin(float value) -> float` | `cmath_sin(value)` |
| `cos(float value) -> float` | `cmath_cos(value)` |
| `tan(float value) -> float` | `cmath_tan(value)` |
| `exp(float value) -> float` | `cmath_exp(value)` |
| `ln(float value) -> float` | `cmath_ln(value)` |
| `log10(float value) -> float` | `cmath_log10(value)` |

包装不复制NaN/Inf、定义域、舍入或ULP判断。底层按冻结规则传播TypeError/X06-RUNTIME-002、ArithmeticError/X06-RUNTIME-009等错误，纯float成功路径不分配数值载荷；包装没有额外错误码或处理分支。错误顺序、C ABI状态和所有权见[冻结契约](../docs/DevDocs/20o-b2-freeze-draft.md)。

源级路径为 `std.cmath`，固定身份 `stdlib.cmath`，标准库接口1；不加入普通包依赖或锁。当前内部ABI1.18/生成器21；B5只将ABI升至1.19，生成器沿用21。ID34～48经VM/LLVM调用同一Runtime内核；最终兼容与发布证据仍后置。

早期LLVM未引用函数发射失败与修复见[B3-4实施记录](../docs/DevDocs/20o-b2-b3-4-progress.md)，保留为历史。当前只收集实际导入可达模块；正式参考向量已迁入共享测试数据。B4-4的双路径、移除、归档和承载证据见[独立记录](../docs/DevDocs/assets/20o-b2-b/entry-20261115-b4-4/README.md)；旧721和最终教程/registry升级仍由B2-C负责。
