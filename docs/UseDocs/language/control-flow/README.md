# 控制流

本主题介绍 Xiao 已验证的分支、循环、函数和错误控制语句。页面的 `verified` 只表示对应 AST、类型或
Runtime 驱动器行为已通过测试；字节码 VM 和 LLVM 执行仍由后续阶段接入。

## 阅读顺序

1. [条件与循环](conditions-and-loops.md)
2. [返回与循环控制](returns-and-control.md)
3. [错误控制流](error-handling.md)
4. [控制流错误](errors.md)

函数声明和调用见[函数](../functions/README.md)，程序入口见[入口模式](../entry/README.md)。工程交接细节见
[开发文档 04](../../../DevDocs/04-functions-and-control.md)。

## 静态边界

条件只接受 `bool`，`for in` 只接受已知可迭代容器或动态值。`break`、`continue`、`return` 的合法位置和
返回类型会被静态检查；`try`、`catch`、`finally`、`raise` 的规则见[错误控制流](error-handling.md)，
其 Runtime 展开顺序见[Runtime 错误与展开](../memory/runtime/errors-and-unwind.md)。
