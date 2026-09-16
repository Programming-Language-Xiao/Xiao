---
id: language.memory.runtime.objects-and-handles
title: Runtime 对象与句柄
status: verified
audience: learner
module: rust.xiao-runtime
stage: "06B"
version: "0.1.0"
related:
  - README.md
  - table-lifecycle.md
  - errors-and-unwind.md
  - ../../../../DevDocs/06b-runtime-objects-and-tables.md
---

# 对象与句柄

运行时对象由不透明对象头和载荷组成。用户代码不依赖对象头布局，只通过运行时句柄访问对象。

## 强引用

复制 `StrongHandle` 会增加强计数；释放最后一个强引用时执行载荷释放钩子。释放钩子只执行一次，随后对象进入已释放状态。

## 弱引用

`WeakHandle` 不保持载荷存活。升级弱引用前必须检查对象仍可用；载荷释放后升级失败。对象头在最后一个弱引用也释放后才销毁。

## 策略边界

引用计数策略通过可插拔接口提供，当前实现仅支持单线程非原子计数，因此不得在线程之间传递句柄。
