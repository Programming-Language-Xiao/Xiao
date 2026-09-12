# Xiao 使用文档

> 面向使用 Xiao 的自然人。这里说明如何安装、编写、运行、调试和发布程序；工程内部的 crate、IR 和实现约束请阅读 [开发文档](../DevDocs/README.md)。

## 阅读路线

1. 新用户从[开始使用](getting-started/README.md)进入，先完成[安装与平台准备](getting-started/installation/README.md)。
2. 再按[语言指南](language/README.md)学习基础语法、容器和模块。
3. 使用命令行、虚拟环境和 REPL 时阅读[工具](tooling/README.md)。
4. 遇到问题先查[故障排查](troubleshooting/README.md)，需要精确字段或命令时查[参考](reference/README.md)。

## 文档状态

当前目录是 A0 建立的用户文档骨架。页面状态使用 `planned`、`draft`、`verified`、`deprecated`；尚未实现的功能不会写成可用承诺。每个完成模块必须同时有代码、测试和 `verified` UseDocs 页面。

## 主题索引

- [开始使用](getting-started/README.md)
- [语言指南](language/README.md)
- [工具与交互](tooling/README.md)
- [任务指南](guides/README.md)
- [稳定参考](reference/README.md)
- [故障排查](troubleshooting/README.md)
- [页面模板](_templates/README.md)

## 反馈与版本

页面中的命令和示例必须注明适用的 Xiao/Runtime 版本。平台差异分别写在 Windows、Linux、macOS 小节；如果页面与实现不一致，应先修复规格和模块登记，再更新用户说明。

