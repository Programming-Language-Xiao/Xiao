---
id: language.basics.p0-syntax.errors
title: P0 错误与当前语法边界
status: verified
audience: learner
module: rust.xiao-syntax
stage: "01"
version: "0.1.0"
related:
  - README.md
  - statements.md
---

# P0 错误与当前语法边界

[返回 P0 主题索引](README.md) · [上一页：语句与简单赋值](statements.md)

## 常见错误

P0 的解析器会保留结构化错误编号，并在换行、反缩进或文件结束处同步，继续检查后续
顶层语句：

| 编号 | 含义 |
| --- | --- |
| `X01-PARSE-001` | 当前 Token 不能开始 P0 表达式 |
| `X01-PARSE-002` | 赋值左侧不是普通或反引号名称 |
| `X01-PARSE-003` | 当前版本不支持缩进代码块 |
| `X01-PARSE-004` | `=` 后缺少右侧表达式 |
| `X01-PARSE-005` | 出现运算、调用、索引等复杂表达式尾部 |

例如，下面的运算和调用不会被猜测成其他语义：

```xiao
a + b
print("hello")
value[0]
```

它们会报告 `X01-PARSE-005`，错误之后同一行的剩余内容会被跳过，下一行仍可解析。
缩进会产生 `X01-PARSE-003`；P0 不把缩进内容当作顶层语句。

## 尚未开放

本阶段不支持 `+`、`-`、比较、逻辑、函数调用、索引/选择器、数组、元组、集合、字典、
`if`、`for`、`while`、`def`、类型声明、`as`、`const` 或模块导入。后续阶段会在不改变
P0 源码位置和错误恢复契约的前提下逐项加入。

词法错误（例如非法字符或未闭合字符串）仍使用 `X01-LEX-*` 编号；解析结果会同时保留
词法和解析诊断。详细位置说明见[结构化诊断](../../../troubleshooting/diagnostics-structure.md)。
