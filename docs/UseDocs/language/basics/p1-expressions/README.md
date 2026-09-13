---
id: language.basics.p1-expressions
title: P1 表达式与选择器
status: verified
audience: learner
module: rust.xiao-syntax
stage: "01E"
version: "0.1.0"
related:
  - ../README.md
  - operators.md
  - calls-and-casts.md
  - selectors.md
  - errors.md
---

# P1 表达式与选择器

本主题说明 Xiao 0.1 已验证的 P1 前端语法：表达式运算、调用、显式转换和
索引选择器。当前实现只建立语法树并提供结构化诊断；它还不能执行 Xiao 程序，
也不会在这一阶段检查类型、容器边界或随机抽取数量。

## 阅读顺序

1. [运算符与优先级](operators.md)：理解表达式如何分组。
2. [调用、构造与转换](calls-and-casts.md)：了解函数调用、`new` 和 `as`。
3. [索引选择器](selectors.md)：使用精确索引、范围、步长和随机选择语法。
4. [错误与边界](errors.md)：查看当前会拒绝的写法和诊断编号。

## 与 P0 的关系

普通名称的简单 `=` 仍保持 P0 的语句形状；P1 的复合赋值和成员/选择器目标
使用扩展赋值语法树。P0 的源码区间、文档注释挂接和顶层错误恢复契约不变。

## 当前不能做什么

示例可以被解析为 AST，但不会产生运行结果。类型推断、`bool` 与整数的加减
规则、容器投影、越界检查、随机数来源和可写性检查分别由后续类型、容器和运行时
阶段实现。
