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
  - advanced-selection.md
  - random-selection.md
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

字典表不能依赖书写顺序进行数字索引，只能沿键名进行精确路径读取。范围、多选、步长和
随机选择会被拒绝。20J 支持单个动态键：同名静态键优先；没有同名键时，`str` 变量或
字符串表达式按键读取，缺失键报告 `X06-RUNTIME-015`。

```xiao
values = {name = 10, other = 20}
key = "other"
print(values[name])       # 静态键 name
print(values[key])        # 动态键 other
print(values[key + ""])   # 动态键表达式
```

## 字典列

尖括号表示保留书写顺序的字典列：

```xiao
column = <name = "Xiao", level = 1>
```

C0 的精确读取允许数字位置、键名和嵌套组合；C1 进一步允许范围、多选、步长和随机选择。
字典列也可以使用显式标量前缀约束全部值：

```xiao
int levels = <first = 1, second = 2>
```

值不满足前缀类型时在类型检查阶段报错，但键和字典列顺序仍然保留。

字典列同样支持动态键，并保留静态键优先。没有同名键时，整数变量按位置、字符串变量按
动态键分派；显式整数表达式继续按位置读取。

```xiao
column = <i = 10, j = 20>
i = 1
key = "j"
print(column[i])       # 同名静态键 i
print(column[key])     # 动态键 j
print(column[i + 0])   # 位置 1
```

字典列中的键必须唯一。若选择器按数字和键名重复选中同一个直接键，或放回抽取产生重复
键，结果会按选择顺序表示为元组，以避免伪造重复键的字典列。09R2 研究 VM 已执行这些
路径和形状规则；无序字典表的高级选择仍在运行时稳定报告 `X06-RUNTIME-017`。详见[高级选择与结果形状](advanced-selection.md)
和[随机选择与种子](random-selection.md)。

## 动态写入与开放字典参数

20K 支持更新已有键或字典列位置：

```xiao
values = {name = 1}
key = "name"
values[key] = 2
column = <first = 3, second = 4>
i = 1
column[i] = 5
```

动态缺失键报告 `X06-RUNTIME-015`，动态位置越界报告 `X06-RUNTIME-017`；赋值不会新增键。
静态可判定的缺失键和越界仍在类型检查阶段拒绝。

函数参数和返回值可以使用开放键集注解 `dict<str, T>`。`T` 可以是标量、资源、`none`、数组、
集合或类型参数，固定字典字面量只要所有值兼容即可传入：

```xiao
def get(dict<str, int> values, str key) -> int
    return values[key]
```

官方 `stdlib.dict` 提供 `get`、`set`、`remove`、`contains`、`keys`、`values` 六个自由函数。
`remove` 的缺失键同样报告 015；`keys` 和 `values` 返回新数组并保持对应迭代顺序。

字典表和字典列也支持直接方法调用，方法与自由函数等价：

```xiao
values = {name = 1}
values.set("name", 2)
print(values.get("name"))
print(values.size())
print(values.is_empty())
```

方法面包括 `get`、`set`、`remove`、`contains`、`keys`、`values`、`size`、`is_empty`。
方法键参数固定为 `str`；字典列的数值位置继续使用 `column[i]`。动态接收者、方法值和
未知方法分别报告 `X03-TYPE-027`、`X03-TYPE-028`、`X03-TYPE-029`。

## 键名与 Unicode

需要 Unicode 的键或名称可以使用反引号：

```xiao
profile = {`显示名` = "星崽"}
```

更多路径示例见[声明路径与精确索引](indexing.md)。
