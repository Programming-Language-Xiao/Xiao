# B3-5B Runtime 与 VM 回执

入口产品为 `d6e5102767f4192416f507432e902884670d0bae`，tree 为
`94940f30f9feeaf6b6fdbd389df372219ec70451`。实施基于文档提交 `93cddeb6` 的本地工作树。
本批实测平台是 Windows `x86_64-pc-windows-msvc`，Rust / Cargo 1.96.0。构建目录独立于 B3-5A。
Runtime ABI 为 1.20，生成器为 27，标准库接口为 1。规划状态为
`b3_5_started=true`、`b3_5_passed=false`。[B3-5A 基线](README.md) · [声明与基线核对](logs/b/declaration-audit.log)

## 声明与分派

| 层 | 本批实现 |
| --- | --- |
| 声明 | 63 `os_log_write` 激活为 `log`，三项 `str` 按 `level`、`message`、`target` 排列，返回 `none`。VM 绑定为 `LogWrite`，ABI 绑定为 `none`，错误键为 `runtime.type_mismatch`。效果为读时钟和写 stdout，所需位按 `clock`、`stdout` 检查，联合值为 `0x22`。已知掩码为 `0xFF`，省略请求掩码时为 `0x7F`。 |
| 类型与字节码 | 类型检查消费共享形参表。TAC 使用既有 `Call` 和 `CallSig` 记载 63，实测 LEB128、定宽 `u16` 与 `.xiaoc` 生产入口。两种编码中的非法 UTF-8 都在解码时拒绝。 |
| Runtime | 每次运行建独立计数，预算在首个安全点前预检。取得输出权后再次核对额度，接纳成功且安全点通过后同时计入条数和完整编码字节，再开始写入。记录复用 `time` 的 UTC 毫秒格式。逐条刷新，写入或刷新失败时保留已写字节与尝试配额。 |
| VM | 根 VM、子任务和表释放钩子共享运行的时钟、配额和日志输出端。默认内存 stdout 捕获在写入时保存记录，生产结果沿现有输出事件交付，调试事件容量独立计算。嵌入式 VM 可安装同步输出端，接纳、写入和刷新分别报告失败。 |
| 标准库 | `std.log` 和 `stdlib.log` 都解析为 `stdlib.log`。公开 `write` 直接调用 `os_log_write`，registry 状态为 `planned`。 |
| 编号守卫 | 五处负例改为未知号 64，墓碑 100 与 `u32::MAX` 仍覆盖。活动区间守卫验证 1～63。逐项对比入口表，差异仅为 63。编译期校验实测接受正确声明，拒绝九种字段漂移。 |

LLVM 沿用已有的未实现分派，`xiao-codegen-llvm` 的文件差异为空。[声明](../../../../../core/rust/crates/xiao-intrinsics/intrinsics.json) · [Runtime](../../../../../core/rust/crates/xiao-runtime/src/log.rs) · [VM](../../../../../core/rust/crates/xiao-vm/src/semantics/log_intrinsics.rs) · [标准库](../../../../../stdlib/log.xiao)

## 本机验证

| 专项 | 通过 | 忽略 | 原始日志 |
| --- | ---: | ---: | --- |
| intrinsic 声明及原有契约 | 12 | 0 | [测试](logs/b/intrinsic.log) |
| Runtime 日志 | 11 | 0 | [复跑](logs/b/runtime-log-retry.log) |
| VM 输出故障与运行控制 | 4 | 0 | [最终专项](logs/b/vm-log-final.log) |
| 驱动、双导入和字节码 | 7 | 0 | [最终专项](logs/b/driver-log-final.log) |
| 五处旧编号守卫及所在测试目标 | 16 | 1 | [字节码解码](logs/b/bytecode-ids.log) · [cmath TAC](logs/b/cmath-tac.log) · [驱动](logs/b/driver-ids.log) |

- 冻结墙钟 `946684800000` 下两个导入各逐字匹配同一份 103 字节 UTF-8/LF 黄金。
  最小调试事件容量下，根 VM 与并发任务合计交付 1,024 条，再次运行用同一驱动器重置额度。
  生产协议进程的 stdout 捕获与 stderr 分离，stderr 为零字节。[驱动回归](logs/b/driver-log-retry.log)
- Runtime 验证四级、空与多字节消息、非法参数零读钟零输出、权限优先级、单条与累计边界、
  并发短写的记录边界、时钟回拨、009 和 011。消息边界为 4,095/4,096/4,097 字节，
  最长记录为 4,128 字节，累计边界为 1,048,575/1,048,576/1,048,577 字节。[Runtime 专项](logs/b/runtime-log-retry.log)
- 受控期限覆盖首次安全点、等待输出权、接纳后、部分写入后和刷新返回。VM 用真实取消令牌、
  较早的运行截止、子任务 `join` 和已关闭的宿主管道复核传播。同步宿主调用可以占用线程直到
  它返回。故障端在写入中等待至运行截止的用例实测由返回后的安全点观察中断，已写的 7 字节保留。
  运行结果之后的 CLI 转发及阻塞宿主 stdout 适配留待原生和平台批验证。[VM 专项](logs/b/vm-log-first.log)

### 发布档 Runtime

从本批工作树用默认特性执行 `cargo build --release --locked -p xiao-runtime`，退出 0。
`llvm-nm` 在 staticlib 中找到已知的 `xiao_runtime_time_utc_now`，`xiao_runtime_log_write` 未定义。
[编译日志](logs/b/runtime-release.log) · [Rust 工具身份](logs/b/rustc-version.log) · [文件与符号核对](logs/b/runtime-digests.log)

| 文件 | 字节 | SHA-256 |
| --- | ---: | --- |
| `xiao_runtime.lib` | 27,882,502 | `ee0c887a737fc796450a53d88398c04d5ef95dfa25d816d91b8aae71ba663065` |
| `libxiao_runtime.rlib` | 7,911,868 | `36a2ca50efc606cdd29835fefb328274c8d671abf90ffe78692fa4993c32588d` |

### 工作区与仓库门禁

| 命令 | 退出 | 原始日志 |
| --- | ---: | --- |
| `cargo test --locked --workspace` | 0 | [完整复跑](logs/b/workspace-test-final.log) |
| `CARGO_PROFILE_TEST_OPT_LEVEL=1 cargo test --locked --workspace`（`cargo fmt` 后） | 0 | [格式化后完整复跑](logs/b/workspace-test-format-final.log) |
| `cargo clippy --workspace --all-targets -- -D warnings` | 0 | [首次](logs/b/workspace-clippy-first.log) · [复核](logs/b/workspace-clippy.log) |
| `bun tools/repo-check/src/cli.ts docs` | 0 | [文档](logs/b/gate-docs.log) |
| `bun tools/repo-check/src/cli.ts layout` | 0 | [复跑](logs/b/gate-layout-retry.log) |
| `bun run check:coverage` | 0 | [复跑](logs/b/gate-coverage-retry.log) |
| `cmd /c git diff --check`、`cmd /c git status --short` | 0 | [差异和文件范围](logs/b/git-final.log) |

专项合计 50 项通过、1 项忽略。工作区复跑（含 `cargo fmt` 后最终树）合计 1,849 项通过，其中 4 项文档测试，64 项忽略，
26 项由测试目标内的筛选排除。文档覆盖为 `12438/13801`（90.12%），公共 API 为
`6249/6249`（100%），原始日志保留 `A0-COVERAGE-002` 警告。改动在本批工作树完成，基线 HEAD 为 `93cddeb6`。

## 首败与复跑

- Runtime 首轮 10 过 1 败。测试把缺失能力名读自 `params`，既有 026 诊断将它记录在
  `context.capability`。修正断言后 11 项通过。[首轮](logs/b/runtime-log.log) · [复跑](logs/b/runtime-log-retry.log)
- 驱动专项首轮 6 过 1 败。释放用例预期先析构再进入 `finally`，本例实际顺序是
  `finally`、析构、`catch`。复跑增加 `print` 与 `chr(-1)` 对照核对既有顺序，7 项通过。
  [首轮](logs/b/driver-log-first.log) · [复跑](logs/b/driver-log-retry.log)
- 目录门禁首轮报 `A0-SIZE-001`，`exec.rs` 为 2,511 行。将原有 TAC 实参求值移入
  `call_arguments.rs` 后，`exec.rs` 为 2,499 行，门禁通过。
  [首轮](logs/b/gate-layout.log) · [复跑](logs/b/gate-layout-retry.log)
- 文档覆盖首轮为 89.35%，公共 API 为 99.97%。补齐新增代码及测试的声明文档后退出 0。
  [首轮](logs/b/gate-coverage.log) · [复跑](logs/b/gate-coverage-retry.log)
- 工作区首轮 1,845 项测试通过，文档测试因找不到 `xiao_bytecode` 的依赖文件中止。专项重编译和首轮测试曾共用
  目标目录，复跑期间只由工作区测试使用该目录。复跑设置测试档优化级别 `CARGO_PROFILE_TEST_OPT_LEVEL=1`。
  [首轮](logs/b/workspace-test.log) · [复跑](logs/b/workspace-test-final.log)

## B3-5C 前置条件

调度中枢审核本回执后，以已验证的 Runtime 日志核心接入原生和冻结 C ABI。原生应用和任务需共享
运行输出、配额和控制上下文，并在 C 探针上核对三项借读、能力 `0x22`、无输出槽、错误及释放。
`xiao-runtime/src/abi_io.rs` 现有的单 bit 能力比较仍是原生 print/input 的旧路径；C 批接入
日志时必须改用 `capability.missing_from(capabilities)`，避免 `clock_and_stdout` 只检查一位。
归档需用仅调用 `log.write` 的用例验证双位需求。

固定 Runtime、工具链、目标和输入，在生成器 27 下对照 O0/O3 的 LLVM IR、C 调用签名、链接符号
和退出码，保留接线前的拒绝及可生成的无日志对照。生成文本差异决定是否申请 27→28，采用 28 需经
星崽裁定，并执行产物兼容和缓存重建验证。[B3-5 实施规划](../../../20o-b3-5-log-implementation-plan.md)
