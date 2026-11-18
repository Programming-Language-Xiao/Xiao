---
id: language.entry
title: 程序入口模式
status: verified
audience: learner
module: rust.xiao-syntax-functions
stage: "04D"
version: "0.1.0"
related:
  - ../README.md
  - ../functions/README.md
  - ../control-flow/README.md
  - ../../../DevDocs/04-functions-and-control.md
---

# 程序入口模式

Xiao 当前前端记录两种入口模式：

## 脚本模式

源码中没有 `[main]` 时，程序记录为脚本模式，顶层语句在未来执行阶段组成默认入口：

```xiao
print("hello")
```

## 工程模式

源码中出现独立的 `[main]` 表头时，程序记录为工程模式：

```xiao
[main]
print("hello")
```

本页只描述入口元数据的静态形状：`Program.entry_mode` 保存入口模式，它不生成启动函数，也不决定多模块
初始化顺序。重复或结构非法的入口表头会产生 `X04-PARSE-006`。模块、配置与启动接线已由第 05、08、09、10
阶段交付，见[命令行参考](../../tooling/cli/README.md)。
