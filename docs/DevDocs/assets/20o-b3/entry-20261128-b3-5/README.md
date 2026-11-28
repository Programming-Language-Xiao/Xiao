# B3-5A 基线与编号回执

## 入口与版本

产品提交为 `d6e5102767f4192416f507432e902884670d0bae`，Git tree 为
`94940f30f9feeaf6b6fdbd389df372219ec70451`。采集时 HEAD 为
`060f5c958d1b91ddc0088d270008762aac8fc9b7`，tree 为 `92cd601803657f318087576ac3c69b68cb3288f6`，
父提交就是产品基线。`git diff --stat d6e51027..060f5c95` 为 2 个文件、126 行新增，
其中实施规划 125 行、`docs/DevDocs/README.md` 入链 1 行。入口工作树干净。[Git 原始输出](logs/git-baseline.log)

| 项目 | 实值 | 基线源码 |
| --- | --- | --- |
| Runtime ABI | 1.20 | `xiao-runtime-abi/src/lib.rs:24` 的 `ABI_MAJOR_VERSION` 和第 26 行的 `ABI_MINOR_VERSION` |
| LLVM 生成器 | 27 | `xiao-codegen-llvm/src/lib.rs:108` 的 `CODEGEN_VERSION` |
| 标准库接口 | 1 | `xiao-standard-library/src/lib.rs:53` 的 `STDLIB_INTERFACE_VERSION` |

源码路径以 `core/rust/crates` 为基准。锁文件与三份夹具在基线 Git 对象、LF 归档和工作树中逐字一致。
`Get-FileHash`、`sha256sum` 各算一次摘要，`Get-Item.Length`、`wc -c` 各算一次字节数。
[结构化清单](baseline.json) · [双重测算](logs/input-digests-final.log) · [Git 对象与源码核对](logs/fixed-source-and-fixtures-final.log)

| 锁文件 | 字节数 | SHA-256 |
| --- | ---: | --- |
| `core/rust/Cargo.lock` | 19,660 | `28d38451ba0490b70d2d136bc3268acad8de02b5658bf826f842069019161c53` |
| `tests/benchmarks/Cargo.lock` | 19,264 | `8ac4f16af6e8baf5b71e0e0b5d89397eb5ae7bf2e3cc5a96053683c6e8b4b577` |

## 编号与能力

逐项按数字 ID 排序比对，实际 64 条声明，无重复、缺失、额外声明或状态差异。
`intrinsics.json:66` 的 63 `os_log_write` 为 `reserved`，`arity`、`parameter_type`、`return_type`、
`vm_binding`、`abi_binding`、`effect`、`capability`、`error` 均为字符串 `none`。
第 67 行的 100 `__reserved_100` 为 `tombstone`。[逐项实测输出](logs/fixed-source-and-fixtures-final.log) · [声明表](../../../../../core/rust/crates/xiao-intrinsics/intrinsics.json)

| ID（升序） | 状态 | 条数 |
| --- | --- | ---: |
| 1～62 | `active` | 62 |
| 63 | `reserved` | 1 |
| 64～99 | 未分配 | 0 |
| 100 | `tombstone` | 1 |

`xiao-intrinsics/src/lib.rs:157` 的 `CAPABILITY_CLOCK` 为位 1（`0x02`），第 165 行的
`CAPABILITY_STDOUT` 为位 5（`0x20`），按 `clock` 后 `stdout` 的冻结检查顺序登记，
位或复算为 `0x22`。已知掩码由第 179 行 `KNOWN_CAPABILITY_MASK` 算得 `0xFF`，
旧请求默认掩码由第 171～177 行 `DEFAULT_CAPABILITY_MASK` 算得 `0x7F`。
`xiao-driver/src/protocol/request.rs:242–244` 读取该默认常量，反序列化入口位于第 218、503 行。
[常量源码](../../../../../core/rust/crates/xiao-intrinsics/src/lib.rs) · [请求源码](../../../../../core/rust/crates/xiao-driver/src/protocol/request.rs)

## 五项旧负例

`aeb8422b` 的测试差异与产品基线逐条对读。五处编号断言延续到了基线，涉及的测试文件中另有一处生成器正例从 26 更新到 27。
下表记录源码断言，测试执行由后续获授权批次取证。[B3-4 回执](../entry-20261127-b3-4/REPORT.md) · [历史差异与当前源码](logs/b3-4-negative-source.log)

| 文件与测试入口（相对 `core/rust/crates`） | 当前断言与拒绝路径 | 63 激活后的用例 |
| --- | --- | --- |
| `xiao-bytecode/src/encode/tests.rs:680` · `decoded_intrinsic_ids_are_checked_in_both_operand_widths` | 对 `Leb128` 和 `FixedU16` 替换已编码签名 ID 为 63、100、`u32::MAX`，`decode` 和 `decode_encoded` 都必须返回 `InvalidFormat`，文本含对应 ID。覆盖解码签名表校验。 | 64 覆盖低位未知号，100 继续覆盖双宽度的墓碑解码拒绝，`u32::MAX` 覆盖极端未知号。 |
| `xiao-bytecode/tests/b4_cmath_tac.rs:338` · `cmath_unknown_intrinsic_ids_are_rejected_before_encoding` | 向调用签名注入 63、100、`u32::MAX`。`verify_production` 发出 `X09-BYTECODE-002` 且消息以对应 ID 结尾。两个操作数宽度的 `encode` 都返回精确 `InvalidFormat`。覆盖生产 TAC 验证及编码入口。 | 64 覆盖未知号，100 继续覆盖生产 TAC 和双宽度编码的墓碑拒绝，保留 `u32::MAX`。 |
| `xiao-driver/tests/20o_b2_b2_divide.rs:26` · `lfloat_divide_requires_the_frozen_public_contract` | 对 63 和 100 分别断言 `active_by_id(...).is_none()`，覆盖活动声明查找的过滤。 | 用 64 验证未分配号无法查得活动声明，100 继续覆盖已登记但非活动的墓碑。 |
| `xiao-driver/tests/20o_b2_b4_cmath_vm.rs:216` · `vm_rejects_invalid_shapes_types_and_unknown_ids` | `mutation == 5` 注入 63，断言 `encode(&program, OperandWidth::Leb128).is_err()`。同测试的其他变异才调用 VM 并断言 `X06-RUNTIME-002`。编号负例覆盖编码入口预检。 | 改用 64 覆盖未分配号的编码拒绝。 |
| `xiao-driver/tests/20o_b3_time_tutorial.rs:134` · `reserved_intrinsic_ids_fail_before_execution` | 将活动签名 50 换成 63，要求 `encode_xiaoc` 返回错误，`run_request` 失败且无 `ModuleLoaded` 或 `IntrinsicOutput` 事件。覆盖归档编码和 VM 执行前 TAC 验证。 | 改用 64，沿用两项拒绝和零事件断言，测试名反映未知号。 |

B3-5B 可用 64 作五处替代号。该号在基线表未分配，100 的既有覆盖可区分已登记墓碑与未分配号。

## 冻结夹具

B3-0E 夹具由 `5c8094ea3be277ceb0d34d106bc58e2b65651250` 登记，
矩阵追溯到 B3-0C 的 `7f1017b7ff0b7b0a89d11943d5d33e0b432d0b3f`。
三件取自 B3-0E 的 `candidates/tests/fixtures/docs/log/`。将源码首行导入 `std.log` 变为
`stdlib.log` 后，独立复算得到 267 字节和矩阵声明的摘要。[夹具矩阵](../entry-20261124-b3-0e/tutorials.json) · [字节与摘要](logs/input-digests-final.log)

| 文件 | 字节数 | SHA-256 |
| --- | ---: | --- |
| `tutorial.xiao` | 264 | `d7ec578f5f0177355a04f054831a5d1aac1b7ab0c24757e05f494992eedf5a8b` |
| `tutorial.input.json` | 41 | `30cf24e7a51b3e75a7ccda7f6448818dcbd4acb0b84e939e9e7083eeb019f129` |
| `tutorial.stdout` | 103 | `267d7726181b9bd4d38d57e4edd26f853ba09ed8aa400db46b589b3c73c27146` |

输入 `wall_clock_unix_ms=946684800000` 从 Unix 纪元按毫秒复算得 `2000-01-01T00:00:00.000Z`，
编码 24 字节。第一、三行分别为 `24 + 1 + 4 + 1 + 12 + 1 = 43` 和
`24 + 1 + 5 + 1 + 12 + 1 = 44` 字节，由时间戳、空格、级别、空格、消息和 LF 组成。
中间行是拒绝带换行消息后由示例打印的 `X06-RUNTIME-012` 加 LF，共 16 字节。
重组 `43 + 16 + 44 = 103` 字节与冻结 UTF-8/LF 黄金逐字一致。[夹具结构复算](logs/fixed-source-and-fixtures-final.log)

## 工具与 Runtime

| 平台与目标 | Rust 与 Cargo | LLVM 工具 | 链接器 |
| --- | --- | --- | --- |
| Windows 本机，`x86_64-pc-windows-msvc` | Rust / Cargo 1.96.0，实际可执行文件在 `C:\Users\xiaoc\.rustup\toolchains\1.96.0-x86_64-pc-windows-msvc\bin`，Rust 内置 LLVM 22.1.2。 | `D:\msys64\ucrt64\bin` 的 `llvm-config.exe`、`clang.exe`、`llvm-as.exe`、`llc.exe` 均为 22.1.2。clang 默认目标为 GNU。 | 构建使用 MSVC `link.exe` 14.51.36256.0。另有 `D:\msys64\ucrt64\bin\ld.exe` GNU 2.46。完整路径见下方及日志。[本机输出](logs/tools-windows.log) |
| Ubuntu 26.04 LTS WSL2，`x86_64-unknown-linux-gnu` | Rust / Cargo 1.96.0，实际可执行文件在 `/home/xiaoczx/.rustup/toolchains/1.96.0-x86_64-unknown-linux-gnu/bin`，Rust 内置 LLVM 22.1.2。 | `/usr/bin/{llvm-config,clang,llvm-as,llc}` 解析到 `/usr/lib/llvm-21/bin`，实测均为 21.1.8。 | `/usr/bin/cc` GCC 15.2.0，`/usr/bin/ld` GNU ld 2.46，`/usr/bin/ld.lld` LLD 21.1.8。[WSL 工具输出](logs/tools-wsl-login.log) |
| macOS，仅 CI 可用，`aarch64-apple-darwin` | B3-4E 历史 runner 的 Rust 1.96.0（内置 LLVM 22.1.2）、Cargo 的 rustup 代理 `/opt/homebrew/opt/rustup/bin/cargo`。 | 历史 runner 的 Homebrew LLVM 23.1.6，`/opt/homebrew/opt/llvm/bin/{clang,llvm-as,llc}` 通过可执行文件检查，clang 版本另有实测。 | B3-4E 保存了 clang 链接命令。该次 CI 的底层链接器独立版本和路径缺证。本批未在 macOS 执行。[B3-4E 回执](../entry-20261127-b3-4/REPORT-1E-CI.md) · [attempt 2 原日志](../entry-20261127-b3-4/logs/1e-ci/macos-maintenance-attempt2.log) |

历史 macOS 作业为 `macos-15-arm64`（macOS 15.8.2），受测提交 `18e7f4fa`，
run `46093774527`、job `140827942402`、attempt 2。更早的 B3-0A run `45033610157`、job
`137242370630`、attempt 1 的 `environment.json` 实测了 Rust/Cargo 1.96.0（`/Users/runner/.rustup/toolchains/1.96.0-aarch64-apple-darwin/bin`）、
Apple `/usr/bin/ld`（`ld-1167.5`），以及 `/opt/homebrew/Cellar/llvm@21/21.1.8_1/bin` 中的
`clang-21`、`llvm-as` 和 `llc`（21.1.8）。作业归档对象可由 `entry.json` 的 macOS CI 摘要定位。
[B3-4E 作业](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/46093774527/job/140827942402) · [B3-0A 清单](../entry-20261122-b3-0/entry.json) · [B3-0A 原始压缩包](../entry-20261122-b3-0/observations.zip)

B1 七库 20 入口、18 项资源、VM/原生双精确入口及 28 守卫（4 份实测、24 项派生）按历史原件核对。
[B1 原始回执](../entry-20261122-b3-0/README.md)

| 目录 | 绝对路径 |
| --- | --- |
| 任务根 | `D:\project\Xiao\.xiao-orchestrator\worktrees\b3-5a` |
| 基线源码 / Cargo 工作目录 | `D:\project\Xiao\.xiao-orchestrator\worktrees\b3-5a\source-lf\core\rust` |
| 正式构建 target | `D:\project\Xiao\.xiao-orchestrator\worktrees\b3-5a\target\lf` |
| Runtime 副本 | `D:\project\Xiao\.xiao-orchestrator\worktrees\b3-5a\runtime` |
| 缓存 / Cargo 缓存 | `D:\project\Xiao\.xiao-orchestrator\worktrees\b3-5a\cache` / `D:\project\Xiao\.xiao-orchestrator\worktrees\b3-5a\cache\cargo-home` |
| 用例输出 | `D:\project\Xiao\.xiao-orchestrator\worktrees\b3-5a\cases` |

以 Rust 1.96.0 从固定提交的 LF 归档构建 Windows release，使用默认特性且未开启 `test-failpoints`。
`CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER` 指向
`C:\Program Files\Microsoft Visual Studio\18\Community\VC\Tools\MSVC\14.51.36231\bin\Hostx64\x64\link.exe`。
两份库原件在该 target 的 `release` 下，Runtime 目录中的副本与原件字节数和摘要一致。
[构建身份](logs/runtime-build-lf-identity.log) · [库摘要原始输出](logs/runtime-digests.log)

| 库 | 字节数 | SHA-256 |
| --- | ---: | --- |
| `xiao_runtime.lib` 静态库 | 27,869,848 | `e1cc77ef7269b10e06850b4497ca8ab1e7eaff0235e50d97dea9709329c719f9` |
| `libxiao_runtime.rlib` | 7,801,406 | `873d67ecb36436a7044c8da43d1c10881ef94684e419e2f8b25c4944997f68b8` |

## 验证与限制

| 命令 | 退出码 | 证据 |
| --- | ---: | --- |
| `cmd /c git diff --stat d6e51027..060f5c95` | 0 | [Git 基线](logs/git-baseline.log) |
| `cargo build --release --locked -p xiao-runtime`，首次归档 | 0 | [首次输出](logs/runtime-build.log) · [首次身份](logs/runtime-build-identity.log) |
| `cargo build --release --locked -p xiao-runtime`，LF 归档 | 0 | [最终输出](logs/runtime-build-lf.log) · [归档及锁文件比对](logs/runtime-source-lf.log) |
| `cargo build --locked -p xiao-doc-coverage-rust --bin xiao-doc-coverage-rust` | 0 | [门禁适配器构建](logs/gate-adapter-build.log) |
| `bun tools/repo-check/src/cli.ts docs` | 0 | [文档门禁](logs/gate-docs.log) |
| `bun tools/repo-check/src/cli.ts layout` | 0 | [目录门禁](logs/gate-layout.log) |
| `bun run check:coverage` | 0 | 有 `A0-COVERAGE-002` 文档覆盖警告。[覆盖门禁](logs/gate-coverage.log) |
| `cmd /c git diff --check`、`cmd /c git status --short`、`cmd /c git diff --exit-code HEAD -- core/rust stdlib tests .github` | 均为 0 | [最终 Git 核对](logs/git-final.log) · [清单与文件复核](logs/manifest-audit.log) |

工作区仅含本回执目录和规划 §四 B3-5A 末尾的一条入链。`intrinsics.json`、`core/rust`、
`stdlib`、`tests`、`.github`、registry 和 B3-4 证据的 Git 核对无差异，暂存区为空。

- Git 本机 `core.autocrlf=true` 令首次归档的两个锁文件写为 CRLF。字节等值核对中止，随后显式以
  `core.autocrlf=false`、`core.eol=lf` 在独立目标目录重建，两份锁与 Git 对象等值。
  工具和夹具探测中的失败输出也随包保留。[首次归档](logs/runtime-source.log) · [首次摘要核对](logs/input-digests.log) · [别名复算首轮](logs/fixed-source-and-fixtures.log)
- WSL 内联探测的引号转发令工具路径为空，改用外部脚本后，非登录 shell 缺 Rust 搜索路径，最终用登录 shell 复测退出 0。
  WSL 的 `boot.systemd` 重复键警告仍在原始输出中。MSVC `link.exe /?` 与 `lib.exe /?` 打印版本和帮助，退出 1100。
  [WSL 内联探测](logs/tools-wsl.log) · [WSL 非登录探测](logs/tools-wsl-final.log) · [Windows 工具输出](logs/tools-windows.log)
- 首轮门禁输出写入同一路径，复跑后逐条保存了命令、退出码与输出。[首轮保留输出](logs/gate-first-shared-output.log)
- macOS 仅 CI 可用。本批只在 Windows 构建基线 Runtime，WSL 只探测工具。
  日志功能、VM/原生教程、C ABI、COFF 和性能的验收结果由后续批次取证。

## B3-5B 前置条件

调度中枢审核本回执并授权 B3-5B 后，以本产品提交、版本和能力位作为入口。
按冻结契约接通 63 的三字符串签名、联合能力、运行级配额、串行交付及 VM，使用 64 替换五处负例并复核 100 的墓碑路径。
新实现的 Runtime 应以其源码重新构建并单独登记摘要。[B3-5 实施规划](../../../20o-b3-5-log-implementation-plan.md) · [日志冻结契约](../entry-20261123-b3-0c/contract-draft.md)
