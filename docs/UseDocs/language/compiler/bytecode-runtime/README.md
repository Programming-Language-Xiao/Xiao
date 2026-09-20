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

状态：研究路径 `verified`，生产路径 `planned`。09R2 研究 VM 已能执行前端生成并经验证的
IR/TAC 研究产物；它仍不是面向用户的 `xiao run`，也不会直接把任意源码当作正式字节码运行。

Xiao 的双模式执行从类型化 IR 分叉：`xiao run` 走字节码解释路径，`xiao build` 走 LLVM 原生
路径。两条路径必须共享同一套类型、溢出、求值顺序、动态检查、随机选择、容器顺序、引用计数、
`drop` 和错误诊断语义，字节码路径不得自行决定其中任何一项。

## 当前进展

09R2 已完成异常控制流、三种研究载体、内存编码、选择器、集合、迭代和表声明的研究闭环：

- 栈式、分类型寄存器式、混合式三种载体共用同一 TAC 语义核、释放计划和 79 条共享向量；
- `SelectorApply`、`BroadcastAssign`、`RandomSeed` 已接通多选、范围、步长、随机、结果形状
  和事务性广播；
- `SetOp`、`SetCompare` 已接通四种集合代数、六种关系比较和成员判断；集合操作数检查使用
  `X06-RUNTIME-021`/`022`，成员检查使用 `X06-RUNTIME-023`，可哈希检查复用
  `X06-RUNTIME-016`；
- `Len`、`IndexGetDynamic` 已接通 `for` 的 CFG 降低；数组、元组、字符串、集合、字典表和
  字典列可迭代，字符串按 Unicode 码点计数，动态右值失败使用 `X06-RUNTIME-024`；
- 表声明已接通单例/实例构造、字段读写、静态方法、初始化失败回滚和确定性析构；
  析构接收者只读，别名与容器持有遵守最后强引用释放规则；
- 编码器支持布局版本 3、41 个稳定 opcode、LEB128/定宽 `u16` 两种操作数宽度、版本拒绝和
  `pc -> IrSpan` 只读映射；
- 动态选择器边界、步长、随机数量和随机种子分别使用 `X06-RUNTIME-017..020`，错误沿统一
  handler/finally/释放路径传播。

方法值的动态派发、正式 `.xiaoc` 分段格式、生产 `xiao run` 和 09R3 正式性能门槛仍未开放。研究代码保持在
`xiao-bytecode/src/research` 与 `xiao-vm/src/research`，
不构成稳定语言接口。

## 已经确定的部分

- 字节码后端只消费已验证的类型化 IR，不重新解析源码、不重新推断类型、不重新计算释放顺序。
- 执行核心与执行时 Runtime 使用 Rust；TypeScript 只承担 CLI / REPL 的终端边界，不复制
  算术、索引、类型转换、生命周期或错误传播逻辑。
- 引用计数是确定性的，没有追踪式垃圾回收；作用域退出、提前返回和错误退出都执行确定性释放。
- 错误展开顺序固定为 `finally -> drop -> 匹配 catch / 继续传播`，清理错误不覆盖主错误。
- 研究编码只在内存 `Vec<u8>` 中流转，不使用 `.xiaoc` 扩展名；正式分段格式、内容寻址和归档仍属后续阶段。

## 阅读顺序

1. [前端流水线](../frontend/README.md)
2. [IR 快照](../ir/README.md)
3. 本页

内部设计细节、三种机型的结构对比和未决风险见
[09R. 字节码寄存器机型特别研究](../../../../DevDocs/09r-bytecode-machine-research.md)。
