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

Xiao 的前端诊断包含稳定的机器编号、可翻译消息键、结构化插值参数、严重级别、源码区间和当前展示文本。
程序或测试应匹配 `code`、`message_id` 和参数，不要匹配本地化后的句子。参数保存
原始类型名、数量等机器值，语言层在展示边界再把它们格式化。

## 诊断字段

| 字段 | 作用 |
| --- | --- |
| `code` | 跨版本和语言稳定的机器编号 |
| `message_id` | 消息目录键；用于查找翻译模板 |
| `params` | 按名称保存的 `Text`、`Integer` 或 `Boolean` 原始值 |
| `severity` | `Error`、`Warning` 或 `Info` |
| `span` | 可选源码区间 |
| `message` | 当前语言的预览文本，不作为判断接口 |

例如集合元素类型错误会同时携带 `actual_type` 与 `expected_type`，切换语言时这两个
参数不变。结构化日志也应保留这些字段；完整的 `XiaoError` 原因链和堆栈属于第 07 阶段。

## L0 常见诊断

| 编号 | 含义 | 处理方式 |
| --- | --- | --- |
| `X01-SOURCE-001` | 源文件不是合法 UTF-8 | 修正文件编码后重新运行 |
| `X01-LEX-001` | 发现无法识别的字符 | 查看标注位置；词法器会继续扫描后续内容 |
| `X01-LEX-002` | 字符串没有闭合 | 在同一行补上匹配引号 |
| `X01-LEX-003` | 字符串转义不受支持 | 改用 L1 支持的转义序列 |
| `X01-LEX-004` | 数字指数缺少数字 | 在 `e`/`E` 后补十进制数字 |
| `X01-LEX-005` | 反引号名称没有闭合 | 补上结尾反引号，且不要跨行 |
| `X01-LEX-006` | 文档注释没有闭合 | 补上下一个 `###` 标记 |
| `X01-LEX-007` | 缩进无法对应已知层级 | 使用四空格倍数并回到已有层级 |
| `X01-LEX-008` | 反引号转义不受支持 | 只转义反引号或反斜杠 |
| `X01-LEX-009` | 关闭分隔符不匹配 | 检查 `()`、`[]`、`{}` 的嵌套顺序 |
| `X01-LEX-010` | 分隔符没有闭合 | 补上对应的右侧分隔符 |

词法错误会保留 `Invalid` Token，因此工具可以一次列出多个问题；这不代表源码已经可以
执行。L1 的字符串和数字错误详见[基础字面量与运算符](../language/lexical/basic-tokens.md)。
第 07 阶段会在同一身份字段上增加原因链、堆栈和运行时错误对象。

遇到位置异常时，先确认文件编码和换行格式，再核对[源码位置](../language/lexical/source-positions.md)。
