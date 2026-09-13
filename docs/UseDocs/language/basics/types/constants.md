---
id: language.basics.types.constants
title: 编译期常量
status: verified
audience: learner
module: rust.xiao-types
stage: "02A"
version: "0.1.0"
related:
  - README.md
  - conversions.md
  - errors.md
---

# 编译期常量

使用 `const` 声明不可变的编译期值：

```xiao
const PORT = 8080
const float PI = 3.1415926
const answer = 1 + 2
```

常量初始化不能依赖输入、普通可变变量或运行时对象。不能在编译期求值时报告
`X02-TYPE-008`；可静态证明的除零、固定宽度溢出和非有限浮点值分别报告
`X02-TYPE-007`。`lint` 十进制字面量不受默认 64 位上限约束。

常量不会改变引用它的普通变量类型，也不能通过普通赋值修改：

```xiao
const limit = 10
# limit = 11  # X02-TYPE-004
```

[上一页：转换与数值运算](conversions.md) · [下一页：错误与运行时检查](errors.md)
