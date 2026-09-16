---
id: language.memory.runtime.table-lifecycle
title: Runtime 表生命周期
status: verified
audience: learner
module: rust.xiao-runtime
stage: "06B"
version: "0.1.0"
related:
  - README.md
  - objects-and-handles.md
  - errors-and-unwind.md
  - ../../../../DevDocs/06b-runtime-objects-and-tables.md
---

# 表生命周期

表实例由运行时按照静态 `TableSignature` 创建。字段类型、可见性和钩子信息在构造前已经确定。

## 状态顺序

`Allocated -> FieldsInitializing -> InitCompleted -> Usable -> Dropping -> Released`

初始化完成后实例才可供普通代码使用。`init` 失败会回滚已初始化字段并释放对象。

## 两种表

- `[Table]` 表示单例对象，由模块级上下文持有。
- `[[Table]]` 表示可实例化对象，可通过构造流程创建多个实例。

## 生命周期钩子

`init` 在字段初始化后运行；离开所有者作用域时进入 `Dropping` 并运行 `drop`。清理期间允许钩子读取仍存活的字段，但不得改变字段类型或新增成员。
