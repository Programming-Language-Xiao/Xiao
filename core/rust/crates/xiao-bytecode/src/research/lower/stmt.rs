//! 语句的三地址降低与块结构重建。
//!
//! 块结构按语句结构重建：`if` 产生分支块与合流块，`while` 产生条件、循环体和
//! 出口三块。这里**不消费** `IrBasicBlock` 作为块划分依据，它只用于把语句映射
//! 回所属作用域。

use xiao_ir::{IrElifBranch, IrExpression, IrExpressionKind, IrSpan, IrStatement, IrStatementKind};

use crate::research::lower::Lowerer;
use crate::research::tac::{TacInstr, TacOp, VReg};

/// 降低一个语句列表，并在结束时按 `exit` 离开所属作用域。
pub(super) fn lower_statements(lowerer: &mut Lowerer<'_>, statements: &[IrStatement], exit: &str) {
    let Some(first) = statements.first() else {
        return;
    };
    let scope = lowerer.scope_of_statement(first.span);
    let entered = scope.filter(|scope| lowerer.innermost_scope() != Some(*scope));
    if let Some(scope) = entered {
        lowerer.enter_scope(scope, first.span);
    }
    for statement in statements {
        lower_statement(lowerer, statement);
    }
    if let Some(scope) = entered {
        lowerer.exit_scope(scope, exit);
    }
}

/// 降低一条语句，并在末尾释放本语句产生的临时堆值。
///
/// 临时值不进释放计划，必须在消费点之后显式释放，否则字面量堆值会一直漏。
fn lower_statement(lowerer: &mut Lowerer<'_>, statement: &IrStatement) {
    dispatch_statement(lowerer, statement);
    lowerer.flush_temporaries(statement.span);
}

/// 按语句形态分派降低。
fn dispatch_statement(lowerer: &mut Lowerer<'_>, statement: &IrStatement) {
    match &statement.kind {
        IrStatementKind::Expression { value } => {
            lowerer.lower_expression(value);
        }
        IrStatementKind::Assignment { target, value }
        | IrStatementKind::Declaration {
            target,
            value: Some(value),
            ..
        }
        | IrStatementKind::ConstDeclaration { target, value, .. } => {
            let source = lowerer.lower_expression(value);
            store_into(lowerer, &target.text, target.span, source);
        }
        IrStatementKind::Declaration { value: None, .. } => {}
        IrStatementKind::ExtendedAssignment {
            target,
            operator,
            value,
        } => lower_extended_assignment(lowerer, target, operator, value, statement.span),
        IrStatementKind::If {
            condition,
            body,
            elif_branches,
            else_body,
        } => lower_if(
            lowerer,
            condition,
            body,
            elif_branches,
            else_body.as_deref(),
            statement.span,
        ),
        IrStatementKind::While { condition, body } => {
            lower_while(lowerer, condition, body, statement.span);
        }
        IrStatementKind::Return { value } => {
            let register = value.as_ref().map(|value| lowerer.lower_expression(value));
            for scope in lowerer.scope_chain_to("function") {
                lowerer.run_plan(scope, "return", statement.span);
            }
            lowerer.emit(TacInstr::new(
                TacOp::Return { value: register },
                statement.span,
            ));
        }
        IrStatementKind::Break => lower_loop_jump(lowerer, "break", statement.span),
        IrStatementKind::Continue => lower_loop_jump(lowerer, "continue", statement.span),
        IrStatementKind::Raise { value } => {
            let register = lowerer.lower_expression(value);
            lowerer.flush_temporaries(statement.span);
            for scope in lowerer.scope_chain_to("function") {
                lowerer.run_plan(scope, "raise", statement.span);
            }
            lowerer.emit(TacInstr::new(
                TacOp::Raise { value: register },
                statement.span,
            ));
        }
        IrStatementKind::Function { .. } | IrStatementKind::Import { .. } => {}
        IrStatementKind::Try { .. }
        | IrStatementKind::For { .. }
        | IrStatementKind::Table { .. } => {
            lowerer.record_unsupported(format!(
                "语句形态尚未降低（{}..{}）",
                statement.span.start, statement.span.end
            ));
        }
    }
}

/// 把表达式结果写入目标绑定。
fn store_into(lowerer: &mut Lowerer<'_>, name: &str, span: IrSpan, source: VReg) {
    let Some(value) = lowerer.value_of_name_at(name, span) else {
        lowerer.record_unsupported(format!(
            "绑定缺少生命周期条目（{}..{}）",
            span.start, span.end
        ));
        return;
    };
    let target = lowerer.register_of(value);
    if target != source {
        lowerer.emit(TacInstr::with_dst(TacOp::Move(source), target, span));
    }
}

/// 降低复合赋值：等价于 `目标 = 目标 <op> 值`。
fn lower_extended_assignment(
    lowerer: &mut Lowerer<'_>,
    target: &IrExpression,
    operator: &str,
    value: &IrExpression,
    span: IrSpan,
) {
    let IrExpressionKind::Name { name } = &target.kind else {
        lowerer.record_unsupported(format!(
            "复合赋值的左值形态尚未降低（{}..{}）",
            target.span.start, target.span.end
        ));
        return;
    };
    let base = operator.strip_suffix('=').unwrap_or(operator);
    let left = IrExpression {
        kind: IrExpressionKind::Name { name: name.clone() },
        ty: target.ty.clone(),
        span: target.span,
    };
    let combined = IrExpression {
        kind: IrExpressionKind::Binary {
            operator: base.to_owned(),
            left: Box::new(left),
            right: Box::new(value.clone()),
        },
        ty: value.ty.clone(),
        span,
    };
    let source = lowerer.lower_expression(&combined);
    store_into(lowerer, &name.text, name.span, source);
}

/// 降低 `if`/`elif`/`else`。
///
/// 每个分支的条件在前驱块里求值，分支体各自成块并以跳转汇入合流块。
fn lower_if(
    lowerer: &mut Lowerer<'_>,
    condition: &IrExpression,
    body: &[IrStatement],
    elif_branches: &[IrElifBranch],
    else_body: Option<&[IrStatement]>,
    span: IrSpan,
) {
    let end = lowerer.new_block(span);
    let mut branches: Vec<(&IrExpression, &[IrStatement])> = vec![(condition, body)];
    for branch in elif_branches {
        branches.push((&branch.condition, &branch.body));
    }
    let last = branches.len() - 1;
    let else_entry = else_body.map(|_| lowerer.new_block(span));
    for (index, (branch_condition, branch_body)) in branches.into_iter().enumerate() {
        let taken = lowerer.new_block(span);
        let next = if index == last {
            else_entry.unwrap_or(end)
        } else {
            lowerer.new_block(span)
        };
        let value = lowerer.lower_expression(branch_condition);
        lowerer.emit(TacInstr::new(
            TacOp::BranchIf {
                condition: value,
                if_true: taken,
                if_false: next,
            },
            span,
        ));
        lowerer.flush_temporaries(span);
        lowerer.switch_to(taken);
        lower_statements(lowerer, branch_body, "normal");
        lowerer.emit(TacInstr::new(TacOp::Jump(end), span));
        lowerer.switch_to(next);
    }
    if let Some(else_body) = else_body {
        lower_statements(lowerer, else_body, "normal");
        lowerer.emit(TacInstr::new(TacOp::Jump(end), span));
    }
    lowerer.switch_to(end);
}

/// 降低 `while`。
///
/// 入口跳转发进前驱块，条件在循环头求值，循环体回到循环头。
fn lower_while(
    lowerer: &mut Lowerer<'_>,
    condition: &IrExpression,
    body: &[IrStatement],
    span: IrSpan,
) {
    let header = lowerer.new_block(span);
    lowerer.emit(TacInstr::new(TacOp::Jump(header), span));
    lowerer.switch_to(header);
    let body_block = lowerer.new_block(span);
    let exit = lowerer.new_block(span);
    let value = lowerer.lower_expression(condition);
    lowerer.emit(TacInstr::new(
        TacOp::BranchIf {
            condition: value,
            if_true: body_block,
            if_false: exit,
        },
        span,
    ));
    lowerer.flush_temporaries(span);
    lowerer.switch_to(body_block);
    lowerer.push_loop(body_block, exit);
    lower_statements(lowerer, body, "normal");
    lowerer.pop_loop();
    lowerer.emit(TacInstr::new(TacOp::Jump(header), span));
    lowerer.switch_to(exit);
}

/// 降低 `break`/`continue`。
fn lower_loop_jump(lowerer: &mut Lowerer<'_>, exit: &str, span: IrSpan) {
    let Some(target) = lowerer.loop_target(exit).copied() else {
        lowerer.record_unsupported(format!(
            "循环跳转没有活动循环（{}..{}）",
            span.start, span.end
        ));
        return;
    };
    for scope in lowerer.scope_chain_to("loop") {
        lowerer.run_plan(scope, exit, span);
    }
    lowerer.emit(TacInstr::new(TacOp::Jump(target), span));
}
