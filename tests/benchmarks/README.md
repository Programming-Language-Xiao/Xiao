# `tests/benchmarks`

这里承载 09R3 的四份基准报告、固定源码和零框架测量器。基准程序必须先经过
`xiao-driver::FrontendCompiler`，再由同一份 `TacProgram` 交给栈式、分类型寄存器式和混合式
载体；没有手搓 TAC、`xiao run` 或 LLVM 原生对照。

`manifest.json` 固定 `FORMAT_VERSION = 3`、opcode `0..40`、Rust `1.96.0`、`release`、
`opt-level = 3`、`codegen-units = 1`、关闭 LTO、3 次预热、11 次测量、输入规模、以中位数和
四分位区间统计。四个程序族分别是深表达式算术、多具名局部循环、深调用递归和容器密集操作；
源码、入口实参和期望结果一起受版本控制。清单同时包含运行时溢出与布尔奇偶加减，容器族覆盖
字符串索引、表实例与 `drop`、`try/finally`、数组/元组/字典/集合及高级选择。程序逐项避开
`string_boolean`、表方法值/动态派发、`*args`/`**kwargs` 展开实参三条拒绝路径。

在 Windows 原生环境运行：

```text
cargo run --release --manifest-path tests/benchmarks/Cargo.toml
```

工具只用 `std::time::Instant`，不引入 `criterion`。报告写入 `reports/`：语义差分、性能、
内存（峰值工作集、`stack_map_entries`、`spill_count`、`call_save_count`）和编码体积；每份都
带平台、协议、布局版本和 opcode 范围。`09r3-freeze.json` 只根据性能报告选择主机型，并明确
记录 Linux/macOS 的待复现状态。WSL 与容器只用于开发，数字不计入验收。

报告重新生成前应确认使用 `core/rust/rust-toolchain.toml` 的 `1.96.0`；任何指令集、ABI 或
编码改动都会使本轮数字作废并要求重新冻结。
