# `cmath` 模块源码

规范源码包含 15 个公开包装，每个函数转发到对应的 `cmath_*` 原语。参数全必填，支持位置与命名调用。[cmath.xiao](cmath.xiao)

`std.cmath` 与 `stdlib.cmath` 使用同一官方加载器，教程与 registry 登记为 verified。B2-C 的固定候选在 Windows、Linux 和 macOS 通过 VM、原生 O0/O3 与删源归档验证。[教程](../docs/UseDocs/library/cmath.md) · [B2-C 证据](../docs/DevDocs/assets/20o-b2-c/entry-20261122-b2-c/README.md)

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

固定身份为 `stdlib.cmath`，当前 ABI 1.19、生成器 22、标准库接口 1。ID 34～48 经 VM 和 LLVM 调用同一 Runtime 内核。官方加载器独立于普通包依赖和锁。

LLVM 从实际导入的可达模块收集源码。B4-4 保留双路径、移除、归档和承载的内部实现记录。[B4-4 双路径证据](../docs/DevDocs/assets/20o-b2-b/entry-20261115-b4-4/README.md)
