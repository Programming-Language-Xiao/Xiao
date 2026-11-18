//! 分类型寄存器机型载体。
//!
//! 虚拟寄存器先按 TAC 冻结类别分组，再用机型中立活跃区间做确定性线性扫描。
//! `Poly` 与释放计划引用的值固定落在帧槽，`None` 使用零宽位置；运行时值的
//! 具体变体不参与任何类别或布局判断。

use std::collections::{BTreeMap, BTreeSet};

use xiao_bytecode::{
    LiveInterval, RegisterClass, TacFunction, TacReleasePlan, VReg, analyze_liveness,
};
use xiao_runtime::{RuntimeResult, RuntimeValue};

use crate::carrier::{Carrier, CarrierContext, CarrierMetrics, empty_register_error};

/// 一个虚拟寄存器在分类型载体中的稳定物理位置。
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum RegisterLocation {
    /// 整数寄存器文件中的下标。
    Int(usize),
    /// 浮点寄存器文件中的下标。
    Float(usize),
    /// 布尔寄存器文件中的下标。
    Bool(usize),
    /// 对象句柄寄存器文件中的下标。
    ObjHandle(usize),
    /// 动态值寄存器文件中的下标。
    Dynamic(usize),
    /// 非易失帧槽中的下标。
    Frame(usize),
    /// 不占任何物理位置的 `none`。
    ZeroWidth,
}

/// 一次跨调用保存的易失寄存器快照。
#[derive(Clone, Debug)]
struct CallSnapshot {
    /// 整数寄存器文件。
    ints: Vec<Option<RuntimeValue>>,
    /// 浮点寄存器文件。
    floats: Vec<Option<RuntimeValue>>,
    /// 布尔寄存器文件。
    bools: Vec<Option<RuntimeValue>>,
    /// 对象句柄寄存器文件。
    objects: Vec<Option<RuntimeValue>>,
    /// 动态值寄存器文件。
    dynamics: Vec<Option<RuntimeValue>>,
}

/// 以分类型寄存器文件和独立帧槽承载虚拟寄存器。
#[derive(Debug)]
pub struct RegisterCarrier {
    allocations: Vec<RegisterLocation>,
    ints: Vec<Option<RuntimeValue>>,
    floats: Vec<Option<RuntimeValue>>,
    bools: Vec<Option<RuntimeValue>>,
    objects: Vec<Option<RuntimeValue>>,
    dynamics: Vec<Option<RuntimeValue>>,
    frame_slots: Vec<Option<RuntimeValue>>,
    zero_width: BTreeSet<VReg>,
    call_snapshots: Vec<CallSnapshot>,
    metrics: CarrierMetrics,
}

impl RegisterCarrier {
    /// 返回一个虚拟寄存器的确定性分配结果。
    #[must_use]
    pub fn allocation(&self, register: VReg) -> Option<RegisterLocation> {
        self.allocations.get(register.get() as usize).copied()
    }

    /// 返回全部分配结果，索引就是 `VReg` 编号。
    #[must_use]
    pub fn allocations(&self) -> &[RegisterLocation] {
        &self.allocations
    }

    /// 返回当前实际占用的物理位置数量。
    #[must_use]
    pub fn occupied(&self) -> usize {
        self.ints
            .iter()
            .chain(&self.floats)
            .chain(&self.bools)
            .chain(&self.objects)
            .chain(&self.dynamics)
            .chain(&self.frame_slots)
            .filter(|slot| slot.is_some())
            .count()
    }

    /// 为超出静态类别表的手工 TAC 保守追加一个帧槽。
    fn ensure_allocation(&mut self, register: VReg) -> RegisterLocation {
        let index = register.get() as usize;
        while self.allocations.len() <= index {
            let slot = self.frame_slots.len();
            self.frame_slots.push(None);
            self.allocations.push(RegisterLocation::Frame(slot));
        }
        self.allocations[index]
    }

    /// 读取一个已分配位置。
    fn slot(&self, location: RegisterLocation) -> Option<&RuntimeValue> {
        match location {
            RegisterLocation::Int(index) => self.ints.get(index)?.as_ref(),
            RegisterLocation::Float(index) => self.floats.get(index)?.as_ref(),
            RegisterLocation::Bool(index) => self.bools.get(index)?.as_ref(),
            RegisterLocation::ObjHandle(index) => self.objects.get(index)?.as_ref(),
            RegisterLocation::Dynamic(index) => self.dynamics.get(index)?.as_ref(),
            RegisterLocation::Frame(index) => self.frame_slots.get(index)?.as_ref(),
            RegisterLocation::ZeroWidth => None,
        }
    }

    /// 可变读取一个已分配位置。
    fn slot_mut(&mut self, location: RegisterLocation) -> Option<&mut Option<RuntimeValue>> {
        match location {
            RegisterLocation::Int(index) => self.ints.get_mut(index),
            RegisterLocation::Float(index) => self.floats.get_mut(index),
            RegisterLocation::Bool(index) => self.bools.get_mut(index),
            RegisterLocation::ObjHandle(index) => self.objects.get_mut(index),
            RegisterLocation::Dynamic(index) => self.dynamics.get_mut(index),
            RegisterLocation::Frame(index) => self.frame_slots.get_mut(index),
            RegisterLocation::ZeroWidth => None,
        }
    }

    /// 把当前易失寄存器文件复制成一次调用保存快照。
    fn snapshot(&self) -> CallSnapshot {
        CallSnapshot {
            ints: self.ints.clone(),
            floats: self.floats.clone(),
            bools: self.bools.clone(),
            objects: self.objects.clone(),
            dynamics: self.dynamics.clone(),
        }
    }

    /// 恢复一次调用保存快照。
    fn restore(&mut self, snapshot: CallSnapshot) {
        self.ints = snapshot.ints;
        self.floats = snapshot.floats;
        self.bools = snapshot.bools;
        self.objects = snapshot.objects;
        self.dynamics = snapshot.dynamics;
    }
}

impl Carrier for RegisterCarrier {
    /// 按逐函数类别、活跃区间与释放计划建立分配。
    fn empty(context: CarrierContext<'_>) -> Self {
        let allocations = allocate_locations(context.function, &context.program.plans);
        let mut sizes = RegisterFileSizes::default();
        for location in &allocations {
            sizes.include(*location);
        }
        Self {
            allocations,
            ints: empty_slots(sizes.ints),
            floats: empty_slots(sizes.floats),
            bools: empty_slots(sizes.bools),
            objects: empty_slots(sizes.objects),
            dynamics: empty_slots(sizes.dynamics),
            frame_slots: empty_slots(sizes.frames),
            zero_width: BTreeSet::new(),
            call_snapshots: Vec::new(),
            // 分类型寄存器式**不需要任何栈映射点**（R1-F 的冻结口径是「无」）：
            // 帧槽由函数布局静态描述，槽位含义不随 pc 变化，跳转目标、调用点和
            // 帧尾都不需要额外描述。因此本类型刻意不覆写 `Carrier::map_point`，
            // `stack_map_entries` 恒为 0。它落帧槽的开销记在 `spill_count`、
            // 跨调用保存的开销记在 `call_save_count`——两者都不要混进映射点数，
            // 否则三机型就不再同量纲。
            metrics: CarrierMetrics::default(),
        }
    }

    /// 从已分配的位置读取值。
    fn read(&self, register: VReg) -> RuntimeResult<RuntimeValue> {
        let Some(location) = self.allocation(register) else {
            return Err(empty_register_error(register));
        };
        if matches!(location, RegisterLocation::ZeroWidth) {
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

    /// 写入已分配的位置并更新帧槽与占用指标。
    fn write(&mut self, register: VReg, value: RuntimeValue) {
        let location = self.ensure_allocation(register);
        if matches!(location, RegisterLocation::ZeroWidth) {
            self.zero_width.insert(register);
        } else if let Some(slot) = self.slot_mut(location) {
            *slot = Some(value);
            if matches!(location, RegisterLocation::Frame(_)) {
                self.metrics.spill_count = self.metrics.spill_count.saturating_add(1);
            }
        }
        self.metrics.peak_occupancy = self.metrics.peak_occupancy.max(self.occupied());
    }

    /// 取出并清空一个已分配位置。
    fn take(&mut self, register: VReg) -> Option<RuntimeValue> {
        let location = self.allocation(register)?;
        if matches!(location, RegisterLocation::ZeroWidth) {
            return self
                .zero_width
                .remove(&register)
                .then_some(RuntimeValue::None);
        }
        self.slot_mut(location)?.take()
    }

    /// 保存当前易失寄存器文件。
    ///
    /// 保存个数只记入 `call_save_count`：它是跨调用保存的**开销**，不是需要
    /// 栈映射的**程序点**。两者量纲不同，混在一起会让本机型在 09R3 的
    /// 「需要的栈映射位置」这一轴上与另两种机型不可比。
    fn begin_call(&mut self) {
        let saved = self
            .ints
            .iter()
            .chain(&self.floats)
            .chain(&self.bools)
            .chain(&self.objects)
            .chain(&self.dynamics)
            .filter(|slot| slot.is_some())
            .count();
        self.metrics.call_save_count = self.metrics.call_save_count.saturating_add(saved as u64);
        self.call_snapshots.push(self.snapshot());
    }

    /// 恢复最近一次易失寄存器文件快照。
    fn end_call(&mut self) {
        if let Some(snapshot) = self.call_snapshots.pop() {
            self.restore(snapshot);
        }
    }

    /// 返回分类型载体指标快照。
    fn metrics(&self) -> CarrierMetrics {
        self.metrics
    }
}

/// 各物理区域需要的槽位数量。
#[derive(Clone, Copy, Debug, Default)]
struct RegisterFileSizes {
    /// 整数槽位。
    ints: usize,
    /// 浮点槽位。
    floats: usize,
    /// 布尔槽位。
    bools: usize,
    /// 对象槽位。
    objects: usize,
    /// 动态槽位。
    dynamics: usize,
    /// 帧槽。
    frames: usize,
}

impl RegisterFileSizes {
    /// 把一个物理位置纳入容量上界。
    fn include(&mut self, location: RegisterLocation) {
        let size = |index: usize| index.saturating_add(1);
        match location {
            RegisterLocation::Int(index) => self.ints = self.ints.max(size(index)),
            RegisterLocation::Float(index) => self.floats = self.floats.max(size(index)),
            RegisterLocation::Bool(index) => self.bools = self.bools.max(size(index)),
            RegisterLocation::ObjHandle(index) => self.objects = self.objects.max(size(index)),
            RegisterLocation::Dynamic(index) => self.dynamics = self.dynamics.max(size(index)),
            RegisterLocation::Frame(index) => self.frames = self.frames.max(size(index)),
            RegisterLocation::ZeroWidth => {}
        }
    }
}

/// 创建指定长度的空槽位数组。
fn empty_slots(length: usize) -> Vec<Option<RuntimeValue>> {
    std::iter::repeat_with(|| None).take(length).collect()
}

/// 按类别与活跃区间生成确定性物理位置表。
fn allocate_locations(function: &TacFunction, plans: &[TacReleasePlan]) -> Vec<RegisterLocation> {
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
    let mut allocations = vec![RegisterLocation::ZeroWidth; function.categories.len()];
    let mut frame = 0usize;

    for (raw, allocation) in allocations.iter_mut().enumerate() {
        let register = VReg::new(raw as u32);
        let class = function.categories.get(register);
        if matches!(class, RegisterClass::None) {
            *allocation = RegisterLocation::ZeroWidth;
        } else if matches!(class, RegisterClass::Poly) || pinned.contains(&register) {
            *allocation = RegisterLocation::Frame(frame);
            frame = frame.saturating_add(1);
        }
    }

    for class in [
        RegisterClass::Int,
        RegisterClass::Float,
        RegisterClass::Bool,
        RegisterClass::ObjHandle,
        RegisterClass::Dynamic,
    ] {
        let mut class_intervals = (0..function.categories.len())
            .map(|raw| VReg::new(raw as u32))
            .filter(|register| {
                function.categories.get(*register) == class && !pinned.contains(register)
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
        class_intervals.sort_by_key(|(register, interval)| (interval.start, register.get()));
        let assigned = linear_scan(&class_intervals);
        for (register, slot) in assigned {
            allocations[register.get() as usize] = location_for_class(class, slot);
        }
    }
    allocations
}

/// 对同一类别的一组区间执行稳定线性扫描。
fn linear_scan(intervals: &[(VReg, LiveInterval)]) -> BTreeMap<VReg, usize> {
    let mut result = BTreeMap::new();
    let mut active = Vec::<(usize, VReg, usize)>::new();
    let mut free = BTreeSet::<usize>::new();
    let mut next = 0usize;
    for (register, interval) in intervals {
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
        result.insert(*register, slot);
        active.push((interval.end, *register, slot));
        active.sort_by_key(|(end, register, _)| (*end, register.get()));
    }
    result
}

/// 把 TAC 类别和类内下标组合成物理位置。
fn location_for_class(class: RegisterClass, slot: usize) -> RegisterLocation {
    match class {
        RegisterClass::Int => RegisterLocation::Int(slot),
        RegisterClass::Float => RegisterLocation::Float(slot),
        RegisterClass::Bool => RegisterLocation::Bool(slot),
        RegisterClass::ObjHandle => RegisterLocation::ObjHandle(slot),
        RegisterClass::Dynamic => RegisterLocation::Dynamic(slot),
        RegisterClass::None => RegisterLocation::ZeroWidth,
        RegisterClass::Poly => RegisterLocation::Frame(slot),
    }
}

/// `RegisterCarrier` 的显式长名称。
pub type TypedRegisterCarrier = RegisterCarrier;

#[cfg(test)]
/// 分类型分配器的确定性、复用与特殊类别回归。
mod tests {
    use super::{RegisterLocation, allocate_locations};
    use xiao_bytecode::{
        BlockId, CategoryMap, RegisterClass, TacBlock, TacFunction, TacInstr, TacOp, VReg,
    };
    use xiao_ir::IrSpan;

    /// 构造单块测试函数。
    fn function(categories: CategoryMap, instructions: Vec<TacInstr>) -> TacFunction {
        TacFunction {
            name: String::new(),
            signature: None,
            entry: BlockId::new(0),
            blocks: vec![TacBlock {
                id: BlockId::new(0),
                scope: 0,
                instructions,
            }],
            parameters: Vec::new(),
            locals: Vec::new(),
            categories,
            scopes: vec![0],
            handlers: Vec::new(),
            value_registers: BTreeMap::new(),
            span: IrSpan::new(0, 1),
        }
    }

    use std::collections::BTreeMap;

    #[test]
    /// 相接但不重叠的同类区间应复用同一个物理寄存器。
    fn reuses_non_overlapping_registers_deterministically() {
        let span = IrSpan::new(0, 1);
        let first = VReg::new(0);
        let second = VReg::new(1);
        let mut categories = CategoryMap::new();
        categories.insert(first, RegisterClass::Int);
        categories.insert(second, RegisterClass::Int);
        let function = function(
            categories,
            vec![
                TacInstr::with_dst(TacOp::LoadNone, first, span),
                TacInstr::new(TacOp::Transfer { value: first }, span),
                TacInstr::with_dst(TacOp::LoadNone, second, span),
                TacInstr::new(
                    TacOp::Return {
                        value: Some(second),
                    },
                    span,
                ),
            ],
        );
        let left = allocate_locations(&function, &[]);
        let right = allocate_locations(&function, &[]);
        assert_eq!(left, right);
        assert_eq!(left[0], RegisterLocation::Int(0));
        assert_eq!(left[1], RegisterLocation::Int(0));
    }

    #[test]
    /// `Poly` 固定落帧槽，`None` 始终走零宽路径。
    fn gives_poly_and_none_dedicated_paths() {
        let mut categories = CategoryMap::new();
        categories.insert(VReg::new(0), RegisterClass::Poly);
        categories.insert(VReg::new(1), RegisterClass::None);
        let function = function(categories, Vec::new());
        let allocations = allocate_locations(&function, &[]);
        assert_eq!(allocations[0], RegisterLocation::Frame(0));
        assert_eq!(allocations[1], RegisterLocation::ZeroWidth);
    }

    #[test]
    /// 冻结释放计划仍会读取的值必须放在非易失帧槽。
    fn pins_release_plan_values_in_frame_slots() {
        let value = VReg::new(0);
        let mut categories = CategoryMap::new();
        categories.insert(value, RegisterClass::ObjHandle);
        let mut function = function(categories, Vec::new());
        function.value_registers.insert(11, value);
        let plan = xiao_bytecode::TacReleasePlan {
            scope: 0,
            exit: "normal".to_owned(),
            actions: vec![xiao_bytecode::TacReleaseAction {
                value: 11,
                order: 0,
                kind: xiao_lifetime::ReleaseActionKind::Strong,
            }],
            transferred: Vec::new(),
        };
        let allocations = allocate_locations(&function, &[plan]);
        assert_eq!(allocations[0], RegisterLocation::Frame(0));
    }
}
