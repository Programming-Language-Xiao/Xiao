# `xiao-codegen-llvm/src`

放置 N0-A LLVM 文本降低、N0-B 动态 Runtime ABI 降低、规范化目标描述、显式工具链驱动和
原生运行观察接口。后端只消费 `xiao-ir`，不把 Rust Runtime 内部布局当作 ABI；稳定 ABI
由同级的 `xiao-runtime-abi` crate 提供。优化、诊断窗口和 CLI 仍属于后续阶段。

`ir.rs` 是静态标量降低器，不能依赖 `dynamic.rs`；`dynamic.rs` 是动态降低门面，职责实现
位于 `dynamic/`，详细边界见 `dynamic/README.md`。`text.rs` 提供 `escape_llvm` 和
`stable_hash` 的 crate 级单一来源，静态和动态路径共同消费；`dynamic_architecture_tests.rs`
锁定模块登记、依赖方向和门面不回流实现。
