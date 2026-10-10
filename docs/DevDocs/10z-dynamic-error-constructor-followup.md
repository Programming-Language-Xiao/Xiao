# 10Z-D. 动态错误构造参数后续立项

> **来源**：[10Z-Z1](10z-z1-scope-and-gap-verdict.md) 的 D 裁定。`ArithmeticError(code = code)`
> 是前端接受、VM 可执行的合法程序；原生后端目前只接受字符串字面量，构建阶段拒绝动态参数。
>
> 状态：**已立项，转后续批次；不纳入 Z-1 实现**。

## 一、已确认缺口

最小源码：

```xiao
code = "DYNAMIC"
raise ArithmeticError(code = code)
```

VM 运行到用户自己的 `raise`，报告错误码 `DYNAMIC`、退出码 3。原生构建返回
`X11-PROTOCOL-007`，消息为“动态错误构造参数（当前只支持字符串字面量）”，构建退出码 2。
这不是两边都拒绝的错误程序，而是语言允许的动态错误参数形态缺少原生产物。

## 二、范围与边界

本立项只覆盖可恢复错误构造器的动态 `code`/`message` 参数；静态字符串字面量、`FatalError` 边界、
未知错误类型和错误控制流保持现有契约。实现必须同时贯通类型化 IR、LLVM Runtime ABI、原生错误槽和
诊断报告，不能只删除 `dynamic/expression.rs` 的拒绝分支。

## 三、实现要求

1. 前端与 VM 的错误身份、`code` 优先级、缺省消息和 `raise` 位置保持不变。
2. LLVM 路径为动态文本参数生成稳定的 `xiao_runtime_error_new_values` 调用，保留源码位置，
   并在错误对象创建失败时走现有可恢复错误路径。
3. Runtime ABI 对动态文本只接受约定的文本值形态；非文本值继续返回稳定类型错误，不能静默转换。
4. ABI 主/次版本、Runtime 组件清单和产物可重复性字段按 10T/10U 的版本规则更新。
5. 增加 VM/原生成对差分、`catch`/未匹配传播、动态 `message`、非文本参数和失败清理用例；
   原生侧必须有受控工具链实跑和一次故意变异的红色证据。

## 四、验收

- `code = "DYNAMIC"` 的 VM 与原生均退出 3，错误码均为 `DYNAMIC`，源码位置一致；
- 动态 `message` 与缺省参数的报告字段一致；
- `ArithmeticError(code = 1)` 等非法形态在两侧保持稳定拒绝；
- `cargo test --workspace`、`clippy`、`fmt`、Bun/TypeScript 门禁及受控原生差分全绿；
- 旧的 B/C 结论和 Z-1 范围不被重新打开。

## 相关页面

- [10Z-Z1. Y3 收口结论与 Z-1 范围](10z-z1-scope-and-gap-verdict.md)
- [10Z-Y3. Y3 阶段汇报与收尾对照](10z-y3-stage-report.md)
- [10U. 表构造参数与函数值 ABI](10u-table-construction-and-function-value-abi.md)
- [10T. 表方法 ABI 实现](10t-table-method-abi-implementation.md)
