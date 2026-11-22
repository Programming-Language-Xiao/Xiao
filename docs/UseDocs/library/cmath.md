---
id: library.cmath
title: 浮点数学
status: verified
audience: learner
module: stdlib.cmath
stage: 20O-B2-C
version: "0.1.0"
related:
  - README.md
  - ../../DevDocs/20o-b2-numeric-foundation-plan.md
  - ../../DevDocs/20o-b2-freeze-draft.md
---

# 浮点数学

## 适用范围与状态

本页覆盖 ABI 1.19、生成器 22、标准库接口 1 的 15 个 `float` 包装。Windows、Linux 和 macOS 已用固定候选验证 VM、原生 O0/O3 与删源归档。[B2-C 验收记录](../../DevDocs/assets/20o-b2-c/entry-20261122-b2-c/README.md)

## 导入与准备

通过 `std.cmath` 或 `stdlib.cmath` 选择导入函数。两条路径都解析到官方身份 `stdlib.cmath`，随标准库提供。示例使用本地别名，避免与同名整数函数冲突。

## 五分钟运行与完整示例

以下程序调用全部 15 个包装，包含位置参数、命名参数、负零和可恢复的定义域错误。

```xiao
from std.cmath import sqrt as root, floor as floor_value, ceil as ceil_value
from std.cmath import round as nearest, trunc as truncate, abs as absolute
from std.cmath import min as smallest, max as largest, pow as power
from stdlib.cmath import sin as sine, cos as cosine, tan as tangent
from stdlib.cmath import exp as exponential, ln as natural_log, log10 as common_log

print(root(4.0))
print(floor_value(value = 1.5))
print(ceil_value(1.5))
print(nearest(value = 2.5))
print(truncate(-1.5))
print(absolute(value = -3.0))
print(smallest(right = 2.0, left = 7.0))
print(largest(7.0, 2.0))
print(power(exponent = 3.0, base = 2.0))
print(sine(-0.0))
print(cosine(value = 0.0))
print(tangent(0.0))
print(exponential(value = 0.0))
print(natural_log(1.0))
print(common_log(value = 1.0))

try
    root(-1.0)
catch failure as ArithmeticError
    print(error_code(failure))
finally
    print("clean")
```

预期输出如下。发生错误时，`finally` 先执行，随后匹配 `catch`。

```text
2
1
2
2
-1
3
2
7
8
-0
1
0
1
0
0
clean
X06-RUNTIME-009
```

三平台用同一份示例和独立黄金核对 VM、原生 O0/O3、`.xiaoc` 和 `.xar`，归档在删除应用源码后重放。[执行源码](../../../tests/fixtures/docs/numeric/cmath-tutorial.xiao) · [黄金输出](../../../tests/fixtures/docs/numeric/cmath-tutorial.stdout)

## 核心概念与所有权

cmath 使用 64 位二进制浮点标量。三角函数以弧度为单位，`round` 在正中间取偶数。计算无需宿主许可。

## 首批范围

| 入口组 | 输入与结果 | 参数 |
| --- | --- | --- |
| sqrt、floor、ceil、round、trunc、abs | 一个 float，返回 float | value |
| min、max、pow | 两个 float，返回 float | min/max 使用 left、right，pow 使用 base、exponent |
| sin、cos、tan、exp、ln、log10 | 一个 float，返回 float | value |

底层使用 Rust libm 0.2.16，登记 ID 34～48 和 15 个 `xiao_runtime_cmath_*` 符号。依赖闭包与构建策略见[冻结契约](../../DevDocs/20o-b2-freeze-draft.md)。

## 错误与恢复

NaN/Inf 输入、非有限结果、`sqrt` 负输入、`ln/log10` 非正输入等使用 `ArithmeticError/X06-RUNTIME-009`。动态实参类型不符使用 `TypeError/X06-RUNTIME-002`，受控输出分配失败使用 `ResourceError/X06-RUNTIME-011`。C ABI 错误顺序及输出槽约定见[冻结契约](../../DevDocs/20o-b2-freeze-draft.md)。

## 资源、性能与安全边界

`sqrt` 正确舍入（0 ULP），取整、`abs`、`min` 和 `max` 按位核对。普通超越函数与 `pow` 的非零结果误差上限为 4 ULP。零、符号、定义域和错误按确切值核对。[B2-C 数值与性能记录](../../DevDocs/assets/20o-b2-c/entry-20261122-b2-c/README.md)

## 示例如何进入 CI

产品回归另核对 30 条导入、逐函数移除和归档身份。17,243 条 MPFR/GMP 独立向量用于核对特殊值、定义域与数值误差。[正式参考向量](../../../tests/fixtures/docs/cmath/README.md)

## 兼容与废弃

公共候选使用 ABI 1.19、生成器 22 与标准库接口 1。加载器按版本核对归档，原生缓存按生成器身份重新生成。[版本矩阵](../../DevDocs/assets/20o-b2-c/entry-20261122-b2-c/README.md)

## 故障排查与下一步

导入失败时，检查诊断、ABI、生成器版本与 `XIAO_STDLIB_ROOT` 指向的标准库根。整数运算入口见[整数数学](math.md)。
