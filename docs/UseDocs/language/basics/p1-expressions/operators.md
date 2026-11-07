---
id: language.basics.p1-expressions.operators
title: P1 运算符与优先级
status: verified
audience: learner
module: rust.xiao-syntax
stage: "01E"
version: "0.1.0"
related:
  - README.md
  - calls-and-casts.md
---

# P1 运算符与优先级

[返回 P1 主题索引](README.md) · [下一页：调用、构造与转换](calls-and-casts.md)

解析器使用 Pratt 算法。下表从高到低列出当前冻结的绑定关系；同一行中的
运算符拥有相同优先级。

| 优先级 | 运算符 | 结合方式 |
| --- | --- | --- |
| 最高 | 调用 `f(x)`、成员 `a.b`、步长和选择器后缀 | 左侧后缀连续应用 |
|  | `**` | 右结合 |
|  | 一元 `+`、`-`、`not` | 前缀 |
|  | `*`、`/`、`//`、`%` | 左结合 |
|  | `+`、`-` | 左结合 |
|  | `<`、`<=`、`>`、`>=`、`==`、`!=`、`in`、`not in`、`is`、`is not` | 左结合 |
|  | `not`、`and`、`or` | 逻辑层级，`not` 高于 `and`，`and` 高于 `or` |

赋值不属于表达式运算符，由语句层处理。幂运算右结合，因此 `a ** b ** c`
会先把右侧 `b ** c` 作为一个整体。括号可以显式改变分组：

```xiao
result = -(base ** power) + offset * scale
allowed = ready and (name in names)
```

`not in` 和 `is not` 是由两个关键字组成的比较运算。单独的 `not` 是一元
运算，不能省略其操作数。P1 只记录运算符和子表达式，不判断操作数类型；例如
集合并集使用 `+` 的规则会在容器/类型阶段实现。

## 软换行

圆括号、选择器方括号和步长花括号内部可以换行，换行不会结束当前表达式：

```xiao
value = factory(
    left + right,
)
```

语句顶层的真实换行仍然结束语句。

## B2-A高精度算术：内部验收通过，最终发布（planned）

lint加减乘、商余和非负整数幂保持精确；lfloat使用有限十进制，语言`/`只接受精确有限结果，非终止结果报可恢复的`X06-RUNTIME-034`。`//`向下取整，`%`与其配对且余数为零或随除数符号；当共同数值类型为lfloat时也支持这两个算子，其余定宽浮点商余继续拒绝。

B2-A的两后端功能、三平台资源/性能已完成内部验收；本页新增范围限定为ABI1.18/生成器20内部快照，最终发布仍planned，页面原有语法的verified范围不因此扩大。`lfloat_divide`与15项`cmath`尚未交付，留B2-B；最终1.19/21产物和教程/registry收口留B2-C。

`lint`是精确整数，`lfloat`是有限十进制，均受资源预算约束。涉及它们的混合运算精确承接整数及实际有限二进制数值；不会先把大整数近似为float。其余定宽旧规则保持现状。

| 操作 | B2-A内部已验证规则 |
| --- | --- |
| `+`、`-`、`*`、一元负号 | 精确系数计算；不隐式舍入，结果超预算报033 |
| `/` | 整数族返回浮点族；涉及lint/lfloat时返回精确有限lfloat；零分母报013，非终止十进制报034 |
| `//`、`%` | 整数族或共同类型为lfloat时，商向下取整，余数为0或随除数符号；lfloat商余仍返回lfloat。商与余数独立检查结果预算 |
| `**` | lint指数非负；lfloat指数须为非负整数值，1.0合法，1.5或负数报009；`0**0=1` |
| 比较、相等和集合哈希 | 精确数值身份忽略显示尾零；非有限来源拒绝；语言字典键仍为str |

非零lfloat加减保留两侧显示scale的最大值，零不抬高另一侧尺度；乘法scale相加；精确除法保留必要尺度。幂n=1保留原尺度，n=0返回1，其他0/±1快路径规范化；精确消去统一为0。下面完整示例逐字显示这些区别，错误与预算详见[转换与数值运算](../types/conversions.md#b2-a内部资源与错误边界)。未使用结果的表达式同样执行可恢复检查。

<!-- b2-a5-arithmetic-executable-example -->
```xiao
lfloat first = 1.50
lfloat second = 1.5
lfloat two = 2.0
lfloat one = 1.00
lfloat negative = -5.5
lfloat huge = 1e100000
lfloat tiny = 1e-100000
lfloat three_tiny = 3e-100000
print(first + second)
print(first - second)
print(first * two)
print(one / 2)
print(one / two)
print(negative // 2)
print(negative % 2)
print(negative // -2)
print(negative % -2)
print(two ** two)
print(one ** 1)
print(one ** 2)
print(first == second)
print(huge > tiny)
print(huge % tiny)
print(huge % three_tiny)
print(lfloat(0) + huge)
print(first)
```

预期输出：

```text
3.00
0
3.000
0.50
0.5
-3
0.5
2
-1.5
4.00
1.00
1
true
true
0
1e-100000
1e100000
1.50
```

[完整源码](../../../../../tests/fixtures/docs/numeric/a5_decimal_arithmetic.xiao)在VM和原生O0/O3逐字执行，显示scale、极端scale差、比较与有界余数共享同一Runtime内核。B2-A内部证据见[A7回归与收口记录](../../../../DevDocs/20o-b2-a7-b2a-closeout.md)；完整1.19/21公共交付与721条F10重跑仍留B2-C。
