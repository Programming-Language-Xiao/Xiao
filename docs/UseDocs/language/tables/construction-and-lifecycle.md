---
id: language.tables.construction
title: 构造与生命周期契约
status: verified
audience: learner
module: rust.xiao-types-tables
stage: "05C"
version: "0.1.0"
related:
  - README.md
  - definitions.md
  - errors.md
  - ../../../DevDocs/06-memory-and-runtime.md
  - ../../../DevDocs/05c-table-static-closure.md
---

# 构造与生命周期契约

只有 `[[Name]]` 表可以作为 `new` 的目标：

```xiao
[[User]]
    def init(self, int id) -> none
        return

user = new User(1)
```

`new` 的参数按 `init` 去掉首个 `self` 后的参数进行位置/关键字匹配。没有 `init` 时只能
无参构造。`[Config]` 单例表不能使用 `new`。

## `init` 与 `drop`

方法的首个参数必须是普通名称 `self`。`init` 可以在 `self` 后声明构造参数，返回类型必须
是 `none`；`drop` 只能声明 `drop(self)`，返回类型也必须是 `none`：

```xiao
[[Resource]]
    def init(self, str path) -> none
        return

    def drop(self) -> none
        return
```

05-C 负责这些静态契约；09R2H 已在研究执行器中接通构造、方法调用和析构。
`[Config]` 在声明处初始化一次，`[[User]]` 在每次 `new` 时重新求值字段默认值并执行 `init`。
最后一个强引用释放时执行一次 `drop`，析构期间的 `self` 只读。

实际释放与失败回滚见 [Runtime 表生命周期](../memory/runtime/table-lifecycle.md)。
正式 `xiao run` 尚未开放，研究执行器通过测试入口验证上述行为。
