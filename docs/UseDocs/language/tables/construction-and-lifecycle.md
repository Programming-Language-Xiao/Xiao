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

05-C 只检查这些签名和返回约束，不执行构造/析构，不保证当前版本已经有作用域退出释放。
实际释放顺序、异常展开和逃逸分析见后续 Runtime 文档。
