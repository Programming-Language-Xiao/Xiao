# `xiao-codegen-llvm` 测试

工程期 10A。

这里验证 N0-A 的类型映射、控制流和拒绝边界。端到端 LLVM 工具链测试只在调用方提供
`XIAO_CLANG`（可选的 `XIAO_LLVM_AS`）时运行；没有工具链时仍验证结构化缺失错误，避免
把 PATH 或某台开发机的安装状态写成 Rust 测试契约。
