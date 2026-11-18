# 04-types 规格快照

本目录承载 P2/S0 的静态标量类型和声明语法快照。解析快照只验证 Token 到 AST
的结构；声明快照由 `core/rust/crates/xiao-syntax/tests/p2_snapshots.rs` 的
`declaration_snapshot_is_stable` 入口读取，不在 JSON 中执行程序。

`xiao-types` 的类型推断与检查仍由其自身内联测试覆盖；本目录没有声称存在一个
会读取 `declarations.json` 的 `xiao-types` harness。

当前首批快照覆盖 `int`/`str`/`bool` 声明、`const` 声明和无初始化声明。
