---
id: language.memory
title: 内存与静态生命周期
status: verified
audience: learner
module: rust.xiao-lifetime
stage: "06A"
version: "0.1.0"
related:
  - ../README.md
  - static-analysis.md
  - release-and-errors.md
  - ../../../DevDocs/06a-lifetime-static-closure.md
---

# 内存与静态生命周期

Xiao 06-A 已经能在程序运行前分析作用域、逃逸、对象拥有关系和每种退出路径需要的释放
计划。用户不需要写 Rust 风格生命周期标注；动态值无法完全证明时会保守转为堆管理，
并留下 Runtime 检查。

## 阅读顺序

1. [静态分析规则](static-analysis.md)
2. [释放、循环引用与诊断](release-and-errors.md)

## 当前能力边界

页面状态 `verified` 表示静态分析 crate 和规格测试已经完成。当前版本尚未创建真实堆对象、
执行引用计数、调用 `drop` 或运行 Xiao 程序；这些执行能力会在 Runtime、字节码和 LLVM
阶段逐步接入。表的 `init/drop` 签名规则见[表与生命周期](../tables/README.md)。
