//! 退出点上的释放计划接线。
//!
//! 释放顺序的唯一来源是生命周期阶段冻结的计划。降低器只决定**在哪些退出点上
//! 执行哪个 `(作用域, 退出边)` 计划**，不复制也不重排动作序列。

use xiao_ir::IrSpan;

use crate::research::lower::Lowerer;
use crate::research::tac::{TacInstr, TacOp};

impl Lowerer<'_> {
    /// 在退出点上发出一个作用域的释放计划。
    ///
    /// 计划不存在时什么都不发：`IrOwnership.release_plans` 是「作用域 × 退出边」
    /// 的笛卡尔积，正常情况下必然存在；缺失说明输入 IR 已被破坏，由验证器报告。
    pub(super) fn run_plan(&mut self, scope: u32, exit: &str, span: IrSpan) {
        if !self.has_plan(scope, exit) {
            return;
        }
        self.emit(TacInstr::new(
            TacOp::RunReleasePlan {
                scope,
                exit: exit.to_owned(),
            },
            span,
        ));
    }

    /// 判断某个 `(作用域, 退出边)` 计划是否存在。
    fn has_plan(&self, scope: u32, exit: &str) -> bool {
        self.plans
            .iter()
            .any(|plan| plan.scope == scope && plan.exit == exit)
    }
}
