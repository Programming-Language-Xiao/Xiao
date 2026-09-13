---
id: language.basics.p1-expressions.calls-and-casts
title: P1 调用、构造与转换
status: verified
audience: learner
module: rust.xiao-syntax
stage: "01E"
version: "0.1.0"
related:
  - README.md
  - operators.md
  - selectors.md
---

# P1 调用、构造与转换

[返回 P1 主题索引](README.md) · [上一页：运算符与优先级](operators.md) · [下一页：索引选择器](selectors.md)

## 普通调用

调用参数按书写顺序保存。允许零参数和尾逗号，参数本身可以是完整表达式：

```xiao
empty = make()
value = factory(a + b,)
```

点号成员访问可以与调用和其他后缀串联，例如 `factory().item`。P1 只记录
调用结构，不执行函数，也不检查参数数量。

## `new` 构造调用

`new Type(args...)` 形成独立的构造调用节点，括号是必需的；模块或命名空间可以
用点号组成限定构造目标：

```xiao
client = new Client(host, port)
remote = new net.Client(host, port)
```

构造目标可以是普通名称、反引号名称或后续允许的名称表达式。构造器的类型、
生命周期和参数检查由后续阶段负责。

## `as` 标量转换

当前语法阶段只接受八种标量目标：`int`、`sint`、`lint`、`float`、`sfloat`、
`lfloat`、`str` 和 `bool`。

```xiao
flag = raw as bool
text = flag as str
```

容器转换采用普通调用形式，例如 `tuple(value)`、`list(value)` 或 `set(value)`；
P1 不把容器名称解析成特殊的类型节点。`as tuple` 等非标量目标会报告
`X01-PARSE-008`。转换是否成功、字符串与布尔值的具体矩阵，以及是否产生新值，
由类型和值阶段实现。

## 复合赋值

语句层支持 `+=`、`-=`、`*=`、`/=`、`//=`、`%=` 和 `**=`。选择器或成员也可
出现在左侧，形成扩展赋值节点：

```xiao
total += delta
items[0] *= factor
record.value = next_value
```

多项选择写入的广播、长度匹配、边界错误和失败回滚尚未在 P1 定义。
