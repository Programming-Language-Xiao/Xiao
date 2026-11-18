---
id: language.lexical.source-positions
title: 源码位置
status: verified
audience: learner
module: rust.xiao-source
stage: "01"
version: "0.1.0"
related:
  - README.md
  - minimal-tokens.md
  - ../../troubleshooting/diagnostics-structure.md
---

# 源码位置

[返回词法主题索引](README.md)

Xiao 前端先把源文件按 UTF-8 校验，再为 Token 和诊断建立统一位置。每个位置同时有原始
字节偏移、行号和列号，因此错误提示可以准确回到源文件。

## 坐标规则

- 行号和列号从 1 开始。
- 字节偏移从 0 开始，指向原始 UTF-8 输入。
- 列号按 Unicode 标量计数；一个中文字符占一列。
- `LF` 和 `CRLF` 都表示一个逻辑换行；CRLF 的原始区间仍覆盖两个字节。
- 文件末尾没有换行时，不会凭空增加换行位置。

## UTF-8 与 Tab

非法 UTF-8 在词法扫描前就会被拒绝，不会用替换字符继续编译。缩进处理阶段会把每个 Tab
统一视为四个空格；位置区间仍保留原始 Tab 字节，方便编辑器和诊断回写。

## 给用户的影响

错误消息中的行列是阅读位置，工具内部的快照和协议使用字节区间。两者属于同一源码，
不会因为 Windows 的 CRLF 或 Linux/macOS 的 LF 而产生不同语义。

下一步请阅读[最小 Token 流](minimal-tokens.md)。
