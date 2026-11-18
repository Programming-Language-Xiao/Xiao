---
id: language.basics.p1-expressions.selectors
title: P1 索引选择器
status: verified
audience: learner
module: rust.xiao-syntax
stage: "01E"
version: "0.1.0"
related:
  - README.md
  - calls-and-casts.md
  - errors.md
---

# P1 索引选择器

[返回 P1 主题索引](README.md) · [下一页：错误与边界](errors.md)

选择器的所有项目必须位于同一对方括号中，项目之间用逗号分隔。精确项目和
范围项目可以混用，解析器按源码顺序保留重复项和重叠范围。

## 精确与嵌套路径

```xiao
first = values[0]
many = values[0, 1]
nested = values[3/2]
named = record[1/`键`]
```

路径段可以是非负整数、负整数、普通名称或反引号名称；`/` 在选择器路径中
表示进入下一层容器，而不是除法。数字索引沿用从 `0` 开始的 Python 风格负索引。
字符串索引单位固定为 Unicode 码点。上述边界和容器类别目前只记录为语法契约，
不会在 P1 执行。

## 范围与边界

闭范围两端都包含；单边范围用比较符号写在方括号内：

```xiao
middle = values[0~2, 5~6]
before = values[<2]
before_inclusive = values[<=2]
after = values[>0]
after_inclusive = values[>=0]
all_values = values[=]
```

范围越界、端点顺序和空结果的具体容器语义由后续阶段报告；P1 只保留端点
路径和包含标记。

## 步长与随机选择

步长写在来源表达式和方括号之间，对每个选择项目独立应用：

```xiao
sample = values{step}[0~8, 12~16]
sample_all = values{2}[=]
```

`[?x]` 表示无放回抽取，`[!?x]` 表示放回抽取；步长和数量都可以是任意
表达式：

```xiao
random_a = values[?count + 1]
random_b = values{base + 1}[!?pick(2)]
```

随机选择先按选择项目的书写顺序展开，再进行抽取。步长为零、负数量、越界和
随机源均留给后续容器/运行时阶段。

## 选择器赋值

选择器可以作为赋值左侧的潜在目标：

```xiao
values[0, 1] = replacement
```

P1 不决定多选写入是广播还是逐项匹配，也不决定失败时是否回滚。
