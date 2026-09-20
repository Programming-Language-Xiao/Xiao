---
id: language.tables
title: 表与生命周期
status: verified
audience: learner
module: rust.xiao-types-tables
stage: "05C"
version: "0.1.0"
related:
  - ../README.md
  - definitions.md
  - members-and-visibility.md
  - construction-and-lifecycle.md
  - errors.md
  - ../../../DevDocs/05c-table-static-closure.md
---

# 表与生命周期

表是 Xiao 中用于组织字段和方法的命名空间/实例模板。当前版本已经验证表头、成员、
可见性和构造签名的静态检查；创建实例、字段读写、方法调用和 `init`/`drop` 已在
09R2H 的三种研究执行器中验证。

## 阅读顺序

1. [定义表](definitions.md)
2. [成员与可见性](members-and-visibility.md)
3. [构造与生命周期契约](construction-and-lifecycle.md)
4. [表错误](errors.md)

## 前置知识

先阅读[基础变量与表达式](../basics/README.md)和[函数](../functions/README.md)，尤其是缩进、
`def`、参数和 `return`。

## 当前边界

表体字段初始化必须是静态纯表达式。正式 `xiao run` 尚未开放，方法值的动态调用仍未
接通；运行期行为见 [Runtime 表生命周期](../memory/runtime/table-lifecycle.md)。
