# `xiao-vm`

## 目录职责

Rust 字节码唯一执行实现：解释循环、调用栈、局部槽、模块加载、错误回溯、确定性随机源和 `-debug` 诊断事件。

## 工程期

09 最小运行闭环；14 接入优化 `.xiaoc` 加载和只读验证；后续可在独立阶段加入 JIT，不原地修改公开字节码。

## 模块放置

`src/` 下按 `interpreter`、`stack`、`loader`、`debug` 和 `random` 分模块；REPL 缓冲区放在 TypeScript CLI。

## 边界

不处理 TypeScript REPL 状态、不解析 CLI 参数、不依赖人类可读日志判断控制流。
