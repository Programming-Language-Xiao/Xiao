//! TAC 的显式控制流后继。
//!
//! 语义核没有隐式 fallthrough：一个块只有指令中显式列出的目标才是后继。
//! 验证器、活跃分析和编码器因此必须共享这里的目标提取规则，不能各自复制
//! 一份近似实现。

use std::collections::BTreeMap;

use crate::research::tac::{BlockId, TacFunction, TacOp};

/// 列出一条指令的所有显式跳转目标。
#[must_use]
pub fn jump_targets(op: &TacOp) -> Vec<BlockId> {
    match op {
        TacOp::Jump(target) => vec![*target],
        TacOp::BranchIf {
            if_true, if_false, ..
        } => vec![*if_true, *if_false],
        TacOp::Check { on_failure, .. } => vec![*on_failure],
        TacOp::CallSub { sub } => vec![*sub],
        TacOp::SelectorApply { .. } => Vec::new(),
        TacOp::BroadcastAssign { .. } => Vec::new(),
        TacOp::RandomSeed { .. } => Vec::new(),
        _ => Vec::new(),
    }
}

/// 构造函数的显式 CFG 后继表。
///
/// 返回表只包含正常/检查/子程序指令直接写出的目标，不会把相邻块自动视为
/// fallthrough。异常边由 [`protected_successors`] 单独补入，便于调用方选择
/// 是否需要异常数据流。
#[must_use]
pub fn successors(function: &TacFunction) -> BTreeMap<BlockId, Vec<BlockId>> {
    function
        .blocks
        .iter()
        .map(|block| {
            let mut targets = Vec::new();
            for instruction in &block.instructions {
                for target in jump_targets(&instruction.op) {
                    if !targets.contains(&target) {
                        targets.push(target);
                    }
                }
            }
            (block.id, targets)
        })
        .collect()
}

/// 给每个受保护块追加其可能进入的 handler。
///
/// `TacHandler.protected` 使用 `[start, end)` 块区间；空 try 体对应的 handler
/// 仍会保留在表中，不能因为没有显式前驱而删除。
pub fn protected_successors(function: &TacFunction, table: &mut BTreeMap<BlockId, Vec<BlockId>>) {
    for handler in &function.handlers {
        for block in &function.blocks {
            if block.id >= handler.protected.0 && block.id < handler.protected.1 {
                let targets = table.entry(block.id).or_default();
                if !targets.contains(&handler.handler) {
                    targets.push(handler.handler);
                }
            }
        }
    }
}
