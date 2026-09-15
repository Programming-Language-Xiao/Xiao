---
id: language.tables.errors
title: 表错误与诊断
status: verified
audience: learner
module: rust.xiao-types-tables
stage: "05C"
version: "0.1.0"
related:
  - README.md
  - members-and-visibility.md
  - construction-and-lifecycle.md
  - ../../troubleshooting/diagnostics-structure.md
  - ../../../DevDocs/05c-table-static-closure.md
---

# 表错误与诊断

表错误包含稳定编号、`message_id`、源码位置和结构化参数。自动化工具应使用编号和参数，
不要依赖某一种显示语言的文本。

| 编号 | 含义 |
| --- | --- |
| `X05-PARSE-005` | 表头名称、位置或尾部非法 |
| `X05-PARSE-006` | 表体包含不允许的语句或额外缩进 |
| `X05-PARSE-007` | 表体缺少换行、缩进或成员 |
| `X05-TYPE-001` | 表名重复或与当前名称冲突 |
| `X05-TYPE-002` | 成员不存在、重复或字段类型冲突 |
| `X05-TYPE-003` | 表外访问私有成员 |
| `X05-TYPE-004` | `new` 目标、参数数量或参数类型错误 |
| `X05-TYPE-005` | `self`、`init` 或 `drop` 签名错误 |
| `X05-TYPE-006` | 字段初始化器包含动态/有副作用表达式 |

解析或检查出现错误时，工具会尽可能继续收集后续诊断；只要存在错误，就不能把结果交给
Runtime 或后端生成可执行产物。
