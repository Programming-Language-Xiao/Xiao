---
id: language.basics.types.errors
title: 类型错误与运行时检查
status: verified
audience: learner
module: rust.xiao-types
stage: "02A"
version: "0.1.0"
related:
  - README.md
  - ../../../troubleshooting/diagnostics-structure.md
---

# 类型错误与运行时检查

类型检查会累积诊断并保留源码位置；程序在进入 VM 或 LLVM 后端前不会执行。常见编号：

| 编号 | 含义 |
| --- | --- |
| `X02-TYPE-001` | 名称未定义 |
| `X02-TYPE-002` | 当前作用域重复声明 |
| `X02-TYPE-003` | 读取未初始化名称 |
| `X02-TYPE-004` | 赋值或修改常量的类型不兼容 |
| `X02-TYPE-005` | 运算操作数不合法 |
| `X02-TYPE-006` | 转换不在矩阵内 |
| `X02-TYPE-007` | 溢出、除零或非有限算术 |
| `X02-TYPE-008` | `const` 初始化不是编译期常量 |
| `X02-TYPE-009` | HM 类型统一或 occurs-check 失败 |

当源值是动态边界、但静态上无法证明会失败时，检查结果会携带运行时检查标记。标记
不是“忽略错误”，而是要求后续 Runtime 在对应源码位置执行范围、内容或类型验证。

[上一页：编译期常量](constants.md) · [诊断结构详解](../../../troubleshooting/diagnostics-structure.md) · [返回类型主题](README.md)
