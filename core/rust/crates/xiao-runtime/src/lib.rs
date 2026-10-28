//! Xiao 执行时值、内存和错误运行库的 crate 入口。
//!
//! 本阶段提供不透明对象头、单线程引用计数、字符串和静态签名表对象。
//! 字节码 VM、LLVM 后端、CLI 与并发调度器只应通过这里的稳定句柄接口接入，
//! 不得复制一套生命周期语义。

/// 稳定 C ABI 的 Runtime 符号实现；公开布局来自 `xiao-runtime-abi`。
mod abi;
/// 22C 共享句柄、32B 共享头和 Runtime ID 注册表。
pub mod concurrency;
/// 平台级未处理异常的最小机器报告入口。
mod crash;
/// 22C Runtime/线程/任务池生命周期骨架。
pub mod scheduler;
/// 22A 冻结的 XCON v1 长度前缀序列化。
pub mod serialization;
/// VM 使用的不透明 Runtime/Thread/Task/Pool 句柄适配。
mod vm_concurrency;

/// 数组、元组、字典表、字典列和集合的运行对象。
pub mod containers;
/// 稳定 Runtime 错误、原因链和结构化参数。
pub mod errors;
/// 不透明对象头、强/弱句柄和引用计数策略。
pub mod memory;
/// VM 与 Runtime ABI 共用的资源操作与能力检查。
pub mod resources;
/// 由 05-C 签名驱动的表对象和状态机。
pub mod tables;
/// 仅供规格测试使用的释放计划驱动器。
pub mod testing;
/// 标量、字符串和统一 Runtime 值。
pub mod value;

/// 重导出 22C 共享句柄和 Runtime 身份基础。
pub use concurrency::{
    LockCancellation, RuntimeRegistry, SHARED_LOCKED, SHARED_WAITERS, SharedGuard, SharedHandle,
    SharedHandleError, SharedHeader, SharedLockError, SharedTag, SharedWeakHandle,
    global_runtime_registry,
};
/// 重导出容器句柄、字典形态和可哈希判定。
pub use containers::{ArrayHandle, DictHandle, DictKind, SetHandle, TupleHandle, is_hashable};
/// 重导出 Runtime 错误身份和结果别名。
pub use errors::{
    ALLOCATION_CODE, BackendLocation, CONCURRENCY_CANCELLED_CODE,
    CONCURRENCY_DETACHED_FAILURE_CODE, CONCURRENCY_POOL_ARGUMENT_CODE,
    CONCURRENCY_POOL_CLOSED_CODE, CONCURRENCY_RUNTIME_CLOSED_CODE, CONCURRENCY_SERIALIZATION_CODE,
    CONTAINER_HASHABILITY_CODE, CONTAINER_INDEX_CODE, CONTAINER_KEY_CODE, CROSS_THREAD_CODE,
    DIVISION_BY_ZERO_CODE, DiagnosticParam, ErrorAccumulator, FATAL_CORRUPT_ARTIFACT_CODE,
    FATAL_HARDWARE_CODE, FATAL_INTERNAL_CODE, FATAL_OUT_OF_MEMORY_CODE,
    FATAL_RUNTIME_INVARIANT_CODE, FATAL_STACK_OVERFLOW_CODE, FatalError, FatalKind, FrameKind,
    INVALID_HANDLE_CODE, INVALID_VALUE_CODE, ITERABLE_CODE, MessageRenderer, NUMERIC_OVERFLOW_CODE,
    PreviewRenderer, RANDOM_COUNT_CODE, RANDOM_SEED_CODE, REFCOUNT_INVARIANT_CODE,
    RESOURCE_CLOSED_CODE, ReportClass, ReportRecord, RuntimeError, RuntimeErrorKind, RuntimeResult,
    SELECTOR_BOUNDS_CODE, SELECTOR_STEP_CODE, SET_COMPARISON_CODE, SET_MEMBERSHIP_CODE,
    SET_OPERATION_CODE, StackFrame, TABLE_DROP_CODE, TABLE_INIT_CODE, TABLE_STATE_CODE,
    TYPE_MISMATCH_CODE, USE_AFTER_RELEASE_CODE, WEAK_UPGRADE_CODE, XiaoError, XiaoErrorKind,
    XiaoResult,
};
/// 重导出对象头和强/弱句柄类型。
pub use memory::{
    CounterStrategyKind, MemoryMeasurement, NonAtomicRefCount, ObjectLayout, RefCountStrategy,
    ReleaseAction, ReleaseEvent, ReleaseTraceGuard, RuntimeTypeTag, StrongHandle, WeakHandle,
    flush_release_trace, start_memory_measurement, start_release_trace,
    start_release_trace_from_env, take_release_events,
};
/// 重导出 22C Runtime/线程/任务池 API。
pub use scheduler::{
    RuntimeCancellation, RuntimeExecError, RuntimeHandle, TaskHandle, TaskPool, ThreadHandle,
    detached_reaper_queue_depth, spawn, take_detached_errors,
};
/// 重导出 XCON v1 编解码和边界常量。
pub use serialization::{
    XCON_MAGIC, XCON_MAX_DEPTH, XCON_MAX_FRAME_BYTES, XCON_MAX_NODES, XCON_MAX_VALUE_BYTES,
    XCON_VERSION, XconError, XconValue, decode_xcon, encode_xcon,
};
/// 重导出表定义、钩子和生命周期状态。
pub use tables::{
    TableDefinition, TableDropHook, TableDropView, TableHooks, TableInitHook, TableInstance,
    TableObject, TableState,
};
/// 重导出释放计划测试驱动器和展开结果。
pub use testing::{CatchRoute, ReleaseExecution, RuntimeBinding, RuntimeDriver, UnwindExecution};
/// 重导出 Runtime 标量和字符串值。
pub use value::{ExternalResourceHandle, ExternalResourceKind, RuntimeValue, StringHandle};
/// 重导出 VM 并发句柄适配。
pub use vm_concurrency::{
    VM_DETACHED_ERROR_CAPACITY, VmConcurrencyValue, VmDetachedCompletionTracker, VmDetachedError,
    VmExecutionError, VmExecutionResult, VmTransferValue, execution_error,
    restore_vm_detached_error, take_vm_detached_error_drops, take_vm_detached_errors,
    take_vm_detached_errors_for, transfer_error,
};

/// 安装当前平台的最小未处理异常报告器。
///
/// 原生 ABI 入口会自动调用此初始化；宿主集成测试和嵌入式启动器也可以显式提前安装，
/// 但平台异常仍然只会生成 Fatal 机器报告，不会进入 Xiao `catch`。
pub fn install_platform_failure_reporter() {
    crash::install_platform_failure_reporter();
}
