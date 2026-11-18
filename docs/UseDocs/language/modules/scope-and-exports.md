---
id: language.modules.scope-and-exports
title: 作用域与顶层导出
status: verified
audience: learner
module: rust.xiao-modules
stage: "05"
version: "0.1.0"
related:
  - README.md
  - imports.md
  - project-layout.md
  - errors.md
  - ../../../DevDocs/05-tables-and-projects.md
---

# 作用域与顶层导出

导入绑定属于出现它的词法作用域。块内导入可以在该函数、分支或循环体中使用，离开块后
不会自动成为模块属性：

```xiao
def load()
    import worker
    return worker.value
```

同一作用域中重复使用同名别名会报绑定冲突；多个没有别名、但共享同一根目录命名空间的
导入可以合并为一个限定符。模块和命名空间限定符不能单独作为值、不能赋值，也不能放入
容器；它们只允许继续访问成员。

## 顶层接口

顶层导入会进入当前文件的模块符号接口，因此可以形成包内再导出。顶层 `[Table]`/`[[Table]]`
也会作为 `Table` 类别进入模块符号接口；表成员的字段类型和私有可见性由类型阶段检查，
模块解析器不重复判断：

```xiao
# base.xiao
value = 1

# bridge.xiao
from base import value

# main.xiao
from bridge import value
```

再导出的符号保留最初来源。当前阶段只建立项目内接口；`config.xiao` 的包外公开清单、
外部依赖和可见性策略留给后续包管理阶段。

## 初始化顺序

直接导入边形成文件模块依赖图，初始化顺序按依赖优先排列。纯目录命名空间不初始化；
通过命名空间成员限定访问具体文件时，图中记录一条延迟的限定使用边。循环依赖是静态错误，
不会向后端提供部分初始化顺序。
