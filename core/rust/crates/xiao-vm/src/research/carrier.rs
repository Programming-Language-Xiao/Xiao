//! 值载体接口：语义核与「值放在哪里」之间唯一的窄边界。
//!
//! 语义核只通过本接口读写值，因此它不认识栈、寄存器或帧槽。三种候选机型的
//! 差异全部落在本接口的实现里。接口刻意保持极小：新增方法等于给所有机型加
//! 负担，也会把机型细节泄回语义核。

use xiao_bytecode::research::VReg;
use xiao_runtime::{RuntimeResult, RuntimeValue};

/// 存放虚拟寄存器值的载体。
pub trait Carrier: Sized {
    /// 创建一个空的帧载体。
    ///
    /// 语义核按此建立新帧，因此它不需要知道机型如何分配槽位或寄存器。
    fn empty() -> Self;

    /// 只读读取一个寄存器；寄存器为空时报错。
    fn read(&self, register: VReg) -> RuntimeResult<RuntimeValue>;

    /// 写入一个寄存器，覆盖原有内容。
    fn write(&mut self, register: VReg, value: RuntimeValue);

    /// 取出并清空一个寄存器；为空时返回 `None`。
    ///
    /// 「取出」与「读取」分开表达，是为了让所有权转移和引用计数语义在接口上
    /// 可见：移动用 `take`，只读参与运算用 `read`。
    fn take(&mut self, register: VReg) -> Option<RuntimeValue>;

    /// 返回当前已占用的载体槽位数量。
    fn depth(&self) -> usize;

    /// 返回载体槽位的历史峰值。
    fn peak(&self) -> usize;
}
