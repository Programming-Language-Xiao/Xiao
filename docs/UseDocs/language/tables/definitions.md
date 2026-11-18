---
id: language.tables.definitions
title: 定义表
status: verified
audience: learner
module: rust.xiao-syntax
stage: "05C"
version: "0.1.0"
related:
  - README.md
  - members-and-visibility.md
  - ../../../DevDocs/05-tables-and-projects.md
---

# 定义表

表头必须顶格并位于文件顶层，表体缩进一级。`[Name]` 定义单例表，`[[Name]]` 定义可实例
化表模板：

```xiao
[Config]
    host = "localhost"
    int port = 8080

[[User]]
    str name = ""
```

表名目前必须是非保留的 ASCII 标识符。表体可以包含字段赋值、显式字段声明、`const` 字段
和 `def` 方法；普通表达式、控制流和嵌套表头不属于表体成员。

## 表的身份

单例表可以通过 `Config.host` 形式作为一个稳定命名空间引用。可实例化表在静态类型层先
表示为构造目标，只有 `new User(...)` 才得到实例类型。表不会因为使用花括号字面量而自动
变成表；花括号仍然表示字典表或集合，具体语义见[容器与集合](../collections/README.md)。

## 方法

方法使用普通函数语法，方法体继续使用缩进：

```xiao
[Config]
    host = "localhost"

    def address(self) -> str
        return self.host
```

当前静态检查会保留方法签名和返回类型，但不会调用方法。
