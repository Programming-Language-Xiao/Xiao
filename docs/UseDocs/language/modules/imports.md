---
id: language.modules.imports
title: 导入本地模块
status: verified
audience: learner
module: rust.xiao-modules
stage: "05"
version: "0.1.0"
related:
  - README.md
  - project-layout.md
  - scope-and-exports.md
  - errors.md
  - ../../../DevDocs/05-tables-and-projects.md
---

# 导入本地模块

Xiao 使用以文件为基础的绝对导入。普通 `.xiao` 文件由项目根目录下的路径映射为模块，
导入时不需要额外的初始化文件。

## 导入整个模块

```xiao
import app.http
import app.http as http
```

没有 `as` 时，绑定的是路径的第一个名称（上例中的 `app`）；有 `as` 时，绑定完整目标
（上例中的 `http`）。一个语句可以列出多个模块：

```xiao
import app.http, app.models as models
```

模块路径段只能使用大小写精确匹配的 ASCII 标识符。路径中的反引号名称、相对前缀、动态
字符串和括号续行不属于当前语法。

## 选择模块符号

```xiao
from app.models import User
from app.models import User as ModelUser, `显示名` as `用户显示名`
```

选择项可以有多个，也可以使用普通或反引号名称作为符号和别名。通配导入（`*`）不支持。
从文件模块选择出的值遵守 Xiao 的类型锁定规则；从目录命名空间选择出的子模块仍是限定符，
不是普通运行时值。

## 位置与加载边界

导入语句可以出现在顶层、函数、分支和循环体。编译期会静态发现所有导入边并检查目标；
Runtime 只有执行到对应语句时才初始化目标模块，同一模块在一次运行中至多初始化一次。
本阶段只验证源码和静态依赖图，不执行模块初始化。
