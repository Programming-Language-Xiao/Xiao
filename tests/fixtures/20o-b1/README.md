# 20O-B0 的 B1 语言边界夹具

工程期：20O-B0；存放不应混入成功教程的拒绝与错误探测，执行逻辑在
[20o_b1_probe.rs](../../../core/rust/crates/xiao-driver/tests/20o_b1_probe.rs)。

- csv-nested-signature.xiao：嵌套数组函数注解，前端 X04-PARSE-005。
- error-code-access.xiao：读取捕获错误的 code 成员，VM X06-RUNTIME-002。
- numeric-overflow.xiao：跨函数整数溢出，VM X06-RUNTIME-009。
- explicit-error.xiao：纯 Xiao Error(code=...) 跨函数后保留 B0_PROBE 身份。

成功教程的源码位于 ../docs/ 下各模块目录。这里只登记现有边界，不修改语言/Runtime。
原生成功示例不等同这些未捕获错误的原生证据；完整结论见
[B0 冻结草案](../../../docs/DevDocs/20o-b0-baseline-and-b1-draft.md)。
