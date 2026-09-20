//! 机型无关的三地址语义核。

/// 解释循环、控制流、调用与错误展开。
mod exec;

/// 重导出解释器、入口实参与终止原因。
pub use self::exec::{BoundArgument, Fault, Vm};
