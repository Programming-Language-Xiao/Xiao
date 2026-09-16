# `xiao-runtime`

## 目录职责

Rust 执行 Runtime 的分层 crate。06-B 首版交付不透明对象头、单线程引用计数、
`Weak`、`str`、标量值和静态签名表生命周期；数组/元组/集合/字典、I/O、字节码执行
和调试窗口由后续阶段接入。

## 工程期

03、06、07 建立语义；09 接入 VM；10/15 为 LLVM 原生程序提供可裁剪 ABI；11C 接入系统文案和调试事件。

## 模块放置

`src/` 下按 `value`、`tables`、`memory`、`errors` 和 `testing` 分模块；`testing` 现提供 07-B 的
`dispatch_catch`/`dispatch_fatal` 规格路由；后续容器、I/O
和调试目录必须沿用同样的单一职责边界。平台系统调用只经 `xiao-platform`。

## 约束

不使用追踪式全堆 GC；平台差异通过 `xiao-platform` 注入；Runtime 组件按调用图裁剪但
不能删除可观察错误、释放、随机或 I/O。06-B 的句柄明确标记为单线程，未来原子计数
策略必须保持同一公开语义。
