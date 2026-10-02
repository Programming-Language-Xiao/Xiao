# `xiao-codegen-llvm` 测试

工程期 10A。

这里验证 N0-A 的类型映射、控制流和拒绝边界。端到端 LLVM 工具链测试使用
`#[ignore]` 明确标出环境门控：默认 `cargo test` 会在汇总中显示 11 条 ignored，显式
执行 `cargo test -p xiao-codegen-llvm -- --ignored` 才会运行它们。显式运行时必须提供
`XIAO_CLANG`、需要汇编验证的测试还要提供 `XIAO_LLVM_AS`，并设置
`XIAO_TARGET_TRIPLE`；缺变量应失败而不是静默返回。Windows 原生准备方式见
`docs/DevDocs/10d-environment-gated-test-spec.md` §4。

`15e_ci_gated.rs` 额外门控真实 Runtime 链接产物的 strip 三态、O0-O3 调试激活位、符号表
与调试路径、重复构建字节比较和平台独立性能基线；显式运行还需要 `XIAO_LLC`、
`XIAO_RUNTIME_LIBRARY`、`XIAO_STRIP` 和 `XIAO_DIAGNOSTICS_PATH`。
