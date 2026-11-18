# `xiao-lifetime/src`

这里按职责保存 06-A 的静态生命周期实现。`model.rs` 是跨阶段数据契约，
`graph.rs` 只处理图算法，`escape.rs` 只遍历 AST 并记录事实，`release.rs`
只把已收集事实编排为释放计划，`diagnostics.rs` 只保存稳定错误身份。
新增逻辑应放入对应模块，禁止把分析器重新堆回 `lib.rs` 或让图算法依赖 AST。
