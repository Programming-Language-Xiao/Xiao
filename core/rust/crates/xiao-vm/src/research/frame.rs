//! 调用帧。
//!
//! 帧承载一个函数的载体、运行时作用域栈和返回去向。值本身不在这里，而在
//! 载体内；帧只保存与机型无关的记账信息。

use xiao_bytecode::research::{BlockId, FuncId, VReg};

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
    /// 正在执行的 `finally` 子程序退出类别栈。
    pub pending_exits: Vec<String>,
    /// 当前正在执行的 finally 子程序入口栈，用于错误路由定位来源。
    pub active_subroutines: Vec<BlockId>,
    /// 尚待外层路由消费的子程序故障来源栈。
    ///
    /// 每层 `run_subroutine` 只登记自己的入口；嵌套子程序的来源由内层路由
    /// 先消费，因而不会被外层入口覆盖。
    pub subroutine_faults: Vec<BlockId>,
    /// 最近命中且仍在执行体内的 catch 上下文。
    ///
    /// 元组保存 `(try 作用域, catch 入口块)`。try 作用域在进入 catch 前
    /// 已经退出，但 catch 体再次出错时仍需要找到对应的 finally。
    pub active_catches: Vec<(u32, BlockId)>,
    /// 当前动态执行轮次已经完成的 `(try 作用域, finally 子程序)`。
    ///
    /// 处理器路由会先执行 finally 再进入 catch；该记账防止 catch 体的
    /// 后续错误把同一份 finally 再跑一次。
    pub completed_finally: Vec<(u32, BlockId)>,
    /// 运行时检查失败跳转携带的检查类别。
    pub pending_check_kind: Option<String>,
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
            pending_exits: Vec::new(),
            active_subroutines: Vec::new(),
            subroutine_faults: Vec::new(),
            active_catches: Vec::new(),
            completed_finally: Vec::new(),
            pending_check_kind: None,
        }
    }

    /// 返回当前最内层作用域。
    #[must_use]
    pub fn innermost_scope(&self) -> Option<u32> {
        self.scopes.last().copied()
    }
}
