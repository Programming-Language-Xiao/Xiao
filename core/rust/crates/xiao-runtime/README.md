# `xiao-runtime`

## 目录职责

Rust 执行 Runtime 的分层 crate。06-B 首版交付不透明对象头、单线程引用计数、
`Weak`、`str`、标量值和静态签名表生命周期；10B 通过独立 ABI 接入数组/元组/集合/字典
和表描述符。I/O、字节码执行、异常对象和调试窗口由后续阶段接入。

## 工程期

03、06、07 建立语义；09 接入 VM；10/15 为 LLVM 原生程序提供可裁剪 ABI；11C 接入系统文案和调试事件；22C 为 VM 并发提供 Runtime 调度器，22D 开始通过独立 C ABI 接入 LLVM 内部并发路径。

## 模块放置

`src/` 下按 `value`、`tables`、`memory`、`errors`、`abi`、`scheduler`、`concurrency` 和 `testing` 分模块；`abi_concurrency.rs` 只负责 C ABI 盒子、XCON 边界、Runtime trampoline、取消与共享锁接线，不公开 `RuntimeHandle`、任务池或共享对象的 Rust 布局；`testing` 现提供 07-B 的
`dispatch_catch`/`dispatch_fatal` 规格路由；后续容器、I/O
和调试目录必须沿用同样的单一职责边界。平台系统调用只经 `xiao-platform`。

## 约束

不使用追踪式全堆 GC；平台差异通过 `xiao-platform` 注入；Runtime 组件按调用图裁剪但
不能删除可观察错误、释放、随机或 I/O。普通 06-B `StrongHandle`/`WeakHandle` 保持单线程；
22C/22D 并发共享对象使用独立 32B 原子头和不透明 C ABI，不把其内部布局替换成普通句柄。
