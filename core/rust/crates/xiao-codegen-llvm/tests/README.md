# `xiao-codegen-llvm` 测试

工程期 10A。

22E 的公开并发元数据与五个教程例由 `xiao-driver/tests/22e_release.rs` 的独立 ignored
用例在三平台精确运行，覆盖实际节、strip 和旧 Runtime 拒绝；格式/容器拒绝另由
`src/native_metadata_tests.rs` 验证。公开构建现在使用同一元数据门禁。

这里验证 N0-A 的类型映射、控制流和拒绝边界；`xiao-driver/tests/22d_llvm_concurrency.rs`
验证同源并发 IR、trampoline 文本和未知并发操作 fail-closed，另外提供一组由三平台 native
job 精确执行的 ignored `llvm-as` 符号验证、32B/16B C 布局对象探针，以及一条覆盖
12 个成功场景、16 个运行时错误和一个嵌套 detached 失败场景的 ignored VM/native 差分。
稳定场景逐事件比较归一化释放顺序；detached 的独立打印对象允许交换，取消握手另核对
稳定共享对象与守卫释放。收尾还覆盖十万次 worker 循环、finally/drop 一次及 XCON 大小边界。
覆盖层次与三平台证据见 [22D §9.3](../../../../../docs/DevDocs/22d-llvm-concurrency-differential.md#93-22d-fix1-收尾矩阵与证据)。端到端 LLVM 工具链测试使用
`#[ignore]` 明确标出环境门控：默认 `cargo test` 会在汇总中显示 11 条 ignored，显式
执行 `cargo test -p xiao-codegen-llvm -- --ignored` 才会运行它们。显式运行时必须提供
`XIAO_CLANG`、需要汇编验证的测试还要提供 `XIAO_LLVM_AS`，并设置
`XIAO_TARGET_TRIPLE`；缺变量应失败而不是静默返回。Windows 原生准备方式见
`docs/DevDocs/10d-environment-gated-test-spec.md` §4。

`15e_ci_gated.rs` 额外门控真实 Runtime 链接产物的 strip 三态、O0-O3 调试激活位、符号表
与调试路径、重复构建字节比较和平台独立性能基线；显式运行还需要 `XIAO_LLC`、
`XIAO_RUNTIME_LIBRARY`、`XIAO_STRIP` 和 `XIAO_DIAGNOSTICS_PATH`。
