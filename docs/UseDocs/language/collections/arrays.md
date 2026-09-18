---
id: language.collections.arrays
title: 数组
status: verified
audience: learner
module: rust.xiao-types
stage: "03A"
version: "0.1.0"
related:
  - README.md
  - indexing.md
  - advanced-selection.md
  - broadcast-assignment.md
  - errors.md
---

# 数组

数组使用方括号，可以保存异构的标量或其他容器。C0/C1 验证静态结构、选择计划和类型
约束；09R2 研究 VM 已执行有序数组的精确读取、高级选择和广播修改，自动扩容仍未开放。

## 创建数组

```xiao
values = [1, "two", true]
nested = [[1, 2], [3, 4]]
empty = []
```

非空数组保留每个位置的类型。空数组的静态类型是“未知长度数组”，真正的默认值填充属于
后续 Runtime。

## 显式元素类型

```xiao
int values = [1, 2, 3]
int empty = []
```

带类型前缀的数组要求所有直接元素可赋给该类型；例如 `int values = [1, "bad"]` 在类型
检查阶段失败。空数组可以先建立未知长度的 `int` 同构形状。

## 下一步

嵌套位置约束和读取方式见[声明路径与精确索引](indexing.md)；多选和范围见[高级选择与结果形状](advanced-selection.md)；
选择器写入见[选择器广播赋值](broadcast-assignment.md)；错误编号见[容器错误](errors.md)。
