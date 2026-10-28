# 边界

该文件集中维护 Runtime 活动计数、关闭 drain、线程句柄、任务句柄、任务池队列和取消上下文；这些状态通过同一组私有字段和条件变量共同完成关闭协议，不能按单个公开函数机械切开。

# 理由

Runtime close、pool shutdown、worker 活动和 detached 回收之间存在交叉锁顺序。拆分时若复制或扩大私有状态，会破坏 22C 已验证的跨 Runtime 关闭、队列取消和 worker 不自 join 约束。当前超限是既有并发调度器与 22D 原生 trampoline 所需上下文辅助共同造成的硬耦合。

# 替代方案

新增的 C ABI 盒子和错误传递已经放在 `abi_concurrency.rs`；调度器只增加最小的当前 Runtime、取消安全点和 detached 记录入口。把 scheduler 的队列/关闭状态拆成多个 crate 或互相引用的模块会引入额外锁边界，不能降低风险。

# 移除计划

在 22D/22E 并发关闭语义和三平台门禁稳定后，按 `RuntimeHandle`、`TaskPool`、`ThreadHandle` 三个协议域重构，并以关闭 drain、取消四位置和 detached 回归作为拆分门槛；重构完成后删除本说明。
