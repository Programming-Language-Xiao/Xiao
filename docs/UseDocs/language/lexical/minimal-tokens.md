---
id: language.lexical.minimal-tokens
title: 最小 Token 流
status: verified
audience: learner
module: rust.xiao-syntax
stage: "01"
version: "0.1.0"
related:
  - README.md
  - source-positions.md
  - ../../troubleshooting/diagnostics-structure.md
---

# 最小 Token 流

[返回词法主题索引](README.md)

01 阶段的 L0 词法器只负责把源码切成稳定的最小 Token，不执行赋值，也不检查变量类型。
L1 在此基础上增加字面量和运算符；本页只保留 L0 的最小闭环，便于核对兼容性。

## 当前识别内容

- ASCII 标识符：字母或下划线开头，后接字母、数字或下划线。
- 十进制整数：连续数字，允许前导零。
- `=`：单独的赋值 Token。
- `LF` 或 `CRLF`：一个逻辑换行 Token。
- `EOF`：文件结束的零宽 Token。
- 其他字符：产生 `Invalid` Token 和 `X01-LEX-001` 诊断，然后继续扫描。

例如 `a = 1` 会依次得到标识符、赋值、整数和 EOF；源码没有尾部换行时不会补换行 Token。

## 尚未支持

字符串、浮点、`true`/`false`/`none`、关键字、括号和表达式运算符已在 L1 词法层开放，
但仍不代表它们可以在当前版本中执行。反引号名称、注释和缩进 Token 的使用方式见[后续词法页面](README.md)。

完整错误编号和恢复建议见[结构化诊断与词法错误](../../troubleshooting/diagnostics-structure.md)。

下一步阅读[基础字面量与运算符 Token](basic-tokens.md)，再回到[基础变量与表达式](../basics/README.md)。
