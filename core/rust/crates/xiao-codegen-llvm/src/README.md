# `xiao-codegen-llvm/src`

放置 N0-A LLVM 文本降低、规范化目标描述、显式工具链驱动和原生运行观察接口。后端只
消费 `xiao-ir`，不把 Rust Runtime 内部布局当作 ABI；稳定 ABI 由同级的
`xiao-runtime-abi` crate 提供。优化、诊断窗口和 CLI 仍属于后续阶段。
