---
id: language.modules.errors
title: 模块错误与诊断
status: verified
audience: learner
module: rust.xiao-modules
stage: "05"
version: "0.1.0"
related:
  - README.md
  - imports.md
  - project-layout.md
  - scope-and-exports.md
  - ../../../DevDocs/05-tables-and-projects.md
---

# 模块错误与诊断

模块分析保留统一诊断记录：`code` 和 `message_id` 是机器接口，参数单独保存，当前中文
文本只是预览。上层可以在不改变错误身份的情况下替换展示语言。

## 稳定编号

| 编号 | 含义 |
| --- | --- |
| `X05-MODULE-001` | 项目根、目录或源码读取失败 |
| `X05-MODULE-002` | 路径不能映射为合法模块名 |
| `X05-MODULE-003` | 文件/命名空间或大小写折叠冲突 |
| `X05-MODULE-004` | 导入目标或命名空间子模块不存在 |
| `X05-MODULE-005` | 文件模块没有被选择导入的符号 |
| `X05-MODULE-006` | 同一词法作用域中的导入名称冲突 |
| `X05-MODULE-007` | 文件模块依赖形成循环 |
| `X05-MODULE-008` | 限定符被单独读取或赋值 |

## 处理建议

发现错误时，分析器仍会尽可能保留可解析的模块和图信息；`is_success()` 只有在没有错误
级别诊断时才返回真。循环图的初始化顺序为空，调用方不得使用不完整结果继续生成 Runtime、
字节码或原生代码。缺失模块、缺失符号和限定符误用应先修正源码，再重新分析项目。

通过目录命名空间访问不存在的子模块（例如 `app.missing.value`）属于“导入目标不存在”，
使用 `X05-MODULE-004`；已经定位到文件模块后缺少导出名称，才使用 `X05-MODULE-005`。
冲突诊断下返回的部分模块记录仅用于错误恢复，不代表分析器选择了可用的模块。
