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
  - advanced-selection.md
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

C0 的单项精确读取只允许一个精确选择项。C1 已在类型阶段验证多选、范围、步长和随机
选择；它们的写法与结果形状见[高级选择与结果形状](advanced-selection.md)。

```xiao
items[0, 1]   # C1 多选
items[0~1]    # C1 范围
items{2}[=]   # C1 步长/全选
items[?1]     # C1 随机
```

数组、元组、字典表和字典列都可以沿已知结构下降；静态已知越界或不存在的键会报错。
数字索引支持 Python 风格负索引，例如 `items[-1]` 表示最后一个元素。`str` 的索引单位
是 Unicode 码点；长度和边界无法在类型阶段确定时，会留下 Runtime 检查标记。

## 路径与高级选择

路径中的 `/` 继续进入嵌套容器，例如 `items[2/1]`。范围端点也可以是嵌套路径，类型
阶段按有序容器的深度优先顺序投影结果；无序字典表只能精确按键读取，集合不支持任何索引。

详见[高级选择与结果形状](advanced-selection.md)和[随机选择与种子](random-selection.md)。

## 下一步

错误编号和排错建议见[容器错误](errors.md)。
