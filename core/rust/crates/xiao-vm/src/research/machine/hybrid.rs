//! 混合式窗口/求值栈载体。
//!
//! 具名值按 `TacFunction.locals` 已冻结的声明顺序进入局部窗口；匿名临时值
//! 使用共享活跃区间在求值栈槽中复用。窗口超过固定容量时才落帧槽，调用点的
//! 栈映射计数与溢出计数始终分开。

use std::collections::{BTreeMap, BTreeSet};

use xiao_bytecode::research::{
    LiveInterval, RegisterClass, TacFunction, TacReleasePlan, VReg, analyze_liveness,
};
use xiao_runtime::{RuntimeResult, RuntimeValue};

use crate::research::carrier::{Carrier, CarrierContext, CarrierMetrics, empty_register_error};

/// 混合式载体的固定局部窗口容量。
pub const HYBRID_WINDOW_CAPACITY: usize = 8;

/// 一个虚拟寄存器在混合式载体中的位置。
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum HybridLocation {
    /// 固定局部窗口下标。
    Window(usize),
    /// 匿名求值栈槽下标。
    EvalStack(usize),
    /// 非易失帧槽下标。
    Frame(usize),
    /// 零宽 `none`。
    ZeroWidth,
}

/// 一次混合式调用保存快照。
#[derive(Clone, Debug)]
struct CallSnapshot {
    /// 局部窗口。
    window: Vec<Option<RuntimeValue>>,
    /// 求值栈槽。
    eval_stack: Vec<Option<RuntimeValue>>,
}

/// 以局部窗口、求值栈和帧槽承载虚拟寄存器。
#[derive(Debug)]
pub struct HybridCarrier {
    allocations: Vec<HybridLocation>,
    window: Vec<Option<RuntimeValue>>,
    eval_stack: Vec<Option<RuntimeValue>>,
    frame_slots: Vec<Option<RuntimeValue>>,
    zero_width: BTreeSet<VReg>,
    call_snapshots: Vec<CallSnapshot>,
    metrics: CarrierMetrics,
    parameter_window: usize,
}

impl HybridCarrier {
    /// 返回一个虚拟寄存器的稳定位置。
    #[must_use]
    pub fn allocation(&self, register: VReg) -> Option<HybridLocation> {
        self.allocations.get(register.get() as usize).copied()
    }

    /// 返回具名参数在局部窗口中的连续前缀长度。
    #[must_use]
    pub const fn parameter_window_len(&self) -> usize {
        self.parameter_window
    }

    /// 返回当前窗口与求值栈中有效值的数量。
    #[must_use]
    pub fn eval_occupancy(&self) -> usize {
        self.window
            .iter()
            .chain(&self.eval_stack)
            .filter(|slot| slot.is_some())
            .count()
    }

    /// 为未知手工寄存器保守追加帧槽。
    fn ensure_allocation(&mut self, register: VReg) -> HybridLocation {
        let index = register.get() as usize;
        while self.allocations.len() <= index {
            let slot = self.frame_slots.len();
            self.frame_slots.push(None);
            self.allocations.push(HybridLocation::Frame(slot));
        }
        self.allocations[index]
    }

    /// 读取一个物理位置。
    fn slot(&self, location: HybridLocation) -> Option<&RuntimeValue> {
        match location {
            HybridLocation::Window(index) => self.window.get(index)?.as_ref(),
            HybridLocation::EvalStack(index) => self.eval_stack.get(index)?.as_ref(),
            HybridLocation::Frame(index) => self.frame_slots.get(index)?.as_ref(),
            HybridLocation::ZeroWidth => None,
        }
    }

    /// 可变读取一个物理位置。
    fn slot_mut(&mut self, location: HybridLocation) -> Option<&mut Option<RuntimeValue>> {
        match location {
            HybridLocation::Window(index) => self.window.get_mut(index),
            HybridLocation::EvalStack(index) => self.eval_stack.get_mut(index),
            HybridLocation::Frame(index) => self.frame_slots.get_mut(index),
            HybridLocation::ZeroWidth => None,
        }
    }

    /// 复制窗口与求值栈，供嵌套调用保存。
    fn snapshot(&self) -> CallSnapshot {
        CallSnapshot {
            window: self.window.clone(),
            eval_stack: self.eval_stack.clone(),
        }
    }

    /// 恢复最近一次调用保存。
    fn restore(&mut self, snapshot: CallSnapshot) {
        self.window = snapshot.window;
        self.eval_stack = snapshot.eval_stack;
    }
}

impl Carrier for HybridCarrier {
    /// 按具名窗口、临时求值栈和共享帧槽建立载体。
    fn empty(context: CarrierContext<'_>) -> Self {
        let allocations = allocate_locations(context.function, &context.program.plans);
        let window = allocations
            .iter()
            .filter_map(|location| match location {
                HybridLocation::Window(index) => Some(index.saturating_add(1)),
                _ => None,
            })
            .max()
            .unwrap_or(0);
        let eval_stack = allocations
            .iter()
            .filter_map(|location| match location {
                HybridLocation::EvalStack(index) => Some(index.saturating_add(1)),
                _ => None,
            })
            .max()
            .unwrap_or(0);
        let frame_slots = allocations
            .iter()
            .filter_map(|location| match location {
                HybridLocation::Frame(index) => Some(index.saturating_add(1)),
                _ => None,
            })
            .max()
            .unwrap_or(0);
        let parameter_window = context
            .function
            .parameters
            .iter()
            .take_while(|register| {
                matches!(
                    allocations.get(register.get() as usize),
                    Some(HybridLocation::Window(_))
                )
            })
            .count();
        Self {
            allocations,
            window: empty_slots(window),
            eval_stack: empty_slots(eval_stack),
            frame_slots: empty_slots(frame_slots),
            zero_width: BTreeSet::new(),
            call_snapshots: Vec::new(),
            metrics: CarrierMetrics::default(),
            parameter_window,
        }
    }

    /// 从窗口、求值栈或帧槽读取值。
    fn read(&self, register: VReg) -> RuntimeResult<RuntimeValue> {
        let Some(location) = self.allocation(register) else {
            return Err(empty_register_error(register));
        };
        if matches!(location, HybridLocation::ZeroWidth) {
            return self
                .zero_width
                .contains(&register)
                .then_some(RuntimeValue::None)
                .ok_or_else(|| empty_register_error(register));
        }
        self.slot(location)
            .cloned()
            .ok_or_else(|| empty_register_error(register))
    }

    /// 写入位置并区分窗口映射与帧槽溢出指标。
    fn write(&mut self, register: VReg, value: RuntimeValue) {
        let location = self.ensure_allocation(register);
        if matches!(location, HybridLocation::ZeroWidth) {
            self.zero_width.insert(register);
        } else if let Some(slot) = self.slot_mut(location) {
            *slot = Some(value);
            if matches!(location, HybridLocation::Frame(_)) {
                self.metrics.spill_count = self.metrics.spill_count.saturating_add(1);
            }
        }
        self.metrics.peak_occupancy = self.metrics.peak_occupancy.max(
            self.eval_occupancy()
                + self
                    .frame_slots
                    .iter()
                    .filter(|slot| slot.is_some())
                    .count(),
        );
    }

    /// 取出并清空一个位置。
    fn take(&mut self, register: VReg) -> Option<RuntimeValue> {
        let location = self.allocation(register)?;
        if matches!(location, HybridLocation::ZeroWidth) {
            return self
                .zero_width
                .remove(&register)
                .then_some(RuntimeValue::None);
        }
        self.slot_mut(location)?.take()
    }

    /// 保存窗口和求值栈，并为调用点增加一个独立映射计数。
    fn begin_call(&mut self) {
        let saved = self.eval_occupancy();
        self.metrics.call_save_count = self.metrics.call_save_count.saturating_add(saved as u64);
        self.metrics.stack_map_entries = self.metrics.stack_map_entries.saturating_add(1);
        self.call_snapshots.push(self.snapshot());
    }

    /// 恢复最近一次调用保存。
    fn end_call(&mut self) {
        if let Some(snapshot) = self.call_snapshots.pop() {
            self.restore(snapshot);
        }
    }

    /// 返回混合式载体的指标快照。
    fn metrics(&self) -> CarrierMetrics {
        self.metrics
    }
}

/// 创建空的可选值槽位。
fn empty_slots(length: usize) -> Vec<Option<RuntimeValue>> {
    std::iter::repeat_with(|| None).take(length).collect()
}

/// 按函数局部顺序和活跃区间分配混合式位置。
fn allocate_locations(function: &TacFunction, plans: &[TacReleasePlan]) -> Vec<HybridLocation> {
    let liveness = analyze_liveness(function, plans);
    let intervals = liveness
        .intervals
        .iter()
        .map(|interval| (interval.register, *interval))
        .collect::<BTreeMap<_, _>>();
    let pinned = plans
        .iter()
        .flat_map(|plan| &plan.actions)
        .filter_map(|action| function.value_registers.get(&action.value))
        .copied()
        .collect::<BTreeSet<_>>();
    let locals = function.locals.iter().copied().collect::<BTreeSet<_>>();
    let mut allocations = vec![HybridLocation::ZeroWidth; function.categories.len()];
    let mut frame = 0usize;
    let mut window = 0usize;
    for (raw, allocation) in allocations.iter_mut().enumerate() {
        let register = VReg::new(raw as u32);
        let class = function.categories.get(register);
        if matches!(class, RegisterClass::None) {
            *allocation = HybridLocation::ZeroWidth;
        } else if matches!(class, RegisterClass::Poly) || pinned.contains(&register) {
            *allocation = HybridLocation::Frame(frame);
            frame = frame.saturating_add(1);
        } else if locals.contains(&register) {
            if window < HYBRID_WINDOW_CAPACITY {
                *allocation = HybridLocation::Window(window);
                window = window.saturating_add(1);
            } else {
                *allocation = HybridLocation::Frame(frame);
                frame = frame.saturating_add(1);
            }
        }
    }

    let mut temporaries = (0..function.categories.len())
        .map(|raw| VReg::new(raw as u32))
        .filter(|register| {
            !locals.contains(register)
                && !pinned.contains(register)
                && !matches!(
                    function.categories.get(*register),
                    RegisterClass::None | RegisterClass::Poly
                )
        })
        .map(|register| {
            let interval = intervals.get(&register).copied().unwrap_or(LiveInterval {
                register,
                start: usize::MAX,
                end: usize::MAX,
            });
            (register, interval)
        })
        .collect::<Vec<_>>();
    temporaries.sort_by_key(|(register, interval)| (interval.start, register.get()));
    let mut active = Vec::<(usize, VReg, usize)>::new();
    let mut free = BTreeSet::<usize>::new();
    let mut next = 0usize;
    for (register, interval) in temporaries {
        let mut retained = Vec::with_capacity(active.len());
        for item in active.drain(..) {
            if item.0 <= interval.start {
                free.insert(item.2);
            } else {
                retained.push(item);
            }
        }
        active = retained;
        let slot = free.pop_first().unwrap_or_else(|| {
            let slot = next;
            next = next.saturating_add(1);
            slot
        });
        allocations[register.get() as usize] = HybridLocation::EvalStack(slot);
        active.push((interval.end, register, slot));
        active.sort_by_key(|(end, register, _)| (*end, register.get()));
    }
    allocations
}

/// `HybridCarrier` 的兼容别名。
pub type WindowStackCarrier = HybridCarrier;

#[cfg(test)]
/// 混合式窗口、临时栈与独立指标的回归。
mod tests {
    use super::{HYBRID_WINDOW_CAPACITY, HybridLocation, allocate_locations};
    use std::collections::BTreeMap;
    use xiao_bytecode::research::{
        BlockId, CategoryMap, RegisterClass, TacBlock, TacFunction, TacInstr, TacOp, VReg,
    };
    use xiao_ir::IrSpan;

    /// 构造单块测试函数。
    fn function(categories: CategoryMap, locals: Vec<VReg>) -> TacFunction {
        TacFunction {
            name: String::new(),
            signature: None,
            entry: BlockId::new(0),
            blocks: vec![TacBlock {
                id: BlockId::new(0),
                scope: 0,
                instructions: vec![TacInstr::new(
                    TacOp::Return { value: None },
                    IrSpan::new(0, 1),
                )],
            }],
            parameters: Vec::new(),
            locals,
            categories,
            scopes: vec![0],
            handlers: Vec::new(),
            value_registers: BTreeMap::new(),
            span: IrSpan::new(0, 1),
        }
    }

    #[test]
    /// 具名值按局部顺序进入窗口，匿名值进入求值栈。
    fn separates_named_window_and_anonymous_eval_stack() {
        let named = VReg::new(0);
        let temporary = VReg::new(1);
        let mut categories = CategoryMap::new();
        categories.insert(named, RegisterClass::Int);
        categories.insert(temporary, RegisterClass::Int);
        let function = function(categories, vec![named]);
        let locations = allocate_locations(&function, &[]);
        assert_eq!(locations[0], HybridLocation::Window(0));
        assert_eq!(locations[1], HybridLocation::EvalStack(0));
    }

    #[test]
    /// 超过窗口容量的具名值落帧槽，避免伪造无限窗口。
    fn spills_named_values_after_window_is_full() {
        let locals = (0..(HYBRID_WINDOW_CAPACITY as u32 + 1))
            .map(VReg::new)
            .collect::<Vec<_>>();
        let mut categories = CategoryMap::new();
        for register in &locals {
            categories.insert(*register, RegisterClass::Int);
        }
        let function = function(categories, locals);
        let locations = allocate_locations(&function, &[]);
        assert!(matches!(locations.last(), Some(HybridLocation::Frame(_))));
    }
}
