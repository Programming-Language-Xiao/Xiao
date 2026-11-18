# 07-error-control 规格快照

本目录承载阶段 07 已冻结的 `try`/`catch`/`finally`/`raise` 语法和静态边界：可恢复
错误值、`FatalError` 不可被普通处理器捕获，以及具体处理器必须先于宽泛处理器。
快照不冻结 `finally` 与 `drop` 的最终运行时精确顺序，也不覆盖 `[debug]` 字段、后置
的 07-C/07-D 或 Runtime 展开实现。

快照沿用第二代 schema：顶层为 `stage`、`status`、`cases`，每条用例为 `name`、
`source`、`expect` 和稳定 `diagnostics` 编号。执行入口是
`core/rust/crates/xiao-types/tests/control_flow_snapshots.rs`；它先运行解析器，
再运行 `TypeChecker`，因此夹具被删除或实现退化都会使测试失败。
