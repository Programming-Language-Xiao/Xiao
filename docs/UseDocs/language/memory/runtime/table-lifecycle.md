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

每次构造先求值字段默认值，再执行 `init`；容器默认值为各实例独立创建。没有初始化器的
字段可以在 `init` 中赋值，提前读取会报告错误。

最后一个强引用释放时进入 `Dropping`，`drop` 恰好执行一次；别名和容器持有都会延后
这一时刻。析构中的 `self` 只读，允许观察仍存活的字段，不能写入字段或复活对象。
初始化失败保留原始原因，清理失败作为次生错误附加；致命故障不执行用户析构代码。

这些路径已在 09R2H 的三种研究执行器中验证；正式 `xiao run` 尚未开放。
