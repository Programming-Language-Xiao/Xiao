---
id: language.basics.p0-syntax.statements
title: P0 语句与简单赋值
status: verified
audience: learner
module: rust.xiao-syntax
stage: "01"
version: "0.1.0"
related:
  - README.md
  - errors.md
---

# P0 语句与简单赋值

[返回 P0 主题索引](README.md) · [下一页：错误与当前边界](errors.md)

## 可以写什么

P0 接受整数、浮点、字符串、布尔值和 `none` 五类字面量，也接受普通 ASCII 名称
和反引号包裹的 UTF-8 名称。每个字面量或名称都可以独立成为一条顶层表达式语句：

```xiao
42
answer
`新变量`
```

简单赋值的左侧必须是一个名称，右侧必须是一个字面量或名称：

```xiao
answer = 42
`新变量` = "hello"
other = answer
```

换行结束一条语句；文件末尾没有真实换行时，最后一条语句由 EOF 结束。多个顶层
语句按源码顺序组成一个程序。P0 只保留源码区间，暂不把数字或字符串转换成运行时值。

## 文档注释

`### ... ###` 文档注释以及其间的空行会关联到紧随其后的成功语句。文件末尾没有
后续语句的文档注释会被保留为孤立注释，不会静默丢失。普通 `#` 注释只影响词法层，
不会成为程序语句。

## 名称写法

普通关键字不能直接用作名称；需要使用关键字或任意 Unicode 文本时，可使用反引号：

```xiao
`def` = "name"
`用户 名称` = 1
```

反引号属于名称的原始源码文本，转义解码和名称绑定规则由后续阶段处理。

下一步请查看[错误与当前边界](errors.md)。
