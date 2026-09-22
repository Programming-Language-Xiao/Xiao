# `xiao-codegen-llvm` 测试

工程期 10A。

这里验证 N0-A 的类型映射、控制流和拒绝边界。端到端 LLVM 工具链测试使用
`#[ignore]` 明确标出环境门控：默认 `cargo test` 会在汇总中显示 4 条 ignored，显式
执行 `cargo test -p xiao-codegen-llvm -- --ignored` 才会运行它们。显式运行时必须提供
`XIAO_CLANG`、需要汇编验证的测试还要提供 `XIAO_LLVM_AS`，并设置
`XIAO_TARGET_TRIPLE`；缺变量应失败而不是静默返回。Windows 原生准备方式见
`docs/DevDocs/10d-environment-gated-test-spec.md` §4。
