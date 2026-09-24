---
id: tooling.config.syntax
title: 静态配置语法
status: verified
audience: learner
module: rust.xiao-config
stage: "05-D/11A-D1"
version: "0.1.0"
related:
  - README.md
  - errors.md
  - ../../language/modules/project-layout.md
  - ../../../DevDocs/05d-config-static-closure.md
---

# 静态配置语法

项目配置文件的名称固定为小写 `config.xiao`。读取配置只收集声明，不运行其中的
函数或项目代码。

## 最小项目配置

项目配置可以声明身份和包外公开模块：

```xiao
[project]
name = "hello"
version = "0.1.0"

[exports]
api = "src/api.xiao"
```

`project.name` 与 `project.version` 都必须是非空字符串。`exports` 的键是包外名称，
值是项目根相对、以 `.xiao` 结尾的模块路径；绝对路径和 `..` 路径会被拒绝。

## 静态值

字段值可以是字符串、整数、有限浮点数、布尔值、数组或字典表，并可递归嵌套：

```xiao
[runtime]
options = [1, 2.5, true, { label = "demo" }]
```

字典表使用花括号，键值使用 `=`，空 `{}` 是空字典。数组元素使用逗号分隔，允许
尾逗号。反引号名称和字符串键会先解码为 UTF-8 文本。

## 保留扩展表

`[CLI]`、`[language]`、`[debug]`、`[VM]`、依赖和构建相关表可以先以静态值保留，
供后续工具阶段读取；本阶段不解释这些表的专属行为。`[language].locale` 的语言
回退和语言包规则由 11C 页面定义。

配置中不能出现 `def`、`if`、`for`、`while`、`import`、函数调用、变量引用、常量
引用、运算表达式或源码 `[main]`。源码中的 `[main]` 仍由语言解析器处理，不写入配置树。

## 本地路径依赖

11A-D1 的依赖条目使用字典形式。`path` 必填且必须是声明包根目录相对路径；`version`
和 `source` 可选，当前只保存为静态约束，不执行完整版本求解或远程源解析：

```xiao
[dependencies]
utils = { path = "../utils", version = "^1.4", source = "local" }

[devdependencies]
testkit = { path = "../testkit" }
```

依赖表字段只允许 `path`、`version` 和 `source`。包源的 `source_id`、`alias` 与展示名
属于包管理器的不同概念；展示名不能作为依赖引用键。

## 下一步

格式不正确时查看[配置错误与修复](errors.md)；需要理解配置与源码边界时返回[项目
文件布局](../../language/modules/project-layout.md)。
