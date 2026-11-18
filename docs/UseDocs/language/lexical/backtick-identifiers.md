---
id: language.lexical.backtick-identifiers
title: 反引号名称
status: verified
audience: learner
module: rust.xiao-syntax
stage: "01"
version: "0.1.0"
related:
  - README.md
  - comments-and-indentation.md
  - ../../troubleshooting/diagnostics-structure.md
---

# 反引号名称

[返回词法主题索引](README.md) · [下一页：注释与缩进](comments-and-indentation.md)

普通变量名使用 ASCII 字母、数字和下划线。需要使用 Unicode、空格或关键字作为名称时，可以用反引号包裹：

```xiao
`新 变量` = 1
`def` = "这是一个名称"
print(`新 变量`)
```

反引号名称会被前端作为一个完整名称读取，首尾反引号属于名称的源码范围。名称内容可以是合法 UTF-8 字符，但不能跨物理换行。

## 转义

反斜杠只用于转义反引号和反斜杠：

```xiao
`包含\`反引号` = 1
`包含\\反斜杠` = 2
```

其他形式的转义会报告 `X01-LEX-008`。未闭合名称会报告 `X01-LEX-005`；如果错误前遇到换行，换行仍会被单独识别，便于继续查看后续问题。

反引号只改变名称可使用的字符范围，不改变变量的静态类型规则。定义和引用时应保持相同的反引号写法。

## 错误排查

看到 `X01-LEX-005` 时，检查名称是否缺少结尾反引号，或是否意外换行。看到 `X01-LEX-008` 时，把不支持的转义改为反引号或反斜杠转义。错误编号是稳定机器字段，界面说明会随语言设置变化。

下一步阅读[注释与缩进](comments-and-indentation.md)，了解名称之外的行结构。
