# `xiao-driver` 测试

本目录覆盖 08-U0 统一前端入口，以及 09-B0-C/D 前端到生产 VM 驱动器、退出码和 X0-A 协议的公共契约。

`22c_vm_concurrency.rs` 覆盖真实源码的线程、任务池、共享锁、Runtime 身份、取消与
清理、detached 错误、XCON 限制和 `.xiaoc`/`.xar` 执行前门禁。
`22d_llvm_concurrency.rs` 覆盖同一 IR 的 LLVM 并发计划、trampoline、shared/lock 取消入口与 fail-closed；根/局部导入别名和分组目标由三平台 native job 显式运行真实 `llvm-as` 验证；另有覆盖五个成功黄金输出、六个运行时错误（含三个 Runtime 031 负例、主/清理错误、finally 内普通及嵌套 helper 返回）和 detached 失败投影的 VM/native 差分门控项。
`u0_frontend.rs` 验证阶段顺序、诊断累积和成功/失败产物边界；`b0_c_driver.rs` 验证脚本与
`[main]` 端到端运行、三段结构化失败、执行前拒绝和取消/超时边界。除 22D 明确标记的
ignored 原生差分外，测试不接入 CLI 或 LLVM。

`b0_d_exit_codes.rs` 验证五种终局的 `ExitCode` 映射、固定进程码、`catch` 消费错误的成功
语义、Fatal 与 locale 中立性。测试仍通过真实前端和生产 VM 驱动器执行，不读取诊断文本来
判断结果。

`n0_a_native_driver.rs` 和 `n0_b_dynamic_native.rs` 是 LLVM/Runtime 外部工具链测试。它们
使用 `#[ignore]` 门控：默认运行汇总明确列出环境依赖用例，显式执行
`cargo test -p xiao-driver -- --ignored` 才会运行；显式运行必须提供
`XIAO_CLANG` 和 `XIAO_TARGET_TRIPLE`；动态 Runtime 测试还需要
`XIAO_LLVM_AS`、`XIAO_RUNTIME_LIBRARY`，并会用传入的 `rustc` 查询
`native-static-libs`，再将清单传给 clang。缺少变量或 Runtime staticlib 时测试直接失败。
Windows 原生准备方式见 `docs/DevDocs/10d-environment-gated-test-spec.md` §4。

`x0_a_protocol.rs` 与 `tests/spec/11x0-protocol` 验证 Rust/TypeScript 共用的帧和消息样本；
它们只检查机器字段，不解析人类可读诊断文本。

10T 的 `d19a_differential.rs` 复用严格比较器验证表回调 O0–O3 与调试执行；macOS CI 显式跳过 GUI 执行、单独验证调试构建。
`native_benchmark_probe.rs` 对 container-dense 追加入口并实跑 80 轮、对照 VM 的 28760；这不是性能取数。

`20d_first_api.rs` 覆盖 `os` 首批资源 API 的 VM/原生差分、能力与错误身份、资源常量和
容器边界。导入签名回归覆盖函数/模块别名、限定路径、括号与函数值复制，并检查局部遮蔽、
赋值覆盖及 LLVM 既有拒绝边界；原生差分继续在受控工具链下显式执行。

`20d_process_api.rs` 使用同一 `process_child_exit7`/`process_child_abort` 子程序验证等待缓存、
信号/异常码、能力、资源类型及产物边界。隔离握手证明关闭和最后引用释放不等待或终止子进程，
并验证 stdio、PATH、环境和 cwd 继承。四项原生 ignored 测试在三平台受控作业中执行；隔离子测试由父用例驱动。

`20e_archive_wiring.rs` 通过真实前端产物和 `XarBuilder` 验证标准库版本、官方模块要求、
能力并集和请求掩码。负例断言在模块加载、函数进入或输出之前拒绝；协议与中英消息目录
使用同一错误身份，归档只包含应用对象。畸形索引的物理 ZIP 负例位于 `xiao-xar/src/tests.rs`。

`18b_build_archive.rs` 覆盖 O0–O3 单文件、唯一 `[main]` 项目入口、本地多模块及显式资源、
函数调用闭包的能力精度、调试对象、第三方拒绝、损坏缓存命中和锁文件只读边界。
实际 `xiao-core` 进程在隔离 `XIAO_HOME` 中验证协议构建/运行、官方加载及 REPL 缓存隔离。
