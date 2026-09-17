//! 调用帧。
//!
//! 帧承载一个函数的载体、运行时作用域栈和返回去向。值本身不在这里，而在
//! 载体内；帧只保存与机型无关的记账信息。

use xiao_bytecode::research::{FuncId, VReg};

use crate::research::carrier::Carrier;

/// 一个执行中的调用帧。
#[derive(Debug)]
pub struct Frame<C: Carrier> {
    /// 对应函数索引。
    pub function: FuncId,
    /// 函数名，用于事件与堆栈。
    pub name: String,
    /// 本帧的值载体。
    pub carrier: C,
    /// 运行时作用域栈，由 `EnterScope`/`ExitScope` 维护。
    pub scopes: Vec<u32>,
    /// 调用方接收返回值的寄存器。
    pub return_to: Option<VReg>,
}

impl<C: Carrier> Frame<C> {
    /// 创建一个空帧。
    #[must_use]
    pub fn new(function: FuncId, name: String, carrier: C, return_to: Option<VReg>) -> Self {
        Self {
            function,
            name,
            carrier,
            scopes: Vec::new(),
            return_to,
        }
    }

    /// 返回当前最内层作用域。
    #[must_use]
    pub fn innermost_scope(&self) -> Option<u32> {
        self.scopes.last().copied()
    }
}
