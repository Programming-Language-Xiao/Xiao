# 10Z-D. 动态错误构造参数实现记录

> **来源**：[10Z-Z1](10z-z1-scope-and-gap-verdict.md) 的 D 裁定。`ArithmeticError(code = code)`
> 是前端接受、VM 可执行的合法程序；**实施前**原生后端只接受字符串字面量，构建阶段拒绝动态参数。
>
> 状态：**已实现（2026-10-12，归入 10R 收尾）**。Windows 受控工具链上的 VM/LLVM 差分与释放轨迹已于 2026-10-13 通过；Linux/macOS 复跑和变异红色证据仍待补。
>
> 进度补记（2026-10-12）：LLVM 动态降低器现已对 `code`/`message` 发射表达式并借用
> `xiao_runtime_error_new_values`；Runtime 对非文本值返回稳定类型错误，失败后沿现有错误路由传播。
> ABI 次版本由 1.11 提升到 **1.12**，静态/动态入口统一使用同一兼容检查。Windows 本机受控差分已补；一次变异红色证据仍待外部执行点。

## 一、已确认缺口

最小源码：

```xiao
code = "DYNAMIC"
raise ArithmeticError(code = code)
```

VM 运行到用户自己的 `raise`，报告错误码 `DYNAMIC`、退出码 3。**实施前**原生构建返回
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
4. ABI 主/次版本、Runtime 组件清单和产物可重复性字段按 10T/10U 的版本规则更新；当前基线为 ABI 1.12。
5. 增加 VM/原生成对差分、`catch`/未匹配传播、动态 `message`、非文本参数和失败清理用例；
   原生侧必须有受控工具链实跑和一次故意变异的红色证据。

## 四、验收

- `code = "DYNAMIC"` 的 VM 与原生均退出 3，错误码均为 `DYNAMIC`，源码位置一致；
- 动态 `message` 与缺省参数的报告字段一致；
- `ArithmeticError(code = 1)` 等非法形态在两侧保持稳定拒绝；
- `cargo test --workspace`、`clippy`、`fmt`、Bun/TypeScript 门禁及受控原生差分全绿；
- 旧的 B/C 结论和 Z-1 范围不被重新打开。

## 五、Windows 受控原生验证（2026-10-13）

在 MSVC `vcvars64`、MSYS2 UCRT64 Clang/LLVM 22.1.2、Runtime release staticlib 与诊断程序构成的受控环境中，
重新执行 `xiao-driver/tests/n0_a_native_driver.rs::optional_frontend_artifact_differential_round_trip`，
覆盖动态 `code`/`message`、非文本参数、`catch`、未匹配传播和释放轨迹，全部通过；
完整 Windows workspace ignored 门禁也通过，其中 `d19a_differential::native_side_matches_the_vm_sides_on_every_case`、
O1–O3 表方法差分和 15E 五项均通过。

审核实测修复了两处旧差分缺陷：LLVM 曾为变量错误参数额外复制句柄，导致释放轨迹比 VM 多一次；
现在直接借用已拥有的输入槽。非文本参数的 VM 类型错误原先缺少构造表达式位置，现补上 IR span，
与原生报告一致。Windows 受控证据不替代 Linux/macOS 复跑、一次故意变异的红色证据或真实 push 运行号。

## 相关页面

- [10Z-Z1. Y3 收口结论与 Z-1 范围](10z-z1-scope-and-gap-verdict.md)
- [10Z-Y3. Y3 阶段汇报与收尾对照](10z-y3-stage-report.md)
- [10U. 表构造参数与函数值 ABI](10u-table-construction-and-function-value-abi.md)
- [10T. 表方法 ABI 实现](10t-table-method-abi-implementation.md)
