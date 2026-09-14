---
id: language.collections.indexing
title: 声明路径与精确索引
status: verified
audience: learner
module: rust.xiao-types
stage: "03A"
version: "0.1.0"
related:
  - README.md
  - arrays.md
  - dictionaries.md
  - errors.md
---

# 声明路径与精确索引

C0 的数字索引从零开始。斜杠只表示继续进入嵌套容器，不表示除法。

## 声明路径

```xiao
int list[3/2]
```

这会为 `list` 登记一条从外层索引 `3` 到内层索引 `2` 的 `int` 约束。没有初始化值时，
类型检查器只输出静态物化计划；它不会在当前阶段创建运行时数组或填入 `0`。

同名容器可以追加更深路径；同一路径后出现的声明覆盖先前约束：

```xiao
str list3[2]
int list3[2/0]
```

如果容器已经用一个静态可知的值初始化，后续追加的路径约束会立刻检查已知元素；
静态越界或类型冲突不会等到运行时才暴露。空数组或未知长度容器则只保留物化计划，
由后续 Runtime 按计划扩容和填充默认值。

## C0 精确读取

```xiao
items = ["python", "rust"]
first = items[0]
```

当前只允许一个精确选择项。下面这些形态会被明确拒绝，交由后续 C1：

```xiao
items[0, 1]   # 多选
items[0~1]    # 范围
items{2}[=]   # 步长/全选
items[?1]     # 随机
```

数组、元组、字典表和字典列都可以沿已知结构下降；静态已知越界或不存在的键会报错。

## 下一步

错误编号和排错建议见[容器错误](errors.md)。
