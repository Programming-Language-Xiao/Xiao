---
id: language.compiler.ir
title: IR 快照
status: verified
audience: contributor
module: rust.xiao-ir
stage: "08A"
version: "0.1.0"
related:
  - ../README.md
  - ../frontend/README.md
  - ../../../../DevDocs/08a-u0-frontend-implementation.md
---

# IR 快照

状态：`verified`，快照版本：`1`。

类型化 IR 是后端的唯一输入。它携带类型、控制流、选择器、模块、所有权、释放计划、
运行时检查和源码区间。快照使用稳定 JSON；字段顺序、节点遍历和内部编号均由前端
规范化，不能把宿主地址或哈希表迭代顺序写入文件。

集合在 IR 中仍然无序，不能生成索引选择器；`?x` 与 `!?x` 保留不同的随机模式。`try`、
`catch`、`finally` 和 `raise` 沿用统一错误与释放计划，不会被后端重新解释。

## 限制

当前页面只说明静态产物。IR 尚不能直接运行；VM、LLVM、优化和 CLI 接入属于后续阶段。
