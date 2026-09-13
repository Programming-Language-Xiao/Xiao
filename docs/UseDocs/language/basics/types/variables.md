---
id: language.basics.types.variables
title: 变量声明与类型锁定
status: verified
audience: learner
module: rust.xiao-types
stage: "02A"
version: "0.1.0"
related:
  - README.md
  - conversions.md
---

# 变量声明与类型锁定

Xiao 的普通变量第一次赋值时推断类型，之后保持该类型。也可以在名称前写标量类型：

```xiao
a = 1                 # 推断为 int
a = 2                 # 同类型赋值
int count = 1         # 显式锁定 int
str name              # 声明但暂未初始化
```

读取 `str name` 这样的未初始化名称会报告 `X02-TYPE-003`。把字符串直接赋给已经锁定
为 `int` 的变量会报告 `X02-TYPE-004`；类型不会因为一次普通赋值悄悄改变。

`sint`/`int`/`lint` 是整数族，`sfloat`/`float`/`lfloat` 是浮点族。能够证明字面量
在较窄范围内时，`sint small = 1` 可以通过；变量表达式的窄化请使用[显式转换](conversions.md)。

[返回类型主题](README.md) · [下一页：转换与数值运算](conversions.md)
