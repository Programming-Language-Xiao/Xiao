//! 三地址自校验与释放序列对账。

use xiao_ir::{IrProgram, ObservedRelease, reconcile_release_plans};

use crate::research::tac::{BlockId, TacOp, TacProgram, VReg};

/// 三地址验证结果。
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct TacVerification {
    /// 未降低的构造；非空表示产物不完整。
    pub unsupported: Vec<String>,
    /// 结构错误说明。
    pub errors: Vec<String>,
}

impl TacVerification {
    /// 判断验证是否通过。
    #[must_use]
    pub fn is_success(&self) -> bool {
        self.unsupported.is_empty() && self.errors.is_empty()
    }
}

/// 校验一份三地址产物。
///
/// 检查三件事：每条跳转都指向存在的块、每条释放计划指令都指向存在的计划、
/// 以及实际引用的 `(作用域, 退出边)` 集合与冻结计划对账一致。
#[must_use]
pub fn verify_program(program: &IrProgram, tac: &TacProgram) -> TacVerification {
    let mut result = TacVerification {
        unsupported: tac.unsupported.clone(),
        errors: Vec::new(),
    };
    let block_count = tac
        .functions
        .iter()
        .map(|function| function.blocks.len())
        .max()
        .unwrap_or(0) as u32;
    for (index, function) in tac.functions.iter().enumerate() {
        let blocks = function.blocks.len() as u32;
        for block in &function.blocks {
            for instruction in &block.instructions {
                for target in jump_targets(&instruction.op) {
                    if target.get() >= blocks {
                        result.errors.push(format!(
                            "函数 {index} 的块 {} 跳转到不存在的块 {}",
                            block.id.get(),
                            target.get()
                        ));
                    }
                }
                if let TacOp::RunReleasePlan { scope, exit } = &instruction.op
                    && !tac
                        .plans
                        .iter()
                        .any(|plan| plan.scope == *scope && plan.exit == *exit)
                {
                    result.errors.push(format!(
                        "函数 {index} 引用了不存在的释放计划 ({scope}, {exit})"
                    ));
                }
            }
        }
    }
    let _ = block_count;
    let observed = observed_plans(tac);
    for error in reconcile_release_plans(program, &observed).errors() {
        result
            .errors
            .push(format!("{}: {}", error.path, error.message));
    }
    result
}

/// 汇总三地址产物实际引用的释放计划。
///
/// 这里比对的是「降低器认为哪些 `(作用域, 退出边)` 会被执行」与冻结计划是否
/// 逐条一致；它验证的是降低器的接线，不验证计划的顺序语义——顺序的唯一来源
/// 始终是 `IrReleasePlan`，降低器不复制动作序列。
fn observed_plans(tac: &TacProgram) -> Vec<ObservedRelease> {
    let mut observed = Vec::new();
    for function in &tac.functions {
        for block in &function.blocks {
            for instruction in &block.instructions {
                let TacOp::RunReleasePlan { scope, exit } = &instruction.op else {
                    continue;
                };
                let Some(plan) = tac
                    .plans
                    .iter()
                    .find(|plan| plan.scope == *scope && plan.exit == *exit)
                else {
                    continue;
                };
                if observed
                    .iter()
                    .any(|item: &ObservedRelease| item.scope == *scope && item.exit == *exit)
                {
                    continue;
                }
                observed.push(ObservedRelease::new(
                    *scope,
                    exit.clone(),
                    plan.actions
                        .iter()
                        .map(|action| xiao_ir::IrReleaseAction {
                            value: action.value,
                            order: action.order,
                            kind: action.kind.as_name().to_owned(),
                        })
                        .collect(),
                ));
            }
        }
    }
    observed
}

/// 列出一条指令的所有跳转目标。
fn jump_targets(op: &TacOp) -> Vec<BlockId> {
    match op {
        TacOp::Jump(target) => vec![*target],
        TacOp::BranchIf {
            if_true, if_false, ..
        } => vec![*if_true, *if_false],
        TacOp::Check { on_failure, .. } => vec![*on_failure],
        _ => Vec::new(),
    }
}

/// 保留 `VReg` 在验证签名中的可见性。
#[allow(dead_code)]
const fn _keep(_: VReg) {}
