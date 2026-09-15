---
id: language.control-flow.conditions-and-loops
title: 条件与循环
status: verified
audience: learner
module: rust.xiao-types-control
stage: "04C"
version: "0.1.0"
related:
  - README.md
  - returns-and-control.md
  - errors.md
  - ../../../DevDocs/04-functions-and-control.md
---

# 条件与循环

分支和循环体使用缩进，不写花括号或冒号：

```xiao
if ready
    print("go")
elif waiting
    print("wait")
else
    print("stop")

for item in values
    print(item)

while ready
    ready = false
```

解析器会保留 `if` 的 `elif`/`else` 分支和每个循环体的源码区间。嵌套块的反缩进只结束当前块，不会吞掉外层
分支或后续顶层语句。

## 条件类型

`if`、`elif` 和 `while` 的条件必须是 `bool`。整数、字符串和容器不会像 Python 一样隐式转换为真值：

```xiao
if true
    print("ok")
```

静态已知的非 `bool` 条件会产生 `X04-TYPE-005`。动态表达式会登记 `BooleanCondition` 检查，交由后续
Runtime 在执行时验证。

## `for in` 可迭代对象

数组、元组、集合、字典表和字典列可以作为已知可迭代对象。集合和字典表的遍历顺序不保证，字典列保留其定义
顺序；本阶段只验证形状，不执行遍历。动态右值会登记 `Iterable` 检查，标量等已知不可迭代值产生
`X04-TYPE-006`。
