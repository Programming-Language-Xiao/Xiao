---
id: troubleshooting.diagnostics-structure
title: 结构化诊断与词法错误
status: verified
audience: contributor
module: rust.xiao-diagnostics
stage: "07"
version: "0.1.0"
related:
  - README.md
  - ../language/lexical/minimal-tokens.md
---

# 结构化诊断与词法错误

[返回故障排查索引](README.md)

Xiao 的前端诊断包含稳定的机器编号、可翻译消息键、严重级别、源码区间和当前展示文本。
程序或测试应匹配 `code` 与 `message_id`，不要匹配本地化后的句子。

## L0 常见诊断

| 编号 | 含义 | 处理方式 |
| --- | --- | --- |
| `X01-SOURCE-001` | 源文件不是合法 UTF-8 | 修正文件编码后重新运行 |
| `X01-LEX-001` | 发现无法识别的字符 | 查看标注位置；词法器会继续扫描后续内容 |
| `X01-LEX-002` | 字符串没有闭合 | 在同一行补上匹配引号 |
| `X01-LEX-003` | 字符串转义不受支持 | 改用 L1 支持的转义序列 |
| `X01-LEX-004` | 数字指数缺少数字 | 在 `e`/`E` 后补十进制数字 |

词法错误会保留 `Invalid` Token，因此工具可以一次列出多个问题；这不代表源码已经可以
执行。L1 的字符串和数字错误详见[基础字面量与运算符](../language/lexical/basic-tokens.md)。
第 07 阶段会在同一身份字段上增加原因链、堆栈和运行时错误对象。

遇到位置异常时，先确认文件编码和换行格式，再核对[源码位置](../language/lexical/source-positions.md)。
