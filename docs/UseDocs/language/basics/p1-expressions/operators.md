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
