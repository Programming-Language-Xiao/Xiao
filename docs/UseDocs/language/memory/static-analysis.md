---
id: language.memory.static-analysis
title: 静态生命周期分析
status: verified
audience: learner
module: rust.xiao-lifetime
stage: "06A"
version: "0.1.0"
related:
  - README.md
  - release-and-errors.md
  - ../control-flow/README.md
  - ../tables/construction-and-lifecycle.md
  - ../../../DevDocs/06a-lifetime-static-closure.md
---

# 静态生命周期分析

每个程序、函数、条件分支、循环体和表体都有独立作用域。普通数值、`bool`、`none`，以及
只含这些值的小型元组可以直接留在栈或寄存器；`str`、数组、字典、集合、函数、表实例和
动态值进入堆生命周期模型。

以下情况会让值自动延长生命周期：

- 从函数 `return` 返回；
- 被嵌套 `def` 读取并形成闭包捕获；
- 从内层作用域存入外层绑定或容器；
- 类型或运行时检查无法在编译期完全确定。

```xiao
def make() -> str
    value = "xiao"
    return value
```

这里的 `value` 会标记为返回逃逸，并从函数的 `return` 清理动作中移除。调用方接管它，
函数不会先释放再返回。

```xiao
def outer() -> str
    value = "xiao"
    def inner() -> str
        return value
    return value
```

`inner` 读取外层局部名称时会形成闭包捕获强边。06-A 只验证该静态关系；当前版本尚不执行
闭包或引用计数。

动态值不会被假定为安全栈值。分析器会保守使用堆强拥有，并生成稳定的 Runtime 检查记录。
这不会把 `bool` 当成数值，也不会改变类型检查阶段已经冻结的类型规则。
