//! Runtime 错误兼容门面。
//!
//! 错误本体统一定义于 [`xiao_diagnostics`]。本模块只重导出旧 Runtime
//! 名称，帮助已有内存、表和测试代码平滑迁移；这里不再维护第二套错误结构。

/// 统一诊断错误类型、稳定错误码及报告器接口的 Runtime 兼容重导出。
pub use xiao_diagnostics::{
    ALLOCATION_CODE, BackendLocation, CONTAINER_HASHABILITY_CODE, CONTAINER_INDEX_CODE,
    CONTAINER_KEY_CODE, CROSS_THREAD_CODE, DIVISION_BY_ZERO_CODE, DiagnosticParam,
    ErrorAccumulator, FATAL_CORRUPT_ARTIFACT_CODE, FATAL_HARDWARE_CODE, FATAL_INTERNAL_CODE,
    FATAL_OUT_OF_MEMORY_CODE, FATAL_RUNTIME_INVARIANT_CODE, FATAL_STACK_OVERFLOW_CODE, FatalError,
    FatalKind, FrameKind, INVALID_HANDLE_CODE, INVALID_VALUE_CODE, MessageRenderer,
    NUMERIC_OVERFLOW_CODE, PreviewRenderer, REFCOUNT_INVARIANT_CODE, ReportClass, ReportRecord,
    RuntimeError, RuntimeErrorKind, RuntimeResult, StackFrame, TABLE_DROP_CODE, TABLE_INIT_CODE,
    TABLE_STATE_CODE, TYPE_MISMATCH_CODE, USE_AFTER_RELEASE_CODE, WEAK_UPGRADE_CODE, XiaoError,
    XiaoErrorKind, XiaoResult,
};
