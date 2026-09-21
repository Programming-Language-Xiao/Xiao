# `xiao-driver` 测试

本目录覆盖 08-U0 统一前端入口，以及 09-B0-C/D 前端到生产 VM 驱动器和退出码的公共契约。
`u0_frontend.rs` 验证阶段顺序、诊断累积和成功/失败产物边界；`b0_c_driver.rs` 验证脚本与
`[main]` 端到端运行、三段结构化失败、执行前拒绝和取消/超时边界。测试不接入 CLI 或 LLVM。

`b0_d_exit_codes.rs` 验证五种终局的 `ExitCode` 映射、固定进程码、`catch` 消费错误的成功
语义、Fatal 与 locale 中立性。测试仍通过真实前端和生产 VM 驱动器执行，不读取诊断文本来
判断结果。
