---
id: language.lexical.comments-and-indentation
title: 注释与缩进
status: verified
audience: learner
module: rust.xiao-syntax
stage: "01"
version: "0.1.0"
related:
  - README.md
  - backtick-identifiers.md
  - ../../troubleshooting/diagnostics-structure.md
---

# 注释与缩进

[返回词法主题索引](README.md) · [上一页：反引号名称](backtick-identifiers.md)

Xiao 用换行和缩进表达代码块，不使用代码块花括号。L2 词法器会保留物理换行，并在代码层级变化处产生 `INDENT` 或 `DEDENT`。

## 注释

### 普通注释

`#` 到当前物理行末尾是普通注释。普通注释不会变成 Token，但这一行的 `LF` 或 `CRLF` 仍然保留：

```xiao
a = 1 # 这段文字不会作为程序值
b = 2
```

### 文档注释

`###` 到下一个 `###` 是一个文档注释。单行和跨行形式都可以：

```xiao
### 这是单行文档注释 ###

###
这是跨行文档注释，可写 @param 等说明。
###
```

整个文档注释作为一个 `DocComment` Token 保存；注释内部的换行属于该 Token，不额外拆成内部换行 Token。未闭合时报告 `X01-LEX-006`。

## 缩进

- 一个 Tab 按四个逻辑空格计算。
- 一级缩进是四个空格，代码层级使用四空格倍数。
- 新层级产生 `INDENT`，返回已有层级产生一个或多个 `DEDENT`。
- 空行、普通注释行和文档注释行不会改变缩进层级。
- 括号、方括号和花括号内部仍保留物理换行，但行首空白不会生成缩进 Token。
- 文件结束时会先补齐剩余 `DEDENT`，再结束 Token 流。

例如：

```xiao
start()
    first()
    second()
finish()
```

如果行首宽度不是四的倍数，或不能回到已经出现的层级，会报告 `X01-LEX-007`，并向下归入最近的已知层级继续扫描。缩进错误不会把后续合法代码吞掉。

下一步回到[基础变量与表达式](../basics/README.md)。遇到错误位置或错误码时，请查看[结构化诊断](../../troubleshooting/diagnostics-structure.md)。
