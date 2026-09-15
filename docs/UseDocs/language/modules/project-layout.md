---
id: language.modules.project-layout
title: 项目文件布局与命名空间
status: verified
audience: learner
module: rust.xiao-modules
stage: "05"
version: "0.1.0"
related:
  - README.md
  - imports.md
  - scope-and-exports.md
  - errors.md
  - ../../../DevDocs/05-tables-and-projects.md
---

# 项目文件布局与命名空间

当前阶段把传给模块分析器的项目根直接作为源码根。每个普通 `.xiao` 文件形成一个文件
模块，子目录自动形成纯目录命名空间：

```text
main.xiao             -> main
app/user.xiao         -> app.user
app/http/client.xiao  -> app.http.client
```

目录命名空间没有初始化代码，只用于限定其中的模块和名称。

## 配置和扫描边界

根目录中的 `config.xiao` 是项目配置，不是模块。子目录一旦包含自己的 `config.xiao`，
该目录被视为后续外部包边界，当前项目扫描会停止在该目录之外。点号开头的目录不参与
扫描；`target` 和 `node_modules` 在本阶段没有特殊排除规则。符号链接不跟随。

## 名称规则

文件名和目录名映射为模块段时必须是 ASCII 标识符（字母或下划线开头，后续可含数字），
且不能使用 Xiao 保留字。导入大小写必须与路径精确一致。下列情况都会报冲突，而不会
静默选择一个胜者：

```text
app.xiao 与 app/user.xiao       # 文件模块与命名空间同名
App.xiao 与 app/...              # 大小写折叠冲突
```

模块发现结果按逻辑名称排序，供后续依赖图和诊断稳定复现。
