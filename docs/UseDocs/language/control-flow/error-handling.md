---
id: language.control-flow.error-handling
title: 错误控制流
status: verified
audience: learner
module: rust.xiao-syntax-error-control
stage: "07B/20O-B1-FIX-B"
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
退出计划和 Runtime 测试驱动器；字节码 VM 与 LLVM 原生后端随后由 09/10 阶段交付，用户命令见[命令行参考](../../tooling/cli/README.md)。

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

## 读取稳定错误码

`error_code(value)` 是全局内建自由函数，无需导入，必须传入一个位置参数。
参数接受 `Error` 及其可恢复子类；返回拥有的不可变 `str`，逐字保留原始 code，
不依赖当前语言设置，不消费或重抛原错误。同一对象可以反复读取，结果可在 catch 结束后保存。
`error.code` 成员访问仍未开放，普通函数的 `Error` 类型注解也未因此开放。

以下完整示例对应 [CI 源码](../../../../tests/fixtures/20o-b1/fix-b/default-and-custom.xiao)，
固定黄金已经过 VM 与三平台受控原生 O0/O3 执行，实际运行号与日志见
[FIX-B 验收记录](../../../DevDocs/20o-b1-fix-b-closeout.md)。
适用版本：Xiao 0.1.0、Runtime ABI 1.18。

<!-- b1-error-code-executable-example -->
```xiao
# FIX-B 完整源码：默认/自定义 code 与同类不同对象。
def divide(int divisor) -> int
    return 10 // divisor

try
    print(divide(0))
catch arithmetic as ArithmeticError
    print(error_code(arithmetic))
default_error = Error()
print(error_code(default_error))
first = Error(code = "MiXeD_雪😀")
second = Error(code = "same-class-other-code")
alias = first
print(error_code(first))
print(error_code(alias))
print(error_code(second))
print(error_code(Error(code = "temporary")))
```

完整输出：

```text
X06-RUNTIME-013
X07-RUNTIME-ERROR
MiXeD_雪😀
MiXeD_雪😀
same-class-other-code
temporary
```

静态可知的非错误参数、零个或多个参数、命名/展开参数在编译期以 `X04-TYPE-002` 拒绝。
动态非错误值产生 `TypeError / X06-RUNTIME-002`；结果字符串分配失败产生
`ResourceError / X06-RUNTIME-011`，原错误保持有效。`FatalError` 仍不可由普通 catch 捕获。

## 清理与作用域

展开顺序固定为 `finally -> drop -> 匹配 catch/继续传播`。清理或 `drop` 的次生错误追加到主错误的
`suppressed` 列表，不覆盖主错误，剩余资源仍继续释放。`try`、`catch` 和 `finally` 是独立静态作用域，处理器
不能使用已经释放的 `try` 局部绑定。

嵌套的 `try/finally` 会按内层到外层依次清理；函数定义处不会把函数体中的错误转移到定义位置的处理器。
循环中的 `break`/`continue` 也会保留原循环目标，经过所有应执行的 `finally` 后再跳转。

按错误码的专用条件/模式匹配、`Result<T, E>` 泛型和 `?` 简化传播属于后续阶段。
