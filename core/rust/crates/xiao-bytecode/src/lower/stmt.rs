//! 语句的三地址降低与块结构重建。
//!
//! 块结构按语句结构重建：`if` 产生分支块与合流块，`while` 产生条件、循环体和
//! 出口三块。这里**不消费** `IrBasicBlock` 作为块划分依据，它只用于把语句映射
//! 回所属作用域。

use xiao_ir::{
    IrCatchClause, IrElifBranch, IrExpression, IrExpressionKind, IrSpan, IrStatement,
    IrStatementKind,
};

use crate::lower::Lowerer;
use crate::tac::{
    ArithOp, CompareOp, RegisterClass, TacConstant, TacHandler, TacInstr, TacOp, VReg,
};

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
        if lowerer.current_block_terminated() {
            break;
        }
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
    if lowerer.current_block_terminated() {
        // 返回、跳转和抛错已经把控制流交给目标；临时值若随终止值
        // 一起转移则不能再发一条不可达 Release。
        lowerer.discard_temporaries();
    } else {
        lowerer.flush_temporaries(statement.span);
    }
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
            store_into(
                lowerer,
                &target.text,
                target.backticked,
                target.span,
                source,
            );
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
            // `raise` 的动态转换检查挂在语句跨度上，而不是表达式跨度上；
            // 显式消费这条检查，失败才能走 `DynamicCheckFailure` 边。
            lowerer.emit_runtime_checks(statement.span, register);
            lowerer.flush_temporaries(statement.span);
            lowerer.emit(TacInstr::new(
                TacOp::Raise { value: register },
                statement.span,
            ));
        }
        IrStatementKind::Function { .. } | IrStatementKind::Import { .. } => {}
        IrStatementKind::Try {
            body,
            catches,
            finally_body,
        } => lower_try(
            lowerer,
            body,
            catches,
            finally_body.as_deref(),
            statement.span,
        ),
        IrStatementKind::For {
            target,
            iterable,
            body,
        } => lower_for(lowerer, target, iterable, body, statement.span),
        IrStatementKind::Table {
            name, table_kind, ..
        } => lowerer.lower_table_declaration(name, table_kind, statement.span),
    }
}

/// 降低一个语句列表但暂不退出最外层作用域。
///
/// `try` 需要先执行 `finally` 再执行该作用域的释放计划，不能直接使用普通
/// `lower_statements` 的「体尾立即退出」行为，因此把退出动作延后到接线阶段。
fn lower_statements_open(lowerer: &mut Lowerer<'_>, statements: &[IrStatement]) -> Option<u32> {
    let first = statements.first()?;
    let scope = lowerer.scope_of_statement(first.span);
    let entered = scope.filter(|scope| lowerer.innermost_scope() != Some(*scope));
    if let Some(scope) = entered {
        lowerer.enter_scope(scope, first.span);
    }
    for statement in statements {
        lower_statement(lowerer, statement);
        if lowerer.current_block_terminated() {
            break;
        }
    }
    entered
}

/// 降低 `try`/`catch`/`finally`，并建立保护区间和子程序处理器。
fn lower_try(
    lowerer: &mut Lowerer<'_>,
    body: &[IrStatement],
    catches: &[IrCatchClause],
    finally_body: Option<&[IrStatement]>,
    span: IrSpan,
) {
    // 先建立独立入口并从前置块跳入。若直接把前置块当作起点，处理器范围会
    // 把 try 之前的指令一并保护，嵌套 try 尤其容易误命中。
    let protected_start = lowerer.new_block(span);
    if !lowerer.current_block_terminated() {
        lowerer.emit(TacInstr::new(TacOp::Jump(protected_start), span));
    }
    lowerer.switch_to(protected_start);

    let try_scope = lowerer
        .scope_for_region(span, "try")
        .or_else(|| {
            body.first()
                .and_then(|statement| lowerer.scope_of_statement(statement.span))
        })
        .unwrap_or_else(|| lowerer.innermost_scope().unwrap_or(0));
    lowerer.enter_region_scope(try_scope, span);
    lower_statements_open(lowerer, body);
    let body_ended = lowerer.current_block_terminated();

    // `body_exit` 是正常离开 try 体的桥；它同时作为 catch 保护范围的终点，
    // 因而 catch 入口和 catch 体不会被同一条 catch 处理器再次捕获。
    let body_exit = lowerer.new_block(span);
    if !body_ended {
        lowerer.emit(TacInstr::new(TacOp::Jump(body_exit), span));
    }
    lowerer.forget_scope(try_scope);

    // catch 体没有 CFG 前驱，必须仍然保留为可由 handler 跳入的块。先建一个
    // 统一出口，catch 正常结束时只离开自己的作用域；本层 finally 已在路由
    // 进入 catch 前执行，不得在这里再跑一遍。
    let catch_exit = (!catches.is_empty()).then(|| lowerer.new_block(span));
    let mut catch_start = None;
    let mut catch_blocks = Vec::with_capacity(catches.len());
    let mut catch_bindings = Vec::with_capacity(catches.len());
    for catch in catches {
        let block = lowerer.new_block(span);
        catch_start.get_or_insert(block);
        catch_blocks.push(block);
        lowerer.switch_to(block);
        let catch_scope = lowerer.scope_for_region(catch.span, "catch").or_else(|| {
            catch
                .body
                .first()
                .and_then(|statement| lowerer.scope_of_statement(statement.span))
        });
        if let Some(scope) = catch_scope {
            lowerer.enter_region_scope(scope, catch.span);
        }
        lower_statements_open(lowerer, &catch.body);
        let binding = lowerer
            .value_of_name_at(
                &catch.binding.text,
                catch.binding.backticked,
                catch.binding.span,
            )
            .map(|value| lowerer.register_of(value));
        catch_bindings.push(binding);
        if !lowerer.current_block_terminated() {
            if let Some(scope) = catch_scope {
                lowerer.exit_scope(scope, "normal");
            }
            if let Some(exit) = catch_exit {
                lowerer.emit(TacInstr::new(TacOp::Jump(exit), catch.span));
            }
        } else if let Some(scope) = catch_scope {
            lowerer.forget_scope(scope);
        }
        lowerer.forget_scope(try_scope);
    }

    // finally 子程序必须在所有 catch 块之后分配，才能用 `[protected_start,
    // finally_sub)` 覆盖 try/catch 体而排除子程序自身。
    let finally_sub = finally_body.map(|_| lowerer.new_block(span));
    let continuation = lowerer.new_block(span);

    if let Some(exit) = catch_exit {
        lowerer.switch_to(exit);
        lowerer.emit(TacInstr::new(TacOp::Jump(continuation), span));
    }

    // 正常离开受保护体：finally -> drop -> continuation。
    lowerer.switch_to(body_exit);
    if let Some(sub) = finally_sub {
        lowerer.emit(TacInstr::new(TacOp::CallSub { sub }, span));
    }
    lowerer.exit_scope(try_scope, "normal");
    lowerer.emit(TacInstr::new(TacOp::Jump(continuation), span));

    if let Some(sub) = finally_sub {
        // 保护区内的 return 需要在已有 drop 计划之前调用子程序；插入范围
        // 严格止于 body_exit。catch 体的非局部退出走另一段连续块范围，
        // 也必须先调用本层 finally，但 catch 正常合流不应重复调用。
        lowerer.patch_nonlocal_with_finally(protected_start, body_exit, sub, span);
        if let Some(catch_start) = catch_start {
            lowerer.patch_nonlocal_with_finally(catch_start, sub, sub, span);
        }
        lowerer.add_handler(TacHandler {
            protected: (protected_start, sub),
            handler: sub,
            scope: try_scope,
            exit: "finally".to_owned(),
            catch_type: None,
            binding: None,
        });
        // 运行时从 body_exit 的 CallSub 进入 finally 时，try 作用域仍然
        // 活动；恢复降低器侧的栈可让 finally 内的 return/break/continue
        // 先发出该作用域的对应释放计划。
        lowerer.remember_scope(try_scope);
        lowerer.switch_to(sub);
        let finally_scope = lowerer.scope_for_region(span, "finally");
        if let Some(scope) = finally_scope {
            lowerer.enter_region_scope(scope, span);
        }
        if let Some(body) = finally_body {
            lower_statements_open(lowerer, body);
        }
        if !lowerer.current_block_terminated() {
            if let Some(scope) = finally_scope {
                lowerer.exit_scope(scope, "normal");
            }
            lowerer.emit(TacInstr::new(TacOp::RetFromSub, span));
        } else if let Some(scope) = finally_scope {
            lowerer.forget_scope(scope);
        }
        lowerer.forget_scope(try_scope);
    }

    for ((catch, block), binding) in catches
        .iter()
        .zip(catch_blocks.iter().copied())
        .zip(catch_bindings)
    {
        lowerer.add_handler(TacHandler {
            protected: (protected_start, body_exit),
            handler: block,
            scope: try_scope,
            exit: "catch".to_owned(),
            catch_type: Some(catch.error_type.text.clone()),
            binding,
        });
    }

    lowerer.switch_to(continuation);
}

/// 把表达式结果写入目标绑定。
pub(super) fn store_into(
    lowerer: &mut Lowerer<'_>,
    name: &str,
    backticked: bool,
    span: IrSpan,
    source: VReg,
) {
    let Some(value) = lowerer.value_of_name_at(name, backticked, span) else {
        lowerer.record_unsupported(format!(
            "绑定缺少生命周期条目（{}..{}）",
            span.start, span.end
        ));
        return;
    };
    let target = lowerer.register_of(value);
    if target == source {
        return;
    }
    // 所有权语义决定用哪条指令：临时值转移进绑定，具名绑定之间必须复制。
    // 一律用 `Move` 会让 `a = b` 清空 `b`，之后再用 `b` 就读到空寄存器。
    let op = if lowerer.is_pending_temporary(source) {
        TacOp::Move(source)
    } else {
        TacOp::Copy(source)
    };
    lowerer.emit(TacInstr::with_dst(op, target, span));
}

/// 降低复合赋值：等价于 `目标 = 目标 <op> 值`。
fn lower_extended_assignment(
    lowerer: &mut Lowerer<'_>,
    target: &IrExpression,
    operator: &str,
    value: &IrExpression,
    span: IrSpan,
) {
    if let IrExpressionKind::Member { object, member } = &target.kind {
        let object = lowerer.lower_expression(object);
        let member = super::name_key(&member.text, member.backticked);
        let source = if operator == "=" {
            lowerer.lower_expression(value)
        } else {
            let left = lowerer.new_register(Lowerer::class_of_type(&target.ty), span);
            lowerer.emit(TacInstr::with_dst(
                TacOp::MemberGet {
                    object,
                    member: member.clone(),
                },
                left,
                span,
            ));
            let name = format!("#member{}", left.get());
            let key = super::name_key(&name, false);
            lowerer.frame.parameter_names.insert(key.clone(), left);
            let combined = IrExpression {
                kind: IrExpressionKind::Binary {
                    operator: operator.strip_suffix('=').unwrap_or(operator).to_owned(),
                    left: Box::new(IrExpression {
                        kind: IrExpressionKind::Name {
                            name: xiao_ir::IrName {
                                text: name,
                                backticked: false,
                                span,
                            },
                        },
                        ty: target.ty.clone(),
                        span,
                    }),
                    right: Box::new(value.clone()),
                },
                ty: target.ty.clone(),
                span: target.span,
            };
            let register = lowerer.lower_expression(&combined);
            lowerer.frame.parameter_names.remove(&key);
            register
        };
        lowerer.emit(TacInstr::new(
            TacOp::MemberSet {
                object,
                member,
                value: source,
            },
            span,
        ));
        return;
    }
    if let IrExpressionKind::Selector { source, .. } = &target.kind {
        let Some(plan) = lowerer.broadcast_assignment_plan_id(target.span) else {
            lowerer.record_unsupported(format!(
                "选择器广播缺少类型计划（{}..{}）",
                target.span.start, target.span.end
            ));
            return;
        };
        let root = lowerer.lower_expression(source);
        let value = lowerer.lower_expression(value);
        lowerer.emit(TacInstr::new(
            TacOp::BroadcastAssign { root, value, plan },
            span,
        ));
        return;
    }
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
        // C2-C 将复合集合赋值的 RuntimeCheck 登记在左值名称跨度；
        // 使用语句跨度会让检查逃过二元表达式的精确消费并落入 unsupported。
        span: target.span,
    };
    let source = lowerer.lower_expression(&combined);
    store_into(lowerer, &name.text, name.backticked, name.span, source);
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
        if !lowerer.current_block_terminated() {
            lowerer.emit(TacInstr::new(TacOp::Jump(end), span));
        }
        lowerer.switch_to(next);
    }
    if let Some(else_body) = else_body {
        lower_statements(lowerer, else_body, "normal");
        if !lowerer.current_block_terminated() {
            lowerer.emit(TacInstr::new(TacOp::Jump(end), span));
        }
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
    // `continue` 必须回到条件头，重新判断循环是否继续；若跳到体入口，
    // 条件会被永久绕过并把一个合法程序变成无限循环。
    lowerer.push_loop(header, exit);
    lower_statements(lowerer, body, "normal");
    lowerer.pop_loop();
    if !lowerer.current_block_terminated() {
        lowerer.emit(TacInstr::new(TacOp::Jump(header), span));
    }
    lowerer.switch_to(exit);
}

/// 降低 `for target in iterable`。
///
/// 迭代器状态不进入 RuntimeValue：来源只求值一次，循环头比较游标和长度，
/// 每轮通过 `IndexGetDynamic` 取值。`continue` 先经过更新桥接块，保证不会
/// 跳过游标递增；来源表达式若产生临时句柄，则由循环出口统一释放。
fn lower_for(
    lowerer: &mut Lowerer<'_>,
    target: &xiao_ir::IrName,
    iterable: &IrExpression,
    body: &[IrStatement],
    span: IrSpan,
) {
    let source = lowerer.lower_expression(iterable);
    let held_source = lowerer.take_temporary(source).then_some(source);
    let length = lowerer.new_register(RegisterClass::Int, span);
    lowerer.emit(TacInstr::with_dst(TacOp::Len { source }, length, span));
    let index = lowerer.emit_constant(TacConstant::Int(0), span);
    let one = lowerer.emit_constant(TacConstant::Int(1), span);

    let header = lowerer.new_block(span);
    let body_block = lowerer.new_block(span);
    let advance = lowerer.new_block(span);
    let exit = lowerer.new_block(span);
    lowerer.emit(TacInstr::new(TacOp::Jump(header), span));

    lowerer.switch_to(header);
    let condition = lowerer.new_register(RegisterClass::Bool, span);
    lowerer.emit(TacInstr::with_dst(
        TacOp::Compare {
            op: CompareOp::Less,
            left: index,
            right: length,
        },
        condition,
        span,
    ));
    lowerer.emit(TacInstr::new(
        TacOp::BranchIf {
            condition,
            if_true: body_block,
            if_false: exit,
        },
        span,
    ));

    lowerer.switch_to(body_block);
    let Some(target_value) = lowerer.value_of_name_at(&target.text, target.backticked, target.span)
    else {
        lowerer.record_unsupported(format!(
            "循环绑定缺少生命周期条目（{}..{}）",
            target.span.start, target.span.end
        ));
        lowerer.switch_to(exit);
        return;
    };
    let element_class = lowerer.class_for_value(target_value);
    let element = lowerer.new_register(element_class, target.span);
    lowerer.emit(TacInstr::with_dst(
        TacOp::IndexGetDynamic { source, index },
        element,
        span,
    ));
    store_into(
        lowerer,
        &target.text,
        target.backticked,
        target.span,
        element,
    );
    // `store_into` 对新建对象使用 Move；源寄存器随后为空，不能让它污染
    // 下一轮或循环外的临时刷新列表。
    lowerer.take_temporary(element);

    lowerer.push_loop(advance, exit);
    lower_statements(lowerer, body, "normal");
    lowerer.pop_loop();
    if !lowerer.current_block_terminated() {
        lowerer.emit(TacInstr::new(TacOp::Jump(advance), span));
    }

    lowerer.switch_to(advance);
    lowerer.emit(TacInstr::with_dst(
        TacOp::Arith {
            op: ArithOp::Add,
            left: index,
            right: one,
        },
        index,
        span,
    ));
    lowerer.emit(TacInstr::new(TacOp::Jump(header), span));

    lowerer.switch_to(exit);
    if let Some(source) = held_source {
        lowerer.emit(TacInstr::new(
            TacOp::Release {
                value: source,
                kind: xiao_lifetime::ReleaseActionKind::Strong,
            },
            span,
        ));
    }
}

/// 降低 `break`/`continue`。
fn lower_loop_jump(lowerer: &mut Lowerer<'_>, exit: &str, span: IrSpan) {
    let Some(target) = lowerer.loop_target(exit) else {
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
