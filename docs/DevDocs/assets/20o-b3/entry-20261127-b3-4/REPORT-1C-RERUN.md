# B3-4D 修复后本地复跑

受测产品为 `00766ca38c478608ee6d979824236f77cd74c00e`，Git 树为
`b73b72f6d0d90ca43a8dd37a133842a9eeb95d6f`。Windows 和 Ubuntu WSL 的 VM、原生
O0/O3 教程均逐字匹配 55 字节黄金。Windows 六份新产物均获产品级 COFF `Complete`。
本批定义的 B3-4D 本地出口已满足，macOS 标为 `unverifiable`。
[源码回执](logs/1c-rerun/source-identity.json) · [Windows 产物矩阵](logs/1c-rerun/native-matrix.json) · [Ubuntu 回执](logs/1c-rerun/linux-ubuntu/result.json)

## 源码与工具链

从固定提交单独导出源码，逐个核对 7,884 个跟踪文件的 Git blob。叠加工作区已有的教程
用例、共享原生测试辅助文件和 env 夹具。两套运行使用同一份源码，分别构建 release Runtime，
使用独立的 target、缓存和原生输出目录。运行前后已复核源码、夹具和 Runtime。
[导出命令](logs/1c-rerun/source-export-retry.json) · [运行后核对](logs/1c-rerun/source-after.json)

| 平台与目标 | 工具链 | 本批 release Runtime SHA-256 |
| --- | --- | --- |
| Windows 11 10.0.26200，`x86_64-pc-windows-msvc` | Rust/Cargo 1.96.0、MSYS2 UCRT64 LLVM 22.1.2、MSVC 链接器 14.51.36256.0、Windows SDK 10.0.26100.0 | `fe42ae1385a46d65d3e6882b9120ade253033fdf17c45101b57e812f7b815089` [构建回执](logs/1c-rerun/runtime.json) |
| Ubuntu 26.04 LTS WSL2，`x86_64-unknown-linux-gnu` | Rust/Cargo 1.96.0、Ubuntu LLVM 21.1.8、GNU ld 2.46 | `12308d5fd6e5b54e1c6a95f4b78bf18c9b9df777e04e5c4c01babe84b25640b1` [构建回执](logs/1c-rerun/linux-ubuntu/result.json) |

Windows 原生产物的 Runtime ABI 为 1.20，生成器为 27，标准库接口为 1。工具回执记录
二进制摘要和版本。两份 Cargo 锁文件以 Git 原始字节参与构建，SHA-256 见源码回执。
[Windows 工具链](logs/1c-rerun/toolchain.json) · [Windows 主机与链接器](logs/1c-rerun/tool-host-msvc.log) · [源码回执](logs/1c-rerun/source-identity.json)

## 教程

`std.env` 和 `stdlib.env` 均初始化 `stdlib.env`。Windows 和 Ubuntu 各用 release
配置跑一次精确 VM 入口，另跑一次精确原生入口。两项均为 1 通过、0 失败、0 忽略。
每个平台的 VM 覆盖双导入，原生覆盖双导入各自 O0/O3。教程用 `0xA0` 和冻结的合成环境，
所有路径退出 0，按字节核对整个 UTF-8/LF 输出及末尾 LF，原生 stderr 为空。
[Windows VM](logs/1c-rerun/tutorial-vm.log) · [Windows 原生](logs/1c-rerun/tutorial-native.log) · [Ubuntu VM](logs/1c-rerun/linux-ubuntu/tutorial-vm.log) · [Ubuntu 原生](logs/1c-rerun/linux-ubuntu/tutorial-native.log)

| 冻结输入 | 字节 | SHA-256 |
| --- | ---: | --- |
| `tutorial.xiao` | 467 | `9ab09cf88eb5e4c16bc08a712f25e14739fee027074e014c8ff0e2ac7beedeeb` |
| `tutorial.input.json` | 69 | `f5233cf1df7c282e2e490cb092b835898882477c2cffe4c084ec194bac780522` |
| `tutorial.stdout` | 55 | `2420b27b0eac750657ba5909a8928fdb74674104601c62ae2ef6d273f2ae5491` |
| 首条导入替换为 `stdlib.env` 的源码 | 470 | `a8cb3149daec0193db2de4b6d42752c38812965de8e624f4a106c879de5c6e1d` |

夹具原始来源为 `7f1017b7ff0b7b0a89d11943d5d33e0b432d0b3f`，源码、输入、黄金与
别名变体均匹配 B3-0E 冻结摘要。 [冻结表](../entry-20261124-b3-0e/tutorials.json)

## Windows 六份 COFF

产品构建入口在各自链接后调用 `verify_artifact_with_link_evidence`。声明与观察组件
均为 `env`、`rc`、`value`、`weak`，未分类 Runtime 符号为空。逐件回执绑定本次提交、
Runtime、产物、map、原始 stdout 和 stderr、产品验证 JSON，以及对应的 clang 编译和
MSVC 链接命令行。六份产物和六份 map 的 SHA-256 分别唯一，且与产品回执一致。
[矩阵索引与摘要](logs/1c-rerun/native-matrix.json) · [教程链接轨迹](logs/1c-rerun/tutorial-native-clang.log) · [权限链接轨迹](logs/1c-rerun/env-native-clang.log)

| 用例 | 能力 | 产品验证 | 退出 | 诊断 | 逐件回执 |
| --- | --- | --- | ---: | --- | --- |
| `std.env` O0 | `0xA0` | COFF `Complete` | 0 | 无 | [回执](logs/1c-rerun/native/env-std-o0/receipt.json) |
| `std.env` O3 | `0xA0` | COFF `Complete` | 0 | 无 | [回执](logs/1c-rerun/native/env-std-o3/receipt.json) |
| `stdlib.env` O0 | `0xA0` | COFF `Complete` | 0 | 无 | [回执](logs/1c-rerun/native/env-stdlib-o0/receipt.json) |
| `stdlib.env` O3 | `0xA0` | COFF `Complete` | 0 | 无 | [回执](logs/1c-rerun/native/env-stdlib-o3/receipt.json) |
| 权限负例 O0 | `0x20` | COFF `Complete` | 3 | `X06-RUNTIME-026` | [回执](logs/1c-rerun/native/denied-o0/receipt.json) |
| 权限负例 O3 | `0x20` | COFF `Complete` | 3 | `X06-RUNTIME-026` | [回执](logs/1c-rerun/native/denied-o3/receipt.json) |

## 启动握手与错误映射

现有原生专项覆盖驱动用系统随机数发起握手、旧标记作普通首参、随机数不匹配及快照
接纳。另从系统随机源为每次直接复验生成新的 128 位 nonce，以归档的四份教程产物
再次逐字核对黄金。复用专项生成的 argv 探针直接运行两次，旧的
`--__xiao-run-environment-stdin` 和 `tail` 都完整保留。直接启动读取合成宿主值
`host-native`，匹配新 nonce 的启动读取注入值 `managed-native`，两次都退出 0。
[驱动专项](logs/1c-rerun/env-native.log) · [随机数复验](logs/1c-rerun/nonce.json)

权限负例在运行期报 `X06-RUNTIME-026`、退出 3。用本批 `std.env` O0/O3 原件经新
nonce 启动，执行前重复名称报 `X09-DRIVER-006`、环境快照超限报 `X09-DRIVER-007`，
均退出 2 且 stdout 为空。26 个启动样本包含 2 个重复名称、8 个超限、16 个成功。
成功样本均退出 0 并匹配黄金。四组边界分别按条目数、UTF-8 名称字节、值字节和累计字节核对。
[快照边界回执](logs/1c-rerun/snapshot-boundaries.json) · [退出码日志](logs/1c-rerun/nonce-and-snapshots.log)

## 命令与退出码

| 平台 | 命令或入口 | 退出码 | 结果 |
| --- | --- | ---: | --- |
| Windows | 构建 `xiao-runtime --release` | 0 | [命令](logs/1c-rerun/commands/runtime-build.json) |
| Windows | `env_tutorial_matches_vm_golden --exact` | 0 | 1 通过。[命令](logs/1c-rerun/commands/tutorial-vm.json) |
| Windows | `env_tutorial_matches_native_golden --exact --ignored` | 0 | 1 通过。[命令](logs/1c-rerun/commands/tutorial-native.json) |
| Windows | 驱动 env 默认及原生专项 | 0、0 | 7 项和 5 项通过。[默认命令](logs/1c-rerun/commands/env.json) · [原生命令](logs/1c-rerun/commands/env-native.json) |
| Windows | Runtime env 及产品 COFF 专项 | 0、0 | 17 项和 4 项通过。[Runtime 命令](logs/1c-rerun/commands/runtime-env.json) · [COFF 命令](logs/1c-rerun/commands/coff-unit.json) |
| Windows | 重编并运行现有 C ABI 探针 | 0、0 | 覆盖状态 0/1/2/3/6/8、借读和输出槽所有权。[回执](logs/1c-rerun/c-abi.json) |
| Windows | `cargo test --workspace` | 0 | 197 组，1,824 通过、0 失败、61 忽略。[命令](logs/1c-rerun/commands/workspace.json) |
| Windows | `cargo fmt --all -- --check` | 0 | 输出为空。[命令](logs/1c-rerun/commands/fmt.json) |
| Ubuntu WSL | release Runtime、精确 VM 和原生教程、驱动默认和原生专项、Runtime env 专项 | 均为 0 | 两个教程入口各 1 项，专项分别 7、5、17 项通过。[命令与日志索引](logs/1c-rerun/linux-ubuntu/result.json) |

## 失败、重试与平台缺项

首次源码校验退出 1。Windows 的 `core.autocrlf=true` 让归档中的
`.cargo/config.toml` 变为 CRLF，和固定 Git blob 字节不符。重试对单次
`git archive` 设置 `core.autocrlf=false`、`core.eol=lf`，导出和逐文件校验成功，
随后使用该源码构建。MSVC 链接器的帮助命令退出 1100，独立的文件版本探测及实际链接均成功。
[首次导出](logs/1c-rerun/source-export.json) · [校验记录](logs/1c-rerun/source-export-check.log) · [重试](logs/1c-rerun/source-export-retry.json) · [链接器探测](logs/1c-rerun/commands/tool-link.json)

默认沙箱枚举 WSL 返回 `E_ACCESSDENIED`。允许访问本机 WSL 服务后，Ubuntu 26.04
探测及实际测试均退出 0。启动时记录到 `wsl.conf` 的重复 `boot.systemd` 配置提示。
[沙箱枚举](logs/1c-rerun/wsl-list-sandbox.log) · [Ubuntu 探测](logs/1c-rerun/wsl-ubuntu-probe.log) · [实际运行](logs/1c-rerun/commands/wsl-ubuntu-run.json)

macOS `aarch64-apple-darwin` 为 `unverifiable`，本机没有 macOS 运行环境。
完整平台矩阵仍缺 macOS 及 CI 回执，规划 §五的五批性能回执仍缺。
