---
id: language.collections.dictionaries
title: 字典表与字典列
status: verified
audience: learner
module: rust.xiao-types
stage: "03A"
version: "0.1.0"
related:
  - README.md
  - indexing.md
  - errors.md
---

# 字典表与字典列

字典键值对使用 `=` 连接。键可以是裸名称、反引号名称或字符串；同一个容器中规范化后的
重复键会被拒绝。

## 字典表

花括号表示无序字典表：

```xiao
profile = {name = "Xiao", level = 1}
empty = {}
```

字典表不能依赖书写顺序进行数字索引，只能沿键名进行 C0 精确路径读取。

## 字典列

尖括号表示保留书写顺序的字典列：

```xiao
column = <name = "Xiao", level = 1>
```

C0 的精确读取允许数字位置、键名和嵌套组合；范围、多选和随机选择仍未实现。字典列也可以
使用显式标量前缀约束全部值：

```xiao
int levels = <first = 1, second = 2>
```

值不满足前缀类型时在类型检查阶段报错，但键和字典列顺序仍然保留。

## 键名与 Unicode

需要 Unicode 的键或名称可以使用反引号：

```xiao
profile = {`显示名` = "星崽"}
```

更多路径示例见[声明路径与精确索引](indexing.md)。
