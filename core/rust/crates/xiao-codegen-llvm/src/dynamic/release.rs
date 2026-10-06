//! 动态降低器的临时值与退出边释放计划发射。

use std::collections::BTreeSet;

use xiao_ir::IrType;

use super::{DynamicGenerator, VALUE_TYPE};
use crate::error::{CodegenError, Result};

impl<'a> DynamicGenerator<'a> {
    /// 释放一个临时 ABI 值。
    pub(super) fn release_value(&mut self, value: String) {
        let slot = self.next_temp();
        self.emit(format!("  {slot} = alloca {VALUE_TYPE}"));
        self.emit(format!("  store {VALUE_TYPE} {value}, ptr {slot}"));
        self.emit(format!(
            "  call void @xiao_runtime_value_release_strong(ptr {slot})"
        ));
    }

    /// 按冻结的所有权计划释放某类正常退出边；旧手工 IR 无计划时才使用稳定兜底。
    pub(super) fn release_for_exit(&mut self, exit: &str) -> Result<()> {
        let has_ownership_metadata = !self.program.ownership.scopes.is_empty()
            || !self.program.ownership.values.is_empty()
            || !self.program.ownership.release_plans.is_empty();
        if !has_ownership_metadata {
            self.release_all_slots_fallback();
            return Ok(());
        }
        let root_scopes = self
            .program
            .ownership
            .scopes
            .iter()
            .filter(|scope| scope.parent.is_none() && scope.kind == "program")
            .map(|scope| scope.id)
            .collect::<BTreeSet<_>>();
        if root_scopes.len() != 1 {
            return Err(CodegenError::InvalidIr {
                message: "动态释放计划缺少唯一 program 根作用域".to_owned(),
            });
        }
        let has_plan = self
            .program
            .ownership
            .release_plans
            .iter()
            .any(|plan| plan.exit == exit && root_scopes.contains(&plan.scope));
        if !has_plan {
            return Err(CodegenError::InvalidIr {
                message: format!("动态释放计划缺少 program/{exit} 退出边"),
            });
        }
        self.release_for_scope(root_scopes.iter().next().copied(), exit)
    }

    /// 按指定作用域执行一条释放边；没有该边时表示该作用域没有需要释放的值。
    pub(super) fn release_for_scope(&mut self, scope: Option<u32>, exit: &str) -> Result<()> {
        if self.program.ownership.scopes.is_empty()
            && self.program.ownership.values.is_empty()
            && self.program.ownership.release_plans.is_empty()
        {
            return Ok(());
        }
        let Some(scope) = scope else {
            return Ok(());
        };
        let mut plans = self
            .program
            .ownership
            .release_plans
            .iter()
            .filter(|plan| plan.exit == exit && plan.scope == scope)
            .collect::<Vec<_>>();
        plans.sort_by_key(|plan| std::cmp::Reverse(plan.scope));
        if plans.is_empty() {
            return Ok(());
        }
        let mut emitted_values = BTreeSet::new();
        let mut emitted_slots = BTreeSet::new();
        for plan in plans {
            let mut actions = plan.actions.iter().collect::<Vec<_>>();
            actions.sort_by_key(|action| action.order);
            for action in actions {
                if plan.transferred.contains(&action.value)
                    || emitted_values.contains(&action.value)
                {
                    continue;
                }
                let Some(slot) = self.value_slots.get(&action.value).copied() else {
                    // 生命周期分析会为表构造符号和匿名表达式临时值登记值编号；
                    // 前者没有运行时槽，后者在构造器/容器调用后已由降低器即时归还。
                    // 只有命名绑定必须映射到入口槽，其他值不能伪造释放地址。
                    if self
                        .program
                        .ownership
                        .values
                        .iter()
                        .find(|value| value.id == action.value)
                        .is_some_and(|value| {
                            value.temporary
                                || matches!(
                                    value.ty,
                                    Some(IrType::Table { ref kind, .. }) if kind == "constructor"
                                )
                        })
                    {
                        continue;
                    }
                    // 函数调用的返回临时值由调用槽转移给接收绑定；生命周期
                    // 记录在函数边界上可能没有同名入口槽，不能为它伪造地址。
                    if !self.function_definitions.is_empty() {
                        continue;
                    }
                    return Err(CodegenError::InvalidIr {
                        message: format!("动态释放计划引用未映射到 ABI 槽的值 {}", action.value),
                    });
                };
                emitted_values.insert(action.value);
                if !emitted_slots.insert(slot.index) {
                    continue;
                }
                match action.kind.as_str() {
                    "strong" => self.emit(format!(
                        "  call void @xiao_runtime_value_release_strong(ptr %slot{})",
                        slot.index
                    )),
                    "weak" => {
                        let status = self.next_temp();
                        self.emit(format!(
                            "  {status} = call i32 @xiao_runtime_value_release_weak(ptr %slot{})",
                            slot.index
                        ));
                        self.check_status(&status);
                    }
                    other => {
                        return Err(CodegenError::InvalidIr {
                            message: format!("动态释放计划包含未知动作类型 {other}"),
                        });
                    }
                }
            }
        }
        Ok(())
    }

    /// 为没有所有权元数据的手工测试 IR 保留逆声明序释放兜底。
    fn release_all_slots_fallback(&mut self) {
        let mut slots = self.slots.values().copied().collect::<Vec<_>>();
        slots.sort_by_key(|slot| std::cmp::Reverse(slot.index));
        for slot in slots {
            self.emit(format!(
                "  call void @xiao_runtime_value_release_strong(ptr %slot{})",
                slot.index
            ));
        }
    }
}
