---
id: language.collections.random-selection
title: 随机选择与种子
status: verified
audience: learner
module: rust.xiao-types-selectors
stage: "03B"
version: "0.1.0"
related:
  - README.md
  - advanced-selection.md
  - broadcast-assignment.md
  - errors.md
---

# 随机选择与种子

本页说明 Xiao 0.1.0 的 C1 随机选择计划。随机选择适用于数组、元组、`str` 和字典列，
不适用于字典表或集合。当前类型阶段只验证数量、来源类型和边界，并记录随机计划；真实
抽样由后续 Runtime 执行。

## 两种抽样模式

```xiao
items = ("a", "b", "c")
items[?2]       # 无放回：同一位置不会重复
items[!?5]      # 放回：每次抽取后放回候选集
```

`?` 表示无放回，`!?` 表示放回。抽取结果按实际抽取顺序排列，并原则上保留来源根容器
类型。放回抽取可以超过候选元素数量；字典列若抽到同一直接键多次，结果使用按顺序排列
的元组，而不是非法的重复键字典列。

## 数量规则

- 数量必须是整数语义值；负数报告 `X03-TYPE-010`。
- 数量为 `0` 时返回来源根容器类型的空结果。
- 无放回数量大于候选数量，或从空来源抽取正数，报告 `X03-TYPE-011`。
- 放回抽取允许数量大于候选数量，但空来源仍不能抽取正数。
- 数量来自无法静态求值的表达式时，类型阶段登记 `RandomCount` 运行时检查。

```xiao
empty = items[?0]      # 空元组
count = input("数量")  # 动态值，运行时检查
picked = items[?count]
```

动态数量的实际合法性和候选集长度由 Runtime 在执行时验证；类型阶段不会为了生成一个
静态空容器而猜测随机结果。

## 随机种子

使用 `random.seed(value)` 为当前执行上下文登记种子：

```xiao
random.seed(42)
picked = items[?2]
```

种子是非负整数语义值，可以由 `int`、`sint` 或 `lint` 提供，静态值会记录在选择计划
旁的种子计划中。负数、浮点数或超出可表示范围的常量报告 `X03-TYPE-013`；参数个数
不是一个时报告 `X03-TYPE-014`。动态种子保留 `RandomSeed` 运行时检查。

类型阶段不会推进随机状态。测试和后端应使用同一可注入随机源，以便在需要时复现抽样
序列；没有设置种子时，随机源的默认来源由 Runtime 阶段另行规定。

## 与其他选择项混用

随机项可以和普通选择项出现在同一方括号中，但类型阶段先按书写顺序建立计划。随机项的
具体路径直到 Runtime 才知道，因此结果类型采用来源类型的保守上界。随机选择不能作为
选择器左值；需要赋值时请使用确定性的多选或范围，见[选择器广播赋值](broadcast-assignment.md)。

## 下一步

普通多选、范围和步长见[高级选择与结果形状](advanced-selection.md)，错误编号见
[容器错误与诊断](errors.md)。

返回[容器与集合](README.md)。
