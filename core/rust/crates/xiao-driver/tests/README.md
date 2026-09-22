# `xiao-driver` 测试

本目录覆盖 08-U0 统一前端入口，以及 09-B0-C/D 前端到生产 VM 驱动器和退出码的公共契约。
`u0_frontend.rs` 验证阶段顺序、诊断累积和成功/失败产物边界；`b0_c_driver.rs` 验证脚本与
`[main]` 端到端运行、三段结构化失败、执行前拒绝和取消/超时边界。测试不接入 CLI 或 LLVM。

`b0_d_exit_codes.rs` 验证五种终局的 `ExitCode` 映射、固定进程码、`catch` 消费错误的成功
语义、Fatal 与 locale 中立性。测试仍通过真实前端和生产 VM 驱动器执行，不读取诊断文本来
判断结果。

`n0_a_native_driver.rs` 和 `n0_b_dynamic_native.rs` 是 LLVM/Runtime 外部工具链测试。它们
使用 `#[ignore]` 门控：默认运行汇总中显示 3 条 ignored，显式执行
`cargo test -p xiao-driver -- --ignored` 才会运行；显式运行必须提供
`XIAO_CLANG` 和 `XIAO_TARGET_TRIPLE`；动态 Runtime 测试还需要
`XIAO_LLVM_AS`、`XIAO_RUNTIME_LIBRARY`，并会用传入的 `rustc` 查询
`native-static-libs`，再将清单传给 clang。缺少变量或 Runtime staticlib 时测试直接失败。
Windows 原生准备方式见 `docs/DevDocs/10d-environment-gated-test-spec.md` §4。
