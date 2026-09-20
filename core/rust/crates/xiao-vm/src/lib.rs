//! Xiao Rust 字节码虚拟机。
//!
//! 09R 冻结的解释循环已经转为生产路径；`research` 子模块只保留兼容重导出，
//! 让研究期共享向量和三机型复现继续使用原路径。

/// 语义核与值载体之间的窄接口。
pub mod carrier;
/// 调用帧与运行时作用域栈。
pub mod frame;
/// 候选机型的载体实现；生产入口仍固定走栈式。
pub mod machine;
/// 值运算的薄包装，语义核经它调用 Runtime 算子表。
pub mod ops;
/// 运行入口、结构化结果与运行指标。
pub mod run;
/// 机型无关的三地址语义核。
pub mod semantics;
/// 调试事件接收器。
pub mod sink;

/// 重导出载体接口、构造上下文、映射点与指标。
pub use carrier::{Carrier, CarrierContext, CarrierMetrics, MapPoint, empty_register_error};
/// 重导出调用帧类型。
pub use frame::Frame;
/// 重导出混合式窗口/求值栈载体。
pub use machine::hybrid::{
    HYBRID_WINDOW_CAPACITY, HybridCarrier, HybridLocation, WindowStackCarrier,
};
/// 重导出分类型寄存器载体与物理位置。
pub use machine::register::{RegisterCarrier, RegisterLocation, TypedRegisterCarrier};
/// 重导出栈式载体。
pub use machine::stack::StackCarrier;
/// 重导出运行入口、结果与指标。
pub use run::{
    RunOutcome, RunResult, VmMetrics, VmOptions, run, run_hybrid, run_register, run_with,
    run_with_machine_seed, run_with_seed, run_with_values,
};
/// 重导出语义核与终止原因。
pub use semantics::{Fault, Vm};
/// 重导出调试事件与接收器。
pub use sink::{NullSink, RecordingSink, VmEvent, VmEventSink};

/// 09R 冻结产物的兼容重导出层；沿革见 09R2D，冻结依据见 09R3。
pub mod research;
