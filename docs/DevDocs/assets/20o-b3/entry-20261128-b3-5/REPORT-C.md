# B3-5C LLVM、C ABI 与原生回执

B3-5C 入口提交为 `b794e2f289c263e611eb92198652fdb1d1847c37`，Git tree 为
`ca17956fea9153c0235eecdd68b2b37f662b0c5b`。独立任务根为
`D:\project\Xiao\.xiao-orchestrator\worktrees\b3-5c`，升版复核使用同级独立目录
`b3-5c-28`。工作区采用 Runtime ABI 1.20、**生成器 28**、标准库接口 1。
规划状态为 `b3_5_started=true`、`b3_5_passed=false`。intrinsic 编号表唯一差异是 63 的 ABI 绑定。
[入口身份](logs/c/baseline-git.log) · [27 基线锁](logs/c/locks.log) · [28 工具、Runtime 与锁文件摘要](logs/c/v28/tool-sha256.json) ·
[实施规划](../../../20o-b3-5-log-implementation-plan.md)

## 27→28 裁定与实测

星崽于 2026-11-28 经 Claude 调度中枢批准 27→28，依据为 B3-5C 的 O0/O3 实际生成文本
差分和无日志对照。已采用 28，并同步更新版本守卫、原生元数据检查和归档缓存构建键。
接线前 28 从固定入口源码只改变 `CODEGEN_VERSION` 后重新编译，与工作区 28 共用普通
release Runtime `0110ac28d86b484ed8a3814878093b49bf7be927ebdf98b32f36e15a57b280d3`、
Rust/Cargo 1.96.0、LLVM 22.1.2、MSVC link 14.51.36256.0、目标 `x86_64-pc-windows-msvc`
及同一份实参和输入源码。普通 release 与教程固定时钟 Runtime 均从本轮工作区重建。
[接线前源码比对](logs/c/v28/before28-source-delta.json) · [构建特性](logs/c/v28/runtime-features.json) ·
[工具版本](logs/c/tools-windows.log) · [完整逐档差分](logs/c/generator-diff.json)

| 实际 28 | O0 | O3 |
| --- | --- | --- |
| 接线前日志 | 拒绝 `动态 intrinsic os_log_write（58..94）`，退出 1 | 同 O0 |
| 接线后日志 | 汇编、链接、执行成功，退出 0 | 汇编、链接、执行成功，退出 0 |
| 日志代码生成指纹 | `xiao-codegen-28-ed6666689f575124` | `xiao-codegen-28-d518085d312d0079` |
| 日志工具链指纹 | `xiao-fnv1a64-912e4242051653ed` | `xiao-fnv1a64-ad0523bc83dd765a` |
| 日志二进制 SHA-256 | `7a8633e33d7bb98ddb8dde07434bfc08507537536b37209303bfb7e36c396600` | `a0145af4bbf3ddcb63652ef014fd4902fbf42d93e96fe6466a934a72c1cd27f7` |

两档日志 IR 摘要均为 `ad6f0bd7e0a725410a57ca50f248e9760debdb417bf6cd64b862cb88cb9e2a8d`。
两档无日志对照前后逐字一致，摘要均为
`84dece0083e9c8613b9cbb5a17983cf51eda9041bf302ff965ba934c9fdd78c6`。
用真实 27 与接线前 28 对同一无日志源码生成 IR，两份文本的差异落在原生元数据的生成器字段。
固定 Runtime 下，工具链指纹和代码生成指纹均随版本变更。接线后的日志 IR 含
`declare i32 @xiao_runtime_log_write(ptr, ptr, ptr, i64)`、三个输入槽和 `i64 34`。
[O0 接线前](logs/c/v28/before28-log-o0.stderr.log) · [O3 接线前](logs/c/v28/before28-log-o3.stderr.log) ·
[实际版本差分](logs/c/v28/actual-version-only.diff) · [LLVM 调用](logs/c/v28/lowering-28.log) ·
[O0 链接](logs/c/v28/after28-log-o0-link.log) · [O3 链接](logs/c/v28/after28-log-o3-link.log)

### 真实产物兼容与缓存

27 校验器从 `b794e2f2` 的 LF 归档按原代码构建，协议握手报告 `codegen: 27`。
28 校验器在同一工具链上从工作区构建，握手报告 `codegen: 28`。
归档索引 1.0 缺少生成器字段，生成器 28 写入索引 1.1 及必需的生成器字段。
真实 27 读取器由既有索引版本门槛拒绝 1.1。28 读取器同时接纳 1.0，并检查 1.1 声明的
生成器上限。`.xar` ZIP 容器、`.xiaoc` 容器、全局缓存索引及原生元数据的格式版本仍为
1.0 或 1；只有归档索引在 28 下使用 1.1。
为统一归档和原生上限，归档库读取 LLVM 后端的版本常量，两份 Cargo 锁各登记一条内部依赖。
[真实 27 构建](logs/c/v28/old27-core-build.log) · [真实 28 构建](logs/c/v28/build-28-tools.json) ·
[验证器 SHA-256](logs/c/v28/tool-sha256.json)

| 校验器和输入 | 观察结果 |
| --- | --- |
| 28 校验真实 27 无日志 O0/O3 原生产物 | 两档均接纳，元数据读到 27，退出码 0。[O0](logs/c/v28/native-28-on-27-o0-verified.json) · [O3](logs/c/v28/native-28-on-27-o3-verified.json) |
| 27 校验真实 28 日志 O0/O3 原生产物 | 两档均在执行前以 `X17-XAR-007` 拒绝，校验退出码 2。[O0](logs/c/v28/native-27-on-28-o0-verified.json) · [O3](logs/c/v28/native-27-on-28-o3-verified.json) |
| 27 执行 28 归档 | 无日志和仅日志归档均在读取入口前以 `X17-XAR-007` 拒绝，退出码 2。CLI 绑定 27 核心也以进程退出码 2 拒绝。[无日志](logs/c/v28/archive-run-27-on-28-verified.json) · [仅日志](logs/c/v28/archive-run-27-on-log-28-verified.json) · [CLI](logs/c/v28/cli27-archive28.json) |
| 28 执行 27 与 28 无日志归档 | 两份均成功并产出运行事件，退出码 0。[27](logs/c/v28/archive-run-28-on-27-verified.json) · [28](logs/c/v28/archive-run-28-on-28-verified.json) |

协议日志分别保存了核心进程状态和校验结果退出码，CLI 复核了 27 拒绝 28 原生及 28 接纳
27 原生的进程退出码 2、0。27/28 核心二进制、原生可执行文件、从 COFF 专用节提取的
元数据、实际生成的归档及索引均有 SHA-256 和可复核原件。
[原生拒绝 CLI](logs/c/v28/cli27-native28.json) · [原生接纳 CLI](logs/c/v28/cli28-native27.json) ·
[原件清单](logs/c/v28/windows-artifact-manifest.json)

同一无日志源码、同一缓存根先由 27 构建，再由 28 构建，两次均实际物化模块，28 第三次
命中。27 与 28 归档 SHA-256 分别为
`8e56944540c5044db7f572089294733e2316ab84bb0c031b061049f7eb03849a` 和
`df6333db65d3744d7ff669cb31313296f0ca7e2d1f4aa400fc00120423f7c0df`。
相同的可移植字节码重建后得到相同内容摘要，归档索引及全局缓存记录摘要随版本变化。
原生无日志对照也以切换生成器之后的指纹实跑了重链。
[实际缓存](logs/c/v28/archive-cache.json) · [28 重建](logs/c/v28/compat-archive-28-miss-verified.json) ·
[28 命中](logs/c/v28/archive-28-hit-verified.json)

## C ABI 与原生交付

| 边界 | 实施与观察 |
| --- | --- |
| 符号 | 63 绑定 `AbiBinding::LogWrite`。发布静态库的 `xiao_runtime_*` 定义集合新增一项 `xiao_runtime_log_write`。C 类型为 `int32_t (const XiaoValue *, const XiaoValue *, const XiaoValue *, uint64_t)`。 |
| 所有权 | 按级别、消息、目标借读三个独立槽，调用方持有强引用。LLVM 先按源码顺序求值，再按形参顺序传指针。C ABI 省略输出槽，生成代码在状态分派前逆序释放三个临时输入。 |
| 状态 | 0 成功、1 空指针或槽形状错误、2 未知标签、重叠槽或非法能力掩码、6 可恢复 Runtime 错误、8 Fatal。三字符串入口的 3 无适用场景。探针核验各失败均可读取现有挂起诊断，并核验成功后的空错误槽。 |
| 校验 | C 结构错误在借读、墙钟和输出前拒绝。字符串构造边界严格解码 UTF-8。ABI 探针在构造时拒绝非法字节，日志入口复用 B3-5B 的 002/026/012 顺序及工作量、条数和字节预算。 |
| 权限 | `abi_io` 改用 `missing_from`。日志先检查 clock 再检查 stdout，进程授权和调用掩码取交集。Windows 两优化档实测 `0x00`、`0x02`、`0x20` 都退出 3、stdout 为空，调用区间和标准库帧保留。ABI 专项核对挂起错误 `context.capability`，原生 stderr 沿现有格式输出 026，归档的预检回执列出缺失位名称。 |
| 运行隔离 | 复用 `xiao_runtime_env_run_native`、nonce、截止纳秒和取消文件。应用和任务持有同一个运行的输出及配额状态。运行取消优先于截止，二者沿状态 8 传播并退出 2，独立任务取消用状态 6 和 027。 |
| stdout | Windows 文件和管道直接交付 UTF-8，控制台使用宽字符。Unix 使用文件描述符并在当前线程处理本次写入产生的 SIGPIPE。日志串行写入并逐条刷新，stdout 与诊断 stderr 分开捕获。 |
| 等待和失败 | 饱和管道只轮询写入就绪，在运行取消、截止或十秒期限退出。接纳失败、部分写入失败和刷新失败均报 `LOG_OUTPUT_FAILED`。接纳阶段零记账，开始写入后保留完整编码的尝试额度和已交付字节。 |

独立 C 探针直接链接 Windows release Runtime，正常、缺失 stdout、已断开的管道、
饱和管道四次退出均为 0。正常路径覆盖 4,096/4,097 字节、重叠槽拒绝、借读及释放。
饱和管道返回 `LOG_TIMEOUT`，耗时 10.04 秒，管道中仅有原先的 4,096 字节。
Runtime 专项验证接纳后零写入、短写留下 7 字节、完整写入后的刷新故障、失败次数入账、
运行取消与截止同到优先 001、工作线程独立取消 027，以及阻塞同步调用返回后的十秒复查。
[C 探针](logs/c/probe.c) · [C LLVM IR](logs/c/c-probe-slot-final.ll) · [运行回执](logs/c/c-probe-slot-final-valid.json) ·
[关闭](logs/c/c-probe-slot-final-closed.json) · [断开](logs/c/c-probe-slot-final-broken.json) · [饱和](logs/c/c-probe-slot-final-saturated.json) ·
[探针产物及释放摘要](logs/c/c-probe-artifact.json) · [公开符号差分](logs/c/runtime-export-diff-final.log) ·
[Rust ABI 专项](logs/c/runtime-special-slot-final.stdout.log)

管道就绪轮询受期限控制。已经进入宿主的同步写入、文件或控制台调用需等宿主返回后复查。
外部写者与 stdout 句柄变更也可能影响就绪与实际写入之间的窗口。驱动沿既有规则给
直接子进程 250 ms 协作退出时间，文件捕获保留截取点前的字节。
原生直接交付到宿主流，逐条刷新核对流状态。管道消费和磁盘持久化由宿主管理。
原始捕获保留每个已写字节。驱动现有的字符串视图会将截断的 UTF-8 字符显示为替代字符。
[Runtime 流适配](../../../../../core/rust/crates/xiao-runtime/src/abi_log_stdout.rs) · [运行控制回执](logs/c/log_native_control_reaches_generated_code-slot-final.stdout.log)

## 生成器 27 历史差分

Windows 使用 Rust/Cargo 1.96.0、clang/llvm-as/llc 22.1.2、MSVC link 14.51.36256.0，
目标 `x86_64-pc-windows-msvc`。两侧生成实验链接同一个普通 release Runtime，
SHA-256 为 `0110ac28d86b484ed8a3814878093b49bf7be927ebdf98b32f36e15a57b280d3`。
教程及共享额度对照单独链接启用 `test-failpoints` 的 release Runtime，沿 time 的已有固定墙钟入口。
[工具版本](logs/c/tools-windows.log) · [工具与 Runtime 摘要](logs/c/tool-and-runtime-sha256-final.json)

| 固定版本 27 | O0 | O3 |
| --- | --- | --- |
| 接线前日志 | 探针退出 1，拒绝 `动态 intrinsic os_log_write（58..94）` | 同 O0 |
| 接线后日志 | 生成、汇编、链接、执行均成功，程序退出 0 | 生成、汇编、链接、执行均成功，程序退出 0 |
| 无日志对照 | 前后 IR 逐字一致，程序退出 0 | 前后 IR 逐字一致，程序退出 0 |
| 固定 Runtime 下工具指纹 | `xiao-fnv1a64-f2520a2a4bb35340` | `xiao-fnv1a64-aea102153902cf3f` |
| 固定能力下生成指纹 | `xiao-codegen-27-ea25704cd79ebd6b` | `xiao-codegen-27-32e17518a398d8aa` |

日志候选实际生成 `declare i32 @xiao_runtime_log_write(ptr, ptr, ptr, i64)`，调用尾参为
`i64 34`，同时发射共享运行入口。两档日志 LLVM 文本 SHA-256 均为
`2f5ba4d9e15c53787b1f67e8f90eef9d623a4b44d24f5fccfbc03da0b1bc155c`。
无日志对照两档均为 `983a68da9b5d036488e0c32de9be04e5ac3a373c78846633b886a67a35d72548`。
优化档由实际 clang/link 命令、不同工具和生成指纹及可执行文件身份区分。
[O0 拒绝原文](logs/c/before-log-o0-product-final.stderr.log) · [O3 拒绝原文](logs/c/before-log-o3-product-final.stderr.log) ·
[LLVM 关键行](logs/c/lowering-27.log) · [27 逐档比较](logs/c/generator-diff-27.json) ·
[O0 链接命令](logs/c/after-log-o0-product-final-link.log) · [O3 链接命令](logs/c/after-log-o3-product-final-link.log)

以相同候选文本独立投影元数据偏移 40 的生成器字段，并以 28 为唯一变参复算两类指纹。
投影文本仅改变元数据的一字节。实际新增的 ABI 声明、调用、启动入口和日志组件在 27 下已观察到，
该结果是本次 27→28 裁定的输入。
[只换版本的元数据差分](logs/c/version-28-only.diff) · [指纹输入与取证工具](logs/c/generator-probe.rs)

### 生成器 27 的缓存身份

- 无日志对照用 B3-5B 的实际 Runtime 与本批 Runtime 各重建一次。ABI 1.20、生成器 27、
  目标、标准库接口、能力掩码和 IR 摘要一致，Runtime 内容与工具指纹失配。
  同一日志 IR 链接旧 Runtime 得到 `xiao_runtime_log_write` 未解析，使用本批 Runtime 重建后退出 0。
  能力从 `0x22` 改成 `0x02` 时生成指纹和 IR 改变，重链的产物实际以 026 退出 3。
  Windows 原生构建入口执行真实重编译和重链，证据清单记录参与对照的静态身份。
  [缓存身份](logs/c/cache-identity.json) · [旧 Runtime 真实链接失败](logs/c/cache-old-runtime-unresolved.stdout.log) ·
  [新能力规格下的退出](logs/c/cache-changed-mask-log-o0-final.json)
- 归档依赖闭包只含 `stdlib.log`，源程序只有一次 `write`。普通应用能力为 `0`，同一缓存根
  切换到日志应用时失配并重新生成，重复日志构建命中已验对象。归档精确声明 `0x22`，
  两种单缺位均在执行前以 026 拒绝、退出 2。完整掩码按固定墙钟输出一条记录。
  [归档能力及实际缓存](logs/c/log_only_archive_requires_both_bits-slot-final.stdout.log)

## 生成器 28 功能复跑

| 专项 | 实际结果 |
| --- | --- |
| 独立 C 探针 | Windows 正常、关闭、断开和饱和 4 次通过，Linux WSL 正常、关闭和断开 3 次通过。Windows 饱和管道返回 `LOG_TIMEOUT`，原有 4,096 字节完整保留。[Windows 正常](logs/c/v28/c-probe-valid.json) · [关闭](logs/c/v28/c-probe-closed.json) · [断开](logs/c/v28/c-probe-broken.json) · [饱和](logs/c/v28/c-probe-saturated.json) · [Linux WSL](logs/c/v28/linux-probe-isolated.json) |
| 原生教程 | 精确筛选 1 项通过。两个导入各跑 O0/O3，四份 stdout 均与 VM 和 103 字节黄金逐字一致，退出 0，stderr 为零。[教程运行](logs/c/v28/log_tutorial_matches_native_golden.stdout.log) · [链接](logs/c/v28/log_tutorial_matches_native_golden-link.log) |
| 双位授权及归档 | 两项精确测试通过。原生两档各三种缺位，均报 026、退出 3、零 stdout，已核对释放。仅日志归档精确记录 `0x22`，两种单缺位均在入口前以 026 拒绝，退出 2。[原生](logs/c/v28/log_native_enforces_each_bit_and_releases_inputs.stdout.log) · [归档](logs/c/v28/log_only_archive_requires_both_bits.stdout.log) · [缺 stdout](logs/c/v28/archive-log-28-mask-2-verified.json) · [缺 clock](logs/c/v28/archive-log-28-mask-32-verified.json) |
| 共享配额 | 精确筛选 1 项通过。O0/O3 每档各启动两次，每次 1,024 条共 35,842 字节，与 VM 逐字一致，随后触发 `LOG_LIMIT_EXCEEDED`。[应用和任务](logs/c/v28/log_native_tasks_share_run_quota.stdout.log) |
| 运行控制及故障 | 生成程序 001、002 专项 1 项，Runtime ABI 5 项通过，覆盖取消优先、任务 027、十秒期限及接纳、短写和刷新故障。[进程](logs/c/v28/log_native_control_reaches_generated_code.stdout.log) · [Runtime ABI](logs/c/v28/runtime-special.stdout.log) |
| 编号及 VM | 五个旧负例逐项运行并命中 64，另跑 100 墓碑守卫，共 6 项通过。VM 日志回归 7 项通过。[字节码](logs/c/v28/id-64-100-bytecode.stdout.log) · [TAC](logs/c/v28/id-64-100-cmath-tac.stdout.log) · [ABI 合约](logs/c/v28/id-64-100-divide.stdout.log) · [VM 编号](logs/c/v28/id-64-cmath-vm.stdout.log) · [归档入口](logs/c/v28/id-64-time.stdout.log) · [墓碑](logs/c/v28/id-100-tombstone.stdout.log) · [VM 日志](logs/c/v28/log-vm-regression.stdout.log) |

Windows 档案收录并逐件重算 208 件原件，包含两侧校验器、O0/O3 可执行文件、IR、map、
真实元数据、归档索引、`verification.json` 和原始输出。ZIP SHA-256 为
`8f24f64a85e05a468c59fbd8585d69f6a43671a38f81d5e2ef7426a5ab2321d6`。
本机 Windows 生成物的声明与观察组件集合一致，Linux WSL 本次覆盖发布 Runtime 和 C ABI。
macOS 无实测。
[Windows 档案](logs/c/v28/windows-artifacts.zip) · [逐件清单](logs/c/v28/windows-artifact-manifest.json) ·
[归档复验](logs/c/v28/pack-artifacts.stdout.log) · [Linux 输出](logs/c/v28/linux-probe-isolated.stdout.log)

### 28 工作区门禁

| 验证 | 结果 |
| --- | --- |
| `cargo test --locked --workspace` | 退出 0。199 组，1,855 项通过，69 项忽略，26 项筛选排除，失败 0。沿用测试档优化级别 1，关闭调试信息。归档索引、归档及 LLVM 专项另跑 110 项通过。[命令与退出](logs/c/v28/workspace-test.json) · [完整计数](logs/c/v28/workspace-test.stdout.log) · [版本专项](logs/c/v28/version-guard.stdout.log) |
| Clippy、格式 | `cargo clippy --workspace --all-targets -- -D warnings` 和 `cargo fmt --check` 均退出 0。[Clippy](logs/c/v28/workspace-clippy.json) · [格式](logs/c/v28/workspace-fmt.json) |
| docs、layout、coverage | 均退出 0。总体 12,511/13,881（90.13%），公共 API 6,266/6,266（100%）。[docs](logs/c/v28/gate-docs-verified.stdout.log) · [layout](logs/c/v28/gate-layout.json) · [coverage](logs/c/v28/gate-coverage.stdout.log) |
| 工作区边界 | `git diff --check`、受保护路径和暂存区核对退出 0。升版涉及代码生成器、归档索引、归档构建与预检、锁文件的内部依赖和对应回归。HEAD 保持入口，所有变更留在工作区。[差异检查](logs/c/v28/git-diff-check-verified.json) · [工作区](logs/c/v28/git-status.stdout.log) · [源码核对](logs/c/v28/source-audit.json) · [基准锁校验](logs/c/v28/benchmarks-lock.json) |

## 生成器 27 的功能验证与产物

| 专项 | 实际验证 | 原始回执 |
| --- | --- | --- |
| C 探针 | Windows 4 次成功，Linux WSL 3 次成功 | [Windows](logs/c/c-probe-slot-final-valid.json) · [Linux](logs/c/linux-wsl-c-slot-final.json) |
| Runtime ABI | 5 项通过，含实际十秒等待 | [运行日志](logs/c/runtime-special-slot-final.stdout.log) |
| O0/O3 教程 | 精确筛选 1 项通过。两个导入各跑两档，四份 stdout 均为 103 字节，与 VM 和独立黄金逐字相等。进程均退出 0、stderr 均为零，释放轨迹各含 10 个对象且各销毁一次。 | [原生](logs/c/log_tutorial_matches_native_golden-slot-final.stdout.log) · [VM 与 B3-5B 回归](logs/c/vm-non-regression-slot-final.stdout.log) |
| 原生能力 | 精确筛选 1 项通过，三种缺位乘 O0/O3 共六次，均退出 3，三个输入均释放。 | [运行回执](logs/c/log_native_enforces_each_bit_and_releases_inputs-slot-final.stdout.log) |
| 应用和任务配额 | 精确筛选 1 项通过。两档各启动两次，每次保留 1,024 条共 35,842 字节，与 VM 相等，下一次交付报 `LOG_LIMIT_EXCEEDED`。 | [运行回执](logs/c/log_native_tasks_share_run_quota-slot-final.stdout.log) |
| 归档能力 | 精确筛选 1 项通过，覆盖普通应用隔离、实际缓存失配重建和同源命中。 | [运行回执](logs/c/log_only_archive_requires_both_bits-slot-final.stdout.log) |
| 原生运行控制 | 精确筛选 1 项通过，O0/O3 均观察到进程内 Fatal 001、002。分别经原生驱动和构建后启动入口保留已捕获 stdout。 | [运行回执](logs/c/log_native_control_reaches_generated_code-slot-final.stdout.log) |

Windows 可执行文件、原始 LLVM IR、MSVC map、`verification.json`、stdout/stderr
和释放轨迹随 ZIP 保存。清单逐件记录原路径、压缩包成员、大小和 SHA-256，
从 ZIP 重新读取了全部 185 个成员复算摘要，其中含 25 个可执行文件、25 份 map 和
22 份验证清单。ZIP 的 SHA-256 为
`01620fcce1be515483b7a4deb8a31719bf262f0529ba2a5bf56e7af8a17a1327`。
生成实验 O0/O3 的二进制 SHA-256 分别为
`78fd1d0162a2a840da89573638a07666ad327739e09b6ccf56c5a2fa6bb82643`、
`28738169cb672b64c69212523f2da6d019596342a44a35be61ae290524e413f2`。
发布档链接使用 map，链接选项未请求 PDB。产品验证器在本机观察到声明组件等于观察组件，
未分类 Runtime 符号为空。Windows 产品级 COFF 收口和三平台正式矩阵归 B3-5D/E。
[原件档案](logs/c/windows-artifacts.zip) · [逐件清单](logs/c/windows-artifact-manifest.json) ·
[教程链接](logs/c/log_tutorial_matches_native_golden-slot-final-link.log) ·
[能力链接](logs/c/log_native_enforces_each_bit_and_releases_inputs-slot-final-link.log)

## 生成器 27 的工作区门禁

| 验证 | 结果与原始回执 |
| --- | --- |
| `cargo test --locked --workspace` | 退出 0，199 组，1,854 项通过，其中 4 项是文档测试。69 项忽略，26 项由目标内的筛选排除。测试档优化级别为 1，调试信息关闭。工作区默认忽略的五项本批日志专项均已显式通过。[命令与退出](logs/c/workspace-test-final.json) · [测试日志](logs/c/workspace-test-final.stdout.log) |
| env 原生控制兼容 | 以本批发布 Runtime 复跑三个 env 原生控制用例，3 项通过。[回执](logs/c/env-control-non-regression.stdout.log) |
| `cargo clippy --workspace --all-targets -- -D warnings`、`cargo fmt --check` | 均退出 0。[Clippy](logs/c/workspace-clippy-final.json) · [格式](logs/c/fmt-final.json) |
| docs、layout、coverage | 均退出 0。总体文档覆盖 12,508/13,878（90.13%），公共 API 6,264/6,264（100%），保留 `A0-COVERAGE-002` 警告。[文档](logs/c/gate-docs-final.json) · [目录](logs/c/gate-layout-final.json) · [覆盖](logs/c/gate-coverage-final.stdout.log) |
| `git diff --check`、受保护路径和暂存区核对 | 均退出 0。HEAD 与入口相同，暂存区为空。[差异检查](logs/c/git-diff-check-final.json) · [保护路径](logs/c/git-protected-files.json) · [暂存区](logs/c/git-stage-audit-final.json) |

## 首败与复跑

- CI 首跑的三平台原生作业把日志教程和共享配额用例链接到发布档 Runtime，固定时钟未生效，
  教程黄金与 VM 比较失败。工作流改为额外构建独立的 `test-failpoints` Runtime 并设置
  `XIAO_FIXED_RUNTIME_LIBRARY`；本地教程与配额两项在固定 Runtime 上复跑通过，CI 待重跑。
- 28 的兼容矩阵首轮使用了带连字符的源文件名。归档构建按模块命名规则退出 101。
  使用固定的 `main.xiao` 路径和相同源码字节重试，两个版本均成功构建。
  取证脚本随后修正了协议的退出码字段位置和必需运行选项。CLI 首轮漏传 `-xar`，
  正式命令复跑退 2，原生校验与归档矩阵均通过。
  [文件名](logs/c/v28/archive-27-miss.stderr.log) · [协议字段](logs/c/v28/compatibility-matrix-retry.stderr.log) ·
  [归档运行选项](logs/c/v28/compatibility-matrix-final.stderr.log) · [CLI 首轮](logs/c/v28/cli-core27-rejects-archive28.json) ·
  [最终矩阵](logs/c/v28/compatibility-matrix-verified.stdout.log)
- 文档门禁首轮发现回执链接指向命令完成后才写出的 JSON。改用门禁启动时已经创建的原始输出文件后通过。
  [首轮](logs/c/v28/gate-docs.stdout.log) · [复跑](logs/c/v28/gate-docs-verified.stdout.log)
- 基线首轮 Git 归档受本机自动换行配置影响，两份锁摘要不符。以 LF 参数重取固定提交后两份逐字一致。
  [首轮归档摘要](logs/c/archive-crlf-first.log) · [最终摘要](logs/c/locks.log)
- 编译首轮缺少 `abi_log` 的显式模块路径。Runtime 单测另有一处旧构造漏填日志状态。
  修正后 C 探针和 Runtime 专项通过。冻结表复核后补上重叠槽的状态 2 验证，
  用最终 Runtime 重跑 C 探针、生成差分及原生专项。
  [模块路径](logs/c/build-candidate-first.log) · [构造漏项](logs/c/runtime-special-build.log) ·
  [最终 Runtime 构建](logs/c/runtime-release-final.json)
- 取证探针首轮误用目标返回类型。接线首轮未向模块合并日志函数的 C 声明，llvm-as 拒绝。
  合并声明并核对 8 的传播后两档成功。
  [探针编译](logs/c/generator-probe-build-after.log) · [首次 LLVM 拒绝](logs/c/after-log-o0.stderr.log) ·
  [最终 O0](logs/c/after-log-o0-product-final.json) · [最终 O3](logs/c/after-log-o3-product-final.json)
- 原生能力断言首轮把缺失位名称当作 stderr 字段。后续按 ABI 挂起上下文、归档预检和原生退出码分别核对。
  按单项筛选首轮出现参数展开把 `--ignored` 拆成字符，以及归档测试取非输出事件的 `text` 字段报错。
  显式传递完整参数并按存在的输出字段取值后，所有精确入口各命中一项并通过。
  [能力首轮](logs/c/native-special-first.stderr.log) · [参数原文](logs/c/log_native_enforces_each_bit_and_releases_inputs.json) ·
  [归档首轮](logs/c/log_only_archive_requires_both_bits.stderr.log)
- 目录门禁首轮的 `artifact.rs` 为 2,503 行。time 组件分类改从冻结符号映射取值，保留原有集合。
  [目录首轮](logs/c/gate-layout.stdout.log) · [最终门禁](logs/c/gate-layout-final.json)
- Linux C 探针首轮的临时 Runtime 路径无法跨 WSL 调用访问，第二轮挂载盘创建目录被拒绝。
  在 Linux 用户缓存目录中完成发布构建、直接链接及三个进程用例。WSL 启动警告保留于原始记录。
  [第一轮](logs/c/linux-c-probe-build.stderr.log) · [第二轮](logs/c/linux-wsl-c-verified.stderr.log) ·
  [最终进程与摘要](logs/c/linux-wsl-c-slot-final.stdout.log)

## B3-5D 前置条件

以 28 固定产品身份。使用冻结黄金迁入按计划命名的教程夹具。
Windows 按 D 批矩阵绑定最终四份教程和四份单缺位产物至真实 COFF 证据。
Linux WSL 本批覆盖了普通 release Runtime、C ABI、缺失 stdout 与真实断开管道。
macOS 本批缺实测，三平台的正式教程、产品验收和性能回执独立进 D/E。教程和 registry 状态为
`planned`，工作区为未提交状态。
