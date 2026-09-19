//! Xiao 执行时值、内存和错误运行库的 crate 入口。
//!
//! 本阶段提供不透明对象头、单线程引用计数、字符串和静态签名表对象。
//! 字节码 VM、LLVM 后端、CLI 与并发调度器只应通过这里的稳定句柄接口接入，
//! 不得复制一套生命周期语义。

/// 数组、元组、字典表、字典列和集合的运行对象。
pub mod containers;
/// 稳定 Runtime 错误、原因链和结构化参数。
pub mod errors;
/// 不透明对象头、强/弱句柄和引用计数策略。
pub mod memory;
/// 由 05-C 签名驱动的表对象和状态机。
pub mod tables;
/// 仅供规格测试使用的释放计划驱动器。
pub mod testing;
/// 标量、字符串和统一 Runtime 值。
pub mod value;

/// 重导出容器句柄、字典形态和可哈希判定。
pub use containers::{ArrayHandle, DictHandle, DictKind, SetHandle, TupleHandle, is_hashable};
/// 重导出 Runtime 错误身份和结果别名。
pub use errors::{
    ALLOCATION_CODE, BackendLocation, CONTAINER_HASHABILITY_CODE, CONTAINER_INDEX_CODE,
    CONTAINER_KEY_CODE, CROSS_THREAD_CODE, DIVISION_BY_ZERO_CODE, DiagnosticParam,
    ErrorAccumulator, FATAL_CORRUPT_ARTIFACT_CODE, FATAL_HARDWARE_CODE, FATAL_INTERNAL_CODE,
    FATAL_OUT_OF_MEMORY_CODE, FATAL_RUNTIME_INVARIANT_CODE, FATAL_STACK_OVERFLOW_CODE, FatalError,
    FatalKind, FrameKind, INVALID_HANDLE_CODE, INVALID_VALUE_CODE, MessageRenderer,
    NUMERIC_OVERFLOW_CODE, PreviewRenderer, RANDOM_COUNT_CODE, RANDOM_SEED_CODE,
    REFCOUNT_INVARIANT_CODE, ReportClass, ReportRecord, RuntimeError, RuntimeErrorKind,
    RuntimeResult, SELECTOR_BOUNDS_CODE, SELECTOR_STEP_CODE, SET_COMPARISON_CODE,
    SET_MEMBERSHIP_CODE, SET_OPERATION_CODE, StackFrame, TABLE_DROP_CODE, TABLE_INIT_CODE,
    TABLE_STATE_CODE, TYPE_MISMATCH_CODE, USE_AFTER_RELEASE_CODE, WEAK_UPGRADE_CODE, XiaoError,
    XiaoErrorKind, XiaoResult,
};
/// 重导出对象头和强/弱句柄类型。
pub use memory::{
    CounterStrategyKind, NonAtomicRefCount, ObjectLayout, RefCountStrategy, RuntimeTypeTag,
    StrongHandle, WeakHandle,
};
/// 重导出表定义、钩子和生命周期状态。
pub use tables::{
    TableDefinition, TableDropHook, TableHooks, TableInitHook, TableInstance, TableObject,
    TableState,
};
/// 重导出释放计划测试驱动器和展开结果。
pub use testing::{
    CatchRoute, ReleaseEvent, ReleaseExecution, RuntimeBinding, RuntimeDriver, UnwindExecution,
};
/// 重导出 Runtime 标量和字符串值。
pub use value::{RuntimeValue, StringHandle};
