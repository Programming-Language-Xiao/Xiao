# `tests/differential`

09R3 差分驱动由 `tests/benchmarks` 的同一个零框架测量器提供：同一份 Xiao 源码先经过
`FrontendCompiler`，只降低一次 `TacProgram`，再以相同语义输入运行三种载体。驱动比较成功/错误
状态、错误码与原因链、返回值、完整释放序列和最大调用深度；任一机型不一致即使进程退出失败，
报告中的 `all_machines_equal` 也必须为 `false`。

在 Windows 原生环境执行：

```text
cargo run --release --manifest-path tests/benchmarks/Cargo.toml
```

语义报告位于 `tests/benchmarks/reports/windows-native-semantic-differential.json`。性能基线是
**同一次构建产出的栈式 Rust 原型**，不是 `xiao build` LLVM 原生模式；LLVM 原生后端属于后续
10/15 阶段。Linux、macOS 只登记在冻结记录的待复现清单中，WSL 与容器数字不进入本阶段验收。
