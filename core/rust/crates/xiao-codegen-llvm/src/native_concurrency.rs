//! LLVM 后端消费的并发调用计划。
//!
//! 并发调用已经在类型阶段解析为 [`xiao_ir::IrConcurrencyCall`]。本模块只把同一份
//! IR 记录整理成按源码区间索引的只读视图；它不会读取源码名称、导入路径或 TAC，
//! 也不会为缺少计划的普通调用猜测并发语义。

use std::collections::BTreeMap;

use xiao_ir::{
    IrBoundaryTransfer, IrCancellationCheck, IrCrossThreadFact, IrExpression, IrExpressionKind,
    IrProgram, IrSelectorItem, IrSpan, IrStatement, IrStatementKind, IrType,
};

use crate::error::{CodegenError, Result};

/// 22A 冻结的并发操作名称。
pub const NATIVE_CONCURRENCY_OPERATIONS: &[&str] = &[
    "runtime_create",
    "current_runtime",
    "runtime_close",
    "spawn",
    "join",
    "detach",
    "cancel",
    "shared_read",
    "shared_mut",
    "read",
    "lock",
    "lock_read",
    "lock_write",
    "pool_create",
    "submit",
    "wait",
    "shutdown",
];

/// 一个可由 LLVM 直接消费的并发调用记录。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NativeConcurrencyCall {
    /// 冻结的操作名称。
    pub operation: String,
    /// 调用源码区间。
    pub span: IrSpan,
    /// 参数源码区间（按求值顺序）。
    pub argument_spans: Vec<IrSpan>,
    /// 已由类型阶段验证的参数类型。
    pub argument_types: Vec<xiao_ir::IrType>,
    /// 已由类型阶段验证的结果类型。
    pub result_type: xiao_ir::IrType,
    /// 与此调用关联的跨线程事实。
    pub cross_thread_facts: Vec<IrCrossThreadFact>,
    /// 与此调用关联的边界复制/序列化动作。
    pub boundary_transfer: Option<IrBoundaryTransfer>,
    /// 与此调用关联的取消检查。
    pub cancellation_checks: Vec<IrCancellationCheck>,
}

impl NativeConcurrencyCall {
    /// 判断操作是否属于冻结的并发入口。
    #[must_use]
    pub fn is_known_operation(&self) -> bool {
        NATIVE_CONCURRENCY_OPERATIONS.contains(&self.operation.as_str())
    }
}

/// 同源 IR 并发调用计划；按 `IrSpan` 唯一索引。
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct NativeConcurrencyPlan {
    calls: Vec<NativeConcurrencyCall>,
    by_span: BTreeMap<IrSpan, usize>,
    by_expression: BTreeMap<usize, usize>,
}

impl NativeConcurrencyPlan {
    /// 从已经验证的 IR 建立只读原生计划。
    ///
    /// 这里只检查计划内部的一致性。未知操作会在真正降低时以结构化
    /// [`CodegenError::Unsupported`] 拒绝，从而保留 D-9 的 fail-closed 行为。
    pub fn from_program(program: &IrProgram) -> Result<Self> {
        let mut expression_calls = BTreeMap::new();
        collect_statement_calls(&program.body, &mut expression_calls);
        for module in &program.modules {
            if module.has_concurrency {
                return Err(CodegenError::Unsupported {
                    feature: "模块体并发暂不支持".to_owned(),
                    span: Some(program.span),
                });
            }
            let mut module_calls = BTreeMap::new();
            collect_statement_calls(&module.body, &mut module_calls);
            if let Some(expression) = module_calls
                .values()
                .flatten()
                .find(|expression| expression_uses_concurrency_boundary(expression))
            {
                return Err(CodegenError::Unsupported {
                    feature: "模块体并发暂不支持".to_owned(),
                    span: Some(expression.span),
                });
            }
        }
        let mut plan = Self::default();
        for call in &program.concurrency_calls {
            let matching = expression_calls
                .get(&call.span)
                .into_iter()
                .flatten()
                .filter(|expression| {
                    let IrExpressionKind::Call { arguments, .. } = &expression.kind else {
                        return false;
                    };
                    expression.ty == call.result_type
                        && arguments
                            .iter()
                            .map(|argument| argument.value.span)
                            .eq(call.argument_spans.iter().copied())
                        && arguments
                            .iter()
                            .map(|argument| &argument.value.ty)
                            .eq(call.argument_types.iter())
                })
                .copied()
                .collect::<Vec<_>>();
            if matching.len() != 1 {
                return Err(CodegenError::InvalidIr {
                    message: format!(
                        "并发调用计划 {}..{} 必须唯一对应参数和结果类型一致的 IR Call",
                        call.span.start, call.span.end
                    ),
                });
            }
            if plan.by_span.contains_key(&call.span) {
                return Err(CodegenError::InvalidIr {
                    message: format!("并发调用 span 重复：{}..{}", call.span.start, call.span.end),
                });
            }
            if call.argument_spans.len() != call.argument_types.len() {
                return Err(CodegenError::InvalidIr {
                    message: format!("并发调用 {} 的参数 span/类型数量不一致", call.operation),
                });
            }
            let boundary_transfer = program
                .ownership
                .boundary_transfers
                .iter()
                .find(|transfer| transfer.span == call.span)
                .cloned();
            let cross_thread_facts = program
                .ownership
                .cross_thread_facts
                .iter()
                .filter(|fact| fact.span == call.span)
                .cloned()
                .collect();
            let cancellation_checks = program
                .ownership
                .cancellation_checks
                .iter()
                .filter(|check| check.span == call.span)
                .cloned()
                .collect();
            let index = plan.calls.len();
            plan.by_span.insert(call.span, index);
            plan.by_expression
                .insert(matching[0] as *const IrExpression as usize, index);
            plan.calls.push(NativeConcurrencyCall {
                operation: call.operation.clone(),
                span: call.span,
                argument_spans: call.argument_spans.clone(),
                argument_types: call.argument_types.clone(),
                result_type: call.result_type.clone(),
                cross_thread_facts,
                boundary_transfer,
                cancellation_checks,
            });
        }
        for span in program
            .ownership
            .cross_thread_facts
            .iter()
            .map(|fact| fact.span)
            .chain(
                program
                    .ownership
                    .boundary_transfers
                    .iter()
                    .map(|transfer| transfer.span),
            )
            .chain(
                program
                    .ownership
                    .cancellation_checks
                    .iter()
                    .map(|check| check.span),
            )
        {
            if !plan.by_span.contains_key(&span) {
                return Err(CodegenError::InvalidIr {
                    message: format!(
                        "并发 facts/transfers/cancellation {}..{} 没有并发调用计划",
                        span.start, span.end
                    ),
                });
            }
        }
        Ok(plan)
    }

    /// 返回所有调用；顺序与 IR 计划表完全一致。
    #[must_use]
    pub fn calls(&self) -> &[NativeConcurrencyCall] {
        &self.calls
    }

    /// 按源码区间查找唯一并发调用。
    #[must_use]
    pub fn lookup(&self, span: IrSpan) -> Option<&NativeConcurrencyCall> {
        self.by_span
            .get(&span)
            .and_then(|index| self.calls.get(*index))
    }

    /// 以已经验证过的入口 IR 节点身份查询，避免模块局部源码区间误命中。
    #[must_use]
    pub(crate) fn lookup_expression(
        &self,
        expression: &IrExpression,
    ) -> Option<&NativeConcurrencyCall> {
        self.by_expression
            .get(&(expression as *const IrExpression as usize))
            .and_then(|index| self.calls.get(*index))
    }

    /// 判断计划是否为空。
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.calls.is_empty()
    }

    /// 对尚未接入 Runtime ABI 的操作给出结构化拒绝。
    pub fn require_known_operation(&self, call: &NativeConcurrencyCall) -> Result<()> {
        if call.is_known_operation() {
            Ok(())
        } else {
            Err(CodegenError::Unsupported {
                feature: format!("原生并发入口 {}", call.operation),
                span: Some(call.span),
            })
        }
    }
}

/// 收集同一 IR 中所有普通 Call 表达式区间；该索引只做结构对账，不解析调用名称。
fn collect_statement_calls<'a>(
    statements: &'a [IrStatement],
    calls: &mut BTreeMap<IrSpan, Vec<&'a IrExpression>>,
) {
    for statement in statements {
        match &statement.kind {
            IrStatementKind::Expression { value }
            | IrStatementKind::Assignment { value, .. }
            | IrStatementKind::ConstDeclaration { value, .. }
            | IrStatementKind::Declaration {
                value: Some(value), ..
            }
            | IrStatementKind::Raise { value } => collect_expression_calls(value, calls),
            IrStatementKind::ExtendedAssignment { target, value, .. } => {
                collect_expression_calls(target, calls);
                collect_expression_calls(value, calls);
            }
            IrStatementKind::If {
                condition,
                body,
                elif_branches,
                else_body,
            } => {
                collect_expression_calls(condition, calls);
                collect_statement_calls(body, calls);
                for branch in elif_branches {
                    collect_expression_calls(&branch.condition, calls);
                    collect_statement_calls(&branch.body, calls);
                }
                if let Some(body) = else_body {
                    collect_statement_calls(body, calls);
                }
            }
            IrStatementKind::While { condition, body } => {
                collect_expression_calls(condition, calls);
                collect_statement_calls(body, calls);
            }
            IrStatementKind::For { iterable, body, .. } => {
                collect_expression_calls(iterable, calls);
                collect_statement_calls(body, calls);
            }
            IrStatementKind::Return { value } => {
                if let Some(value) = value {
                    collect_expression_calls(value, calls);
                }
            }
            IrStatementKind::Function {
                parameters, body, ..
            } => {
                for parameter in parameters {
                    if let Some(default) = &parameter.default {
                        collect_expression_calls(default, calls);
                    }
                }
                collect_statement_calls(body, calls);
            }
            IrStatementKind::Table { body, .. } => {
                collect_statement_calls(body, calls);
            }
            IrStatementKind::Try {
                body,
                catches,
                finally_body,
            } => {
                collect_statement_calls(body, calls);
                for catch in catches {
                    collect_statement_calls(&catch.body, calls);
                }
                if let Some(body) = finally_body {
                    collect_statement_calls(body, calls);
                }
            }
            IrStatementKind::Declaration { value: None, .. }
            | IrStatementKind::Import { .. }
            | IrStatementKind::Break
            | IrStatementKind::Continue => {}
        }
    }
}

/// 递归收集 IR 普通调用节点；只记录结构与源码区间，不检查名称。
fn collect_expression_calls<'a>(
    expression: &'a IrExpression,
    calls: &mut BTreeMap<IrSpan, Vec<&'a IrExpression>>,
) {
    match &expression.kind {
        IrExpressionKind::Call {
            callee,
            arguments: args,
        } => {
            calls.entry(expression.span).or_default().push(expression);
            collect_expression_calls(callee, calls);
            collect_argument_calls(args, calls);
        }
        IrExpressionKind::NewCall {
            callee,
            arguments: args,
        } => {
            collect_expression_calls(callee, calls);
            collect_argument_calls(args, calls);
        }
        IrExpressionKind::IntrinsicCall {
            arguments: args, ..
        }
        | IrExpressionKind::DictBuiltin {
            arguments: args, ..
        } => collect_argument_calls(args, calls),
        IrExpressionKind::Array { elements }
        | IrExpressionKind::Tuple { elements }
        | IrExpressionKind::Set { elements } => {
            for element in elements {
                collect_expression_calls(element, calls);
            }
        }
        IrExpressionKind::DictTable { entries } | IrExpressionKind::DictColumn { entries } => {
            for entry in entries {
                collect_expression_calls(&entry.value, calls);
            }
        }
        IrExpressionKind::Group { expression: inner }
        | IrExpressionKind::Cast {
            expression: inner, ..
        }
        | IrExpressionKind::Unary { operand: inner, .. } => collect_expression_calls(inner, calls),
        IrExpressionKind::Binary { left, right, .. } => {
            collect_expression_calls(left, calls);
            collect_expression_calls(right, calls);
        }
        IrExpressionKind::Member { object, .. } => collect_expression_calls(object, calls),
        IrExpressionKind::Selector {
            source,
            selector,
            step,
            ..
        } => {
            collect_expression_calls(source, calls);
            if let Some(step) = step {
                collect_expression_calls(step, calls);
            }
            for item in &selector.items {
                match item {
                    IrSelectorItem::ExactExpression { expression, .. }
                    | IrSelectorItem::Random {
                        count: expression, ..
                    } => collect_expression_calls(expression, calls),
                    _ => {}
                }
            }
        }
        IrExpressionKind::Literal { .. } | IrExpressionKind::Name { .. } => {}
    }
}

/// 按调用参数顺序递归收集实参中的调用节点。
fn collect_argument_calls<'a>(
    arguments: &'a [xiao_ir::IrCallArgument],
    calls: &mut BTreeMap<IrSpan, Vec<&'a IrExpression>>,
) {
    for argument in arguments {
        collect_expression_calls(&argument.value, calls);
    }
}

/// 判断一个类型是否携带 22B/22C 并发边界值。
fn type_uses_concurrency_boundary(ty: &IrType) -> bool {
    match ty {
        IrType::Runtime
        | IrType::Pool
        | IrType::Thread { .. }
        | IrType::Task { .. }
        | IrType::Shared { .. }
        | IrType::SharedMut { .. }
        | IrType::LockGuard { .. } => true,
        IrType::Function {
            parameters,
            return_type,
        } => {
            parameters.iter().any(type_uses_concurrency_boundary)
                || type_uses_concurrency_boundary(return_type)
        }
        IrType::Array { shape } => match shape {
            xiao_ir::IrArrayShape::Homogeneous { element, .. } => {
                type_uses_concurrency_boundary(element)
            }
            xiao_ir::IrArrayShape::Heterogeneous { elements } => {
                elements.iter().any(type_uses_concurrency_boundary)
            }
            xiao_ir::IrArrayShape::Unknown => false,
        },
        IrType::Tuple { elements } => elements.iter().any(type_uses_concurrency_boundary),
        IrType::DictTable { entries } | IrType::DictColumn { entries } => entries
            .iter()
            .any(|entry| type_uses_concurrency_boundary(&entry.value)),
        IrType::Set { members, .. } => members.iter().any(type_uses_concurrency_boundary),
        _ => false,
    }
}

/// 模块体尚未建立独立的并发计划；在 lowering 前识别其边界调用并结构化拒绝，
/// 防止它们退回普通动态调用而产生误导诊断或 fail-open 行为。
fn expression_uses_concurrency_boundary(expression: &IrExpression) -> bool {
    let has_boundary_type = type_uses_concurrency_boundary(&expression.ty);
    match &expression.kind {
        IrExpressionKind::Call { callee, arguments } => {
            has_boundary_type
                || type_uses_concurrency_boundary(&callee.ty)
                || arguments.iter().any(|argument| {
                    type_uses_concurrency_boundary(&argument.value.ty)
                        || expression_uses_concurrency_boundary(&argument.value)
                })
                || expression_uses_concurrency_boundary(callee)
        }
        IrExpressionKind::NewCall { callee, arguments } => {
            has_boundary_type
                || arguments
                    .iter()
                    .any(|argument| expression_uses_concurrency_boundary(&argument.value))
                || expression_uses_concurrency_boundary(callee)
        }
        IrExpressionKind::IntrinsicCall { arguments, .. }
        | IrExpressionKind::DictBuiltin { arguments, .. } => {
            has_boundary_type
                || arguments
                    .iter()
                    .any(|argument| expression_uses_concurrency_boundary(&argument.value))
        }
        _ => expression_children(expression)
            .into_iter()
            .any(expression_uses_concurrency_boundary),
    }
}

/// 取得非调用表达式的递归子节点；只用于模块体并发边界预检。
fn expression_children(expression: &IrExpression) -> Vec<&IrExpression> {
    match &expression.kind {
        IrExpressionKind::Array { elements }
        | IrExpressionKind::Tuple { elements }
        | IrExpressionKind::Set { elements } => elements.iter().collect(),
        IrExpressionKind::DictTable { entries } | IrExpressionKind::DictColumn { entries } => {
            entries.iter().map(|entry| &entry.value).collect()
        }
        IrExpressionKind::Group { expression }
        | IrExpressionKind::Cast { expression, .. }
        | IrExpressionKind::Unary {
            operand: expression,
            ..
        } => vec![expression],
        IrExpressionKind::Binary { left, right, .. } => vec![left, right],
        IrExpressionKind::Member { object, .. } => vec![object],
        IrExpressionKind::Selector {
            source,
            step,
            selector,
            ..
        } => {
            let mut children = vec![source.as_ref()];
            if let Some(step) = step {
                children.push(step);
            }
            for item in &selector.items {
                match item {
                    xiao_ir::IrSelectorItem::ExactExpression { expression, .. }
                    | xiao_ir::IrSelectorItem::Random {
                        count: expression, ..
                    } => {
                        children.push(expression);
                    }
                    _ => {}
                }
            }
            children
        }
        IrExpressionKind::Literal { .. } | IrExpressionKind::Name { .. } => Vec::new(),
        IrExpressionKind::Call { .. }
        | IrExpressionKind::NewCall { .. }
        | IrExpressionKind::IntrinsicCall { .. }
        | IrExpressionKind::DictBuiltin { .. } => Vec::new(),
    }
}

/// 从同一份 IR 建立 LLVM 并发计划的公开入口。
pub fn native_concurrency_plan(program: &IrProgram) -> Result<NativeConcurrencyPlan> {
    NativeConcurrencyPlan::from_program(program)
}

#[cfg(test)]
/// 22D 内部并发适配实现。
mod tests {
    use super::*;
    use xiao_ir::{
        IrConcurrencyCall, IrEntryMode, IrExpression, IrExpressionKind, IrModule, IrName,
        IrProgram, IrSpan, IrStatement, IrStatementKind, IrType,
    };

    /// 构造用于计划单元测试的冻结并发调用记录。
    fn call(operation: &str, span: IrSpan) -> IrConcurrencyCall {
        IrConcurrencyCall {
            operation: operation.to_owned(),
            span,
            argument_spans: vec![span],
            argument_types: vec![IrType::Runtime],
            result_type: IrType::Runtime,
        }
    }

    /// 构造一个带运行时实参的普通 Call 节点。
    fn expression_call(span: IrSpan) -> IrStatement {
        IrStatement {
            kind: IrStatementKind::Expression {
                value: IrExpression {
                    kind: IrExpressionKind::Call {
                        callee: Box::new(IrExpression {
                            kind: IrExpressionKind::Name {
                                name: IrName {
                                    text: "f".to_owned(),
                                    backticked: false,
                                    span,
                                },
                            },
                            ty: IrType::Dynamic,
                            span,
                        }),
                        arguments: vec![xiao_ir::IrCallArgument {
                            kind: "positional".to_owned(),
                            name: None,
                            span,
                            value: IrExpression {
                                kind: IrExpressionKind::Name {
                                    name: IrName {
                                        text: "runtime".to_owned(),
                                        backticked: false,
                                        span,
                                    },
                                },
                                ty: IrType::Runtime,
                                span,
                            },
                        }],
                    },
                    ty: IrType::Runtime,
                    span,
                },
            },
            span,
            leading_docs: Vec::new(),
        }
    }

    #[test]
    /// 验证计划按已解析 IR 节点索引且不依赖 callee 名称。
    fn plan_indexes_ir_calls_without_name_resolution() {
        let span = IrSpan::new(2, 8);
        let mut program = IrProgram::new(IrEntryMode::Script, vec![expression_call(span)], span);
        program.concurrency_calls.push(call("spawn", span));
        let plan = NativeConcurrencyPlan::from_program(&program).expect("计划应建立");
        assert_eq!(
            plan.lookup(span).map(|call| call.operation.as_str()),
            Some("spawn")
        );
        assert!(plan.lookup(IrSpan::new(9, 10)).is_none());
    }

    #[test]
    /// 验证重复计划区间在 lowering 前被拒绝。
    fn duplicate_spans_are_rejected() {
        let span = IrSpan::new(2, 8);
        let mut program = IrProgram::new(IrEntryMode::Script, vec![expression_call(span)], span);
        program.concurrency_calls.push(call("spawn", span));
        program.concurrency_calls.push(call("join", span));
        assert!(NativeConcurrencyPlan::from_program(&program).is_err());
    }

    #[test]
    /// 验证未知并发操作不能落到普通动态调用。
    fn unknown_operation_is_fail_closed() {
        let span = IrSpan::new(2, 8);
        let mut program = IrProgram::new(IrEntryMode::Script, vec![expression_call(span)], span);
        program.concurrency_calls.push(call("future_api", span));
        let plan = NativeConcurrencyPlan::from_program(&program).expect("计划结构有效");
        let error = plan
            .require_known_operation(plan.lookup(span).expect("调用存在"))
            .expect_err("未知入口必须拒绝");
        assert!(error.to_string().contains("原生并发入口 future_api"));
    }

    #[test]
    /// 验证计划按入口节点匹配，不会被模块内相同局部区间干扰。
    fn metadata_must_match_one_typed_call_and_cannot_intercept_another_module() {
        let span = IrSpan::new(2, 8);
        let original = expression_call(span);
        let mut module_call = expression_call(span);
        let IrStatementKind::Expression { value } = &mut module_call.kind else {
            unreachable!()
        };
        value.ty = IrType::None;
        let IrExpressionKind::Call { arguments, .. } = &mut value.kind else {
            unreachable!()
        };
        arguments[0].value.ty = IrType::None;
        let mut program = IrProgram::new(IrEntryMode::Script, vec![original], span);
        program.modules.push(IrModule {
            name: "helper".to_owned(),
            kind: "file".to_owned(),
            path: "helper.xiao".to_owned(),
            body: vec![module_call],
            has_concurrency: false,
            symbols: Vec::new(),
        });
        program.concurrency_calls.push(call("runtime_close", span));
        let plan = NativeConcurrencyPlan::from_program(&program).unwrap();
        let IrStatementKind::Expression { value: first } = &program.body[0].kind else {
            unreachable!()
        };
        let IrStatementKind::Expression { value: second } = &program.modules[0].body[0].kind else {
            unreachable!()
        };
        assert!(plan.lookup_expression(first).is_some());
        assert!(plan.lookup_expression(second).is_none());
        program.concurrency_calls[0].argument_types[0] = IrType::None;
        assert!(NativeConcurrencyPlan::from_program(&program).is_err());
        program.concurrency_calls[0].argument_types[0] = IrType::Runtime;
        program.body.push(expression_call(span));
        assert!(
            NativeConcurrencyPlan::from_program(&program).is_err(),
            "入口内同形同区间的歧义输入必须拒绝"
        );
    }

    /// 没有并发调用计划时不允许丢弃所有权边界事实。
    #[test]
    fn orphan_concurrency_facts_are_rejected() {
        let span = IrSpan::new(2, 8);
        let mut program = IrProgram::new(IrEntryMode::Script, Vec::new(), span);
        program
            .ownership
            .cross_thread_facts
            .push(xiao_ir::IrCrossThreadFact::new(1, 1, true, true, span));
        assert!(NativeConcurrencyPlan::from_program(&program).is_err());
    }

    #[test]
    /// 模块体并发必须在 lowering 前以明确的阶段边界拒绝。
    fn module_concurrency_is_rejected_structurally() {
        let span = IrSpan::new(2, 8);
        let mut program = IrProgram::new(IrEntryMode::Script, Vec::new(), span);
        program.modules.push(IrModule {
            name: "helper".to_owned(),
            kind: "file".to_owned(),
            path: "helper.xiao".to_owned(),
            body: vec![expression_call(span)],
            has_concurrency: true,
            symbols: Vec::new(),
        });
        let error = NativeConcurrencyPlan::from_program(&program)
            .expect_err("模块体并发不能退回普通动态调用");
        assert!(matches!(
            error,
            CodegenError::Unsupported { feature, .. }
                if feature == "模块体并发暂不支持"
        ));
    }
}
