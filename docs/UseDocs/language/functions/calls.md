---
id: language.functions.calls
title: 调用函数
status: verified
audience: learner
module: rust.xiao-types-functions
stage: "04B"
version: "0.1.0"
related:
  - README.md
  - definitions.md
  - errors.md
  - ../../../DevDocs/04-functions-and-control.md
---

# 调用函数

调用可以混合位置参数和关键字参数：

```xiao
def add(int left, int right = 1) -> int
    return left + right

answer = add(2, right = 3)
```

类型检查器会先登记同一程序中的函数签名，所以递归调用和出现在定义之前的调用也能参与推断。参数类型可以
来自注解、函数体、返回语句和后续调用；所有约束收集完后仍无法确定的类型会报错，而不会静默变成任意类型。

## 传参规则

- 位置参数不能跟在关键字参数后面；
- 位置专用参数不能使用关键字；
- 一个普通参数不能重复传入；
- 未知关键字只有在函数声明 `**kwargs` 时才允许；
- `*values` 只能展开已知可迭代容器或动态值；
- `**options` 只能展开字典表、字典列或动态值；
- 缺少默认值的普通参数必须提供。

普通位置实参传给 `*args` 时，静态检查比较的是可变参数的元素类型，而不是数组外壳类型。参数展开当前只
记录为 AST 和类型计划，不会真的读取容器或创建调用栈。

## 反引号名称

定义和调用必须使用相同的名称形式：

```xiao
def `处理`(int value) -> int
    return value

answer = `处理`(1)
```

类型层使用规范化键匹配名称，同时保留源码区间和原始拼写供诊断使用。
