---
id: language.functions.errors
title: 函数错误
status: verified
audience: learner
module: rust.xiao-types-functions
stage: "04D"
version: "0.1.0"
related:
  - README.md
  - definitions.md
  - calls.md
  - ../../troubleshooting/README.md
  - ../../../DevDocs/04-functions-and-control.md
---

# 函数错误

函数错误使用稳定编号和 `message_id`。中文文案只是当前显示语言；排查和自动化测试应使用编号、结构化参数和
源码位置。当前页面只覆盖静态解析/类型错误，不代表 Runtime 已经运行。

| 编号 | 含义 |
| --- | --- |
| `X04-PARSE-001` | 函数或控制流头部结构非法 |
| `X04-PARSE-002` | 参数列表结构非法，例如缺少逗号或重复参数 |
| `X04-PARSE-003` | 缺少换行或缩进代码块 |
| `X04-PARSE-005` | 返回类型注解无法解析 |
| `X04-TYPE-001` | 函数声明冲突或重复定义 |
| `X04-TYPE-002` | 参数数量、名称、种类或类型不匹配 |
| `X04-TYPE-003` | 返回值与函数返回类型不一致 |
| `X04-TYPE-004` | 参数或返回类型无法推断 |

错误结果仍会尽量继续检查后续语句。动态参数、动态返回值或动态展开只会留下供 Runtime 消费的检查计划，不能
被当作静态成功的证明。完整的控制流错误见[控制流错误](../control-flow/errors.md)。
