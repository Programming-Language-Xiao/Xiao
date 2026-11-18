---
id: language.collections.tuples
title: 元组
status: verified
audience: learner
module: rust.xiao-syntax
stage: "03A"
version: "0.1.0"
related:
  - README.md
  - indexing.md
  - advanced-selection.md
  - random-selection.md
---

# 元组

元组采用 Python 风格圆括号，长度固定并按位置保存类型。

```xiao
empty = ()
one = ("only",)
pair = ("x", 1)
group = (1) # 没有逗号时是分组表达式，不是元组
```

C1 已验证元组的多选、范围、步长和随机选择计划；09R2 研究 VM 已执行读取、抽样和结果
构造。选择结果保留元组的根类型和必要的嵌套形状，零命中时返回空元组。

单项精确读取和嵌套路径遵循[声明路径与精确索引](indexing.md)的规则。
高级选择写法见[高级选择与结果形状](advanced-selection.md)，随机规则见[随机选择与种子](random-selection.md)。
