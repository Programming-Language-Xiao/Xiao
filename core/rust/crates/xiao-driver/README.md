# `xiao-driver`

## 目录职责

编排源码读取、前端、生产字节码 VM、优化、LLVM 后端、Runtime、缓存和归档请求，提供给 TypeScript CLI 的稳定结构化服务接口。

## 工程期

08 建立前端驱动；09-B0-C 已接入内部前端到 VM 运行驱动器，09-B0-D 冻结退出码；10 接入构建；11–18 接入配置、包、优化、缓存和 `.xar`。

## 模块放置

请求模型、流水线编排、版本协商和服务边界放在 `src/`；终端命令路由放在 `cli/ts/src/commands`。

## 边界

不保存终端编辑状态、不生成本地化文案、不暴露 Rust 内部布局；请求/结果使用稳定的
`DRIVER_VERSION`，并携带前端目标、VM 参数、诊断和结构化运行报告。优化配置仍由后续阶段接入。

## 08A/U0 交付

`src/frontend.rs` 提供 `FrontendRequest`、`FrontendContext`、`FrontendCompiler` 和
`FrontendArtifact`。流水线固定调用解析、模块、类型、生命周期和 IR 验证；错误诊断会
累积，错误时不返回 IR。定向规格位于 `tests/u0_frontend.rs`，对应 UseDocs 为
`docs/UseDocs/language/compiler/frontend/README.md`。

## 09-B0-C 运行驱动器

`src/run.rs` 提供 `DriverRequest`、`FrontendVmDriver` 和单一的 `DriverOutcome`：前端失败
保留完整诊断，执行前验证/请求错误保留稳定编号和报告，进入 VM 后保留 B0-B 的完整结果、
事件、指标和报告。驱动器固定消费真实 `FrontendArtifact`，经生产 `lower_program` 和
`xiao_vm::run_request` 执行；它不提供入口函数或机型选择参数，也不复制基准工具的函数零搬迁。

取消与超时目前在前端完成、降低完成和 VM 调用前后采样。VM 指令循环检查点记录为
`B0-C-CANCEL-001`，出口批次为 `11/X0`；因此接口已可表达边界拒绝，但不会声称能中途打断
正在运行的 VM。测试位于 `tests/b0_c_driver.rs`。

## 09-B0-D 退出码

`ExitCode` 冻结驱动器的五种终局及其 `0..=4` 进程码，`DriverOutcome::exit_code()` 只根据
结果阶段和 `RunResult` 分支派生，不读取诊断编号或本地化文本。`Success` 包括被 `catch`
消费的错误；取消、超时和其它执行前拒绝统一为 `ArtifactRejected`。CLI 接线仍由 11/X0
调用 `as_process_code()` 完成。契约测试位于 `tests/b0_d_exit_codes.rs`。

## 10A 前端到 LLVM 内部驱动器

`src/native.rs` 提供 `NativeBuildRequest`、`FrontendNativeDriver` 和
`NativeBuildResult`。驱动器先调用同一个 `FrontendCompiler`，再把同一份已验证 IR 交给
`xiao-codegen-llvm`；工具链路径由请求注入，缺失或失败以结构化错误返回。这里不接用户可见
的 `xiao build`，也不重新解析源码或发现宿主工具链。
