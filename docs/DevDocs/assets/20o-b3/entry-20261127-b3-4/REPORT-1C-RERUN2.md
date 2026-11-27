# B3-4D FIX-B 本地复跑

Windows 与 Ubuntu WSL 的 VM 双导入和原生双导入 O0/O3 均逐字匹配 55 字节黄金，Windows 六份新构建的 PE/COFF 产物均获产品级 `Complete`。受测提交为 `1e42f501af20890157a4ce28d96d451a70fc3e39`，Git tree 为 `7631e0692c5734b34f8b2f0b31a1bca7f971c223`。macOS 标为 `unverifiable`。[Windows 复核](logs/1c-rerun2/windows/audit.json) · [Ubuntu WSL 复核](logs/1c-rerun2/ubuntu-wsl/audit.json)

## 源码、Runtime 与工具链

从固定 worktree 导出该提交，使用 Git 原始字节，在 Windows 和 WSL 各自的文件系统上解包。逐个对账 10,705 个跟踪文件的 Git blob，核对两份 Cargo 锁文件、教程夹具和测试入口。两端各用独立的 Cargo target、原生任务目录和 Xiao 缓存。运行结束后两份源码副本的 `source_unchanged` 均为 true，固定 worktree 的跟踪文件原始字节总摘要前后一致，Git 状态只列本批新增证据。[导出与源码索引](logs/1c-rerun2/source-archive.json) · [工作树前](logs/1c-rerun2/worktree-before.json) · [工作树后](logs/1c-rerun2/worktree-after.json)

| 本地平台与目标 | 固定工具链 | 发布 Runtime 静态库 SHA-256 |
| --- | --- | --- |
| Windows 11 10.0.26200，`x86_64-pc-windows-msvc` | Rust/Cargo 1.96.0、MSYS2 UCRT64 LLVM 22.1.2、MSVC 链接器 14.51.36256.0、Windows SDK 10.0.26100.0 [工具回执](logs/1c-rerun2/windows/toolchain.json) | `1a0363e466d77d320c379029ad306b420a493acbbf34ac08e555bbabf03dd93a` [构建回执](logs/1c-rerun2/windows/runtime.json) |
| Ubuntu 26.04 LTS / WSL2 6.6.114.1，`x86_64-unknown-linux-gnu` | Rust/Cargo 1.96.0、Ubuntu LLVM 21.1.8、GNU ld 2.46 [工具回执](logs/1c-rerun2/ubuntu-wsl/toolchain.json) | `e1bb44e9142353c35217e608c1fbd49b69d137b0b9a53becb02d20b3e5d70eaa` [构建回执](logs/1c-rerun2/ubuntu-wsl/runtime.json) |

原生静态库以 release 配置从 FIX-B 重建，Cargo 事件记录优化级别 3、调试断言关闭、Runtime 特性列表为空。产品回执记录 Runtime ABI 1.20、生成器 27、标准库接口 1。源码复核在两端构建 Runtime 后记录摘要，末次复核所得摘要一致。[Windows 构建后](logs/1c-rerun2/windows/source-ready.json) · [Windows 结束](logs/1c-rerun2/windows/source-after.json) · [WSL 构建后](logs/1c-rerun2/ubuntu-wsl/source-ready.json) · [WSL 结束](logs/1c-rerun2/ubuntu-wsl/source-after.json)

两份 Cargo 锁文件的 SHA-256 分别为 core/rust/Cargo.lock `28d38451ba0490b70d2d136bc3268acad8de02b5658bf826f842069019161c53` 与 tests/benchmarks/Cargo.lock `8ac4f16af6e8baf5b71e0e0b5d89397eb5ae7bf2e3cc5a96053683c6e8b4b577`。夹具来源提交为 `7f1017b7ff0b7b0a89d11943d5d33e0b432d0b3f`。下表按原始字节计算 SHA-256，黄金以 LF 结尾。[冻结夹具表](../entry-20261124-b3-0e/tutorials.json)

| 冻结输入 | 字节 | SHA-256 |
| --- | ---: | --- |
| tutorial.xiao | 467 | 9ab09cf88eb5e4c16bc08a712f25e14739fee027074e014c8ff0e2ac7beedeeb |
| tutorial.input.json | 69 | f5233cf1df7c282e2e490cb092b835898882477c2cffe4c084ec194bac780522 |
| tutorial.stdout | 55 | 2420b27b0eac750657ba5909a8928fdb74674104601c62ae2ef6d273f2ae5491 |
| 首行改用 `stdlib.env` 的源码 | 470 | a8cb3149daec0193db2de4b6d42752c38812965de8e624f4a106c879de5c6e1d |

## 教程

两端分别执行 `env_tutorial_matches_vm_golden` 和 `env_tutorial_matches_native_golden`。每个入口均为 1 通过、0 失败、0 忽略。`std.env` 与 `stdlib.env` 均加载 `stdlib.env`。VM 双导入和原生双导入各自 O0/O3 都用能力 `0xA0`、冻结合成快照、退出 0。四个原生进程的 stderr 各为空。归档保留原生原始字节，VM 输出由测试记录的输出事件还原并再次核对。[Windows VM](logs/1c-rerun2/windows/tutorial-vm.json) · [Windows 原生](logs/1c-rerun2/windows/native-matrix.json) · [WSL VM](logs/1c-rerun2/ubuntu-wsl/tutorial-vm.json) · [WSL 原生](logs/1c-rerun2/ubuntu-wsl/native-matrix.json)

精确集成测试的 release VM 依 Cargo 的开发依赖启用了 `test-failpoints`。另在两端分别构建 release `xiao-core`，经实际标准输入输出协议重跑相同双导入。Cargo 编译事件绑定特性为空的 Runtime rlib，两个请求及其进程在每个平台均退出 0，输出各 55 字节且逐字相同。发布 VM 回执含原始协议帧、独立任务目录和 Runtime 摘要。[Windows 发布 VM](logs/1c-rerun2/windows/vm-product.json) · [WSL 发布 VM](logs/1c-rerun2/ubuntu-wsl/vm-product.json)

## Windows 六份 COFF

六份均由产品构建入口调用 `verify_artifact_with_link_evidence` 得到 COFF `Complete`。声明与观察组件均为 env、rc、value、weak，未分类 Runtime 符号为空。每份回执绑定本批提交、tree、Runtime SHA、二进制和 map 原件、stdout/stderr、产品验证行及 Clang 和 MSVC 的实际命令行。归档后逐件重算摘要，六份二进制和六份 map 的摘要各自互异。 [归档矩阵](logs/1c-rerun2/windows/native-matrix.json) · [教程链接轨迹](logs/1c-rerun2/windows/tutorial-native-clang.log) · [权限链接轨迹](logs/1c-rerun2/windows/env-native-clang.log)

| 用例 | 能力 | 产品验证 | 进程退出 | 诊断 | 逐件回执 |
| --- | --- | --- | ---: | --- | --- |
| `std.env` O0 | `0xA0` | COFF `Complete` | 0 | 无 | [回执](logs/1c-rerun2/windows/native/env-std-o0/receipt.json) |
| `std.env` O3 | `0xA0` | COFF `Complete` | 0 | 无 | [回执](logs/1c-rerun2/windows/native/env-std-o3/receipt.json) |
| `stdlib.env` O0 | `0xA0` | COFF `Complete` | 0 | 无 | [回执](logs/1c-rerun2/windows/native/env-stdlib-o0/receipt.json) |
| `stdlib.env` O3 | `0xA0` | COFF `Complete` | 0 | 无 | [回执](logs/1c-rerun2/windows/native/env-stdlib-o3/receipt.json) |
| 权限负例 O0 | `0x20` | COFF `Complete` | 3 | `X06-RUNTIME-026` | [回执](logs/1c-rerun2/windows/native/denied-o0/receipt.json) |
| 权限负例 O3 | `0x20` | COFF `Complete` | 3 | `X06-RUNTIME-026` | [回执](logs/1c-rerun2/windows/native/denied-o3/receipt.json) |

## 权限、快照与启动握手

- 两端 O0/O3 权限负例在运行期均报 `X06-RUNTIME-026`、退出 3、stdout 为空。Ubuntu WSL 的对应两件 ELF 产物也获 `Complete`。[Windows 原生矩阵](logs/1c-rerun2/windows/native-matrix.json) · [WSL 原生矩阵](logs/1c-rerun2/ubuntu-wsl/native-matrix.json)
- 两端各对 `std.env` O0/O3 做 26 次执行前快照复验。重复键触发 `X09-DRIVER-006` 两次，超限定额触发 `X09-DRIVER-007` 八次，均退出 2 且 stdout 为空。其余 16 次退出 0 并逐字匹配黄金。覆盖条目数、UTF-8 名字与值的字节数和 1 MiB 累计边界。原生专项还覆盖 FIX-B 的 6,299,650 字节原始载荷上限及上限加一字节。[Windows 边界](logs/1c-rerun2/windows/snapshot-boundaries.json) · [WSL 边界](logs/1c-rerun2/ubuntu-wsl/snapshot-boundaries.json)
- 原生专项通过驱动的随机 nonce 握手与不匹配参数检查。两端各以系统随机源生成新 128 位 nonce，直接复验四份教程和边界。旧标记 `--__xiao-run-environment-stdin` 在直接启动时保留为首个用户参数，合成宿主值为 `host-native`。启用新握手后，旧标记与 tail 仍保留，读取值变为 `managed-native`。两种启动都退出 0。每端 32 个复验 nonce 各不重复。[Windows 握手](logs/1c-rerun2/windows/nonce.json) · [WSL 握手](logs/1c-rerun2/ubuntu-wsl/nonce.json)
- FIX-B 原生运行控制每端留存 20 个中断样本。取消 `X09-DRIVER-001` 和截止 `X09-DRIVER-002` 各 10 个，退出均为 2。专项包含 O0/O3 执行前、运行中及 set/unset 提交前旧值和配额检查。独立任务取消仍为 `X06-RUNTIME-027`、退出 3。[Windows 控制](logs/1c-rerun2/windows/run-control.json) · [WSL 控制](logs/1c-rerun2/ubuntu-wsl/run-control.json)

## 命令与退出码

命令回执含实际参数、工作目录与 stdout/stderr 原始文件及摘要。两端使用 Rust 1.96.0、锁定依赖、显式宿主目标执行。

| 命令或入口 | Windows | Ubuntu WSL |
| --- | --- | --- |
| cargo build -p xiao-runtime --release | 0 [命令](logs/1c-rerun2/windows/commands/runtime-build.json) | 0 [命令](logs/1c-rerun2/ubuntu-wsl/commands/runtime-build.json) |
| `env_tutorial_matches_vm_golden` --exact | 0，1 项 [命令](logs/1c-rerun2/windows/commands/tutorial-vm.json) | 0，1 项 [命令](logs/1c-rerun2/ubuntu-wsl/commands/tutorial-vm.json) |
| `env_tutorial_matches_native_golden` --exact --ignored | 0，1 项 [命令](logs/1c-rerun2/windows/commands/tutorial-native.json) | 0，1 项 [命令](logs/1c-rerun2/ubuntu-wsl/commands/tutorial-native.json) |
| cargo build -p xiao-driver --release --bin `xiao-core` 及双导入 | 构建、两个进程、两个请求均为 0 [回执](logs/1c-rerun2/windows/vm-product.json) | 构建、两个进程、两个请求均为 0 [回执](logs/1c-rerun2/ubuntu-wsl/vm-product.json) |
| 驱动 env 常规、原生专项 | 0、0，7 项和 8 项 [常规](logs/1c-rerun2/windows/commands/env.json) · [原生](logs/1c-rerun2/windows/commands/env-native.json) | 0、0，7 项和 8 项 [常规](logs/1c-rerun2/ubuntu-wsl/commands/env.json) · [原生](logs/1c-rerun2/ubuntu-wsl/commands/env-native.json) |
| Runtime env 专项 | 0，19 项 [命令](logs/1c-rerun2/windows/commands/runtime-env.json) | 0，19 项 [命令](logs/1c-rerun2/ubuntu-wsl/commands/runtime-env.json) |
| COFF 验证器专项 | 0，4 项 [命令](logs/1c-rerun2/windows/commands/coff-unit.json) | 平台不适用 |
| 现有 C ABI 探针 | 编译与运行均为 0 [回执](logs/1c-rerun2/windows/c-abi.json) | 编译与运行均为 0 [回执](logs/1c-rerun2/ubuntu-wsl/c-abi.json) |
| cargo test --workspace | 0，197 组、1,826 通过、0 失败、64 忽略 [命令](logs/1c-rerun2/windows/commands/workspace.json) | 0，196 组、1,826 通过、0 失败、66 忽略 [命令](logs/1c-rerun2/ubuntu-wsl/commands/workspace.json) |
| cargo fmt --all -- --check | 0 [命令](logs/1c-rerun2/windows/commands/fmt.json) | 0 [命令](logs/1c-rerun2/ubuntu-wsl/commands/fmt.json) |

## 失败、重试与平台缺项

- 预检 Git tree 参数被 PowerShell 拆分，退出 128。为参数整体加引号后退出 0，得到固定 commit 和 tree。[首次预检](logs/1c-rerun2/preflight-first.json) · [重试](logs/1c-rerun2/commands/worktree-head-before.json)
- 两端证据准备脚本首次都退出 1，读取 C 探针时漏写 support 目录。修正后沿用已解包源码，从全量 blob 核对继续运行。[Windows 首次](logs/1c-rerun2/windows-run.stderr.log) · [Windows 重试](logs/1c-rerun2/windows-retry.exit) · [WSL 首次](logs/1c-rerun2/wsl-run.stderr.log) · [WSL 重试](logs/1c-rerun2/wsl-retry.exit)
- WSL 首次解析旧标记日志时把测试行前缀算进路径，证据脚本退出 1。修正解析后，在独立任务目录复跑握手和边界，退出 0。[首次](logs/1c-rerun2/wsl-capture.stderr.log) · [重试](logs/1c-rerun2/wsl-capture-retry.stdout.log)
- 发布 VM 证据脚本两端均为 1 → 1 → 1 → 0。两端的发布构建均退出 0。首次脚本误将单包 Runtime 静态库与 `xiao-core` 依赖图产出的静态库要求为同一摘要，修正后绑定实际编译产物。第二次子进程的合成环境缺宿主用户目录，协议请求报 `X11-PROTOCOL-002` / `X05-CACHE-001`、退出 2。第三次调用把重试标签错作参数，日志独占写入保护终止脚本。最终给子进程提供独立用户目录，以正确参数复跑，两端双导入均退出 0。[Windows 与 WSL 尝试](logs/1c-rerun2/launches.json)
- MSVC 的 `link /?` 版本探测返回 1100，输出给出链接器版本。实际六次链接与产品验证均成功。[链接器探测](logs/1c-rerun2/windows/commands/tool-link.json)
- 本机缺 macOS 运行环境，`aarch64-apple-darwin` 的教程及原生矩阵为 `unverifiable`。本批为本地证据，CI 作业回执缺项。[平台记录](logs/1c-rerun2/platforms.json)
