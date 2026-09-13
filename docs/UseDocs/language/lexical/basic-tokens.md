---
id: language.lexical.basic-tokens
title: 基础字面量与运算符 Token
status: verified
audience: learner
module: rust.xiao-syntax
stage: "01"
version: "0.1.0"
related:
  - README.md
  - minimal-tokens.md
  - ../../troubleshooting/diagnostics-structure.md
---

# 基础字面量与运算符 Token

[返回词法主题索引](README.md) · [上一页：最小 Token 流](minimal-tokens.md)

本页说明 Xiao 0.1 已验证的 L1 词法形状。它描述源码如何被前端识别，不代表当前版本已经能执行赋值、函数或容器程序。

## 可以识别的内容

### 字面量

```xiao
true false none
12 001 12.50 .5 1e3
"双引号字符串" '单引号字符串'
```

`true`、`false`、`none`、整数、浮点和字符串各自产生不同的 Token。数字的位宽、溢出和最终 `str`/`bool` 类型由后续类型阶段检查；这里不会因为字面量很大而静默截断。

### 保留字

控制流、函数、导入、转换和类型名称按保留字识别，例如 `def`、`if`、`return`、`import`、`as`、`int`、`str` 和 `bool`。保留字区分大小写；需要把关键字作为变量名时，可以使用 L2 的反引号名称。

### 运算符和分隔符

支持基础 Token：`+`、`-`、`*`、`/`、`//`、`%`、`**`、`+=`、`-=`、`*=`、`/=`、`//=`、`%=`、`**=`、`==`、`!=`、`<`、`<=`、`>`、`>=`，以及 `()`、`[]`、`{}`、`,`、`:`、`.`、`~`、`?`、`!`、`!?`、`@` 和 `$`。

`!?` 是一个整体标记，供后续选择器表示“放回抽取”；`/` 暂时只是一个 Token，既可能是除法也可能是数组路径分隔符，最终解释交给解析器和类型层。集合并集使用 `+` 的规则也不会在词法阶段改变 `+` 的身份。

## 字符串转义与错误

L1 接受常用的换行、制表和引号转义，例如 `"a\\n b"`。字符串必须在同一行闭合。未闭合字符串报告 `X01-LEX-002`，不支持的转义报告 `X01-LEX-003`；错误之后前端会继续扫描后面的名称或换行，方便一次显示多个问题。

数字指数必须包含数字，例如 `1e+2` 合法，而 `1e+` 报告 `X01-LEX-004`。错误码是机器接口，界面上的说明文字可能随 `[language]` 设置变化。

## 当前边界

反引号 UTF-8 名称、注释和缩进已经在 L2 词法层验证；AST 和可执行语法仍在后续里程碑。请阅读[反引号名称](backtick-identifiers.md)和[注释与缩进](comments-and-indentation.md)了解新增 Token，再查看[最小 Token 流](minimal-tokens.md)核对 EOF 与 CRLF 细节；遇到错误位置问题，请查看[结构化诊断](../../troubleshooting/diagnostics-structure.md)。

下一步阅读[基础变量与表达式](../basics/README.md)。
