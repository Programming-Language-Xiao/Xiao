---
id: tooling.config.syntax
title: 静态配置语法
status: verified
audience: learner
module: rust.xiao-config
stage: "05-D/11A-D1/E3A"
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
供后续工具阶段读取；`[sources]` 在 11A-E3A 起已校验具名条目。`[language].locale` 的语言
回退和语言包规则由 11C 页面定义。

配置中不能出现 `def`、`if`、`for`、`while`、`import`、函数调用、变量引用、常量
引用、运算表达式或源码 `[main]`。源码中的 `[main]` 仍由语言解析器处理，不写入配置树。

## 本地路径依赖

11A-D1 的本地路径依赖使用字典形式。`path` 必填且必须是声明包根目录相对路径；
`version` 和 `source` 可选，运行时会校验本地包版本及来源：

```xiao
[dependencies]
utils = { path = "../utils", version = "^1.4", source = "local" }

[devdependencies]
testkit = { path = "../testkit" }
```

远程索引包不写 `path`，但必须写版本约束；`source` 可省略并按配置顺序选择。
例如在 `[sources]` 声明名为 `community` 的静态源，再在 `[dependencies]`
声明 `utils = { version = "1.2.*", source = "community" }`。
直接声明与索引里的传递依赖共用 SemVer 解析；只写 `source` 而没有
`version`/`path`/`git` 仍是不完整的依赖声明。

E3C 还允许直接 Git 仓库声明（只解析静态声明，不进行远程版本求解）：

```xiao
[dependencies]
lib = { git = "https://github.com/acme/lib.git", rev = "v1.2.0", version = "^1" }
```

`git` 与 `path`/`source` 互斥；`rev`、`tag`、`branch` 中必须且仅能选一个。
缺省引用、多个引用或不安全的引用会报配置错误；本地路径同步遇到 Git 依赖会明确拒绝，
此种独立 Git 依赖仍待单独接线；通过 `[sources]` 的 `git-index` 可参与远程闭环。
包源的 `source_id`、`alias` 与展示名
属于包管理器的不同概念；展示名不能作为依赖引用键。

## 离线多源声明

```xiao
[sources]
official = { kind = "registry", location = "https://example.org/packages", display = "官方" }
team = { kind = "git-index", location = "https://github.com/team/packages.git", protocol = 1 }
```

直接源必须有 `kind` 与 `location`，还可有 `alias`、`display`、`protocol`；
省略 alias 时使用左侧键名。另可声明 `{ list = "...", digest = "<SHA-256>" }`
形式的清单引用（只允许 `list`/`digest` 两字段）。书写顺序决定优先级，
但所有直接源先于导入列表展开；字段错误和别名冲突会拒绝。
Git 稀疏索引源可额外声明唯一的 `rev`、`tag` 或 `branch`；省略时使用 HEAD。
E3C 提供同步网络适配器，E3D1 已将联邦求解及源码正文校验接入包操作；细节见
[包源声明与离线索引](../cli/package-sources.md)。

## 下一步

格式不正确时查看[配置错误与修复](errors.md)；需要理解配置与源码边界时返回[项目
文件布局](../../language/modules/project-layout.md)。
