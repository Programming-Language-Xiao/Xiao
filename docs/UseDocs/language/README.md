# 语言指南

按由浅入深的顺序学习 Xiao 的源代码写法。每个主题只记录已经冻结且经过规格测试的行为。

## 推荐顺序

1. [源码位置与最小 Token](lexical/README.md)
2. [基础变量与表达式](basics/README.md)
3. [容器与集合](collections/README.md)
4. [函数](functions/README.md)
5. [控制流](control-flow/README.md)
6. [程序入口](entry/README.md)
7. [表与生命周期](tables/README.md)
8. [模块与工程](modules/README.md)
9. [内存与静态生命周期](memory/README.md)

## 相关主题

运行代码前可先阅读[开始使用](../getting-started/README.md)；语言错误的恢复方法见[故障排查](../troubleshooting/README.md)。

函数、控制流和入口页面的 `verified` 只覆盖 04 阶段静态解析与类型检查；内存页面的
`verified` 只覆盖 06-A 静态生命周期计划。当前版本仍不表示已经执行 Xiao 程序。
