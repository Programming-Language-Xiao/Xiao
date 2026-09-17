---
id: language.compiler.bytecode-runtime
title: 字节码运行路径
status: planned
audience: contributor
module: rust.xiao-bytecode
stage: "09R1"
version: "0.1.0"
related:
  - ../README.md
  - ../ir/README.md
  - ../frontend/README.md
  - ../../../../DevDocs/09r-bytecode-machine-research.md
---

# 字节码运行路径

状态：`planned`。本页说明规划中的字节码运行路径，**当前还不能执行任何 Xiao 源码**。

Xiao 的双模式执行从类型化 IR 分叉：`xiao run` 走字节码解释路径，`xiao build` 走 LLVM 原生
路径。两条路径必须共享同一套类型、溢出、求值顺序、动态检查、随机选择、容器顺序、引用计数、
`drop` 和错误诊断语义，字节码路径不得自行决定其中任何一项。

## 当前进展

字节码机型尚未冻结。09R 特别研究工程正在比较栈式、分类型寄存器式和混合式三种候选机型，
研究结果将决定最终机型、函数调用约定、异常与清理转移规则、指令编码和性能门槛。在研究结束
之前，任何"Xiao 已经能运行"或"已经选定某种机型"的说法都不成立。

## 已经确定的部分

- 字节码后端只消费已验证的类型化 IR，不重新解析源码、不重新推断类型、不重新计算释放顺序。
- 执行核心与执行时 Runtime 使用 Rust；TypeScript 只承担 CLI / REPL 的终端边界，不复制
  算术、索引、类型转换、生命周期或错误传播逻辑。
- 引用计数是确定性的，没有追踪式垃圾回收；作用域退出、提前返回和错误退出都执行确定性释放。
- 错误展开顺序固定为 `finally -> drop -> 匹配 catch / 继续传播`，清理错误不覆盖主错误。
- 公开字节码扩展名为 `.xiaoc`，一个文件对应一个模块；正式分段格式、内容寻址和归档仍属后续阶段。

## 阅读顺序

1. [前端流水线](../frontend/README.md)
2. [IR 快照](../ir/README.md)
3. 本页

内部设计细节、三种机型的结构对比和未决风险见
[09R. 字节码寄存器机型特别研究](../../../../DevDocs/09r-bytecode-machine-research.md)。
