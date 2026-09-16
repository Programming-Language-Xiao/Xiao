---
id: language.control-flow.error-handling
title: 错误控制流
status: verified
audience: learner
module: rust.xiao-syntax-error-control
stage: "07B"
version: "0.1.0"
related:
  - README.md
  - errors.md
  - ../memory/runtime/errors-and-unwind.md
  - ../../troubleshooting/errors-and-reports.md
  - ../../../DevDocs/07-concurrency-and-errors.md
---

# 错误控制流

Xiao 首版错误控制流提供 `try`、`catch`、`finally` 和 `raise`。07-B 已完成语法树、静态边界、生命周期
退出计划和 Runtime 测试驱动器；这不代表字节码 VM 或 LLVM 已经能执行用户程序。

## 基本写法

```xiao
try
    value = divide(left, right)
catch err as ArithmeticError
    print(err)
catch fallback as Error
    print("other error")
finally
    close_resource()
```

`raise error` 主动抛出可恢复错误。`catch` 头部必须写成 `catch 绑定名 as 错误类型名`，可以有多个，按源码
顺序匹配第一个处理器。`finally` 最多一个，正常结束、`raise`、`return`、`break`、`continue` 和未匹配传播
都会经过它。

## 匹配边界

- `ArithmeticError`、`MemoryError`、`TableError`、`TypeError`、`ResourceError` 等具体类型应写在 `Error`
  或 `XiaoError` 之前。
- `FatalError` 表示不可恢复故障，不能由普通 `catch` 捕获。
- 没有匹配处理器时，错误保持原有 `code`、原因链、堆栈和上下文，继续向外传播。
- 错误文本会随语言设置改变；程序和测试应匹配稳定错误码和 `message_id`，不要匹配中文文案。

## 清理与作用域

展开顺序固定为 `finally -> drop -> 匹配 catch/继续传播`。清理或 `drop` 的次生错误追加到主错误的
`suppressed` 列表，不覆盖主错误，剩余资源仍继续释放。`try`、`catch` 和 `finally` 是独立静态作用域，处理器
不能使用已经释放的 `try` 局部绑定。

嵌套的 `try/finally` 会按内层到外层依次清理；函数定义处不会把函数体中的错误转移到定义位置的处理器。
循环中的 `break`/`continue` 也会保留原循环目标，经过所有应执行的 `finally` 后再跳转。

错误码匹配、条件/模式匹配、`Result<T, E>` 泛型和 `?` 简化传播属于后续阶段。
