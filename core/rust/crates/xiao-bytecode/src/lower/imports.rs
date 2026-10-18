//! 函数读取不可变顶层导入的降低，复用 VM 的模块单次初始化与导出成员读取。

use std::collections::BTreeMap;

use xiao_ir::{IrExpressionKind, IrImportItem, IrProgram, IrSpan, IrStatement, IrStatementKind};

use super::{Lowerer, name_key};
use crate::tac::{RegisterClass, TacInstr, TacOp, VReg};

/// 仅收集所有顶层可执行语句之前的导入，且要求该绑定之后没有任何模块级覆盖。
pub(super) fn stable_imports(program: &IrProgram) -> BTreeMap<String, IrImportItem> {
    let mut imports = BTreeMap::new();
    for statement in &program.body {
        match &statement.kind {
            IrStatementKind::Import { items, .. } => {
                for item in items {
                    let key = name_key(&item.binding.text, item.binding.backticked);
                    if !writes_binding(&program.body, &key, statement.span) {
                        imports.insert(key, item.clone());
                    }
                }
            }
            IrStatementKind::Function { .. } => {}
            _ => break,
        }
    }
    imports
}

/// 检查模块级写入；函数与表的局部绑定不修改模块导入。
fn writes_binding(statements: &[IrStatement], key: &str, declaration: IrSpan) -> bool {
    statements.iter().any(|statement| match &statement.kind {
        IrStatementKind::Assignment { target, .. }
        | IrStatementKind::Declaration { target, .. }
        | IrStatementKind::ConstDeclaration { target, .. } => name_key(&target.text, target.backticked) == key,
        IrStatementKind::ExtendedAssignment { target, .. } => matches!(&target.kind, IrExpressionKind::Name { name } if name_key(&name.text, name.backticked) == key),
        IrStatementKind::Import { items, .. } => statement.span != declaration && items.iter().any(|item| name_key(&item.binding.text, item.binding.backticked) == key),
        IrStatementKind::If { body, elif_branches, else_body, .. } => {
            writes_binding(body, key, declaration)
                || elif_branches.iter().any(|branch| writes_binding(&branch.body, key, declaration))
                || else_body.as_ref().is_some_and(|body| writes_binding(body, key, declaration))
        }
        IrStatementKind::For { target, body, .. } => name_key(&target.text, target.backticked) == key || writes_binding(body, key, declaration),
        IrStatementKind::While { body, .. } => writes_binding(body, key, declaration),
        IrStatementKind::Try { body, catches, finally_body } => {
            writes_binding(body, key, declaration)
                || catches.iter().any(|catch| name_key(&catch.binding.text, catch.binding.backticked) == key || writes_binding(&catch.body, key, declaration))
                || finally_body.as_ref().is_some_and(|body| writes_binding(body, key, declaration))
        }
        IrStatementKind::Function { name, .. } | IrStatementKind::Table { name, .. } => name_key(&name.text, name.backticked) == key,
        _ => false,
    })
}

impl Lowerer<'_> {
    /// 顶层导入已先于任何入口调用执行；函数内重复读取仅命中 VM 模块实例缓存。
    pub(super) fn global_import_reference(
        &mut self,
        name: &str,
        backticked: bool,
        span: IrSpan,
    ) -> Option<VReg> {
        if self.functions.is_empty() {
            return None;
        }
        let key = name_key(name, backticked);
        let item = self.global_imports.get(&key)?.clone();
        let shadowed = self
            .candidates_of_name(name, backticked)
            .iter()
            .any(|(_, scope)| {
                self.frame.scope_stack.contains(scope)
                    && self
                        .program
                        .ownership
                        .scopes
                        .iter()
                        .any(|candidate| candidate.id == *scope && candidate.kind != "program")
            });
        if shadowed {
            return None;
        }
        let module = self.new_register(RegisterClass::ObjHandle, span);
        self.emit(TacInstr::with_dst(
            TacOp::ImportModule {
                module: item.module.clone(),
                binding_module: if item.selected.is_some() {
                    item.module
                } else {
                    item.binding_module
                },
            },
            module,
            span,
        ));
        if let Some(selected) = item.selected {
            let register = self.new_register(RegisterClass::Dynamic, span);
            self.emit(TacInstr::with_dst(
                TacOp::MemberGet {
                    object: module,
                    member: name_key(&selected.text, selected.backticked),
                },
                register,
                span,
            ));
            Some(register)
        } else {
            Some(module)
        }
    }
}
