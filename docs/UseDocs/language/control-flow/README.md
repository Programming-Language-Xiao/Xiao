# 控制流

本主题介绍 Xiao 04 阶段已经验证的分支、循环和函数控制语句。页面的 `verified` 只表示静态 AST/类型行为已
通过测试；当前阶段不执行循环，也不插入资源释放。

## 阅读顺序

1. [条件与循环](conditions-and-loops.md)
2. [返回与循环控制](returns-and-control.md)
3. [控制流错误](errors.md)

函数声明和调用见[函数](../functions/README.md)，程序入口见[入口模式](../entry/README.md)。工程交接细节见
[开发文档 04](../../../DevDocs/04-functions-and-control.md)。

## 静态边界

条件只接受 `bool`，`for in` 只接受已知可迭代容器或动态值。`break`、`continue`、`return` 的合法位置和
返回类型会被静态检查；运行时执行顺序、异常、`finally` 和生命周期释放留给后续阶段。
