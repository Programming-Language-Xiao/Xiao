---
id: tooling.cli.protocol
title: Rust 核心进程协议
status: verified
audience: CLI 集成开发者
module: rust.xiao-driver
stage: 11X0/17C
related:
  - ../../../DevDocs/11x0-cli-protocol-and-toolchain.md
  - README.md
  - debug.md
---

# Rust 核心进程协议

X0-A 的核心入口是 `xiao-core` 子进程。调用方先发送一个 `hello` 帧完成版本协商，
再发送 `run`、`run_archive`、`test`、`build`、`environment`、`package`、`repl_packages`、`cancel` 或 `shutdown`。本页描述已经验证的机器边界；用户可见的
`xiao` 命令、项目测试和独立分发已经接入；`xiao build` 及主机工具链发现也已接入 X0-E。

## 帧格式

每帧由 8 字节大端无符号长度和 UTF-8 JSON 负载组成。长度只计算 JSON 字节，最大负载是
16 MiB。干净 EOF 表示输入结束；部分长度、部分负载、非法 UTF-8、非法 JSON 和超长帧
都会返回 `X11-PROTOCOL-001`。调用方应读取稳定 `code`，不解析人类可读消息。

## 版本与结果

`protocol_version = 1` 和统一 `core_version = 1` 决定兼容性。前端、驱动器、字节码格式、
Runtime ABI 和 LLVM 版本只在 `versions` 中用于诊断。失配返回 `X11-PROTOCOL-004`，
不会执行用户源码。

`run` 和 `build` 响应始终包含 `request_id`、冻结的 `exit_code`（0 到 4）和 `exit_name`。
运行响应还提供结构化诊断、错误报告、事件和指标；执行前拒绝的 `error` 响应也保留可选
`report`；构建响应提供产物路径与工具链指纹。
`metrics.peak_live_bytes` 是本次运行中同时存活的 Xiao Runtime 管理对象字节数峰值：
包含对象头、载荷、字符串字节与容器元素/键存储，不重复计算共享引用的目标对象。
它不包含 VM 栈/帧、编译期结构和字符串驻留表，也不是进程 RSS；旧响应缺少此字段时
客户端应按未知计量处理，不能伪报为零。
`test` 请求携带按项目相对路径排序的 `cases`；`test_result` 响应提供整体退出码、通过/失败
统计和按请求顺序排列的逐用例诊断、报告、事件、指标与协议错误。
`environment` 请求携带项目根、逻辑环境名、原始 `config.xiao`、目标和工具链描述；核心只把配置
解析为静态 `ConfigDocument`，返回 `environment_result` 元数据。项目根只用于计算环境落点，
不进入配置、目标、工具链或汇总指纹；元数据中的 `lockfile_summary` 当前为 `null`，由后续锁文件阶段填写。
`package` 请求携带 `sync`/`install`、项目根、可选激活环境绝对路径、静态配置文本、
目标与工具链以及同步开关；Rust 返回 `package_result` 的环境绝对路径、创建/变化与
锁文件状态。错误依旧走 `error`，CLI 不重新判定目标或生成锁文件。
`package` 是以 `hello.capabilities` 协商的兼容新增操作；不支持时 CLI 在发送请求前失败，
不把旧核心的未知请求当作可用功能。

## 归档运行

`run_archive` 请求携带 `path`、统一 `RunOptions` 和 `debug` 位。核心读取 `.xar` 后，
按固定顺序验证唯一索引、全部对象、Runtime ABI、目标平台和入口，成功后才调用统一 VM。
入口只取 `ArchiveIndex.entry`，不会从成员扫描猜测。归档运行结果仍使用 `result` 的
`exit_code`、`exit_name`、报告、事件和指标字段，操作名为 `run_archive`；执行前失败沿用
`X17-XAR-006`（缺少对象）、`007`（版本不兼容）、`008`（运行条件不满足）和 `009`（校验失败）。

## REPL 环境包视图（I4a）

`hello.capabilities` 中的 `"package"` **只表示包管理**（`sync`、`install`、`lock`、
`update`、`add`、`remove`）；`"repl_packages"` 才表示环境包视图与静态接口查询。
它们是两种不同操作，不共享 `PackageRequest.operation`。新增只读操作保持协议版本 1。
客户端在旧核心缺少 `"repl_packages"` 时**不要发送该请求**，应返回无第三方包的空视图；
缺少新请求的可选字段也不得导致崩溃。

`repl_packages` 请求携带 `request_id`、`protocol_version`、`core_version`，并可带
`active_environment`（绝对路径，省略或为 `null` 时只查全局 `envs/global`）和
`module_path`（如 `lib.api`，省略时只枚举包根、不读取源码）。激活环境与全局环境不会合并；
全局环境尚未安装时返回空数组，显式指定的环境不存在则返回错误。
`repl_packages_result` 返回 `environment_path`、按名称排序的 `packages`（每项含 `root`、
版本及来源 `identity`）和可选 `interface`；接口含 `module_path`、按名称排序的
`exports`，每项有 `name`、`kind` 和可为空的函数 `signature`。读取接口复用 05-B
模块符号与 04 静态类型签名，不执行包代码；包名中含 `-`、`.` 或关键字等不可
作为单段 Xiao 模块名的包不进入自动登记视图。若多个映射争用根名，返回
`X11-REPL-PACKAGE-001`（`details.root`、`details.candidates`、`details.environment`），
要求显式 `import`，绝不按扫描顺序任选；读取失败为 `X11-REPL-PACKAGE-002`，
`details` 给出环境、包名及底层原因。

本请求**不预加载**模块，也不由查询接口执行模块。`run` 从当前环境的只读包映射登记
包根，访问包内模块内容时由 VM 首次编译、初始化并缓存成功结果；未访问时不执行。
本地项目的 `import` 则在执行到语句时初始化对应模块，两个来源使用不同的模块身份。
在同一个核心进程会话内，多次 `run` 共享成功初始化的模块和表运行时状态；每次 `run`
仍重新创建主程序帧和主程序单例，普通顶层变量不跨请求保持；模块单例继续复用。
会话内 `run` 按接收顺序串行执行，
当前环境路径、E0 环境指纹或包身份/源码对象摘要变化时，核心丢弃整份会话并从空状态继续。
加载失败为可恢复错误，携带包名、来源环境及底层原因，失败不缓存，后续引用可重试；
不能由客户端在 `run` 前调用查询接口冒充加载。旧客户端每次调用新启核心，因此仍保持
一次调用一份 VM 状态；长驻复用只对同一个核心进程会话生效。
共享机器样本见 `tests/spec/11x0-protocol/repl-packages-request.json` 和
`tests/spec/11x0-protocol/repl-packages-response.json`。
取消通过同一请求 ID 绑定 `CancellationToken`，其结果使用 `ArtifactRejected` 的进程码 2。

`optimization.debug = true` 是强制诊断位。它携带可选的 `diagnostics` 等级、日志目标和
聚焦规则，核心会在进入前端/VM 前启动独立 `xiao-diagnostics` 终端会话；诊断通道使用
独立的 8 字节长度帧，不复用核心 stdout。构建响应在该位开启时增加旁置激活位摘要，普通
构建的 `diagnostic_activation` 为空。终端窗口细节见[-debug 诊断窗口](debug.md)。

X0-B 的 `xiao run`、X0-T 的 `xiao test`、X0-E 的 `xiao build` 和 11A-E0 的 `xiao venv` 已消费这条协议；命令行为、非 TTY
呈现和 `print` 尚未实现的限制见[xiao run 与 CLI 外壳](shell.md)，测试规则见[`xiao test`](test.md)，
构建参数与工具链发现见[`xiao build`](build.md)，
分发目录和核心发现见[独立打包与核心发现](packaging.md)。

## 共享契约

Rust 与 TypeScript 当前采用共享 JSON fixture 的窄类型策略。样本位于
`tests/spec/11x0-protocol/`，两侧测试都必须读取同一批样本并验证回环。新增字段先更新
样本和两侧显式类型；不兼容的消息变更必须更新协议版本或核心版本。可兼容的附加计量
字段保留现有版本，客户端不得因缺失该字段而崩溃；`peak_live_bytes` 缺失时摘要显示未知。

## 运行控制

`run.options` 和 `test.options` 包含 `max_call_depth`、`event_capacity`、`timeout_ms`、
`checkpoints_enabled` 和 `checkpoint_interval`。后两个字段控制 VM 热循环取消检查点；缺失
时 Rust 核心按默认值启用检查点并使用默认间隔。它们只控制运行时轮询，不改变
`metrics.instructions` 或既有 `Error`/`Fatal` 结果语义。

TypeScript `ProtocolClient.runSource` 接收 `AbortSignal`。CLI 入口通过 `AbortController` 监听
`SIGINT`，信号触发后由客户端发送 `cancel` 帧；核心侧取消和超时统一返回 `ArtifactRejected`
进程码 `2`。取消不进入用户 `catch`，但 VM 会尝试现有 `finally` 和释放计划。
`xiao-core` 的 stdin 专用于此帧协议，不能混入终端输入的原始字节；语言级交互输入需要
后续独立的 VM I/O 与协议通道。I1b 的多行执行只临时退出终端 raw mode 以恢复 SIGINT，
不宣称当前 `input()` 已可使用。
