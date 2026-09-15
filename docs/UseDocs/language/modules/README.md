# 模块与工程

本主题按“语法 → 项目发现 → 作用域/导出 → 错误”阅读，说明当前已验证的本地 `.xiao`
模块边界。包管理器、`config.xiao` 依赖解析和外部源会在后续阶段单独加入；配置文件的静态
声明格式见[配置文件](../../tooling/config/README.md)；本主题不要求
创建 Python 风格的 `__init__.py`。

## 前置知识

先完成[基础变量与表达式](../basics/README.md)，并了解[开始使用](../../getting-started/README.md)中的项目目录概念。

## 阅读顺序

1. [导入语法](imports.md)
2. [项目文件布局与命名空间](project-layout.md)
3. [作用域与顶层导出](scope-and-exports.md)
4. [模块错误与诊断](errors.md)

表是模块中可导出的顶层符号；表自身的字段、成员和生命周期静态契约见[表与生命周期](../tables/README.md)。

## 相关主题

环境和安装命令见[工具与交互](../../tooling/README.md)；设计和实现交接见
[05. 表、模块与工程模型](../../../DevDocs/05-tables-and-projects.md)。
