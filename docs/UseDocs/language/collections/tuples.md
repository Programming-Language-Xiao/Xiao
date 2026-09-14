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
---

# 元组

元组采用 Python 风格圆括号，长度固定并按位置保存类型。

```xiao
empty = ()
one = ("only",)
pair = ("x", 1)
group = (1) # 没有逗号时是分组表达式，不是元组
```

C0 只建立 AST 和静态类型；元组的范围、多选、步长和随机选择留给 C1。

单项精确读取和嵌套路径遵循[声明路径与精确索引](indexing.md)的规则。
