# `xiao-runtime/src/testing`

## 工程期

06-B。

## 职责与边界

这里放 06-B 的 Rust 规格测试驱动器。驱动器把 `xiao-lifetime` 的 `ReleasePlan` 映射
到真实 Runtime 句柄，并验证 `finally`、`drop`、错误原因链、`catch` 类型路由和 Fatal 隔离，以及计数平衡。它不启动 VM、
LLVM、CLI 或并发执行器。
