---
id: language.control-flow.returns-and-control
title: 返回与循环控制
status: verified
audience: learner
module: rust.xiao-types-control
stage: "04C"
version: "0.1.0"
related:
  - README.md
  - conditions-and-loops.md
  - errors.md
  - ../functions/definitions.md
  - ../../../DevDocs/04-functions-and-control.md
---

# 返回与循环控制

`return` 只能出现在函数体中，可以带表达式，也可以省略表达式：

```xiao
def choose(bool flag) -> int
    if flag
        return 1
    return 0

def log_value(value)
    return
```

同一个函数的所有返回路径必须统一类型；省略返回值的函数在静态收尾时返回 `none`。无注解函数会结合函数体、
调用点和返回语句推断类型，无法确定时报告 `X04-TYPE-004`。

`break` 和 `continue` 只能控制最近的 `for` 或 `while`：

```xiao
for item in values
    if item
        continue
    break
```

函数边界会重置循环深度，因此嵌套函数中的循环控制不能跳出到外层函数。函数外的 `return`，以及循环外的
`break`/`continue`，分别产生稳定的返回或循环控制诊断。运行时的实际跳转和资源清理尚未开放。
