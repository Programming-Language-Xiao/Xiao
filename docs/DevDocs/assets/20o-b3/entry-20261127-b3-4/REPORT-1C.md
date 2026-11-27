# B3-4-ENV-1C 本地验收

> **历史证据**　本报告记录 `B3-4C-FIX-A` 之前的 `0e983b0f` 产品证据，已被
> [`REPORT-1C-RERUN.md`](REPORT-1C-RERUN.md) 在 `00766ca3` 上的复跑取代；保留作修复前对照，
> 不作为当前产品结论。

Windows 的 env 教程在 VM 双导入和原生双导入 O0/O3 上均逐字匹配黄金，退出码均为 0。
六份 MSVC 产物均由产品验证器报告 COFF `Complete`。两份权限负例均报告
`X06-RUNTIME-026`，运行期未捕获错误按既有映射退出 3；只有执行前快照 006/007 要求退出 2。
本批据此把权限负例判为符合契约。
[原生矩阵](logs/1c/native-matrix.json) · [验收日志](logs/1c/native-acceptance.log)

## 源码与工具

产品提交为 `0e983b0f6a3c1104f07541460d029d0ffc4bfd17`，Git 树为
`14a2e5391e86465c5d334d75d2bef72e0db256a4`。在独立任务根导出并逐文件核对该树，
叠加本批测试与夹具后执行。源码回执列出叠加文件以及运行前后的摘要。Windows 导出对两个
锁文件应用了工作树 CRLF 转换。回执同时记录已测字节和 Git 原件的 SHA-256，并逐字核对
换行还原后的内容。
[源码导出](logs/1c/source-export.log) · [运行输入](logs/1c/source-identity.json) · [全部本地运行后的源码复核](logs/1c/source-after-all.json)

使用 Windows 11 `10.0.26200`，目标 `x86_64-pc-windows-msvc`，Rust 和 Cargo 1.96.0，
MSYS2 UCRT64 clang、llvm-as、llc 22.1.2，MSVC 链接器 14.51.36256.0 和
Windows SDK 10.0.26100.0。固定的 B3-4C release Runtime SHA-256 为
`2dba315ba4aa6353b45eb21b704999936a9d8370ba21b7171f3f32e1d83577ef`。
六份产物的 ABI 为 1.20，生成器为 27，标准库接口为 1。回执记录了工具二进制摘要以及
实际 clang/MSVC 命令。
[工具链](logs/1c/toolchain.json) · [链接轨迹](logs/1c/clang-commands.log)

## 教程与夹具

从 B3-0E 迁入 `tests/fixtures/docs/env/tutorial.*`，用目录级 Git 属性固定黄金和输入的
LF。候选原始来源为 `7f1017b7ff0b7b0a89d11943d5d33e0b432d0b3f`，冻结表最近修改
提交为 `5c8094ea3be277ceb0d34d106bc58e2b65651250`。
[冻结表](../entry-20261124-b3-0e/tutorials.json)

| 文件或变体 | 字节 | SHA-256 |
| --- | ---: | --- |
| `tutorial.xiao` | 467 | `9ab09cf88eb5e4c16bc08a712f25e14739fee027074e014c8ff0e2ac7beedeeb` |
| `tutorial.stdout` | 55 | `2420b27b0eac750657ba5909a8928fdb74674104601c62ae2ef6d273f2ae5491` |
| `tutorial.input.json` | 69 | `f5233cf1df7c282e2e490cb092b835898882477c2cffe4c084ec194bac780522` |
| 仅替换首条导入的 `stdlib.env` 变体 | 470 | `a8cb3149daec0193db2de4b6d42752c38812965de8e624f4a106c879de5c6e1d` |

`20o_b3_env_tutorial.rs` 新增两个独立入口。VM 从 `RunOptions.environment` 接纳
`[["XIAO_B3_TUTORIAL_VALUE", "initial"]]`，观察到两条导入均初始化 `stdlib.env`。
原生入口透传同一个数组，同时向子进程放入不同的同名合成值，确认输出取自显式快照。
两个入口都检查完整的 55 字节，包括最后一个 LF。原生成功用例的 stderr 为空。
[VM 精确执行](logs/1c/tutorial-vm-exact.log) · [原生精确执行](logs/1c/tutorial-native-exact.log)

原生教程与既有 `20o_b3_env` 共用固定工具链、任务目录和产物回执代码。权限负例沿用
B3-4C 的直接原生测试，能力掩码 `0x20` 缺少 `environment` 位。既有专项核对两后端
运行时错误的一致性，本批产物验收按要求核对退出码 2。

## Windows 六份 COFF 产物

产品构建器在每次 MSVC 链接后调用 `verify_artifact_with_link_evidence`。六份产物的
声明组件与观察组件均为 `env`、`rc`、`value`、`weak`，未分类 Runtime 符号均为空。
逐件核对了产物、对应 map 的 SHA-256 和链接命令。每份回执可定位原始 `.exe`、`.exe.map`、
stdout、stderr 及产品验证回执。从原生精确入口选取四份教程产物，从既有原生专项选取两份
权限产物。

| 用例 | 能力 | 目标 | 生成器 | COFF | 退出 | 本批首个失败 | 逐件回执 |
| --- | --- | --- | ---: | --- | ---: | --- | --- |
| `std.env` O0 | `0xA0` | `x86_64-pc-windows-msvc` | 27 | `Complete` | 0 | 无 | [回执](logs/1c/native/env-std-o0/receipt.json) |
| `std.env` O3 | `0xA0` | `x86_64-pc-windows-msvc` | 27 | `Complete` | 0 | 无 | [回执](logs/1c/native/env-std-o3/receipt.json) |
| `stdlib.env` O0 | `0xA0` | `x86_64-pc-windows-msvc` | 27 | `Complete` | 0 | 无 | [回执](logs/1c/native/env-stdlib-o0/receipt.json) |
| `stdlib.env` O3 | `0xA0` | `x86_64-pc-windows-msvc` | 27 | `Complete` | 0 | 无 | [回执](logs/1c/native/env-stdlib-o3/receipt.json) |
| `denied` O0 | `0x20` | `x86_64-pc-windows-msvc` | 27 | `Complete` | 3 | 无；运行期 026 正确 | [回执](logs/1c/native/denied-o0/receipt.json) |
| `denied` O3 | `0x20` | `x86_64-pc-windows-msvc` | 27 | `Complete` | 3 | 无；运行期 026 正确 | [回执](logs/1c/native/denied-o3/receipt.json) |

| 产物 | `.exe` SHA-256 | `.exe.map` SHA-256 |
| --- | --- | --- |
| `env-std-o0` | `91943721e8151cf3d32f2157a2d88a87722797f9eb88f641f909609972d5015c` | `4eb65a5998fcfb52b8f088ca306da7d5bda7040d024ec2fc3043a540e294ee45` |
| `env-std-o3` | `add2d72564aa814f7dd144fb64bd899d29367631892f40c9a6edeadc6cb1fcf9` | `b670cb04c02d86ab44ce5cecee565e1f7461aa8573fd388d7bd4b481162094b5` |
| `env-stdlib-o0` | `ddf84d3aa56432a233411903fa7a1e141c65f005a46f0ad7f422f8386d77bc91` | `7d6d52b6ae8d2ec50d87921ade0801dc5a6a5c8c51e48878469289835a885001` |
| `env-stdlib-o3` | `2e8547894617c15ce4b05dcc87c60105b351e429a8bfd4fa8b781adf48fffc79` | `85d5bb354e27a0d7a2c17fefcd3e628a4cd79c087de5f1a5916b829cdf4a0923` |
| `denied-o0` | `cb4339e22f46ab370972d4bc802827a07e462e506e4e98f9c98b80568ac045c4` | `1b3ad55f3c48bcca74cc305fa1053953a618baad5b82746cdd829fbf1fb8757e` |
| `denied-o3` | `b4fc3cd58ac3bd1a885ae2b7761ea1874f3f954526347edc3b80dddb584f8204` | `aa165a5d9e2ffbcef46b53498f473984c22ea65dae691011f70d24751c3fe99a` |

## 本机命令与退出码

| 命令 | 退出码 | 结果与原始日志 |
| --- | ---: | --- |
| `cargo +1.96.0 test --locked --manifest-path core/rust/Cargo.toml -p xiao-driver --test 20o_b3_env_tutorial -- --nocapture` | 0 | VM 1 通过，原生 1 忽略。[日志](logs/1c/tutorial-vm.log) |
| `cargo +1.96.0 test --locked --manifest-path core/rust/Cargo.toml -p xiao-driver --test 20o_b3_env_tutorial -- --ignored --nocapture --test-threads=1` | 0 | 原生 1 通过，覆盖双导入与 O0/O3。[日志](logs/1c/tutorial-native.log) |
| `cargo +1.96.0 test --locked --manifest-path core/rust/Cargo.toml -p xiao-driver --test 20o_b3_env -- --nocapture` | 0 | 7 通过，原生 3 忽略。[日志](logs/1c/env.log) |
| `cargo +1.96.0 test --locked --manifest-path core/rust/Cargo.toml --workspace` | 0 | 197 组，1,822 通过，0 失败，59 忽略。[日志](logs/1c/workspace.log) |
| `cargo +1.96.0 fmt --manifest-path core/rust/Cargo.toml --all -- --check` | 0 | 输出为空。[日志](logs/1c/fmt.log) |
| `cargo +1.96.0 test --locked --manifest-path core/rust/Cargo.toml -p xiao-driver --test 20o_b3_env_tutorial env_tutorial_matches_vm_golden -- --exact --nocapture` | 0 | 精确入口 1 通过，0 失败，0 忽略。[日志](logs/1c/tutorial-vm-exact.log) |
| `cargo +1.96.0 test --locked --manifest-path core/rust/Cargo.toml -p xiao-driver --test 20o_b3_env_tutorial --release env_tutorial_matches_vm_golden -- --exact --nocapture` | 0 | 发布配置精确入口 1 通过，0 失败，0 忽略。[日志](logs/1c/tutorial-vm-release.log) |
| `cargo +1.96.0 test --locked --manifest-path core/rust/Cargo.toml -p xiao-driver --test 20o_b3_env_tutorial env_tutorial_matches_native_golden -- --exact --ignored --nocapture --test-threads=1` | 0 | 精确入口 1 通过，0 失败，0 忽略。[日志](logs/1c/tutorial-native-exact.log) |
| `cargo +1.96.0 test --locked --manifest-path core/rust/Cargo.toml -p xiao-driver --test 20o_b3_env -- --ignored --nocapture --test-threads=1` | 0 | 原生回归 3 通过，包含启动快照和 O0/O3 运行时错误对照。[日志](logs/1c/env-native.log) |
| `cargo +1.96.0 test --locked --manifest-path core/rust/Cargo.toml -p xiao-runtime --lib env -- --nocapture --test-threads=1` | 0 | Runtime 与 C ABI 专项 17 通过。[日志](logs/1c/runtime-env.log) |

命令回执包含运行时间、实际退出码以及原始日志 SHA-256。使用同一固定 Runtime 重编并执行
B3-4C 的 C ABI 探针，两步均退出 0，覆盖 0/1/2/3/6/8、借读及输出槽所有权。
[命令回执](logs/1c/commands.json) · [探针回执](logs/1c/c-abi.json) · [探针输出](logs/1c/c-abi-run.log)

使用归档的 `std.env` O0/O3 原件复验快照总数、名称字节、值字节和累计字节四组上下界。
24 个样本均符合预期，其中 16 个产出冻结黄金、退出 0，8 个以 `X09-DRIVER-007`
退出 2 及空 stdout 拒绝。
[原始日志](logs/1c/snapshot-boundaries.log) · [边界回执](logs/1c/snapshot-boundaries.json)

## 失败与重试

两份权限产物的 stdout 为空，stderr 的机器行均带有
`code=X06-RUNTIME-026 message_id=runtime.capability_unavailable` 和 `exit_code=3`。
逐件重跑保留的同一产物后，退出码、诊断和输出摘要都复现。产品把未捕获的运行时权限错误
映射到进程退出 3，符合既有进程错误映射；执行前快照 006/007 才是退出 2。现有归档准入
缺位测试独立观察到 `X06-RUNTIME-026` 和退出 2，两者是不同入口层次。最初测试脚本把
运行期 026 误按 2 断言，已改回 3，不再记作产品失败。
[O0 重试](logs/1c/native/denied-o0/retry.json) · [O3 重试](logs/1c/native/denied-o3/retry.json)

工具身份采集首次把 `link.exe /?` 的退出码 1100 当作探测失败。该调用已输出链接器版本，
文件版本复核退出 0，实际六次链接均在 clang 轨迹中定位。
[首次探测](logs/1c/tool-link.log) · [文件版本](logs/1c/tool-link-file-version.log)

最初从默认沙箱枚举与启动 WSL 均收到 `E_ACCESSDENIED`。重试读到本机 Ubuntu WSL 的
x86_64 环境。首次工具探测找不到 PATH 上的 Rust，于用户安装目录确认 1.96.0 后离线执行。
[首次枚举](logs/1c/platform-linux.log) · [首次启动](logs/1c/platform-linux-ubuntu.log) · [工具探测](logs/1c/platform-linux-probe.log) · [Rust 安装路径](logs/1c/platform-linux-rust-paths.log)

## Linux 本机补验

Ubuntu 26.04 WSL2 使用同一份导出源码和测试夹具，显式指定
`x86_64-unknown-linux-gnu`，使用 Rust 1.96.0、LLVM 21.1.8 和离线锁定依赖构建
release Runtime，SHA-256 为 `3b1a77457e08b754db50b27c3ad8ff00838b619a4a91fa2b1fc924485ace8494`。
两个教程精确入口各通过 1 项，原生四条路径逐字匹配黄金。驱动默认专项通过 7 项，原生
专项通过 3 项，Runtime 和 C ABI 专项通过 17 项，五条命令均退出 0。
[命令与工具回执](logs/1c/linux-ubuntu/result.json) · [VM](logs/1c/linux-ubuntu/tutorial-vm.log) · [原生](logs/1c/linux-ubuntu/tutorial-native.log)

## 平台缺项与 B3-4E 建议

| 平台 | 状态 | 本批证据 |
| --- | --- | --- |
| Windows `x86_64-pc-windows-msvc` | 教程、COFF 与权限 026/退出 3 通过 | VM、原生 O0/O3、Runtime 与 C ABI 专项，以及六份原件 |
| Linux `x86_64-unknown-linux-gnu` | 本机 WSL 专项通过 | Ubuntu WSL2 的 VM、原生 O0/O3、驱动、Runtime 和 C ABI。[本机回执](logs/1c/linux-ubuntu/result.json) |
| macOS `aarch64-apple-darwin` | `unverifiable` | 本机为 Windows，缺少 macOS 运行环境 |

B3-4E 按运行期 026/退出 3、执行前 006/007/退出 2 的层次对账，不再把 3 当失败。
Linux 本机专项已通过，macOS 和 Linux/macOS 的 CI 作业回执仍需补齐。§五的环境性能
仍缺五批独立进程的中位数与峰值驻留回执。B3-4E 再对账提交、Runtime、六份 map 的产物
绑定及 CI 身份，并依据完整矩阵裁定收口。
