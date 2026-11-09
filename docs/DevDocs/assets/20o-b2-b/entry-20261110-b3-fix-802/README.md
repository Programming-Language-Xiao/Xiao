# B3-4／802 回归修复证据

本目录记录 `802b6ff9` 之后的最小回归修复。修复把 LLVM 动态降低器的标准库函数预注册范围收紧为实际导入图，避免 `stdlib.hex` 的原生构建提前生成 planned `cmath` 原语；同时补上子 VM 别名快照、`.xar` 归档身份和 planned 边界测试。

修复提交为 `ea9e33baa8dc0cdd03c2108b8ba1a89797bcf850`，基线为 `802b6ff9`。本目录的测试运行绑定该修复工作树，日志保留原始字节并由目录内 `.gitattributes` 的 `* -text -diff` 保护。B3-4 仍未收口：`cmath` 的源码包装、C ABI 与 intrinsic 已存在，语言接线、可执行 VM/LLVM 入口和归档运行仍待 B4。

| 执行点 | 命令 | 结果 |
| --- | --- | --- |
| 包装与 hex 原生隔离 | `cargo +1.96.0 test --locked --manifest-path core/rust/Cargo.toml -p xiao-driver --test 20o_b2_b3_wrapper -- --nocapture` | 7 passed / 0 failed；`stdlib.hex` 原生降低通过，未预加载 `stdlib.cmath` |
| 子 VM 与归档边界 | `cargo +1.96.0 test --locked --manifest-path core/rust/Cargo.toml -p xiao-driver --test 22c_vm_concurrency -- --nocapture` | 29 passed / 0 failed；两条 cmath 别名均记录为 `stdlib.cmath`，planned 入口按 X06-RUNTIME-012 拒绝；`.xar` 索引别名规范化通过，归档执行器缺标准库加载器时按边界拒绝 |
| LLVM 后端回归 | `cargo +1.96.0 test --locked --manifest-path core/rust/Cargo.toml -p xiao-codegen-llvm` | 55 个单测通过；相关测试仅保留既有受控 ignored 项 |

证据日志：

- `wrapper.log`：7 项包装契约、planned cmath 探测及 hex 原生隔离；
- `concurrency.log`：29 项并发/子 VM/归档回归；
- `archive-wiring.log`：既有 20E 归档门禁 11 项通过；
- `codegen.log`：LLVM crate 全量测试输出。

这组证据证明 802 的 hex 回归已修复，并证明子 VM/归档的规范身份与 planned 拒绝边界；它不宣称 `cmath` 已可执行、B3-4 已收口或三平台通过。
