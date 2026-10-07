//! 动态降低器的 IR 谓词与静态边界判断。
//!
//! 本模块只消费类型化 IR，不持有 LLVM 发射器状态。

use xiao_ir::{
    IrExpression, IrExpressionKind, IrName, IrProgram, IrStatement, IrStatementKind, IrType,
};

/// 把 IR 字段类型映射到 `xiao-runtime-abi` 的稳定字段标签。
///
/// 这些数值与 ABI crate 中的 `XiaoFieldType` 一一对应；不在 N0-B 描述范围内的
/// 类型返回 `None`，避免把复杂类型错误地降级为 dynamic。
pub(super) fn abi_field_type(ty: &IrType) -> Option<u32> {
    match ty {
        IrType::Dynamic => Some(0),
        IrType::Scalar { name } => match name.as_str() {
            "int" => Some(1),
            "sint" => Some(2),
            "float" => Some(3),
            "sfloat" => Some(4),
            "bool" => Some(5),
            "str" => Some(6),
            _ => None,
        },
        _ => None,
    }
}

/// 返回与生命周期分析相同的稳定绑定键。
///
/// `IrName::text` 只保留展示文本，而 `IrOwnership` 使用 `ascii:`/`backtick:` 前缀区分
/// 两类绑定；动态槽必须消费同一键，否则释放计划无法找到前端登记的值。
pub(super) fn name_key(name: &IrName) -> String {
    format!(
        "{}:{}",
        if name.backticked { "backtick" } else { "ascii" },
        name.text
    )
}

/// 判断一个生命周期作用域是否是另一个作用域的祖先。
pub(super) fn scope_is_ancestor(
    scopes: &[xiao_ir::IrScope],
    ancestor: u32,
    descendant: u32,
) -> bool {
    let mut current = Some(descendant);
    while let Some(id) = current {
        if id == ancestor {
            return true;
        }
        current = scopes
            .iter()
            .find(|scope| scope.id == id)
            .and_then(|scope| scope.parent);
    }
    false
}

/// 返回最近的函数或表字段辅助帧；程序及模块入口为 None，不混用不同帧的同名槽。
pub(super) fn scope_function_owner(scopes: &[xiao_ir::IrScope], id: u32) -> Option<u32> {
    let mut current = Some(id);
    while let Some(id) = current {
        let scope = scopes.iter().find(|scope| scope.id == id)?;
        if matches!(scope.kind.as_str(), "function" | "table") {
            return Some(id);
        }
        current = scope.parent;
    }
    None
}

/// 判断类型是否需要 Runtime 对象或 ABI 值。
fn type_uses_runtime(ty: &IrType) -> bool {
    match ty {
        IrType::None => false,
        IrType::Scalar { name } => matches!(name.as_str(), "str" | "lint" | "lfloat"),
        IrType::Dynamic
        | IrType::Array { .. }
        | IrType::Tuple { .. }
        | IrType::DictTable { .. }
        | IrType::DictColumn { .. }
        | IrType::Set { .. }
        | IrType::Table { .. }
        | IrType::Variable { .. } => true,
        IrType::Function {
            parameters,
            return_type,
        } => parameters.iter().any(type_uses_runtime) || type_uses_runtime(return_type),
    }
}

/// 递归检查程序是否含有动态值、容器或表。
pub(crate) fn program_uses_runtime(program: &IrProgram) -> bool {
    if !program.table_signatures.is_empty()
        || !program.runtime_checks.is_empty()
        || !program.modules.is_empty()
    {
        return true;
    }
    program.body.iter().any(statement_uses_runtime)
}

/// 递归检查语句是否含有动态值。
fn statement_uses_runtime(statement: &IrStatement) -> bool {
    match &statement.kind {
        IrStatementKind::Expression { value }
        | IrStatementKind::Assignment { value, .. }
        | IrStatementKind::ConstDeclaration { value, .. } => expression_uses_runtime(value),
        IrStatementKind::ExtendedAssignment { target, value, .. } => {
            expression_uses_runtime(target) || expression_uses_runtime(value)
        }
        IrStatementKind::Declaration {
            declared_type,
            value,
            ..
        } => {
            type_uses_runtime(declared_type) || value.as_ref().is_some_and(expression_uses_runtime)
        }
        IrStatementKind::If {
            condition,
            body,
            elif_branches,
            else_body,
        } => {
            expression_uses_runtime(condition)
                || body.iter().any(statement_uses_runtime)
                || elif_branches.iter().any(|branch| {
                    expression_uses_runtime(&branch.condition)
                        || branch.body.iter().any(statement_uses_runtime)
                })
                || else_body
                    .as_deref()
                    .is_some_and(|body| body.iter().any(statement_uses_runtime))
        }
        IrStatementKind::While { condition, body } => {
            expression_uses_runtime(condition) || body.iter().any(statement_uses_runtime)
        }
        IrStatementKind::For { iterable, body, .. } => {
            expression_uses_runtime(iterable) || body.iter().any(statement_uses_runtime)
        }
        IrStatementKind::Return { value } => value.as_ref().is_some_and(expression_uses_runtime),
        IrStatementKind::Raise { .. } => true,
        IrStatementKind::Try { .. } => true,
        IrStatementKind::Table { .. } => true,
        IrStatementKind::Function {
            parameters,
            return_type,
            body,
            ..
        } => {
            parameters
                .iter()
                .any(|parameter| type_uses_runtime(&parameter.ty))
                || type_uses_runtime(return_type)
                || body.iter().any(statement_uses_runtime)
        }
        IrStatementKind::Import { .. } => true,
        IrStatementKind::Break | IrStatementKind::Continue => false,
    }
}

/// 递归检查表达式是否含有动态值。
fn expression_uses_runtime(expression: &IrExpression) -> bool {
    type_uses_runtime(&expression.ty)
        || match &expression.kind {
            IrExpressionKind::Array { elements }
            | IrExpressionKind::Tuple { elements }
            | IrExpressionKind::Set { elements } => elements.iter().any(expression_uses_runtime),
            IrExpressionKind::DictTable { entries } | IrExpressionKind::DictColumn { entries } => {
                entries
                    .iter()
                    .any(|entry| expression_uses_runtime(&entry.value))
            }
            IrExpressionKind::Group { expression }
            | IrExpressionKind::Cast { expression, .. }
            | IrExpressionKind::Unary {
                operand: expression,
                ..
            } => expression_uses_runtime(expression),
            IrExpressionKind::Binary { left, right, .. } => {
                expression_uses_runtime(left) || expression_uses_runtime(right)
            }
            IrExpressionKind::Call { callee, arguments }
            | IrExpressionKind::NewCall { callee, arguments } => {
                expression_uses_runtime(callee)
                    || arguments
                        .iter()
                        .any(|argument| expression_uses_runtime(&argument.value))
            }
            IrExpressionKind::IntrinsicCall { id, arguments } => {
                let intrinsic_requires_runtime = xiao_intrinsics::IntrinsicId::new(*id)
                    .and_then(xiao_intrinsics::active_by_id)
                    .is_some_and(|declaration| {
                        matches!(
                            declaration.kind,
                            xiao_intrinsics::IntrinsicKind::Print
                                | xiao_intrinsics::IntrinsicKind::Input
                                | xiao_intrinsics::IntrinsicKind::SetConstructor
                                | xiao_intrinsics::IntrinsicKind::ErrorConstructor
                        )
                    });
                intrinsic_requires_runtime
                    || arguments
                        .iter()
                        .any(|argument| expression_uses_runtime(&argument.value))
            }
            IrExpressionKind::Member { object, .. } => expression_uses_runtime(object),
            IrExpressionKind::Selector { source, step, .. } => {
                expression_uses_runtime(source)
                    || step.as_deref().is_some_and(expression_uses_runtime)
            }
            IrExpressionKind::Literal { .. } | IrExpressionKind::Name { .. } => false,
        }
}

/// 判断类型是否需要数组、元组、字典或集合的 Runtime ABI。
fn type_uses_container_abi(ty: &IrType) -> bool {
    match ty {
        IrType::Array { .. }
        | IrType::Tuple { .. }
        | IrType::DictTable { .. }
        | IrType::DictColumn { .. }
        | IrType::Set { .. } => true,
        IrType::Function {
            parameters,
            return_type,
        } => parameters.iter().any(type_uses_container_abi) || type_uses_container_abi(return_type),
        _ => false,
    }
}

/// 递归判断语句是否实际触及容器值。
pub(super) fn statement_uses_container_abi(statement: &IrStatement) -> bool {
    match &statement.kind {
        IrStatementKind::Expression { value }
        | IrStatementKind::Assignment { value, .. }
        | IrStatementKind::ConstDeclaration { value, .. } => expression_uses_container_abi(value),
        IrStatementKind::ExtendedAssignment { target, value, .. } => {
            expression_uses_container_abi(target) || expression_uses_container_abi(value)
        }
        IrStatementKind::Declaration {
            declared_type,
            value,
            ..
        } => {
            type_uses_container_abi(declared_type)
                || value.as_ref().is_some_and(expression_uses_container_abi)
        }
        IrStatementKind::If {
            condition,
            body,
            elif_branches,
            else_body,
        } => {
            expression_uses_container_abi(condition)
                || body.iter().any(statement_uses_container_abi)
                || elif_branches.iter().any(|branch| {
                    expression_uses_container_abi(&branch.condition)
                        || branch.body.iter().any(statement_uses_container_abi)
                })
                || else_body
                    .as_deref()
                    .is_some_and(|body| body.iter().any(statement_uses_container_abi))
        }
        IrStatementKind::While { condition, body } => {
            expression_uses_container_abi(condition)
                || body.iter().any(statement_uses_container_abi)
        }
        IrStatementKind::For { iterable, body, .. } => {
            expression_uses_container_abi(iterable) || body.iter().any(statement_uses_container_abi)
        }
        IrStatementKind::Return { value } => {
            value.as_ref().is_some_and(expression_uses_container_abi)
        }
        IrStatementKind::Raise { value } => expression_uses_container_abi(value),
        IrStatementKind::Try {
            body,
            catches,
            finally_body,
        } => {
            body.iter().any(statement_uses_container_abi)
                || catches
                    .iter()
                    .any(|catch| catch.body.iter().any(statement_uses_container_abi))
                || finally_body
                    .as_deref()
                    .is_some_and(|body| body.iter().any(statement_uses_container_abi))
        }
        IrStatementKind::Table { body, .. } => body.iter().any(statement_uses_container_abi),
        IrStatementKind::Function {
            parameters,
            return_type,
            body,
            ..
        } => {
            parameters
                .iter()
                .any(|parameter| type_uses_container_abi(&parameter.ty))
                || type_uses_container_abi(return_type)
                || body.iter().any(statement_uses_container_abi)
        }
        IrStatementKind::Import { .. } | IrStatementKind::Break | IrStatementKind::Continue => {
            false
        }
    }
}

/// 递归判断表达式是否实际构造或访问容器值。
fn expression_uses_container_abi(expression: &IrExpression) -> bool {
    type_uses_container_abi(&expression.ty)
        || match &expression.kind {
            IrExpressionKind::Array { elements }
            | IrExpressionKind::Tuple { elements }
            | IrExpressionKind::Set { elements } => {
                elements.iter().any(expression_uses_container_abi)
            }
            IrExpressionKind::DictTable { entries } | IrExpressionKind::DictColumn { entries } => {
                entries
                    .iter()
                    .any(|entry| expression_uses_container_abi(&entry.value))
            }
            IrExpressionKind::Group { expression }
            | IrExpressionKind::Cast { expression, .. }
            | IrExpressionKind::Unary {
                operand: expression,
                ..
            } => expression_uses_container_abi(expression),
            IrExpressionKind::Binary { left, right, .. } => {
                expression_uses_container_abi(left) || expression_uses_container_abi(right)
            }
            IrExpressionKind::Call { callee, arguments }
            | IrExpressionKind::NewCall { callee, arguments } => {
                expression_uses_container_abi(callee)
                    || arguments
                        .iter()
                        .any(|argument| expression_uses_container_abi(&argument.value))
            }
            IrExpressionKind::IntrinsicCall { arguments, .. } => arguments
                .iter()
                .any(|argument| expression_uses_container_abi(&argument.value)),
            IrExpressionKind::Member { object, .. } => expression_uses_container_abi(object),
            IrExpressionKind::Selector { source, step, .. } => {
                expression_uses_container_abi(source)
                    || step.as_deref().is_some_and(expression_uses_container_abi)
            }
            IrExpressionKind::Literal { .. } | IrExpressionKind::Name { .. } => false,
        }
}
