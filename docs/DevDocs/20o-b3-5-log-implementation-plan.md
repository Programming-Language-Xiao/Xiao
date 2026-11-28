# 20O-B3-5. `log` 实施规划

> **状态（2026-11-28）**　`b3_5_planned=true`、`b3_5_started=true`、`b3_5_entry_ready=true`、`b3_5_passed=false`。B3-4 已收口。B3-5A～E 的实施由 Claude 会话按授权调度。本规划批的交付范围仅为本文。

## 一、入口与范围

| 入口 | 固定身份与依据 |
| --- | --- |
| 产品 | 提交 `d6e5102767f4192416f507432e902884670d0bae`，Git tree `94940f30f9feeaf6b6fdbd389df372219ec70451`。Runtime ABI 1.20、生成器 27、标准库接口 1。[入口产品](https://github.com/Programming-Language-Xiao/Xiao/commit/d6e5102767f4192416f507432e902884670d0bae) · [ABI 常量](../../core/rust/crates/xiao-runtime-abi/src/lib.rs) · [生成器常量](../../core/rust/crates/xiao-codegen-llvm/src/lib.rs) |
| B3-4 收口 | `b3_4_passed=true`。本地教程、六份 Windows COFF 和五批性能的复跑绑定 `1e42f501`。CI 三平台 env 验证绑定 `18e7f4fa`，维护工作流 run `46093774527` 的 attempt 2 通过，首轮性能失败另存。[B3-4 实施规划](20o-b3-4-env-implementation-plan.md) · [教程与 COFF](assets/20o-b3/entry-20261127-b3-4/REPORT-1C-RERUN2.md) · [本地性能](assets/20o-b3/entry-20261127-b3-4/REPORT-1D-RERUN.md) · [B3-4E CI 回执](assets/20o-b3/entry-20261127-b3-4/REPORT-1E-CI.md) |
| 编号 | 入口中 1～62 为 `active`，63 `os_log_write` 为 `reserved`，形参、返回、分派、效果和能力字段均为 `none`。100 为 `tombstone`。产品实施只激活 63。[入口 intrinsic 表](../../core/rust/crates/xiao-intrinsics/intrinsics.json) |
| 教程与 registry | B3-0E 的教程草案和 registry 候选为 `planned`。基线 registry 尚无 `stdlib.log` 条目。今后按实际落地范围登记为 `planned`，公开验证与状态升级归 B3-7。[B3-0E 教程规划](20o-b3-0e-tutorials-fixtures-plan.md) · [日志教程草案](assets/20o-b3/entry-20261124-b3-0e/tutorials/log.md) · [registry](../module-registry.json) |

`std.log` 和 `stdlib.log` 共用规范身份 `stdlib.log`，公开入口为 `write(str level, str message, str target) -> none`。三个参数必填，支持位置和命名调用。标准库包装沿用 `env` 的直接转发方式，VM 和原生共用 Runtime 的日志规则。[B3 总规划](20o-b3-host-libraries-plan.md) §B3-5、证据矩阵与风险禁止 · [B3-0C 规划](20o-b3-0c-fs-env-log-plan.md) §B3-0C3 · [B3-0C 冻结契约](assets/20o-b3/entry-20261123-b3-0c/contract-draft.md) · [env 包装](../../stdlib/env.xiao)

## 二、记录、权限与运行控制

| 项目 | 冻结口径 |
| --- | --- |
| 级别与目标 | 级别严格接受小写 `debug`、`info`、`warn`、`error`，编码为对应 ASCII 大写。目标必填且仅接受 `stdout`。消息为严格 UTF-8，保留有效文本，可为空，拒绝 CR、LF、NUL 和非法编码。 |
| 时钟与格式 | 复用 `time` 的 UTC 墙钟和毫秒格式化。Unix 毫秒按 `time` 规则取得，墙钟回拨如实反映。输出为 24 字节的 `YYYY-MM-DDTHH:mm:ss.SSSZ`，随后是一个 ASCII 空格、大写级别、一个空格、消息和一个 LF。时区固定 UTC，宿主时区与区域设置不参与格式化。期限使用单调钟。[time 冻结契约](assets/20o-b3/entry-20261123-b3-0b/contract-draft.md) · [B3-0C 日志裁定](assets/20o-b3/entry-20261123-b3-0c/decisions.json) |
| 交付与刷新 | VM 接入可报告接纳、写入和刷新失败的运行输出端，原生交付到受控 stdout。每次调用构造完整记录，在同一运行输出端串行写入并逐条刷新，成功以写入和刷新均完成为准。并发记录的字节保持边界，可观察顺序取实际交付次序。 |
| 失败归属 | 运行输出端拒绝接纳、写入失败或刷新失败报 `ResourceError/LOG_OUTPUT_FAILED`。已写字节保留，含部分记录，禁止自动重放。运行结果交付后的 CLI 或封装工具转发失败由该工具报告。诊断保留错误身份、调用点和可用原因，省略完整消息及环境数据。 |
| 能力 | 按 `clock`（位 1）再 `stdout`（位 5）检查，联合掩码为 `0x22`。VM、原生和 C ABI 均须实际检查。模块与归档声明两位，沿用已知掩码 `0xFF` 和旧请求省略能力时的默认掩码 `0x7F`。[B3-0D 规划](20o-b3-0d-version-intrinsic-abi-coff-plan.md) §5.2 · [20D 能力上下文](20d-capability-context.md) |
| 字节与条数 | 单条消息最多 4,096 UTF-8 字节，完整编码上限为 4,128 字节（24 字节时间戳、最长 5 字节级别、两个空格、消息和 LF）。同一运行最多尝试交付 1,024 条，完整编码累计最多 1 MiB（1,048,576 字节）。各上限均包含边界。按完整编码在开始交付时同时计入条数和字节数，写入或刷新失败仍计入。交付前被拒绝或取消的调用不占用额度。并发任务共享本运行的账，新运行重置。 |

函数先检查运行期实参类型 `X06-RUNTIME-002`，再按顺序检查能力 `X06-RUNTIME-026`，随后校验参数 `X06-RUNTIME-012`、输入长度及工作量预算、运行控制安全点，最后执行宿主操作。只有输入合法且预算允许才读墙钟。超限报 `ResourceError/LOG_LIMIT_EXCEEDED`，定宽转换溢出沿 `X06-RUNTIME-009`，受控分配失败沿 `X06-RUNTIME-011`。时钟与格式化复用 `time` 对宿主时钟失败和格式域越界的 `X06-RUNTIME-012`。C ABI 的值结构校验和整次运行的入口校验沿各自的既有前置规则。[B3-0C 冻结契约](assets/20o-b3/entry-20261123-b3-0c/contract-draft.md)「错误与取消」 · [time 错误表](assets/20o-b3/entry-20261123-b3-0b/contract-draft.md)

从本次操作的首次运行控制安全点起算 10 秒单调时钟期限，等待受更早的运行截止约束。安全点先处理既有运行控制，运行取消 `X09-DRIVER-001` 先于运行截止 `X09-DRIVER-002`，独立任务取消沿 `X06-RUNTIME-027` 和既有任务交付规则。自身期限到达报 `ResourceError/LOG_TIMEOUT`。检查覆盖等待输出权、I/O 前、写入分块之间、交付前、刷新和返回边界。在可等待处使用受控等待或显式报告不支持，登记不可中断宿主调用及返回后复查的实测限制。观察到中断时保留已写字节，已开始的尝试保留配额。运行期错误沿既有调用 span、`catch`、`finally` 和释放机制传播。[B3-0C 冻结契约](assets/20o-b3/entry-20261123-b3-0c/contract-draft.md)「错误与取消」 · [取消安全点](09b0e-vm-cancellation-checkpoint.md) · [B3-4C-FIX-B](assets/20o-b3/entry-20261127-b3-4/REPORT-FIX-B.md)

## 三、intrinsic、C ABI 与版本

| 项目 | 实施口径 |
| --- | --- |
| intrinsic 63 | `os_log_write(str level, str message, str target) -> none`。三个字符串按序借读，无返回值。登记可观察的读时钟、写标准输出效果及两项能力，贯通类型、IR、TAC、VM、LLVM 和归档。现有 intrinsic 声明只表达单项能力，实施时须让 63 的声明及消费端准确表达两位联合要求，沿用既有能力位和既有原语语义。[B3-0D 规划](20o-b3-0d-version-intrinsic-abi-coff-plan.md) §5.1 · [B3-0F 联合冻结表](assets/20o-b3/entry-20261124-b3-0f/freeze-table.json) · [声明类型](../../core/rust/crates/xiao-intrinsics/src/lib.rs) |
| C ABI | `extern "C"` 返回 `i32`，三个指针在调用期间借读并由调用方强引用覆盖，末尾传入 `u64 capabilities`。按无结果入口省略 `out`。成功 0，既有结构错误 1（空指针或形状）、2（未知标签或无效参数）、3（无效句柄），可恢复运行错误 6，Fatal 8。取消沿现有通道，保留诊断与释放行为。[冻结 C ABI 表](assets/20o-b3/entry-20261124-b3-0d/c-abi.json) · [B3-0D 规划](20o-b3-0d-version-intrinsic-abi-coff-plan.md) §5.4 |
| ABI 与格式 | ABI 1.20 已覆盖 B3-0D 联合增量及 63 的冻结 C 签名。标准库接口 1，`.xiaoc` 和 `.xar` 容器为 1.0，IR 与原生元数据格式为 1。生成器 28 的归档索引采用 1.1，新增必需的生成器字段，读取器兼容 1.0 索引。全局缓存索引为 1.0。旧 1.19/22 公共产物按冻结规则执行前拒绝。[B3-0D 规划](20o-b3-0d-version-intrinsic-abi-coff-plan.md) §5.3 · [版本复核](assets/20o-b3/entry-20261128-b3-5/REPORT-C.md) |

```c
int32_t xiao_runtime_log_write(
    const XiaoValue* level,
    const XiaoValue* message,
    const XiaoValue* target,
    uint64_t capabilities
);
```

### 生成器 27→28 的判据

1. A、B 使用入口生成器 27。C 固定同一 Runtime、工具链、目标和输入，在 O0/O3 记录 27 下 log lowering 接线前后的 LLVM IR、C 调用声明与参数、实际链接符号和退出码。如接线前拒绝 63，保留拒绝原文，另用同一份可生成的无 log 对照记录前后 IR 与摘要。将候选生成文本与更换生成器版本本身造成的差异分开取证。[B3-4C 生成器裁定与回执](assets/20o-b3/entry-20261127-b3-4/REPORT-1B.md)
2. O0/O3 实际差分确认新增 `xiao_runtime_log_write` 调用和启动接线，且无日志对照前后逐字一致。星崽于 2026-11-28 经 Claude 调度中枢批准 27→28，C 批采用 28。[B3-5C 回执](assets/20o-b3/entry-20261128-b3-5/REPORT-C.md)
3. 记录批准来源、前后工具与生成指纹、缓存失配和重建。用实际 28 校验器接纳真实 27 对照产物，并用实际 27 校验器执行前拒绝 28 归档和原生元数据，保留版本诊断及退出码。缓存按 ABI、生成器、Runtime 内容、目标、能力和标准库身份核对。墙钟样本、配额和流状态只随运行存在。[B3-0D 规划](20o-b3-0d-version-intrinsic-abi-coff-plan.md) §5.3 · [原生元数据校验](../../core/rust/crates/xiao-codegen-llvm/src/native_metadata.rs)

## 四、分批实施

### B3-5A　基线与编号

- **必读**　B3 总规划 §B3-5/证据矩阵/禁止项、B3-0C 的 log 冻结契约、B3-0D §5.1～5.4、B3-4E CI 回执、入口 intrinsic 表和 `stdlib/env.xiao`。[B3 总规划](20o-b3-host-libraries-plan.md) · [B3-0C 规划](20o-b3-0c-fs-env-log-plan.md) · [B3-0D 规划](20o-b3-0d-version-intrinsic-abi-coff-plan.md) · [B3-4E CI 回执](assets/20o-b3/entry-20261127-b3-4/REPORT-1E-CI.md)
- **范围**　固定入口提交/tree、Rust workspace 与 benchmarks 两份 Cargo 锁、版本、编号和权限表，记录三平台可用的工具及 B3-0E log 夹具摘要。建立独立任务根和 target、Runtime、缓存、用例输出目录。盘点 B3-4 将 60 改为 63 的五项负例，明确在 63 激活后以未知号和既有墓碑继续覆盖相应拒绝路径。[B3-4A/B 实施报告](assets/20o-b3/entry-20261127-b3-4/REPORT.md) · [B3-0E 夹具矩阵](assets/20o-b3/entry-20261124-b3-0e/tutorials.json)
- **验证与产出**　基线回执列出 63 保留、1～62 活动、100 墓碑，核对工具、锁与冻结字节。B1 七库 20 入口、18 项资源、双精确入口和 28 守卫沿原证据核对，其中 4 项实测、24 项派生。[B3-0A 原始证据](assets/20o-b3/entry-20261122-b3-0/README.md)
- **禁止**　提前激活 63、分配其他 ID、改变版本、把前批证据登记为 log 实测。
- [B3-5A 基线与编号回执](assets/20o-b3/entry-20261128-b3-5/README.md)

### B3-5B　Runtime 与 VM

- **必读**　B3-0C 的交付、预算与运行控制条款，B3-0B 的 UTC 时钟，现有 VM 输出路径及 B3-4C-FIX-B 运行控制。[冻结契约](assets/20o-b3/entry-20261123-b3-0c/contract-draft.md) · [time 契约](assets/20o-b3/entry-20261123-b3-0b/contract-draft.md) · [VM 输出与分派](../../core/rust/crates/xiao-vm/src/semantics/exec.rs) · [B3-4C-FIX-B](assets/20o-b3/entry-20261127-b3-4/REPORT-FIX-B.md)
- **范围**　激活 63，更新三字符串签名、效果和联合能力声明。复用现有时钟、运行控制与错误传播，在 Runtime 实现预检、编码、运行级配额及串行交付。接通 VM/TAC、标准库薄包装、双路径和可报告失败的 VM 输出端，更新 A 盘点的编号负例。
- **验证与产出**　以独立输出端验证字节、刷新次数、两位授权、错误优先级、配额争用与运行间隔离。用受控时钟、部分写入、刷新故障及三类运行控制覆盖交付前后边界。提交 Runtime/VM 专项回执、声明与分派差异及原始日志给 C。
- **禁止**　更动 1～62 和 100 的语义、引入进程级日志配置或计数、吞掉输出失败、自动重放、提前接原生或升生成器。
- [B3-5B Runtime 与 VM 回执](assets/20o-b3/entry-20261128-b3-5/REPORT-B.md)

### B3-5C　LLVM、C ABI 与原生

- **必读**　B3-0D 的 ABI、C ABI、缓存与 COFF 规则，B3-4C 的生成文本比较和 FIX-B 的原生运行控制，现有原生输出入口。[B3-0D 规划](20o-b3-0d-version-intrinsic-abi-coff-plan.md) · [B3-4C 回执](assets/20o-b3/entry-20261127-b3-4/REPORT-1B.md) · [B3-4C-FIX-B](assets/20o-b3/entry-20261127-b3-4/REPORT-FIX-B.md) · [原生输出](../../core/rust/crates/xiao-runtime/src/abi_io.rs)
- **范围**　实现冻结 C 符号和 LLVM lowering，向原生应用及任务传递同一运行的输出、配额及控制上下文。按既有入口和调用约定承载 C ABI，实际消费能力掩码。在双路径下验证类型、IR、TAC、`.xiaoc`、原生元数据、导出、移除及普通包隔离。用仅调用 `log.write` 的用例验证归档对 `0x22` 的需求，排除其他输出语句贡献的能力位。按 §三完成版本取证和裁定。
- **验证与产出**　保留 O0/O3 的 IR、符号、链接命令和实际调用回执。独立 C 探针直接链接本批 Runtime，核对参数顺序、借读、无输出槽、实际状态码与待处理错误。对照 VM 与原生的字节、诊断、运行控制和释放行为，归档版本裁定及兼容、缓存回执。
- **禁止**　新增隐式全开包装、单独一套原生日志语义、公共固定时钟选项或未经裁定的版本、ABI 布局与协议变化。
- [B3-5C LLVM、C ABI 与原生回执](assets/20o-b3/entry-20261128-b3-5/REPORT-C.md)

### B3-5D　教程、边界与平台

- **必读**　B3-0E 教程规划与夹具表，B3-3D/E 的逐字验收及产品验证口径，B3-4D 复跑与 B3-4E 三平台记录。[B3-0E 教程规划](20o-b3-0e-tutorials-fixtures-plan.md) · [log 夹具矩阵](assets/20o-b3/entry-20261124-b3-0e/tutorials.json) · [B3-3 实施规划](20o-b3-3-fs-implementation-plan.md) · [B3-3 三平台收口](assets/20o-b3/entry-20261125-b3-3-final/README.md) · [B3-4D 复跑](assets/20o-b3/entry-20261127-b3-4/REPORT-1C-RERUN2.md) · [B3-4E CI](assets/20o-b3/entry-20261127-b3-4/REPORT-1E-CI.md)
- **范围**　以 28 固定产品身份。按冻结 SHA-256 迁入 `tests/fixtures/docs/log/tutorial.xiao`、`tutorial.input.json`、`tutorial.stdout`，核对别名源码摘要。以 `wall_clock_unix_ms=946684800000` 和掩码 `0x22`，使用隔离捕获和受控时钟。教程目标为 `20o_b3_log_tutorial`，精确入口为 `log_tutorial_matches_vm_golden` 与 `log_tutorial_matches_native_golden`。
- **验证与产出**　三平台实跑 §五，精确筛选须各命中 1 项并通过，程序退出 0，双导入在 VM/O0/O3 都逐字匹配独立的 103 字节 UTF-8/LF 黄金。Windows 逐件绑定双导入 O0/O3 教程产物及两种单缺位的 O0/O3 权限产物，保存二进制、链接图和产品验证回执。记录发布运行与固定时钟、故障注入用构建的特性及各自证据。
- **禁止**　改写冻结黄金、依赖用户环境或真实终端、提前升教程/registry 为 `verified`、把未执行或未验证的平台记为通过。

### B3-5E　性能与证据收口

- **必读**　B3-0C「资源与性能」、B3-3E 审计与自反例、B3-4E 本地五批和 CI 首败复跑、B3 总规划 §B3-6/B3-7。[冻结契约](assets/20o-b3/entry-20261123-b3-0c/contract-draft.md) · [B3-3 收口](assets/20o-b3/entry-20261125-b3-3-final/README.md) · [B3-4E 五批复跑](assets/20o-b3/entry-20261127-b3-4/REPORT-1D-RERUN.md) · [B3-4E CI 回执](assets/20o-b3/entry-20261127-b3-4/REPORT-1E-CI.md) · [B3 总规划](20o-b3-host-libraries-plan.md)
- **范围**　按 §五在普通 release VM、原生 O0/O3 做三平台五批采样，核对实际链接 Runtime 的发布身份并关闭 `test-failpoints`。用固定 B3-5 产品补齐受影响的矩阵、工作区和维护 CI，封存 A～E 回执。
- **验证与产出**　按 §六只读审计与重算摘要。自反例至少覆盖单缺位被放行、黄金或产物替换、O0/O3 调换、重复产物、错误 map、CI attempt 遗漏和缺平台误报。交付性能原始样本、身份、审计、失败与重试记录及阶段收口或受限交接回执。
- **禁止**　用故障注入构建声称发布吞吐、覆盖首败、把 B3-4 产物写成 B3-5 成功、提前宣告六模块公共收口。

## 五、测试矩阵

Windows `x86_64-pc-windows-msvc`、Linux `x86_64-unknown-linux-gnu`、macOS `aarch64-apple-darwin` 均以 Rust 1.96.0 release Runtime 和 VM、固定 LLVM/链接器与显式目标执行。功能对照使用 VM 与原生 O0/O3，ABI 用同一产品的分平台 C 探针。受控场景使用独立输出端、合成消息和时钟，另用真实受控管道验证宿主输出路径。[B3-0C 冻结契约](assets/20o-b3/entry-20261123-b3-0c/contract-draft.md) · [B3-4E CI 格式](assets/20o-b3/entry-20261127-b3-4/REPORT-1E-CI.md)

| 验收项 | 必测用例与判据 |
| --- | --- |
| 级别、参数与编码 | 四级分别断言 ASCII 大写记录。位置和命名调用、空消息与多字节 UTF-8 成功。错误大小写、未知或空级别、CR/LF/NUL 报 012 且零读钟、零写入。非法编码在可达的解码边界拒绝，C ABI 畸形值另验结构错误。 |
| 时间戳与时区 | 固定墙钟、非零毫秒、Unix 纪元前取整和墙钟回拨。两个宿主时区下比较同一固定瞬时的 24 字节 UTC 格式。真实墙钟冒烟采用冻结格式和可验证的受控观察。分开注入墙钟转换溢出及格式域失败。 |
| 目标与权限 | `stdout` 独立捕获，日志字节不串到 stderr。以 `stderr`、其他目标和空目标验证 012。测试 0x22、仅 stdout、仅 clock、两位均缺及旧请求默认值。联合缺位先报 clock。动态类型不符与缺位并存先报 002，缺位与非法参数并存先报 026，非法参数与超限并存先报 012，预算失败先于首次控制安全点。否决路径核对读钟及输出计数。 |
| 并发与刷新 | 同一运行并发写入带独立标识的记录，逐条核对完整字节、交付顺序和刷新后立即可见。多任务争用末尾配额时核对尝试数与累计字节，新运行从零开始。 |
| 输出失败 | 实际捕获与关闭流、VM 接纳失败、未写任何字节即失败、部分写入后失败、完整写入后刷新失败。核对 `LOG_OUTPUT_FAILED`、已写字节、尝试入账、无自动重放及诊断省略完整消息。模拟运行完成后的 CLI 转发失败，核对工具自身退出码和错误归属。 |
| 限额 | 测消息 4,095/4,096/4,097 UTF-8 字节，最长级别的编码边界 4,128 字节。分别测 1,023/1,024/1,025 次尝试及累计 1,048,575/1,048,576/1,048,577 字节。分别触发条数和字节上限，验证分隔符与 LF 计入、失败尝试占额及 `LOG_LIMIT_EXCEEDED`。 |
| 取消与期限 | 注入运行取消 001、运行截止 002、单独任务取消 027 和操作期限 `LOG_TIMEOUT`，覆盖首次安全点、等待输出权、I/O 前、分块间、交付前及刷新/返回处。运行截止早于 10 秒时保留 002，运行取消与截止同到时先报 001。核对交付前零输出和零入账、交付后保留已写字节与配额以及既有清理行为。 |
| VM/O0/O3/C ABI | 对照语言输出、错误身份、诊断、退出码和释放。C 探针核对 `i32 (ptr, ptr, ptr, i64)`、0/1/2/3/6/8、错误通道及正常借读。三平台双导入逐字匹配教程黄金，验证 63 活动、未知号和墓碑拒绝、归档能力、原生组件和版本兼容。 |
| 性能 | 每次运行写 256/512/1,024 条 64 字节合成消息。计时含实际运行输出端交付和刷新。每个校准重复用新运行视图，每负载五批独立进程，预热一次，从 50 ms 起计并校准至 100 ms，增长至多 16 倍，总重复至多 `2^24`。五批中位数 VM ≤10 秒、原生各档 ≤2 秒，峰值驻留增量 ≤64 MiB，适用翻倍比 ≤3。最大单条、累计字节边界、真实等待和故障注入各自留回执。[冻结性能协议](assets/20o-b3/entry-20261123-b3-0c/contract-draft.md) |

## 六、证据格式

实施时在 `docs/DevDocs/assets/20o-b3/entry-<日期>-b3-5/` 归档。参考 B3-3D/E 的逐件产物与只读审计，以及 B3-4E 的平台、作业和失败重试记录。[B3-3 收口](assets/20o-b3/entry-20261125-b3-3-final/README.md) · [B3-4E CI](assets/20o-b3/entry-20261127-b3-4/REPORT-1E-CI.md)

| 记录 | 必填字段 |
| --- | --- |
| 源码与输入 | 固定提交和 Git tree，干净工作树及采样前后 `source_unchanged`，两份基线 Cargo 锁的 SHA-256 及实际使用的其他锁。夹具来源提交、源码、别名、合成输入、黄金和字节摘要，运行控制与故障注入方式。 |
| Runtime 与工具 | Runtime 文件及 SHA-256、构建档、特性、ABI、生成器、标准库接口，VM/探针/采样程序身份。Rust、Cargo、LLVM、编译器、链接器和验证器版本、路径与身份。平台、Runner 镜像、目标三元组、导入、后端、优化档、独占任务根及输出端。 |
| 运行与 CI | 用例与工作流提交，完整命令和 cwd，进程退出码、结构化诊断、原始 stdout/stderr、原始协议或测试日志。实际 CI run/job/attempt、链接及作业结论逐项绑定矩阵。性能保留校准轮次、五批样本、中位数、峰值驻留增量、翻倍比和对应受测身份。 |
| 产物与验证 | 逐件记录二进制、IR、链接命令/轨迹、map 或 PDB 与 `verification.json` 的路径和 SHA-256。Windows 按产品验证器报告 COFF `Complete`，声明组件集合等于观察组件集合，未分类 Runtime 符号为空，使用产物真实 COFF 符号表或与产物哈希绑定的受信完整链接图。Linux 和 macOS 登记实际 ELF/Mach-O 回执。[B3-0D COFF 政策](20o-b3-0d-version-intrinsic-abi-coff-plan.md) §5.5 |
| 失败、重试与缺项 | 首败和每次重试保留各自身份、日志及原因，注明修复源码或条件差异。缺平台/工具、不可中断或不可验证项逐格标明实际尝试、影响与补验范围。标记 `failed`、`missing` 或 `unverifiable` 的必测项进入受限交接。 |

## 七、退出条件与禁止项

1. B3-5A～E 必测项全部通过后才置 `b3_5_passed=true`。63 激活且原有编号口径通过守卫。双导入、VM、原生 O0/O3 和 C ABI 的日志记录、错误、配额及控制边界一致，版本裁定、归档和缓存回执齐备。
2. 固定 B3-5 产品的三平台功能、教程、逐件原生产物和五批性能齐备。Windows 应验产物均获产品级 COFF `Complete`。工作区、维护 CI、只读复核与自反例齐备，方可交接 B3-6 的六模块承载和 B3-7 最终公共矩阵。教程和 registry 保持 `planned`。
3. 任一必测失败、缺失或不可验证时，`b3_5_passed=false`，逐项列出受限交接事项。B3-4 的历史 Runtime、日志、CI 和 COFF 产物只用于入口核验及参考格式，禁止代替 B3-5 实跑；禁止以同链接候选、仅导出表、未绑定 sidecar 或 `UnverifiedCoffExports` 代替 Windows 产品验证。
4. 禁止扩大日志目标、级别、公共接口或能力。禁止引入日志过滤、后台队列、隐式缓存或全局配置，禁止绕过限额、运行控制和错误上报。禁止修改 B3-4 证据、B2 数值语义，启动 B4～B9、重跑旧 721 或核销历史债务。
5. B3-5-PLAN 仅新增本文，产品代码、ABI、intrinsic 表、标准库、registry、教程和原有文档均为本批只读输入。提交和推送属于后续获授权批次。
