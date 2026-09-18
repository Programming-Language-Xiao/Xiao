//! 栈式机型载体。
//!
//! 虚拟寄存器直接映射为帧内槽位下标：`dst` 是写入槽位，源操作数从槽位读取。
//! 这与寄存器机型的差别只在槽位如何分配，不在语义。

use xiao_bytecode::research::VReg;
use xiao_runtime::{RuntimeResult, RuntimeValue};

use crate::research::carrier::{
    Carrier, CarrierContext, CarrierMetrics, MapPoint, empty_register_error,
};

/// 以一个槽位数组承载全部虚拟寄存器。
#[derive(Debug, Default)]
pub struct StackCarrier {
    slots: Vec<Option<RuntimeValue>>,
    peak: usize,
    stack_map_entries: usize,
}

impl StackCarrier {
    /// 创建一个空载体。
    #[must_use]
    pub const fn new() -> Self {
        Self {
            slots: Vec::new(),
            peak: 0,
            stack_map_entries: 0,
        }
    }

    /// 返回指定槽位的只读引用。
    #[must_use]
    pub fn slot(&self, register: VReg) -> Option<&RuntimeValue> {
        self.slots.get(register.get() as usize)?.as_ref()
    }

    /// 返回当前有效槽位的数量。
    #[must_use]
    pub fn occupied(&self) -> usize {
        self.slots.iter().filter(|slot| slot.is_some()).count()
    }
}

impl Carrier for StackCarrier {
    /// 创建一个空载体；栈式布局无需读取函数上下文。
    fn empty(_context: CarrierContext<'_>) -> Self {
        Self::new()
    }

    /// 读取槽位内容；越界或为空都报无效句柄错误。
    fn read(&self, register: VReg) -> RuntimeResult<RuntimeValue> {
        self.slot(register)
            .cloned()
            .ok_or_else(|| empty_register_error(register))
    }

    /// 写入槽位并更新峰值。
    fn write(&mut self, register: VReg, value: RuntimeValue) {
        let index = register.get() as usize;
        if self.slots.len() <= index {
            self.slots.resize_with(index + 1, || None);
        }
        self.slots[index] = Some(value);
        self.peak = self.peak.max(self.occupied());
    }

    /// 取出槽位内容并清空它。
    fn take(&mut self, register: VReg) -> Option<RuntimeValue> {
        self.slots.get_mut(register.get() as usize)?.take()
    }

    /// 栈式载体无需跨调用保存值：槽位就是帧内下标，不随调用变化。
    fn begin_call(&mut self) {}

    /// 栈式在**每个跳转目标**都需要可验证的栈深。
    ///
    /// 这是 R1-F 冻结的机型分界。调用点与帧尾**不计入**：调用不是 TAC 的块
    /// 跳转，被调帧的形参由签名决定而不是由本帧栈深描述；把它们也算进来会让
    /// 栈式与混合式在同一口径下重复计数，三种机型就不再可比。
    fn map_point(&mut self, point: MapPoint) {
        if matches!(point, MapPoint::JumpTarget) {
            self.stack_map_entries = self.stack_map_entries.saturating_add(1);
        }
    }

    /// 返回栈式载体的指标快照。
    fn metrics(&self) -> CarrierMetrics {
        CarrierMetrics {
            peak_occupancy: self.peak,
            stack_map_entries: self.stack_map_entries,
            ..CarrierMetrics::default()
        }
    }
}
