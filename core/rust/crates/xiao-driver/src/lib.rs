//! Xiao 编译、运行和构建请求编排接口的 crate 入口。
//!
//! 当前提供 08-U0 统一前端和 09-B0-C 内部运行驱动器：源码只经过一次解析、模块分析、
//! 类型检查和生命周期分析，成功后交给 `xiao-ir` 验证，再由生产字节码和 VM 接口执行。
//! 用户可见的 CLI 接线仍留给 11/X0。

/// 统一前端请求、上下文、结果和编排器。
mod frontend;
/// 统一前端向 VM 提供按需模块编译的接线。
mod module_loader;

/// `-debug` 独立诊断进程与终端启动编排。
pub mod diagnostics;

/// 19B 由当前格式和 ABI 常量生成的兼容矩阵。
mod compatibility;
/// 前端到 LLVM 原生后端的内部构建驱动器。
mod native;
/// 环境包源码及其只读静态接口。
mod packages;
/// X0-A 子进程协议、帧编解码和 Rust 核心入口。
pub mod protocol;
/// 前端产物到生产 VM 的内部运行驱动器。
mod run;

/// 重导出兼容矩阵和当前版本清单。
pub use compatibility::{
    CompatibilityAction, CompatibilityAxis, CompatibilityCell, CompatibilityEvidence,
    CompatibilityStatus, CompatibilityVersions, TargetSupportCell, TargetSupportStatus,
    compatibility_matrix, current_compatibility_versions, target_support_matrix,
};

/// 重导出统一前端公共接口。
pub use frontend::{
    ExternalModuleGraph, FRONTEND_VERSION, FrontendArtifact, FrontendCompiler, FrontendContext,
    FrontendError, FrontendIoError, FrontendRequest, compile, empty_type_results,
};

/// 重导出前端到 VM 的结构化运行接口。
pub use run::{
    CancellationToken, DRIVER_CANCELLED_CODE, DRIVER_CONTROL_CODE, DRIVER_TIMEOUT_CODE,
    DRIVER_VERSION, DriverError, DriverExecution, DriverOutcome, DriverPhase, DriverRequest,
    ExitCode, FrontendVmDriver, RunControl, run, run_request,
};

/// 重导出前端到 LLVM 原生程序的结构化构建接口。
pub use native::{FrontendNativeDriver, NativeBuildRequest, NativeBuildResult, NativeDriverError};
/// 可注入的环境包映射及其具体文件模块。
pub use packages::{PackageModule, PackageRegistry};

/// 重导出 X0-A 协议契约和标准输入/输出服务。
pub use protocol::{
    BUILD_ERROR_CODE, CANCELLED_ERROR_CODE, CORE_CRASH_CODE, CORE_VERSION, CoreVersions,
    DiagnosticConfig, DiagnosticFocusConfig, FRAME_ERROR_CODE, FRAME_LENGTH_BYTES, FrameError,
    MAX_FRAME_BYTES, OptimizationConfig, PROTOCOL_VERSION, ProtocolArtifact,
    ProtocolBackendLocation, ProtocolDiagnostic, ProtocolDiagnosticActivation, ProtocolError,
    ProtocolErrorBody, ProtocolEvent, ProtocolMetrics, ProtocolParam, ProtocolReport,
    ProtocolRequest, ProtocolResponse, ProtocolRuntimeConfig, ProtocolSpan, ProtocolStackFrame,
    ProtocolTarget, ProtocolValue, REQUEST_ERROR_CODE, RunOptions, SourceIdentity, ToolchainSpec,
    ToolchainVersionsSpec, UNSUPPORTED_OPERATION_CODE, VERSION_MISMATCH_CODE, core_crash_response,
    decode_frame, dispatch, encode_frame, read_frame, read_request, serve, serve_stdio,
    write_frame,
};
