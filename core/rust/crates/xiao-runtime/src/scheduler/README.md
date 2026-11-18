# `xiao-runtime/src/scheduler` helpers

## 工程期

22D：把线程局部 Runtime/取消上下文、根 Runtime 查找、detached 通道和可观测的中断等待辅助从调度核心拆出，减少 `scheduler.rs` 的耦合与体积。

## 职责与边界

本目录只提供 `scheduler.rs` 消费的私有辅助 trait 和函数，不维护第二份 Runtime 状态，不改变 Runtime/pool 的关闭顺序、句柄消费方式或 027–032 错误身份。Runtime activity、pool queue 和 worker drain 仍由父模块单一管理。
