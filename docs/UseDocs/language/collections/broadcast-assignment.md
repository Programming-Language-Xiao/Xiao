---
id: language.collections.broadcast-assignment
title: 选择器广播赋值
status: verified
audience: learner
module: rust.xiao-types-selectors
stage: "03B"
version: "0.1.0"
related:
  - README.md
  - advanced-selection.md
  - random-selection.md
  - errors.md
---

# 选择器广播赋值

本页说明 Xiao 0.1.0 在 C1 类型阶段对选择器左值的约束。确定性的多选或范围可以接收
一个标量，标量会复制到每一个命中位置；当前版本只生成事务性写入计划，后续 Runtime
才会修改真实容器。

## 基本写法

```xiao
items = [1, 2, 3]
items[0, 2] = 0
```

右侧 `0` 是标量，因此会同时成为索引 `0` 和 `2` 的新值。范围、全选和嵌套路径也遵循
同一规则：

```xiao
items[0~1] = 9
nested = [[1, 2], [3, 4]]
nested[0/0, 1/1] = 7
```

## 必须满足的条件

1. 根容器必须是直接名称，且已经初始化、可变。
2. 右侧必须是能赋给每个目标元素的标量；数组、元组、字典、集合和 `none` 不会与目标
   逐项配对，违反时报告 `X03-TYPE-012`。
3. 只支持普通 `=`。`+=`、`-=` 等复合赋值需要先读取目标，当前不作为选择器广播写入。
4. 随机选择没有确定目标，不能写在赋值左侧；复杂表达式根（例如函数返回值）也暂不支持。
5. 目标边界或长度无法静态确定时，Runtime 必须在任何写入前验证全部目标；一个目标失败，
   整次写入回滚，不允许留下部分修改。

类型阶段会检查静态目标的元素类型。若标量不兼容，使用 `X03-TYPE-001`；不可变或未
初始化根、随机目标、容器右值等选择器赋值约束使用 `X03-TYPE-012`。这些编号和事务
语义在字节码与 LLVM 后端中保持一致。

## 当前边界

C1 的 `BroadcastAssignmentPlan` 只记录根名称、静态目标路径、右值类型、动态边界标志和
事务要求。它不会创建数组、执行写入或改变 Runtime 状态，因此在尚未接入 Runtime 的构建
中，示例只可用于类型检查和诊断验证。

## 下一步

选择语法、嵌套结果和步长见[高级选择与结果形状](advanced-selection.md)，随机选择见
[随机选择与种子](random-selection.md)，错误编号见[容器错误与诊断](errors.md)。

返回[容器与集合](README.md)。
