# B3-4-ENV-1B 实施报告

B3-4C 从 `aeb8422be2e2d260f283c3e6ecea0acec8080122` 开始，入口树为
`8b2f3c5dda5ad23359c7c8430e1837d90c017736`。本次交付包括 VM/TAC、LLVM、三个
环境 C ABI 符号，以及源码、字节码、归档和原生运行入口的环境快照。工作区产品版本为
ABI 1.20、生成器 27、标准库接口 1。[入口身份](logs/1b/source-baseline.log) · [工作区源码校验](logs/1b/source-final.log)

## 改动文件

下表的 Rust 文件路径相对于 `core/rust/crates`。

| 文件 | 实施 |
| --- | --- |
| `stdlib/env.xiao`、`xiao-types/src/checker/expression.rs` | 提供 get、set、unset，沿已冻结的形参与类型检查接通 60～62。 |
| `xiao-driver/src/{frontend,module_loader,native}.rs` | 在前端和后端解析同一个 `stdlib.env`，支持 `std.env`、`stdlib.env`、位置与命名调用。 |
| `xiao-vm/src/{lib,run}.rs`、`xiao-vm/src/semantics/{exec,concurrent,env_intrinsics,tables}.rs` | 分派给共享环境 Runtime，沿 VM 的故障与控制通道传播。并发任务与表析构帧访问同一次运行的快照。 |
| `xiao-driver/src/{run,archive_run,lib}.rs`、`xiao-driver/src/protocol/{request,run,service}.rs`、`xiao-xar/src/{lib,runner}.rs` | 按运行入口接纳快照，传递 VM 或归档入口。归档从依赖闭包记录位 7，缺授权时以 026 拒绝并指出 `environment`。 |
| `xiao-runtime/src/{environment,abi,abi_env,abi_concurrency,arguments}.rs`、`xiao-runtime-abi/src/lib.rs` | 过滤 Windows 驱动器 cwd 伪变量，安装单次原生上下文，校验 ABI 结构、出入槽、运行授权和控制。 |
| `xiao-codegen-llvm/src/{lib,build,artifact,ir,dynamic}.rs`、`xiao-codegen-llvm/src/dynamic/{cmath,entry,expression,predicate}.rs` | 发射借读 C ABI 调用、输出槽和清理路径。静态和动态 main 可安装快照。原生产物登记 `env` 组件，生成器升到 27。 |
| `xiao-driver/tests/20o_b3_env.rs`、`xiao-driver/tests/support/b3_env_native.rs`、`xiao-runtime/src/abi_env_tests.rs` | 增加真实源码、进程入口、归档、原生差分和 C ABI 专项。 |
| `xiao-driver/tests/{20o_b2_b4_cmath_llvm,20o_b2_b4_cmath_vm,20o_b2_b4_frontend,20o_b2_b5_versions,20o_static_float_equality}.rs` | 五处当前生成器断言随 B3-4-GEN 升到 27。 |

## 运行入口与 C ABI

| 入口 | 省略环境 | 显式环境 |
| --- | --- | --- |
| CLI 标准流核心进程、直接原生可执行文件 | 位 7 获授权时在用户代码前由 `EnvironmentSnapshot::from_host` 捕获一次，缺位时宿主快照读取为零 | 接纳有序名值对，`[]` 为空视图 |
| Rust 嵌入协议、原生注入 | 每次运行新建空视图。原生 `(NULL, 0)` 表示省略 | 经 Runtime 的同一接纳流程，原生载荷为 UTF-8 JSON 顶层数组 |
| 驱动构建后启动原生程序 | 捕获交给子进程启动适配器，父驱动只校验请求选项 | 由驱动接纳后经私有标准输入传递载荷，启动适配器消费传输标记，保留应用参数 |

Windows 启动层排除 `=C:` 形式的驱动器伪变量，再严格解码并按 `CompareStringOrdinal`
检查名称。快照含非法编码、非法条目或重复名称时拒绝整份快照，报告 006。配额错误报告
007。两者都在执行前返回退出码 2。显式载荷与能力验证采用同一运行请求，C ABI 使用入口
固定掩码和调用携带掩码共同授权。Runtime 专项中的数据来源回调验证授权调用一次和未授权
调用零次，进程级测试用非法宿主样本对照显式输入与缺授权的启动行为。CLI 授权取自
核心标准流请求中的 `RunOptions.capabilities`。本批原生转发覆盖 Rust 驱动的构建并启动
入口。
[Runtime 专项](logs/1b/runtime-env-final.log) · [驱动专项](logs/1b/driver-env-final.log) ·
[核心过程回执](logs/1b/vm-entry-receipt-final.log) · [原生专项](logs/1b/native-final-source.log)

`get` 和 `unset` 以独立初始化的 `out` 槽收取结果。LLVM 按源码求值顺序持有实参，
再按形参顺序传递借读指针，调用后逆序释放实参。成功时 Runtime 释放旧 `out` 并写入
新值，失败时保留原槽。`set` 省略输出槽。三入口在检查类型、权限、参数和预算之后
执行控制安全点。原生任务取消沿已有通道交付 027，在提交前取消会保留旧视图。源级差分
对照 stdout、错误身份、退出码和释放轨迹，原生 O0、O3 的成功用例各销毁 22 个被追踪
对象。C 探针直接链接本批发布 Runtime，Rust 侧另有可控内部故障与取消测试。
[C 符号](logs/1b/c-abi-symbols-final.log) · [C 探针](logs/1b/c-abi-probe.c) ·
[探针执行](logs/1b/c-abi-probe-final.log) · [原生运行原文与释放轨迹](logs/1b/native-output-final/denied-o0.stderr) ·
[原生文件摘要](logs/1b/native-artifact-hashes.log)

## LLVM IR、符号与生成器裁定

B3-4-GEN 的批准来源是星崽 2026-11-27 的生成器问题卡，由 `7936b307` 记录在
B3-4 实施规划 §三。用相同 IR 输入、同一份固定发布 Runtime、LLVM 22.1.2 和
`x86_64-pc-windows-msvc` 比较接线前后。固定 Runtime SHA-256 为
`6606a78f1003d09322f8df2cc61c584e418f3be183f63bae6b5cf91f19407be1`。
[批准裁定](../../../20o-b3-4-env-implementation-plan.md#三已批准裁定2026-11-27) ·
[工具身份](logs/1b/toolchain.log) · [固定 Runtime](logs/1b/generator-runtime.log)

| 比较 | O0 | O3 |
| --- | --- | --- |
| 接线前生成器 26 对环境输入 | 拒绝 `os_env_get`，无 IR | 拒绝 `os_env_get`，无 IR |
| 接线后仍用 26 对同一环境输入 | 三个环境调用与 main 启动调用均生成 | 三个环境调用与 main 启动调用均生成 |
| 将接线前后的 26 用于同一无环境对照 | IR 字节一致 | IR 字节一致 |
| 接线后使用 27 | 生成 IR，MSVC 链接图含环境符号，产品验证为 `Complete` | 生成 IR，MSVC 链接图含环境符号，产品验证为 `Complete` |

三个操作的 LLVM 声明为 `i32 (ptr, ptr, i64)`。get 与 unset 的第二个指针指向各自
的独立输出槽，set 的第二个指针是值输入，三处实参的末尾掩码为 `160`。main 调用
`xiao_runtime_env_run_native(ptr, i64, i32)`。对照得到的文本差异满足批准的升版条件。
实际链接符号、IR 摘要和产物及 map 的 SHA-256 均已记录。
[接线前](logs/1b/lowering-before-26.log) · [接线后 26](logs/1b/lowering-after-26.log) ·
[接线后 27](logs/1b/lowering-after-27.log) · [IR 和符号回执](logs/1b/ir-and-symbols.log)

| 缓存身份 | 26 | 27 |
| --- | --- | --- |
| 固定 Runtime 与目标下的工具指纹 | `xiao-fnv1a64-253256b8d2facb68` | `xiao-fnv1a64-d7d57b7a91d20313` |
| 环境 O0 生成指纹 | `xiao-codegen-26-aa186ec3e9142797` | `xiao-codegen-27-024ada7077324dba` |
| 环境 O3 生成指纹 | `xiao-codegen-26-222c8e133957c16a` | `xiao-codegen-27-95aa0b5f1958b467` |

工具指纹使用生成器、目标、工具和 Runtime 身份，生成指纹包含优化档、ABI、能力和
语言上下文。运行快照只进入执行上下文。新校验器实际接纳 26 对照产物，退出 0。以
`aeb8422b` 构建、上限为 26 的实际校验器在两档均拒绝 27 的 env 产物，诊断
`X17-XAR-007`，退出 2。27 校验器对 27 env 产物退出 0。
[27 校验 26 的 O0](logs/1b/validator-27-on-26-o0.log) · [O3](logs/1b/validator-27-on-26-o3.log) ·
[26 校验 27 的 O0](logs/1b/validator-26-on-27-o0.log) · [O3](logs/1b/validator-26-on-27-o3.log)

## 验证回执

使用 Rust 1.96.0，主测试和 release Runtime 分别使用独立的 `target` 与
`target-release`。命令启用 `--locked`，原生使用 MSYS2 UCRT64 的 clang、llvm-as、
llc，显式选择 MSVC 目标，由本机 MSVC 和 Windows SDK 链接。
[发布 Runtime 构建](logs/1b/runtime-release-final.log) ·
[发布 Runtime 摘要](logs/1b/runtime-release-final-sha256.log)

| 命令 | 退出码 | 回执 |
| --- | ---: | --- |
| `cargo +1.96.0 test --locked --manifest-path core/rust/Cargo.toml -p xiao-vm` | 0 | [VM](logs/1b/vm-verified.log) |
| `cargo +1.96.0 test --locked --manifest-path core/rust/Cargo.toml -p xiao-runtime` | 0 | [Runtime](logs/1b/runtime-attempt-1.log) |
| `cargo +1.96.0 fmt --manifest-path core/rust/Cargo.toml --all -- --check` | 0 | [格式](logs/1b/fmt-final-source.log) |
| `cargo +1.96.0 test --locked --manifest-path core/rust/Cargo.toml -p xiao-runtime --lib env -- --nocapture --test-threads=1` | 0 | 17 项通过。[环境与 ABI](logs/1b/runtime-env-final.log) |
| `cargo +1.96.0 test --locked --manifest-path core/rust/Cargo.toml -p xiao-driver --test 20o_b3_env -- --nocapture` | 0 | 7 项通过，3 项原生测试由下一条命令显式运行。[源码及入口](logs/1b/driver-env-final.log) |
| `cargo +1.96.0 test --locked --manifest-path core/rust/Cargo.toml -p xiao-driver --test 20o_b3_env -- --ignored --nocapture --test-threads=1` | 0 | 3 项通过，覆盖 O0/O3、启动快照、并发控制、析构、退出和释放。[原生](logs/1b/native-final-source.log) |
| `cargo +1.96.0 test --locked --manifest-path core/rust/Cargo.toml --workspace` | 0 | 196 组，1,821 项通过、0 失败、58 忽略。[完整日志](logs/1b/workspace-final.log) |
| `clang -target x86_64-pc-windows-msvc -std=c11 -Wall -Wextra -Werror …`，随后执行 C 探针 | 0、0 | 编译链接发布 Runtime 后执行，验证 0/1/2/3/6/8 和结果所有权。[编译](logs/1b/c-abi-clang-attempt-4.log) · [执行](logs/1b/c-abi-probe-final.log) |

Windows 验证包含等价名称、重复项、cwd 伪变量和拒绝非法 UTF-16。Runtime 测试覆盖名称
256/257 字节、值 65,536/65,537 字节、1,024 对、合计 1 MiB、NUL、等号和未知能力位。
Linux/macOS 精确比较分支有对应的条件编译测试，实际执行回执归 B3-4D。

## 失败与重试

| 首次观察 | 重试结果 |
| --- | --- |
| Git 自带 tar 被本机拒绝创建映射，退出 256。 | Windows 系统 tar 解出固定提交，退出 0，随后基线构建通过。[首次](logs/1b/baseline-extract-attempt-1.log) · [重试](logs/1b/baseline-extract-attempt-2.log) |
| 首轮编译检查遇到驱动私有字段访问与并行修改期间的新方法尚未编入，退出 101。 | 使用 `ProtocolError` 访问方法，完成启动接线，专项和原生构建均通过。[首次](logs/1b/check-attempt-1.log) |
| Runtime 测试依次遇到数组访问 API、借用和挂起错误测试隔离问题，三次退出 101。 | 改用 `element`，独立构造非法槽，按 ABI 规则消费挂起诊断，最终退出 0。[第一次](logs/1b/runtime-env-attempt-1.log) · [第二次](logs/1b/runtime-env-attempt-2.log) · [第三次](logs/1b/runtime-env-attempt-3.log) |
| 驱动首次专项退出 101，发现数组打印使用调试形状，归档缺权限提示遗漏名称，进程夹具清空后缺少主目录变量。 | 用源码迭代验证零项与一项结果，补归档缺能力名称，传入既有主目录变量以满足驱动缓存布局。第三轮退出 0。[首次](logs/1b/driver-env-attempt-1.log) · [第二次](logs/1b/driver-env-attempt-2.log) |
| 用 rustc 直接构建比较工具缺少跨 crate 的 Windows 链接搜索路径，退出 1。 | 同一探针按 Cargo example 构建。接线前探针退出 1，并记录预期的不支持诊断。接线后探针退出 0。[首次](logs/1b/probe-baseline-compile.log) · [基线构建](logs/1b/probe-build-before-26.log) |
| C 探针首次链接把 MSVC 库视作输入路径，退出 1。随后探针以机器字读取布尔载荷，断言失败。尝试动态 CRT 时出现未解析符号，退出 1120。 | 按 ABI 用 `uint8_t` 检查布尔成员，经 `-Xlinker` 传原生库并指定 `/NODEFAULTLIB:msvcrt`，编译和执行退出 0。[首次链接](logs/1b/c-abi-clang.log) · [首次执行](logs/1b/c-abi-probe.log) · [动态 CRT](logs/1b/c-abi-clang-final.log) |
| 升版后的首次驱动和工作区回归遇到固定值 26 的 cmath 断言，退出 101。 | 五处当前版本断言更新为 27，保留历史兼容输入。[驱动首次](logs/1b/driver-attempt-1.log) · [工作区首次](logs/1b/workspace-attempt-1.log) |
| 两后端的析构夹具都在读取后析构临时值，首次断言退出 101。 | 让析构钩子直接读取当前快照，验证主程序提交的值。VM 与原生均通过。[VM 首次](logs/1b/vm-env-verified.log) · [原生首次](logs/1b/native-verified.log) · [VM 重试](logs/1b/vm-env-drop-retry.log) · [原生重试](logs/1b/native-env-entry-retry.log) |
| 核心过程探针首次漏填目标字段，进程拒绝协议帧。 | 补齐指针宽度、字节序与对象格式，六个真实源码用例有逐案结果、退出码和原始输出。[首次](logs/1b/vm-entry-receipt.log) · [重试](logs/1b/vm-entry-receipt-final.log) · [原始响应](logs/1b/vm-output/denied.json) |

## 后续批次

B3-4D 接续冻结教程、两种导入、原生优化档与三平台矩阵，并为计划中的六份 Windows
产物分别保存绑定的 COFF 回执。B3-4E 对照固定源码、工具、Runtime、产物和运行日志做
证据审计，再依据完整矩阵裁定收口。当前代码和回执均留在工作区。

**历史证据边界**：B3-4C 升生成器 27 不回溯改写 B3-1/B3-2/B3-3 的原生证据。B3-3 的
fs 三平台证据仍绑定 `3506455e`/生成器 26；B3-4D/E 只声明 env，B3-7 在最终产品上重跑
六模块矩阵后才能更新对应阶段结论。不得把本报告的 env 专项写成六模块当前通过。
