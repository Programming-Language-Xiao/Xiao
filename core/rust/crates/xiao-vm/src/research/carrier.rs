//! 值载体接口：语义核与「值放在哪里」之间唯一的窄边界。
//!
//! 语义核只通过本接口读写值，因此它不认识栈、寄存器或帧槽。三种候选机型的
//! 差异全部落在本接口的实现里。接口刻意保持极小：新增方法等于给所有机型加
//! 负担，也会把机型细节泄回语义核。

use xiao_bytecode::research::{CategoryMap, FuncId, TacFunction, TacProgram, VReg};
use xiao_runtime::{RuntimeError, RuntimeResult, RuntimeValue};

/// 创建一帧载体所需的机型中立上下文。
///
/// 上下文只借用 TAC 语义产物，不包含任何栈、窗口或物理寄存器字段。载体在
/// 构造期据此完成自己的布局，语义核不参与布局决策。
#[derive(Clone, Copy, Debug)]
pub struct CarrierContext<'a> {
    /// 整份 TAC 产物。
    pub program: &'a TacProgram,
    /// 当前函数编号。
    pub function_id: FuncId,
    /// 当前函数。
    pub function: &'a TacFunction,
    /// 当前函数局部编号空间内的类别表。
    pub categories: &'a CategoryMap,
    /// 新帧进入后的调用深度。
    pub call_depth: usize,
}

/// 一个载体向语义核上报的机型中立指标。
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct CarrierMetrics {
    /// 载体占用的历史峰值。
    pub peak_occupancy: usize,
    /// 写入独立帧槽的次数。
    pub spill_count: u64,
    /// 载体建立的可验证映射点数量。
    pub stack_map_entries: usize,
    /// 跨调用保存值的次数。
    pub call_save_count: u64,
}

/// 构造三种载体共用的空寄存器错误。
#[must_use]
pub fn empty_register_error(register: VReg) -> RuntimeError {
    RuntimeError::invalid_handle(format!("寄存器 {} 为空", register.get()))
}

/// 存放虚拟寄存器值的载体。
pub trait Carrier: Sized {
    /// 根据当前函数上下文创建一个空的帧载体。
    ///
    /// 语义核只提供机型中立信息，不参与槽位、窗口或寄存器布局。
    fn empty(context: CarrierContext<'_>) -> Self;

    /// 只读读取一个寄存器；寄存器为空时报错。
    fn read(&self, register: VReg) -> RuntimeResult<RuntimeValue>;

    /// 写入一个寄存器，覆盖原有内容。
    fn write(&mut self, register: VReg, value: RuntimeValue);

    /// 取出并清空一个寄存器；为空时返回 `None`。
    ///
    /// 「取出」与「读取」分开表达，是为了让所有权转移和引用计数语义在接口上
    /// 可见：移动用 `take`，只读参与运算用 `read`。
    fn take(&mut self, register: VReg) -> Option<RuntimeValue>;

    /// 在本帧调用另一个函数前保存需要跨调用存活的值。
    fn begin_call(&mut self) {}

    /// 在被调函数返回后恢复本帧保存的值。
    fn end_call(&mut self) {}

    /// 返回机型中立指标快照。
    fn metrics(&self) -> CarrierMetrics;
}
