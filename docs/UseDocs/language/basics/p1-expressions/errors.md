---
id: language.basics.p1-expressions.errors
title: P1 表达式错误与边界
status: verified
audience: learner
module: rust.xiao-syntax
stage: "01E"
version: "0.1.0"
related:
  - README.md
  - selectors.md
  - ../../../troubleshooting/diagnostics-structure.md
---

# P1 表达式错误与边界

[返回 P1 主题索引](README.md) · [上一页：索引选择器](selectors.md)

解析器遇到错误时尽量保留已经建立的节点，并同步到逗号、右分隔符、换行、
反缩进或文件结束，让后续顶层语句仍有机会被解析。

## 稳定诊断编号

| 编号 | 典型原因 |
| --- | --- |
| `X01-PARSE-006` | 空选择器、非法路径段、缺少范围端点或随机数量 |
| `X01-PARSE-007` | 缺少 `)`、`]`、`}` 等配对分隔符 |
| `X01-PARSE-008` | `as` 后不是八种受支持的标量类型 |
| `X01-PARSE-009` | 一元/二元/分组/调用表达式缺少操作数 |
| `X01-PARSE-010` | 赋值右侧无效或表达式后仍有未预期内容 |

例如，以下写法会被拒绝：

```xiao
value[]       # 空选择器
value[1~]     # 缺少范围右端点
value as tuple # P1 的 as 目标必须是标量
value[0],[1]  # 方括号外的续写选择项
```

多选和多范围的逗号只能出现在同一对方括号内部。`value[0],[1]` 不会被猜测
成两个选择器；它会保留 P1 的未预期尾部诊断。

## 阶段边界

P1 不执行 Xiao 代码，也不检查：

- 目标是否真的可索引、可写或属于支持该选择器的容器；
- 索引/范围是否越界、端点顺序是否有效、步长是否为零；
- 随机数量是否非负、无放回数量是否超过可选元素数；
- `as` 的源值类型、转换失败和运行时错误。

这些问题应由后续类型、容器和运行时阶段产生各自稳定的语义诊断，而不是在
语法阶段猜测。
