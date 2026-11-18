---
id: language.functions.definitions
title: 定义函数
status: verified
audience: learner
module: rust.xiao-syntax-functions
stage: "04A"
version: "0.1.0"
related:
  - README.md
  - calls.md
  - errors.md
  - ../../../DevDocs/04-functions-and-control.md
---

# 定义函数

函数使用 `def` 开头，函数头不写冒号，函数体使用缩进：

```xiao
def greet(str name = "world") -> str
    return name
```

解析阶段已经验证函数名、参数、返回类型和缩进体的结构；示例不会在当前静态阶段执行。

## 参数形式

普通参数可以位于位置或关键字位置。`/` 之前的参数只能按位置传入，裸 `*` 之后的参数只能按关键字传入：

```xiao
def sample(int left, int right = 0, /, label = "ok", *, bool verbose = false)
    return
```

还可以声明 `*items` 和 `**options`：

```xiao
def collect(*items, **options)
    return
```

参数之间必须使用逗号。重复名称、错误的默认值顺序、重复的可变参数和 `**options` 后继续声明参数会产生
参数语法诊断。反引号名称可以用于函数和参数，例如 ``def `处理`(int value) -> int``；引用时也必须保留
反引号形式。

## 返回值

`return expression` 返回一个值；只写 `return` 表示空值。没有显式返回值的函数在静态收尾时统一为 `none`，
同一个函数的不同返回路径必须统一类型。返回类型注解可以写标量或 `none`。

## 作用域边界

函数参数拥有独立的局部作用域，可以遮蔽外层普通名称。静态检查完成后会退出该作用域，再更新外层函数签名；
因此参数与函数同名不会破坏函数调用的解析。闭包捕获和资源释放属于后续 Runtime/生命周期阶段。
