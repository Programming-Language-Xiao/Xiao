//! 机型无关的 TAC 活跃区间分析。
//!
//! 这里的结果只描述虚拟寄存器在语义 CFG 中何时可能被读取，物理寄存器、窗口
//! 和帧槽的选择留给载体。释放计划与异常 handler 都是数据流的一部分，不能被
//! 线性扫描悄略。

use std::collections::{BTreeMap, BTreeSet};

use crate::research::cfg::{protected_successors, successors};
use crate::research::lower::TacReleasePlan;
use crate::research::tac::{BlockId, TacFunction, TacInstr, TacOp, VReg};

/// 一个虚拟寄存器的半开活跃区间 `[start, end)`。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LiveInterval {
    /// 虚拟寄存器。
    pub register: VReg,
    /// 首次定义或使用的位置。
    pub start: usize,
    /// 最后一次使用之后的位置。
    pub end: usize,
}

impl LiveInterval {
    /// 判断两个区间是否有重叠。
    #[must_use]
    pub const fn overlaps(self, other: Self) -> bool {
        self.start < other.end && other.start < self.end
    }
}

/// 一个函数的活跃分析结果。
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Liveness {
    /// 按寄存器编号排列的区间。
    pub intervals: Vec<LiveInterval>,
    /// 每个块入口的活跃寄存器。
    pub live_in: BTreeMap<BlockId, Vec<VReg>>,
    /// 每个块出口的活跃寄存器。
    pub live_out: BTreeMap<BlockId, Vec<VReg>>,
    /// 每个块第一条指令在全函数线性位置中的偏移。
    pub block_starts: BTreeMap<BlockId, usize>,
    /// 每条指令的全函数线性起始位置，按块编号保存。
    pub instruction_positions: BTreeMap<BlockId, Vec<usize>>,
}

/// 对一个函数执行活跃区间分析。
///
/// `plans` 必须是同一份 `TacProgram.plans`，因为 `RunReleasePlan` 通过
/// `value_registers` 间接读取动作中的值。分析是纯函数，输入不被修改。
#[must_use]
pub fn analyze(function: &TacFunction, plans: &[TacReleasePlan]) -> Liveness {
    let mut cfg = successors(function);
    protected_successors(function, &mut cfg);

    let mut block_use = BTreeMap::<BlockId, BTreeSet<VReg>>::new();
    let mut block_def = BTreeMap::<BlockId, BTreeSet<VReg>>::new();
    let mut positions = BTreeMap::<BlockId, Vec<usize>>::new();
    let mut starts = BTreeMap::<BlockId, usize>::new();
    let mut position = 0usize;

    for block in &function.blocks {
        starts.insert(block.id, position);
        let mut uses = BTreeSet::new();
        let mut defs = BTreeSet::new();
        let mut block_positions = Vec::with_capacity(block.instructions.len());
        for instruction in &block.instructions {
            block_positions.push(position);
            let (instruction_uses, instruction_def) =
                instruction_use_def(function, instruction, plans);
            for register in instruction_uses {
                if !defs.contains(&register) {
                    uses.insert(register);
                }
            }
            defs.extend(instruction_def);
            position = position.saturating_add(1);
        }
        positions.insert(block.id, block_positions);
        block_use.insert(block.id, uses);
        block_def.insert(block.id, defs);
    }

    let mut live_in = function
        .blocks
        .iter()
        .map(|block| (block.id, BTreeSet::new()))
        .collect::<BTreeMap<_, _>>();
    let mut live_out = live_in.clone();
    loop {
        let mut changed = false;
        for block in function.blocks.iter().rev() {
            let mut out = BTreeSet::new();
            if let Some(targets) = cfg.get(&block.id) {
                for target in targets {
                    if let Some(values) = live_in.get(target) {
                        out.extend(values);
                    }
                }
            }
            let mut input = block_use.get(&block.id).cloned().unwrap_or_default();
            for register in &out {
                if !block_def
                    .get(&block.id)
                    .is_some_and(|defs| defs.contains(register))
                {
                    input.insert(*register);
                }
            }
            if live_out.get(&block.id) != Some(&out) {
                live_out.insert(block.id, out);
                changed = true;
            }
            if live_in.get(&block.id) != Some(&input) {
                live_in.insert(block.id, input);
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }

    let mut ranges = BTreeMap::<VReg, (usize, usize)>::new();
    for block in &function.blocks {
        let Some(block_start) = starts.get(&block.id).copied() else {
            continue;
        };
        let block_end = block_start.saturating_add(block.instructions.len());
        for register in live_in.get(&block.id).into_iter().flatten() {
            extend_range(
                &mut ranges,
                *register,
                block_start,
                block_end.max(block_start + 1),
            );
        }
        for register in live_out.get(&block.id).into_iter().flatten() {
            extend_range(
                &mut ranges,
                *register,
                block_start,
                block_end.max(block_start + 1),
            );
        }
        for (index, instruction) in block.instructions.iter().enumerate() {
            let at = block_start + index;
            let (uses, defs) = instruction_use_def(function, instruction, plans);
            for register in uses.into_iter().chain(defs) {
                extend_range(&mut ranges, register, at, at.saturating_add(1));
            }
        }
    }
    let intervals = ranges
        .into_iter()
        .map(|(register, (start, end))| LiveInterval {
            register,
            start,
            end: end.max(start.saturating_add(1)),
        })
        .collect();

    Liveness {
        intervals,
        live_in: live_in
            .into_iter()
            .map(|(block, values)| (block, values.into_iter().collect()))
            .collect(),
        live_out: live_out
            .into_iter()
            .map(|(block, values)| (block, values.into_iter().collect()))
            .collect(),
        block_starts: starts,
        instruction_positions: positions,
    }
}

/// 把一次局部活跃事实合并进寄存器的总区间。
fn extend_range(
    ranges: &mut BTreeMap<VReg, (usize, usize)>,
    register: VReg,
    start: usize,
    end: usize,
) {
    ranges
        .entry(register)
        .and_modify(|range| {
            range.0 = range.0.min(start);
            range.1 = range.1.max(end);
        })
        .or_insert((start, end));
}

/// 提取一条指令的读取与定义集合，包括释放计划的间接读取。
fn instruction_use_def(
    function: &TacFunction,
    instruction: &TacInstr,
    plans: &[TacReleasePlan],
) -> (BTreeSet<VReg>, BTreeSet<VReg>) {
    let mut uses = BTreeSet::new();
    let mut defs = BTreeSet::new();
    match &instruction.op {
        TacOp::Move(value)
        | TacOp::Copy(value)
        | TacOp::Box(value)
        | TacOp::Unbox(value)
        | TacOp::Release { value, .. }
        | TacOp::Transfer { value } => {
            uses.insert(*value);
        }
        TacOp::Cast { value, .. } => {
            uses.insert(*value);
        }
        TacOp::Arith { left, right, .. }
        | TacOp::Compare { left, right, .. }
        | TacOp::SetOp { left, right, .. }
        | TacOp::SetCompare { left, right, .. } => {
            uses.insert(*left);
            uses.insert(*right);
        }
        TacOp::Len { source } => {
            uses.insert(*source);
        }
        TacOp::IndexGetDynamic { source, index } => {
            uses.insert(*source);
            uses.insert(*index);
        }
        TacOp::NewArray { elements }
        | TacOp::NewTuple { elements }
        | TacOp::NewSet { elements } => uses.extend(elements.iter().copied()),
        TacOp::NewDictTable { entries } | TacOp::NewDictColumn { entries } => {
            uses.extend(entries.iter().map(|(_, value)| *value));
        }
        TacOp::IndexGet { source, .. } => {
            uses.insert(*source);
        }
        TacOp::SelectorApply {
            source,
            step,
            random_counts,
            ..
        } => {
            uses.insert(*source);
            uses.extend(step.iter().copied());
            uses.extend(random_counts.iter().flatten().copied());
        }
        TacOp::BroadcastAssign { root, value, .. } => {
            uses.insert(*root);
            uses.insert(*value);
        }
        TacOp::RandomSeed { value, .. } => {
            uses.insert(*value);
        }
        TacOp::BranchIf { condition, .. } => {
            uses.insert(*condition);
        }
        TacOp::Call { arguments, .. } => {
            uses.extend(arguments.iter().map(|argument| argument.value));
        }
        TacOp::CallDynamic { callee, arguments } => {
            uses.insert(*callee);
            uses.extend(arguments.iter().map(|argument| argument.value));
        }
        TacOp::Return { value } => {
            uses.extend(value.iter().copied());
        }
        TacOp::Raise { value } => {
            uses.insert(*value);
        }
        TacOp::MakeError { code, message, .. } => {
            uses.extend(code.iter().chain(message.iter()).copied());
        }
        TacOp::Check { value, .. } => {
            uses.insert(*value);
        }
        TacOp::RunReleasePlan { scope, exit } => {
            if let Some(plan) = plans
                .iter()
                .find(|plan| plan.scope == *scope && plan.exit == *exit)
            {
                for action in &plan.actions {
                    if let Some(register) = function.value_registers.get(&action.value) {
                        uses.insert(*register);
                    }
                }
            }
        }
        TacOp::LoadConst(_)
        | TacOp::LoadNone
        | TacOp::LoadFunc(_)
        | TacOp::Jump(_)
        | TacOp::CallSub { .. }
        | TacOp::RetFromSub
        | TacOp::EnterScope(_)
        | TacOp::ExitScope { .. } => {}
    }
    if let Some(dst) = instruction.dst {
        defs.insert(dst);
    }
    (uses, defs)
}

#[cfg(test)]
/// 活跃分析的最小 CFG 与间接读取回归。
mod tests {
    use super::analyze;
    use crate::research::{BlockId, CategoryMap, TacBlock, TacFunction, TacInstr, TacOp, VReg};
    use xiao_ir::IrSpan;

    /// 构造只保留数据流字段的测试函数。
    fn function(blocks: Vec<TacBlock>, handlers: Vec<crate::research::TacHandler>) -> TacFunction {
        TacFunction {
            name: String::new(),
            signature: None,
            entry: BlockId::new(0),
            blocks,
            parameters: Vec::new(),
            locals: Vec::new(),
            categories: CategoryMap::new(),
            scopes: vec![0],
            handlers,
            value_registers: std::collections::BTreeMap::new(),
            span: IrSpan::new(0, 1),
        }
    }

    #[test]
    /// 相邻块不是后继，显式跳转目标才传播活跃值。
    fn does_not_invent_fallthrough_and_tracks_explicit_branch() {
        let span = IrSpan::new(0, 1);
        let value = VReg::new(0);
        let stray = VReg::new(1);
        let function = function(
            vec![
                TacBlock {
                    id: BlockId::new(0),
                    scope: 0,
                    instructions: vec![TacInstr::new(TacOp::Jump(BlockId::new(2)), span)],
                },
                TacBlock {
                    id: BlockId::new(1),
                    scope: 0,
                    instructions: vec![TacInstr::new(TacOp::Return { value: Some(stray) }, span)],
                },
                TacBlock {
                    id: BlockId::new(2),
                    scope: 0,
                    instructions: vec![TacInstr::new(TacOp::Return { value: Some(value) }, span)],
                },
            ],
            Vec::new(),
        );
        let result = analyze(&function, &[]);
        assert!(
            result
                .intervals
                .iter()
                .any(|interval| interval.register == value)
        );
        assert!(result.live_out[&BlockId::new(0)].contains(&value));
        assert!(!result.live_out[&BlockId::new(0)].contains(&stray));
    }

    #[test]
    /// handler 入口使用的值必须沿异常边活过整个保护区间。
    fn protected_blocks_keep_handler_inputs_live() {
        let span = IrSpan::new(0, 1);
        let value = VReg::new(0);
        let binding = VReg::new(1);
        let handler = crate::research::TacHandler {
            protected: (BlockId::new(0), BlockId::new(1)),
            handler: BlockId::new(1),
            scope: 0,
            exit: "catch".to_owned(),
            catch_type: Some("Error".to_owned()),
            binding: Some(binding),
        };
        let function = function(
            vec![
                TacBlock {
                    id: BlockId::new(0),
                    scope: 0,
                    instructions: vec![TacInstr::with_dst(TacOp::LoadNone, value, span)],
                },
                TacBlock {
                    id: BlockId::new(1),
                    scope: 0,
                    instructions: vec![TacInstr::new(TacOp::Return { value: Some(value) }, span)],
                },
            ],
            vec![handler],
        );
        let result = analyze(&function, &[]);
        assert!(result.live_out[&BlockId::new(0)].contains(&value));
    }

    #[test]
    /// 释放计划动作通过值编号间接读取对应寄存器。
    fn release_plan_is_an_indirect_register_use() {
        let span = IrSpan::new(0, 1);
        let value = VReg::new(0);
        let mut function = function(
            vec![TacBlock {
                id: BlockId::new(0),
                scope: 0,
                instructions: vec![
                    TacInstr::with_dst(TacOp::LoadNone, value, span),
                    TacInstr::new(
                        TacOp::RunReleasePlan {
                            scope: 0,
                            exit: "normal".to_owned(),
                        },
                        span,
                    ),
                ],
            }],
            Vec::new(),
        );
        function.value_registers.insert(7, value);
        let plan = crate::research::TacReleasePlan {
            scope: 0,
            exit: "normal".to_owned(),
            actions: vec![crate::research::TacReleaseAction {
                value: 7,
                order: 0,
                kind: xiao_lifetime::ReleaseActionKind::Strong,
            }],
            transferred: Vec::new(),
        };
        let result = analyze(&function, &[plan]);
        let interval = result
            .intervals
            .iter()
            .find(|interval| interval.register == value)
            .expect("释放动作引用的寄存器应有活跃区间");
        assert_eq!((interval.start, interval.end), (0, 2));
    }
}
