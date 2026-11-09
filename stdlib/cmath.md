# `cmath` 模块源码（planned）

工程阶段：20O-B2-B3-4。规范源码为 [cmath.xiao](cmath.xiao)，只包含15个公开包装，每个函数仅转发到已确认的 `cmath_*` 原语。参数全必填，沿既有位置/命名规则，无默认值、重载、别名或隐式转换。

当前仅为内部源码包装。类型/IR/VM/LLVM及归档接线留B4，不能据此把 `std.cmath` 或 `stdlib.cmath` 写成可执行导入。registry与[教程](../docs/UseDocs/library/cmath.md)继续planned，当前没有完整产品示例或发布验收结论。

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

源级目标路径为 `std.cmath`，固定身份 `stdlib.cmath`，标准库接口1；不加入普通包依赖或锁。当前仍是内部ABI1.18/生成器20，B5统一切1.19/21。内部active34～48不代表VM/LLVM已支持，也不能被公开产物提前消费。

新增源码仍会暴露现有LLVM未引用函数发射问题，故本包装在独立任务中验证并保留原生失败样本，不自动并入主工作区；见[B3-4实施记录](../docs/DevDocs/20o-b2-b3-4-progress.md)。B4须处理模块/函数隔离与语言接线，并在迁移正式向量后补齐平台证据；旧721和最终教程/registry升级仍由B2-C负责。
