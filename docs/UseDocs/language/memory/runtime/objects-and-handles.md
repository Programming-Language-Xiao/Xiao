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

## 集合载荷

集合句柄保存去重后的有序元素快照，成员判定和去重使用值相等性与线性扫描，不依赖
哈希索引。研究 VM 在 09R2F1 中通过该句柄执行并集、交集、差集、对称差、子集/超集
比较和成员判断；元素不可哈希时复用 `X06-RUNTIME-016`。元素序列只用于稳定的研究
向量观察，不改变集合在语言层的无序语义。

## 策略边界

引用计数策略通过可插拔接口提供，当前实现仅支持单线程非原子计数，因此不得在线程之间传递句柄。
