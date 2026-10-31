# `xiao-codegen-llvm/src`

22E 的 `native_metadata.rs` / `native_metadata_sections.rs` / `native_metadata_tests.rs`
保存公开并发产物的有界编解码、ELF/PE/Mach-O 专用节读取与拒绝回归。构建后从真实文件
重新读取并校验 ABI、最小位集、目标和标准库身份；不执行入口，也不使用外部解析命令。

放置 N0-A LLVM 文本降低、N0-B 动态 Runtime ABI 降低、22D 同源 IR 并发计划与 trampoline
发射、规范化目标描述、显式工具链驱动和
原生运行观察接口。后端只消费 `xiao-ir`，不把 Rust Runtime 内部布局当作 ABI；稳定 ABI
由同级的 `xiao-runtime-abi` crate 提供。优化、诊断窗口和 CLI 仍属于后续阶段。

`ir.rs` 是静态标量降低器，不能依赖 `dynamic.rs`；`dynamic.rs` 是动态降低门面，职责实现
位于 `dynamic/`，详细边界见 `dynamic/README.md`。`text.rs` 提供 `escape_llvm` 和
`stable_hash` 的 crate 级单一来源，静态和动态路径共同消费；`dynamic_architecture_tests.rs`
锁定模块登记、依赖方向和门面不回流实现。
