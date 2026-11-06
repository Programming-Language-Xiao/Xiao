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

## B2-A5高精度算术（planned）

lint加减乘、商余和非负整数幂保持精确；lfloat使用有限十进制，语言`/`只接受精确有限结果，非终止结果报可恢复的`X06-RUNTIME-034`。`//`向下取整，`%`与其配对且余数为零或随除数符号；当共同数值类型为lfloat时也支持这两个算子，其余定宽浮点商余继续拒绝。

lfloat非负整数值指数合法，`0**0=1`；小数/负指数报009。输入、工作与结果仍有固定预算，超限报033。未使用结果的运算同样检查错误。以下新增范围仍planned，不改变本页原有运算符语法的verified范围；显式精度除法与cmath留B2-B。

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

[完整源码](../../../../../tests/fixtures/docs/numeric/a5_decimal_arithmetic.xiao)在VM和原生O0/O3逐字执行，显示scale、极端scale差、比较与有界余数共享同一Runtime内核。A6性能与A7/B2-C最终收口尚未完成。
