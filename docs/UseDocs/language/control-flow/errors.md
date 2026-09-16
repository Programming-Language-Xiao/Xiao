---
id: language.control-flow.errors
title: 控制流错误
status: verified
audience: learner
module: rust.xiao-types-control
stage: "04D"
version: "0.1.0"
related:
  - README.md
  - conditions-and-loops.md
  - returns-and-control.md
  - ../../troubleshooting/README.md
  - ../../../DevDocs/04-functions-and-control.md
  - ../../../DevDocs/07-concurrency-and-errors.md
---

# 控制流错误

控制流诊断的编号和 `message_id` 稳定，显示文本由语言设置决定。自动化工具应读取结构化字段，而不是匹配中文
文案。

| 编号 | 含义 | 处理方式 |
| --- | --- | --- |
| `X04-PARSE-003` | 缺少缩进代码块 | 在头部下一行提供至少一层缩进 |
| `X04-PARSE-004` | 控制流头部或控制语句尾部非法 | 删除尾随表达式并检查缩进 |
| `X04-TYPE-005` | 条件不是 `bool` | 显式转换或提供布尔表达式 |
| `X04-TYPE-006` | `for in` 右值不可迭代 | 使用数组、元组、集合、字典或动态容器 |
| `X04-TYPE-007` | 循环控制位于循环外 | 将语句移入最近循环 |
| `X04-TYPE-003` | 返回类型不一致 | 统一所有返回路径的类型 |

动态条件和动态可迭代对象不会被静态检查器直接执行，而会登记 Runtime 检查。错误控制流见[错误控制流](error-handling.md)。
