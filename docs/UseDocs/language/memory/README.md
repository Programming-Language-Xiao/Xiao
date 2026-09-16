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
2. [Runtime 对象与句柄](runtime/objects-and-handles.md)
3. [表生命周期执行](runtime/table-lifecycle.md)
4. [Runtime 错误与展开](runtime/errors-and-unwind.md)
5. [释放、循环引用与诊断](release-and-errors.md)

## 当前能力边界

页面状态 `verified` 表示对应页面描述的实现和测试已经完成。06-A 页面仍只描述静态分析；
Runtime 子页描述可由 Rust 测试驱动器直接验证的对象、表和错误展开能力，不代表已经接入
字节码 VM 或 LLVM。表的静态 `init/drop` 签名规则见[表与生命周期](../tables/README.md)。
