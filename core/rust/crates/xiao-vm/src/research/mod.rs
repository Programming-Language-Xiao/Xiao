//! 09R 冻结产物的兼容重导出层。
//!
//! 实质实现位于 crate 根的生产模块；这里保留 `research::...` 全部既有路径，
//! 供研究期三机型向量和基准设施使用。新生产代码应直接依赖父 crate 根路径。

/// 兼容暴露生产载体接口。
pub use crate::carrier;
/// 兼容暴露生产调用帧。
pub use crate::frame;
/// 兼容暴露生产机型模块。
pub use crate::machine;
/// 兼容暴露生产值运算模块。
pub use crate::ops;
/// 兼容暴露生产运行入口。
pub mod run {
    /// 兼容暴露生产运行入口的全部公开项。
    pub use crate::run::*;
}
/// 兼容暴露生产语义核。
pub use crate::semantics;
/// 兼容暴露生产事件接收器。
pub use crate::sink;

/// 兼容暴露生产 VM 的根级类型、常量和运行函数。
pub use crate::{
    Carrier, CarrierContext, CarrierMetrics, Fault, Frame, HYBRID_WINDOW_CAPACITY, HybridCarrier,
    HybridLocation, MapPoint, NullSink, RecordingSink, RegisterCarrier, RegisterLocation,
    RunOutcome, RunResult, StackCarrier, TypedRegisterCarrier, Vm, VmEvent, VmEventSink, VmMetrics,
    VmOptions, WindowStackCarrier, empty_register_error, run_hybrid, run_register, run_with,
    run_with_machine_seed, run_with_seed, run_with_values,
};

/// 兼容保留运行函数的旧根路径。
pub use run::run;
