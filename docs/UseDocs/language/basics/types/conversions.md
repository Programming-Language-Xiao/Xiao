---
id: language.basics.types.conversions
title: 转换与数值运算
status: verified
audience: learner
module: rust.xiao-types
stage: "02A"
version: "0.1.0"
related:
  - README.md
  - variables.md
  - constants.md
---

# 转换与数值运算

显式转换使用“源值 `as` 目标类型”，构造式写法与它共享同一规则：

```xiao
str raw = input("true/false")
bool flag = raw as bool
bool same = bool(raw)
str text = flag as str       # 结果是小写 true 或 false
```

`str` 到 `bool` 只接受 `true`、`True`、`false`、`False`；其他字符串在运行时检查或
已知为常量时在编译期报告 `X02-TYPE-006`。整数和浮点的安全加宽可以隐式发生，窄化
必须显式转换，并可能产生范围检查。显式浮点到整数转换按向零截断，例如
`1.9 as int` 的结果为 `1`；截断后的值超出目标固定宽度时仍报告
`X02-TYPE-007`。隐式声明不会借用这条截断规则。

数值提升顺序为 `sint < int < lint` 与 `sfloat < float < lfloat`。`/` 返回浮点族，
`//` 与 `%` 只接受整数。布尔值有一个专用规则：只有布尔在左、整数在右时，偶数保持、
奇数翻转，结果仍是 `bool`：

```xiao
flag = false
flag += 3   # true
flag -= 2   # true
```

反向运算、布尔与布尔、布尔与浮点数的加减都会报告 `X02-TYPE-005`。

[上一页：变量声明](variables.md) · [下一页：编译期常量](constants.md)
