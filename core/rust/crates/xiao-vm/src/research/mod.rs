//! 09R2 研究子模块：三地址解释器与事件接收器。
//!
//! 本子模块是为 09R 特别研究工程建立的原型，**不是稳定执行接口**。三种候选
//! 机型在 09R3 冻结前都不得被当作生产 VM 暴露。
//!
//! 分层是刻意的：`semantics/` 是机型无关的语义核，`carrier.rs` 是它唯一依赖
//! 的窄接口，`machine/` 下每个文件是一种载体实现。批次 2 增加寄存器机型时只
//! 需要新增载体，语义核零改动。

/// 语义核与值载体之间的窄接口。
pub mod carrier;
/// 调用帧与运行时作用域栈。
pub mod frame;
/// 候选机型的载体实现。
pub mod machine;
/// 值运算的薄包装，语义核经它调用 Runtime 算子表。
pub mod ops;
/// 运行入口、结构化结果与运行指标。
pub mod run;
/// 机型无关的三地址语义核。
pub mod semantics;
/// 调试事件接收器。
pub mod sink;

/// 重导出载体接口。
pub use carrier::Carrier;
/// 重导出调用帧类型。
pub use frame::Frame;
/// 重导出栈式载体。
pub use machine::stack::StackCarrier;
/// 重导出运行入口、结果与指标。
pub use run::{RunOutcome, RunResult, VmMetrics, VmOptions, run};
/// 重导出语义核与终止原因。
pub use semantics::{Fault, Vm};
/// 重导出调试事件与接收器。
pub use sink::{NullSink, RecordingSink, VmEvent, VmEventSink};
