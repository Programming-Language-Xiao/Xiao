//! 动态降低器的槽收集、生命周期边界与 ABI 槽读写。

use std::collections::{BTreeMap, BTreeSet};

use xiao_ir::{IrName, IrStatement, IrStatementKind};

use super::predicate::{name_key, scope_is_ancestor};
use super::{DynamicGenerator, Slot, VALUE_TYPE};
use crate::error::{CodegenError, Result};

impl<'a> DynamicGenerator<'a> {
    /// 收集所有动态入口名称；本批将它们统一存为 ABI 值槽。
    pub(super) fn collect_slots(&mut self, statements: &[IrStatement]) -> Result<()> {
        for statement in statements {
            match &statement.kind {
                IrStatementKind::Assignment { target, .. }
                | IrStatementKind::Declaration { target, .. }
                | IrStatementKind::ConstDeclaration { target, .. } => {
                    self.insert_slot(target)?;
                }
                IrStatementKind::If {
                    body,
                    elif_branches,
                    else_body,
                    ..
                } => {
                    self.collect_slots(body)?;
                    for branch in elif_branches {
                        self.collect_slots(&branch.body)?;
                    }
                    if let Some(body) = else_body {
                        self.collect_slots(body)?;
                    }
                }
                IrStatementKind::Table {
                    name, table_kind, ..
                } if table_kind == "singleton" => {
                    self.insert_slot(name)?;
                }
                IrStatementKind::While { body, .. } | IrStatementKind::For { body, .. } => {
                    self.collect_slots(body)?;
                }
                IrStatementKind::Try {
                    body,
                    catches,
                    finally_body,
                } => {
                    self.collect_slots(body)?;
                    for catch in catches {
                        self.collect_slots(&catch.body)?;
                    }
                    if let Some(body) = finally_body {
                        self.collect_slots(body)?;
                    }
                }
                IrStatementKind::Function { .. }
                | IrStatementKind::Expression { .. }
                | IrStatementKind::ExtendedAssignment { .. }
                | IrStatementKind::Import { .. }
                | IrStatementKind::Return { .. }
                | IrStatementKind::Break
                | IrStatementKind::Continue
                | IrStatementKind::Table { .. }
                | IrStatementKind::Raise { .. } => {}
            }
        }
        Ok(())
    }

    /// 收集表字段默认值，确保真实表源码不会在原生路径被静默丢弃。
    ///
    /// 字段值在每次 `new`/singleton 构造时重新求值；这与字节码侧的字段辅助函数
    /// 保持一致。方法和其他表体语句需要函数表或异常边，当前批次明确结构化拒绝。
    pub(super) fn collect_table_initializers(&mut self) -> Result<()> {
        for statement in &self.program.body {
            let IrStatementKind::Table { name, body, .. } = &statement.kind else {
                continue;
            };
            let signature = self
                .program
                .table_signatures
                .iter()
                .find(|signature| signature.name == name.text)
                .ok_or_else(|| CodegenError::InvalidIr {
                    message: format!("表 {} 没有登记签名", name.text),
                })?;
            let mut initializers = Vec::new();
            let mut seen = BTreeSet::new();
            for member in body {
                let (target, value) = match &member.kind {
                    IrStatementKind::Assignment { target, value }
                    | IrStatementKind::ConstDeclaration { target, value, .. } => (target, value),
                    IrStatementKind::Declaration {
                        target,
                        value: Some(value),
                        ..
                    } => (target, value),
                    IrStatementKind::Declaration { value: None, .. } => continue,
                    IrStatementKind::Function { .. } => {
                        return Err(CodegenError::Unsupported {
                            feature: "动态表方法（ABI 尚未携带函数表）".to_owned(),
                            span: Some(member.span),
                        });
                    }
                    _ => {
                        return Err(CodegenError::Unsupported {
                            feature: "动态表声明/初始化".to_owned(),
                            span: Some(member.span),
                        });
                    }
                };
                let key = name_key(target);
                let Some(signature_member) = signature.members.iter().find(|item| item.name == key)
                else {
                    return Err(CodegenError::InvalidIr {
                        message: format!("表 {} 的字段 {} 没有登记签名", name.text, target.text),
                    });
                };
                if signature_member.method || !seen.insert(key.clone()) {
                    return Err(CodegenError::Unsupported {
                        feature: "动态表字段初始化".to_owned(),
                        span: Some(member.span),
                    });
                }
                initializers.push((key, value.clone()));
            }
            self.table_initializers
                .insert(name.text.clone(), initializers);
        }
        Ok(())
    }

    /// 插入一个未初始化的 ABI 值槽。
    fn insert_slot(&mut self, name: &IrName) -> Result<()> {
        let key = name_key(name);
        if !self.slots.contains_key(&key) {
            let index = self.slots.len();
            self.slots.insert(key, Slot { index });
        }
        Ok(())
    }

    /// 将前端所有权值编号关联到同名 ABI 槽，后续只消费冻结的释放计划。
    pub(super) fn collect_value_slots(&mut self) {
        for value in &self.program.ownership.values {
            let Some(name) = value.name.as_deref() else {
                continue;
            };
            // 正式前端总是提供带前缀的键；裸键回退只服务旧的手写测试 IR。
            let slot = self
                .slots
                .get(name)
                .copied()
                .or_else(|| self.slots.get(&format!("ascii:{name}")).copied());
            let Some(slot) = slot else {
                continue;
            };
            self.value_slots.insert(value.id, slot);
        }
    }

    /// 检查本批动态 CFG 能消费的释放计划边界。
    ///
    /// 生成器目前只在程序入口和 `return` 边发射根作用域计划。分支/循环作用域的
    /// 局部拥有值若被静默忽略会泄漏，因此在生成任何 LLVM 文本前结构化拒绝这类 IR；
    /// 没有局部释放动作的分支和循环仍可正常降低。异常退出计划同样留给 N0-C。
    pub(super) fn validate_release_scope_boundary(&self) -> Result<()> {
        let has_ownership_metadata = !self.program.ownership.scopes.is_empty()
            || !self.program.ownership.values.is_empty()
            || !self.program.ownership.release_plans.is_empty();
        if !has_ownership_metadata {
            return Ok(());
        }
        // 当前 N0-B 的槽表按稳定名称索引，尚未携带词法作用域。遮蔽绑定若继续
        // 进入降低会把内层值写进外层槽，并丢失外层值的释放动作；在块级槽位接入
        // 前必须把这种 IR 结构化拒绝。
        let mut binding_scopes = BTreeMap::<String, u32>::new();
        for value in &self.program.ownership.values {
            let Some(name) = value.name.as_deref() else {
                continue;
            };
            if let Some(previous_scope) = binding_scopes.get(name) {
                let nested_shadow =
                    scope_is_ancestor(&self.program.ownership.scopes, *previous_scope, value.scope)
                        || scope_is_ancestor(
                            &self.program.ownership.scopes,
                            value.scope,
                            *previous_scope,
                        );
                if nested_shadow {
                    return Err(CodegenError::Unsupported {
                        feature: format!("动态槽名称 {name} 在多个作用域遮蔽（待块级槽位降低）"),
                        span: Some(value.span),
                    });
                }
                if *previous_scope == value.scope || !nested_shadow {
                    continue;
                }
            } else {
                binding_scopes.insert(name.to_owned(), value.scope);
            }
        }
        let roots = self
            .program
            .ownership
            .scopes
            .iter()
            .filter(|scope| scope.parent.is_none() && scope.kind == "program")
            .collect::<Vec<_>>();
        if roots.len() != 1 {
            return Err(CodegenError::InvalidIr {
                message: "动态释放计划缺少唯一 program 根作用域".to_owned(),
            });
        }
        let root = roots[0].id;
        if let Some(scope) = self
            .program
            .ownership
            .scopes
            .iter()
            .filter(|scope| scope.id != root)
            .find(|scope| {
                self.program
                    .ownership
                    .release_plans
                    .iter()
                    .any(|plan| plan.scope == scope.id && !plan.actions.is_empty())
            })
        {
            return Err(CodegenError::Unsupported {
                feature: format!(
                    "动态嵌套作用域释放计划（{} 作用域，待块级释放降低）",
                    scope.kind
                ),
                span: Some(scope.span),
            });
        }
        Ok(())
    }

    /// 从槽复制一个 ABI 值。
    pub(super) fn load_slot(&mut self, name: &IrName) -> Result<String> {
        let key = name_key(name);
        let slot = self
            .slots
            .get(&key)
            .copied()
            .or_else(|| self.slots.get(&name.text).copied())
            .ok_or_else(|| CodegenError::InvalidIr {
                message: format!(
                    "名称 {} 没有动态值槽（{}..{}）",
                    name.text, name.span.start, name.span.end
                ),
            })?;
        let output = self.next_temp();
        self.emit(format!("  {output} = alloca {VALUE_TYPE}"));
        self.emit(format!(
            "  store {VALUE_TYPE} zeroinitializer, ptr {output}"
        ));
        self.checked_status_call(format!(
            "@xiao_runtime_value_copy(ptr %slot{}, ptr {output})",
            slot.index
        ));
        let value = self.next_temp();
        self.emit(format!("  {value} = load {VALUE_TYPE}, ptr {output}"));
        Ok(value)
    }

    /// 将新值写入槽并释放旧值。
    pub(super) fn store_slot(&mut self, name: &IrName, value: String) -> Result<()> {
        let key = name_key(name);
        let slot = self
            .slots
            .get(&key)
            .copied()
            .or_else(|| self.slots.get(&name.text).copied())
            .ok_or_else(|| CodegenError::InvalidIr {
                message: format!("名称 {} 没有动态值槽", name.text),
            })?;
        self.emit(format!(
            "  call void @xiao_runtime_value_release(ptr %slot{})",
            slot.index
        ));
        self.emit(format!(
            "  store {VALUE_TYPE} {value}, ptr %slot{}",
            slot.index
        ));
        Ok(())
    }

    /// 发射空值构造器调用。
    pub(super) fn none_value(&mut self) -> String {
        self.emit_value_call("xiao_runtime_value_none", "")
    }
}
