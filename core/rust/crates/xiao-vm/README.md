# `xiao-vm`

## 目录职责

Rust 字节码唯一执行实现：解释循环、调用栈、局部槽、模块加载、错误回溯、确定性随机源和 `-debug` 诊断事件。

## 工程期

09 最小运行闭环；14 接入优化 `.xiaoc` 加载和只读验证；后续可在独立阶段加入 JIT，不原地修改公开字节码。

## 模块放置

`src/` 下按 `interpreter`、`stack`、`loader`、`debug` 和 `random` 分模块；REPL 缓冲区放在 TypeScript CLI。

## 09R 研究边界

09R2 起在 `src/research/` 建立研究用解释器框架，栈式、分类型寄存器式和混合式三种候选机型
共用同一组语义向量和同一个 `VmEventSink` 事件接收器。该子模块属于 09R 特别研究工程，
**不是稳定执行接口**：三种机型在 09R3 冻结前都不得被当作生产 VM 暴露，也不得让
TypeScript 层依赖其中任何一型的内部结构。

冻结结论与施工顺序见 [09R. 字节码寄存器机型特别研究](../../../../docs/DevDocs/09r-bytecode-machine-research.md)。

## 边界

不处理 TypeScript REPL 状态、不解析 CLI 参数、不依赖人类可读日志判断控制流。
