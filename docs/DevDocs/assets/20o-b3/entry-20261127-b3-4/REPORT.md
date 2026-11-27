# B3-4-ENV-1A 实施报告

## 结论

B3-4A 固定了 `3506455ee5839214082df15af4c760f8d0361e66` 和 Git 树
`6c5b1177d7ca7faa2d7237c80cb83d970cc630b6`。固定提交的两份锁文件、历史
Runtime 和工具链身份、B1 原始守卫及 Windows 六份 fs 产物回执均可复核。
22D/FIX.2 严格审计退出 0。[入口基线](baseline.json) · [审计日志](logs/baseline-22d-audit.log)

B3-4B 激活了 60～62，63 为 `reserved`，100 为 `tombstone`。逐项比较固定提交的
intrinsic 声明，变更仅涉及 60～62。ABI 为 1.20，生成器为 26，标准库接口为 1。
[源码核对](logs/source-check-after.log)

本机专项验证覆盖 Windows x86_64 MSVC。驱动支持有序 `[[name, value]]`，区分字段省略
与显式空数组。嵌入和原生注入所用的数据层方法为每次接纳创建独立视图。Runtime 先检查
实参类型和能力，再检查参数和预算，并在提交前检查运行控制。宿主来源回调仅在位 7 获授权
时调用。Windows 以 `CompareStringOrdinal` 比较名称，Linux 和 macOS 分支精确比较
原始 UTF-8 字节。系统环境保持独立，局部更新随运行视图释放。B3-4A/B 完成后 workspace
为 195 组、1,810 通过、0 失败、55 忽略；B3-4C～E 尚未开始。

## 改动文件

| 文件 | 改动 |
| --- | --- |
| `xiao-intrinsics/intrinsics.json`、`xiao-intrinsics/build.rs`、`xiao-intrinsics/src/lib.rs` | 登记新签名、效果、能力、类型错误、静态 VM 与 ABI 绑定键及 C 符号。构建与单测校验声明。 |
| `xiao-runtime/src/environment.rs`、`xiao-runtime/src/environment_tests.rs` | 快照接纳、严格 Unicode、平台名称比较、配额、局部读写、提交安全点及单测。 |
| `xiao-runtime/src/lib.rs`、`xiao-runtime/src/resources.rs` | 导出环境核心，复用既有能力校验。 |
| `xiao-driver/src/protocol/request.rs`、`xiao-driver/src/lib.rs` | 定义并导出 `RunEnvironment`，增加 `RunOptions.environment` 和数据层快照接纳。 |
| `xiao-driver/src/protocol_tests.rs` | 校验线协议形状、省略与显式空数组、顺序、显式快照和错误编号。 |
| `xiao-bytecode/src/encode/tests.rs`、`xiao-bytecode/tests/b4_cmath_tac.rs`、`xiao-driver/tests/20o_b2_b2_divide.rs`、`xiao-driver/tests/20o_b2_b4_cmath_vm.rs`、`xiao-driver/tests/20o_b3_time_tutorial.rs` | 五项旧负例从 60 号改为 63 号；保留未知编号与墓碑覆盖。 |
| 当前证据目录 | 入口基线、说明、原始日志及本报告。 |

代码文件均相对于 `core/rust/crates`。工作区以 `7936b307afeafec628def0fbaa7708f65fc29106`
为基准，改动全部留在工作区。

## 验证命令

| 命令 | 退出码 | 结果与原始日志 |
| --- | ---: | --- |
| `cargo +1.96.0 test --locked --manifest-path core/rust/Cargo.toml -p xiao-intrinsics` | 0 | 11 项通过。[日志](logs/intrinsics-final.log) |
| `cargo +1.96.0 test --locked --manifest-path core/rust/Cargo.toml -p xiao-runtime --lib env` | 0 | 13 项通过，其中 12 项为新增环境单测。[日志](logs/runtime-env-final.log) |
| `cargo +1.96.0 test --locked --manifest-path core/rust/Cargo.toml -p xiao-driver --lib run_environment` | 0 | 4 项通过。[日志](logs/driver-env-attempt-1.log) |
| `cargo +1.96.0 test --locked --manifest-path core/rust/Cargo.toml -p xiao-driver --test 22d_llvm_concurrency -- --list` | 0 | 编译成功，列出 10 项。[日志](logs/driver-22d-list-final.log) |
| `cargo +1.96.0 test --locked --manifest-path core/rust/Cargo.toml --workspace` | 101 | 首次运行：五项旧 60 号负例触发提前退出。[首次日志](logs/workspace-attempt-1.log) · [完整失败](logs/workspace-no-fail-fast.log) |
| `cargo +1.96.0 test --locked --manifest-path core/rust/Cargo.toml --workspace --no-fail-fast` | 0 | 修正五项旧负例后：195 组，1,810 项通过、0 失败、55 项忽略。[最终日志](logs/workspace-after-testfix.log) |

三个专项在 `cargo fmt` 后复跑仍为 11/13/4 通过：`logs/intrinsics-after-fmt.log`、
`logs/runtime-env-after-fmt.log`、`logs/driver-env-after-fmt.log`。

Runtime 单测覆盖 1,024 对、名称 256 字节、值 65,536 字节和总计 1 MiB 的边界，包含
`get`、`unset` 名称超长的 012，运行内写入超限的 `ENV_LIMIT_EXCEEDED`，以及非法、超限
快照的 006、007。配额替换、并发可见性、返回值所有权和失败保留均有单测。

入口采集首轮将声明文件顺序当作编号顺序，退出 1。按编号排序后重试退出 0。
intrinsic 首轮断言仍把活动上限写为 59，退出 101，更新后重试退出 0。
Runtime 首轮使用 `-p xiao-runtime --lib environment`，测试线程返回了线程绑定的
`RuntimeValue`，编译退出 101。修正后以同一筛选重试退出 0，最终按规定的 `env`
筛选复跑也退出 0。最终测试在线程内消费该值，由线程共享环境视图。
[基线首次](logs/baseline-capture-attempt-1.log) · [基线重试](logs/baseline-capture-attempt-2.log) ·
[声明首次](logs/intrinsics-attempt-1.log) · [声明重试](logs/intrinsics-attempt-2.log) ·
[Runtime 首次](logs/runtime-env-attempt-1.log) · [Runtime 重试](logs/runtime-env-attempt-2.log)

源码核对在完整复跑构建期间和测试结束后各执行一次，两次退出 0，本批源码改动的字节
摘要一致。复核了两个锁文件、版本常量、受保护路径、暂存区及 `git diff --check`。
五项旧负例修正和 `cargo fmt` 只改变测试/格式，不改变产品语义。[格式回执](logs/fmt-after-testfix.log)
[构建期间核对](logs/source-check-before.log) · [结束后核对](logs/source-check-after.log) · [最终源码核对](logs/source-check-final.log)

## 失败项与边界

首次工作区运行出现五项失败，原因都是旧负例仍把已激活的 60 号当作 reserved。五项已按
“60～62 active、63 reserved”修正为检查 63；未知编号和 100 墓碑覆盖保持不变。修正后
workspace 退出 0。原始失败日志保留用于追溯。[完整失败输出](logs/workspace-no-fail-fast.log) ·
[修正后通过](logs/workspace-after-testfix.log)

| 测试 | 修正 |
| --- | --- |
| `decoded_intrinsic_ids_are_checked_in_both_operand_widths` | 负例编号 60→63；仍断言未知编号和墓碑被拒绝。 |
| `cmath_unknown_intrinsic_ids_are_rejected_before_encoding` | 负例编号 60→63；保留生产 TAC 拒绝覆盖。 |
| `lfloat_divide_requires_the_frozen_public_contract` | `active_by_id(60)`→`active_by_id(63)`，仍断言 63 非活动。 |
| `vm_rejects_invalid_shapes_types_and_unknown_ids` | 注入的 invalid intrinsic 60→63；仍断言编码前拒绝。 |
| `reserved_intrinsic_ids_fail_before_execution` | 逐项 60～63→仅检查 63；60～62 的 VM/LLVM 正向接线归 B3-4C。 |

环境调用的 VM、LLVM、C ABI 和后端验证仍由 B3-4C 接线后执行；本批只证明 Runtime/驱动
数据层和现有 workspace 不退化。

## B3-4C 接线

- CLI 和直接原生入口在解析能力后，根据字段是否省略选择环境来源。显式输入直接经
  `RunOptions::environment_snapshot` 接纳。授权且省略时用 `EnvironmentSnapshot::from_host`
  包住一次实际读取，并由 Windows 启动适配层过滤驱动器 cwd 伪变量。源码、`.xiaoc`
  和 `.xar` 的驱动运行入口可参考 `xiao-driver/src/protocol/run.rs` 的参数接纳位置。
  接入执行入口时，将快照 006/007 映射到拒绝执行和退出码 2。
- 本批为原生启动注入记录的载荷编码是 UTF-8 JSON 顶层名值对数组，与 `RunOptions.environment`
  字段同构。以借读字节指针和长度表达有值载荷，`(NULL, 0)` 表示省略，字节
  `5B 5D`（`[]`）表示显式空快照。接纳后由 Runtime 持有字符串。跨进程传送与入口
  签名落在 B3-4C，需保证 CLI 转原生时整次运行只捕获一次。
- VM/TAC 在 `xiao-vm/src/semantics/exec.rs` 的 `execute_intrinsic` 接入 60～62，
  复用运行控制安全点。沿 VM 的运行请求与并发派发和 Runtime 的 `NativeEntryContext`
  传递同一次运行的 `Arc<EnvironmentSnapshot>`。在前端和标准库接通命名参数、
  `std.env` 与 `stdlib.env` 的规范身份及归档能力要求。
- LLVM 在 `xiao-codegen-llvm/src/dynamic/expression.rs` 的 `emit_intrinsic`、
  入口生成与产物组件归类处接线，连同静态路径核对支持范围。沿 B3-0D-CABI 实现
  `xiao_runtime_env_get`、`xiao_runtime_env_set`、`xiao_runtime_env_unset` 和
  所有权、输出槽保留、状态码、取消通道。用固定 Runtime 和工具比较 LLVM IR，再据
  B3-4-GEN 判定是否申请生成器 27。
- 五项既有负例已在本批改为 63；B3-4C 只需补 60～62 的活动入口、权限边界和真实源码/原生
  执行验证，不重复保留编号的负例。
