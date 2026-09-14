---
id: language.collections.errors
title: 容器错误与诊断
status: verified
audience: learner
module: rust.xiao-types
stage: "03A"
version: "0.1.0"
related:
  - README.md
  - indexing.md
  - ../../troubleshooting/README.md
---

# 容器错误与诊断

C0 的容器错误在类型检查阶段产生稳定编号。具体文案会随 `[language]` 国际化设置变化，
编号和源码位置不变。

| 编号 | 含义 | 常见原因 |
| --- | --- | --- |
| `X03-TYPE-001` | 容器元素类型不匹配 | 显式 `int` 数组含字符串 |
| `X03-TYPE-002` | 字典键重复 | `name = 1` 后再次写 `"name" = 2` |
| `X03-TYPE-003` | 路径段种类错误 | 对数组使用键名路径 |
| `X03-TYPE-004` | 静态索引越界 | 固定数组读取不存在的位置 |
| `X03-TYPE-005` | 字典键不存在 | 读取未声明的键 |
| `X03-TYPE-006` | 路径格式错误 | 负索引或无法解析的数字 |
| `X03-TYPE-007` | 选择器形态未实现 | 范围、多选、步长或随机项 |

语法阶段还会用 `X03-PARSE-004` 拒绝 `const name[path]`。`const` 的编译期常量语义与
容器位置锁定不是同一件事，后者尚未进入 C0。

未知长度数组的边界无法在 C0 静态阶段证明；这类值会交给后续 Runtime 检查，
不应通过改写源码为范围或随机选择来绕过诊断。

更完整的运行时堆栈、日志和调试窗口属于第 07/11 阶段，当前页面只覆盖 C0 编译期诊断。
