# `xiao-runtime`

## 目录职责

Rust 执行 Runtime：值表示、数组/元组/集合/字典、字符串、引用计数、`Weak`、`drop`、错误展开、输入输出和运行时动态检查。

## 工程期

03、06、07 建立语义；09 接入 VM；10/15 为 LLVM 原生程序提供可裁剪 ABI；11C 接入系统文案和调试事件。

## 模块放置

`src/` 下按 `value`、`collections`、`memory`、`errors`、`io` 和 `debug` 分模块；平台系统调用只经 `xiao-platform`。

## 约束

不使用追踪式全堆 GC；平台差异通过 `xiao-platform` 注入；Runtime 组件按调用图裁剪但不能删除可观察错误、释放、随机或 I/O。
