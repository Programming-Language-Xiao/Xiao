# 函数

本主题介绍 Xiao 04 阶段已经验证的函数语法和静态检查边界。这里的 `verified` 只表示解析器和类型检查器
已经验证；函数执行见[命令行参考](../../tooling/cli/README.md)。

## 阅读顺序

1. [定义函数](definitions.md)
2. [调用函数](calls.md)
3. [函数错误](errors.md)

函数与控制流的整体交接范围见[开发文档 04](../../../DevDocs/04-functions-and-control.md)。条件和循环见
[控制流](../control-flow/README.md)，程序入口见[入口模式](../entry/README.md)。

## 当前边界

已验证的是 `def`、缩进体、参数形状、调用参数 AST、签名预登记和静态类型约束。调用栈与字节码/LLVM 生成
已由 09/10 阶段交付；闭包、参数展开执行和 Runtime 错误仍属后续阶段。
