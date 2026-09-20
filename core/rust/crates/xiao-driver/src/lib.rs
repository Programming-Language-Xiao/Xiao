//! Xiao 编译、运行和构建请求编排接口的 crate 入口。
//!
//! 当前提供 08-U0 统一前端和 09-B0-C 内部运行驱动器：源码只经过一次解析、模块分析、
//! 类型检查和生命周期分析，成功后交给 `xiao-ir` 验证，再由生产字节码和 VM 接口执行。
//! 用户可见的 CLI 接线仍留给 11/X0。

/// 统一前端请求、上下文、结果和编排器。
mod frontend;

/// 前端产物到生产 VM 的内部运行驱动器。
mod run;

/// 重导出统一前端公共接口。
pub use frontend::{
    ExternalModuleGraph, FRONTEND_VERSION, FrontendArtifact, FrontendCompiler, FrontendContext,
    FrontendError, FrontendIoError, FrontendRequest, compile, empty_type_results,
};

/// 重导出前端到 VM 的结构化运行接口。
pub use run::{
    CancellationToken, DRIVER_CANCELLED_CODE, DRIVER_CONTROL_CODE, DRIVER_TIMEOUT_CODE,
    DRIVER_VERSION, DriverError, DriverExecution, DriverOutcome, DriverPhase, DriverRequest,
    FrontendVmDriver, RunControl, run, run_request,
};
