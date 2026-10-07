//! 动态降低器的临时值与退出边释放计划发射。

use std::collections::BTreeSet;

use xiao_ir::IrType;

use super::{DynamicGenerator, VALUE_TYPE};
use crate::error::{CodegenError, Result};

impl<'a> DynamicGenerator<'a> {
    /// 返回表达式像 VM 临时寄存器一样归当前帧拥有；循环再次执行同一位置时先覆盖旧值。
    pub(super) fn store_return_site(
        &mut self,
        expression: &xiao_ir::IrExpression,
        value: &str,
        temporary: bool,
    ) {
        // 节点身份区分共享源码区间的 IR；编号只按首次发射顺序，绝不把地址写入文本。
        let site = expression as *const xiao_ir::IrExpression;
        let slot =
            if let Some((_, slot)) = self.return_sites.iter().find(|(known, _)| *known == site) {
                slot.clone()
            } else {
                let slot = format!("%xiao.return.site{}", self.return_sites.len());
                self.return_sites.push((site, slot.clone()));
                self.frame_temporaries.push(slot.clone());
                slot
            };
        let release = self.value_release_symbol();
        self.emit(format!("  call void @{release}(ptr {slot})"));
        self.emit(format!("  store {VALUE_TYPE} {value}, ptr {slot}"));
        self.emit(format!("  store ptr {slot}, ptr %xiao.return.source"));
        self.emit(format!(
            "  store i1 {temporary}, ptr %xiao.return.temporary"
        ));
    }

    /// 函数作用域计划执行后，按帧槽顺序归还返回临时值，包括被 finally 覆盖的值。
    pub(super) fn release_frame_temporaries(&mut self) {
        let slots = self.frame_temporaries.clone();
        let release = self.value_release_symbol();
        for slot in slots {
            self.emit(format!("  call void @{release}(ptr {slot})"));
        }
    }

    /// 把表达式已经拥有的值移入语句临时槽，不增加引用；终止边交给统一帧清理。
    pub(super) fn own_statement_temporary(
        &mut self,
        expression: &xiao_ir::IrExpression,
        value: &str,
    ) -> String {
        self.own_statement_temporary_at(expression, expression, value)
    }

    /// 缺省实参的表达式会被多个调用复用，因此还以调用接收者节点区分求值站点。
    fn own_statement_temporary_at(
        &mut self,
        expression: &xiao_ir::IrExpression,
        context: &xiao_ir::IrExpression,
        value: &str,
    ) -> String {
        let site = (
            expression as *const xiao_ir::IrExpression,
            context as *const xiao_ir::IrExpression,
        );
        let slot = if let Some((_, slot)) = self
            .statement_sites
            .iter()
            .find(|(known, _)| *known == site)
        {
            slot.clone()
        } else {
            let slot = format!("%xiao.statement.site{}", self.statement_sites.len());
            self.statement_sites.push((site, slot.clone()));
            self.frame_temporaries.push(slot.clone());
            slot
        };
        let release = self.value_release_symbol();
        self.emit(format!("  call void @{release}(ptr {slot})"));
        self.emit(format!("  store {VALUE_TYPE} {value}, ptr {slot}"));
        if !self.statement_temporaries.contains(&slot) {
            self.statement_temporaries.push(slot.clone());
        }
        slot
    }

    /// 调用实参需要一份绑定引用；堆表达式原值另由语句帧持有，不能在调用返回时提前销毁。
    pub(super) fn store_call_argument(
        &mut self,
        expression: &xiao_ir::IrExpression,
        context: &xiao_ir::IrExpression,
        value: &str,
        slot: &str,
    ) {
        if super::predicate::is_heap_temporary(expression) {
            let owner = self.own_statement_temporary_at(expression, context, value);
            self.checked_status_call_at(
                format!("@xiao_runtime_value_copy(ptr {owner}, ptr {slot})"),
                expression.span,
            );
        } else {
            self.emit(format!("  store {VALUE_TYPE} {value}, ptr {slot}"));
        }
    }

    /// 普通语句消费完结果后归还临时对象，析构错误仍进入当前 try/catch 的错误边。
    pub(super) fn flush_statement_temporaries(&mut self, span: xiao_ir::IrSpan) {
        let slots = std::mem::take(&mut self.statement_temporaries);
        if slots.is_empty() {
            return;
        }
        let release = self.value_release_symbol();
        for slot in slots {
            self.emit(format!("  call void @{release}(ptr {slot})"));
        }
        self.check_pending_error_at(span);
    }

    /// 全部临时槽放在入口，避免 finally 的 stackrestore 使尚需清理的地址失效。
    pub(super) fn initialize_frame_temporaries(&mut self) {
        let entry = self
            .lines
            .iter()
            .position(|line| line == "entry:")
            .expect("函数入口")
            + 1;
        let allocations = self
            .frame_temporaries
            .iter()
            .flat_map(|slot| {
                [
                    format!("  {slot} = alloca {VALUE_TYPE}"),
                    format!("  store {VALUE_TYPE} zeroinitializer, ptr {slot}"),
                ]
            })
            .collect::<Vec<_>>();
        self.lines.splice(entry..entry, allocations);
    }

    /// 表回调可把只读视图借给普通函数；这类程序的临时/强槽需按实际值标签归还。
    pub(super) fn value_release_symbol(&self) -> &'static str {
        if self.callback.is_some() || (self.function_mode && !self.method_definitions.is_empty()) {
            "xiao_runtime_value_release_any"
        } else {
            "xiao_runtime_value_release_strong"
        }
    }
    /// 函数退出仅消费所属作用域的冻结计划，已清空槽重复经过出口时无操作。
    pub(super) fn release_function_scopes(&mut self, exit: &str) -> Result<()> {
        if self.callback.as_ref().is_some_and(|context| context.fields) {
            self.release_all_slots_fallback();
            return Ok(());
        }
        let Some(root) = self.function_scope else {
            self.release_all_slots_fallback();
            return Ok(());
        };
        let mut scopes = self
            .program
            .ownership
            .scopes
            .iter()
            .filter(|scope| {
                super::predicate::scope_function_owner(&self.program.ownership.scopes, scope.id)
                    == Some(root)
            })
            .map(|scope| scope.id)
            .collect::<Vec<_>>();
        scopes.sort_unstable_by(|a, b| b.cmp(a));
        for scope in scopes {
            self.release_for_scope(Some(scope), exit)?;
            // LLVM 表达式读取返回绑定时已复制一份到调用方结果槽；计划中的
            // transferred 对应 VM 的移动语义，这里还须归还函数持有的原引用。
            if exit == "return" {
                let transferred = self
                    .program
                    .ownership
                    .release_plans
                    .iter()
                    .filter(|plan| plan.scope == scope && plan.exit == exit)
                    .flat_map(|plan| plan.transferred.iter())
                    .filter_map(|value| self.value_slots.get(value).copied())
                    .collect::<Vec<_>>();
                for slot in transferred {
                    let release = self.value_release_symbol();
                    self.emit(format!("  call void @{release}(ptr %slot{})", slot.index));
                }
            }
        }
        Ok(())
    }
    /// 释放一个临时 ABI 值。
    pub(super) fn release_value(&mut self, value: String) {
        let slot = self.next_temp();
        self.emit(format!("  {slot} = alloca {VALUE_TYPE}"));
        self.emit(format!("  store {VALUE_TYPE} {value}, ptr {slot}"));
        let release = self.value_release_symbol();
        self.emit(format!("  call void @{release}(ptr {slot})"));
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
                    "strong" => {
                        let release = self.value_release_symbol();
                        self.emit(format!("  call void @{release}(ptr %slot{})", slot.index));
                    }
                    "weak" => {
                        // 显式 return 的清理和统一函数出口可能经过同一槽；None 已归还。
                        let tag = self.next_temp();
                        self.emit(format!("  {tag} = load i32, ptr %slot{}", slot.index));
                        let present = self.next_temp();
                        self.emit(format!("  {present} = icmp ne i32 {tag}, 0"));
                        let release = self.next_label("weak.release");
                        let done = self.next_label("weak.released");
                        self.emit(format!(
                            "  br i1 {present}, label %{release}, label %{done}"
                        ));
                        self.emit_label(&release);
                        let status = self.next_temp();
                        self.emit(format!(
                            "  {status} = call i32 @xiao_runtime_value_release_weak(ptr %slot{})",
                            slot.index
                        ));
                        self.check_status(&status);
                        self.emit(format!("  br label %{done}"));
                        self.emit_label(&done);
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
            let release = self.value_release_symbol();
            self.emit(format!("  call void @{release}(ptr %slot{})", slot.index));
        }
    }
}
