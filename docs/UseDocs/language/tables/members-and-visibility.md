---
id: language.tables.members
title: 表成员与可见性
status: verified
audience: learner
module: rust.xiao-types-tables
stage: "05C"
version: "0.1.0"
related:
  - README.md
  - definitions.md
  - errors.md
  - ../../../DevDocs/05c-table-static-closure.md
---

# 表成员与可见性

字段和方法共享同一个表成员名称空间。成员名以下划线开头时默认私有，其余成员默认公开：

```xiao
[Config]
    _token = "internal"
    host = "localhost"

    def public_host(self) -> str
        return self.host
```

表内部可以使用 `self.member` 访问自身成员；单例表可以使用 `Config.member` 访问公开成员。
实例字段使用 `instance.member`。表外访问 `_token` 之类的私有成员会产生稳定的可见性错误，
而不是被静默当作动态值。

## 类型锁定

字段初始化后类型由显式声明或初始化表达式确定。显式声明的初值必须可赋给声明类型：

```xiao
[Config]
    int port = 8080
```

字段初始化器只能使用字面量、已知常量、纯运算和纯容器；`input`、`print`、普通函数调用、
`new`、成员链和选择器不能作为表字段初始化器。
