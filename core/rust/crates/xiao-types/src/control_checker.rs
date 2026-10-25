//! 04-C 条件、循环、返回和入口的静态检查。
//!
//! 控制流检查只建立词法作用域和类型约束，不执行分支，也不修改运行时
//! 状态。每个缩进块使用独立类型作用域；后续 IR 阶段可以据此生成基本块
//! 和确定性释放边。

use xiao_diagnostics::{CatchTypeKind, DiagnosticParam, error_kind_of};
use xiao_source::SourceSpan;
use xiao_syntax::{CatchClause, ElifBranch, Expression, Statement};

use crate::containers::ArrayType;
use crate::diagnostics::{
    CATCH_FATAL_CODE, CATCH_ORDER_CODE, CATCH_TYPE_CODE, CONDITION_TYPE_CODE, ITERABLE_TYPE_CODE,
    LOOP_CONTROL_CODE, RAISE_TYPE_CODE,
};
use crate::types::Type;

use super::{ConcurrencyFlow, ConcurrencyScopeState, RuntimeCheckKind, TypeChecker};

impl<'source> TypeChecker<'source> {
    /// 检查 `try` 的主体、按序 `catch` 和最终清理体。
    pub(super) fn check_try_statement(
        &mut self,
        body: &[Statement],
        catches: &[CatchClause],
        finally_body: Option<&[Statement]>,
        _span: SourceSpan,
    ) -> ConcurrencyFlow {
        let body_flow = self.check_scoped_body_with_concurrency_flow(body);
        let mut body_for_merge = body_flow.clone();
        let mut flows = Vec::new();
        if catches.is_empty() {
            flows.push(body_for_merge.clone());
        } else {
            body_for_merge.errors.clear();
            flows.push(body_for_merge.clone());
        }
        let mut saw_broad = false;
        for catch in catches {
            let type_name = self.source.slice(catch.error_type.span);
            let catch_kind = (!catch.error_type.backticked)
                .then(|| error_kind_of(type_name))
                .flatten();
            if type_name.is_empty() || catch_kind.is_none() {
                self.type_error(
                    CATCH_TYPE_CODE,
                    "x07.type.invalid_catch_type",
                    catch.error_type.span,
                    "catch 的错误类型必须是错误类型名称".to_string(),
                );
            }
            if matches!(catch_kind, Some(CatchTypeKind::Fatal)) {
                self.type_error(
                    CATCH_FATAL_CODE,
                    "x07.type.fatal_not_catchable",
                    catch.error_type.span,
                    "FatalError 不可由普通 catch 恢复".to_string(),
                );
            }
            let is_broad = matches!(catch_kind, Some(CatchTypeKind::AnyRecoverable));
            if saw_broad && !is_broad {
                self.type_error(
                    CATCH_ORDER_CODE,
                    "x07.type.catch_order",
                    catch.error_type.span,
                    "宽泛 catch 必须位于具体错误类型之后".to_string(),
                );
            }
            saw_broad |= is_broad;
            let catch_base = Self::merge_concurrency_scope_state_values(&body_flow.errors)
                .or_else(|| body_flow.normal.clone())
                .or_else(|| {
                    Self::merge_concurrency_scope_state_values(
                        &body_flow
                            .returns
                            .iter()
                            .chain(body_flow.breaks.iter())
                            .chain(body_flow.continues.iter())
                            .cloned()
                            .collect::<Vec<_>>(),
                    )
                })
                .unwrap_or_else(|| self.concurrency_scope_state());
            self.restore_concurrency_scope_state(&catch_base);
            self.push_scope();
            self.reset_concurrency_binding(&self.name_key(catch.binding));
            if let Err(error) =
                self.environment
                    .declare_mutable(self.name_key(catch.binding), Type::Dynamic, true)
            {
                self.environment_error(catch.binding.span, error);
            }
            self.register_top_level_functions(&catch.body);
            let catch_flow = self.check_statement_sequence(&catch.body);
            flows.push(self.pop_scope_with_concurrency_flow(catch_flow));
        }
        let mut result = Self::merge_concurrency_flows(&flows);
        if let Some(body) = finally_body {
            let finally_base = result
                .normal
                .clone()
                .or_else(|| {
                    Self::merge_concurrency_scope_state_values(
                        &result
                            .returns
                            .iter()
                            .chain(result.breaks.iter())
                            .chain(result.continues.iter())
                            .cloned()
                            .collect::<Vec<_>>(),
                    )
                })
                .unwrap_or_else(|| self.concurrency_scope_state());
            self.restore_concurrency_scope_state(&finally_base);
            let finally_flow = self.check_scoped_body_with_concurrency_flow(body);
            if finally_flow.normal.is_some() {
                let mut combined = finally_flow;
                combined.returns.extend(result.returns);
                combined.breaks.extend(result.breaks);
                combined.continues.extend(result.continues);
                combined.errors.extend(result.errors);
                result = combined;
            } else {
                result = finally_flow;
            }
        }
        let restore_state = result.normal.clone().or_else(|| {
            Self::merge_concurrency_scope_state_values(
                &result
                    .returns
                    .iter()
                    .chain(result.breaks.iter())
                    .chain(result.continues.iter())
                    .chain(result.errors.iter())
                    .cloned()
                    .collect::<Vec<_>>(),
            )
        });
        if let Some(state) = restore_state.as_ref() {
            self.restore_concurrency_scope_state(state);
        }
        self.set_last_concurrency_flow(result.clone());
        result
    }

    /// 检查 `raise` 只能抛出可恢复错误对象或动态错误边界。
    ///
    /// 动态值不登记 `dynamic_conversion`：`raise` 的 Runtime ABI 是错误
    /// 边界本身，负责校验并传播错误对象；如果在这里先做通用动态转换，
    /// 合法的错误值会被错误地改写成 `TypeError`，原生降低也无法与字节码
    /// 的 `Raise` 语义保持一致。
    pub(super) fn check_raise_statement(&mut self, value: &Expression, span: SourceSpan) {
        let ty = self.check_expression(value);
        if !ty.is_dynamic() && !matches!(ty, Type::Variable(_)) {
            self.type_error(
                RAISE_TYPE_CODE,
                "x07.type.raise_requires_error",
                span,
                "raise 的操作数必须是可恢复错误对象".to_string(),
            );
        }
    }

    /// 检查一条 `if`/`elif`/`else` 条件链。
    pub(super) fn check_if_statement(
        &mut self,
        condition: &Expression,
        body: &[Statement],
        elif_branches: &[ElifBranch],
        else_body: Option<&[Statement]>,
    ) -> ConcurrencyFlow {
        self.check_condition(condition);
        let mut false_state = self.concurrency_scope_state();
        let mut flows = vec![self.check_scoped_body_with_concurrency_flow(body)];
        for branch in elif_branches {
            self.restore_concurrency_scope_state(&false_state);
            self.check_condition(&branch.condition);
            false_state = self.concurrency_scope_state();
            flows.push(self.check_scoped_body_with_concurrency_flow(&branch.body));
        }
        if let Some(body) = else_body {
            self.restore_concurrency_scope_state(&false_state);
            flows.push(self.check_scoped_body_with_concurrency_flow(body));
        } else {
            flows.push(ConcurrencyFlow {
                normal: Some(false_state),
                ..ConcurrencyFlow::default()
            });
        }
        let result = Self::merge_concurrency_flows(&flows);
        if let Some(state) = result.normal.as_ref() {
            self.restore_concurrency_scope_state(state);
        }
        self.set_last_concurrency_flow(result.clone());
        result
    }

    /// 检查一个 `for name in iterable` 循环。
    pub(super) fn check_for_statement(
        &mut self,
        target: xiao_syntax::Name,
        iterable: &Expression,
        body: &[Statement],
    ) -> ConcurrencyFlow {
        let iterable_type = self.check_expression(iterable);
        let element_type = match iterable_element_type(&iterable_type) {
            Some(ty) => ty,
            None if matches!(iterable_type, Type::Variable(_)) => Type::Dynamic,
            None => {
                self.type_error_with_params(
                    ITERABLE_TYPE_CODE,
                    "x04.type.for_requires_iterable",
                    iterable.span(),
                    format!("for 的右侧必须是可迭代容器，实际为 {}", iterable_type),
                    [(
                        "actual_type".to_owned(),
                        DiagnosticParam::Text(iterable_type.to_string()),
                    )],
                );
                Type::Dynamic
            }
        };
        let dynamic_iterable =
            iterable_type.is_dynamic() || matches!(iterable_type, Type::Variable(_));
        if matches!(iterable_type, Type::Variable(_)) {
            // `for` 只要求运行时可迭代；未被其他约束解析的参数应像动态条件
            // 一样落到 Dynamic，而不是在函数收尾时留下无法推断的类型变量。
            self.context.bind_dynamic(&iterable_type);
        }
        if dynamic_iterable {
            self.push_runtime_check(iterable.span(), RuntimeCheckKind::Iterable);
        }
        let before_loop = self.concurrency_scope_state();
        self.push_concurrency_loop();
        self.loop_depth = self.loop_depth.saturating_add(1);
        self.push_scope();
        self.reset_concurrency_binding(&self.name_key(target));
        self.register_top_level_functions(body);
        if let Err(error) =
            self.environment
                .declare_mutable(self.name_key(target), element_type.clone(), true)
        {
            self.environment_error(target.span, error);
        } else {
            self.bind_concurrency_handle(self.name_key(target), &element_type, None);
        }
        let body_flow = self.check_statement_sequence(body);
        let body_flow = self.pop_scope_with_concurrency_flow(body_flow);
        self.pop_concurrency_loop();
        self.loop_depth = self.loop_depth.saturating_sub(1);
        let mut normal_states = vec![before_loop];
        normal_states.extend(body_flow.normal.iter().cloned());
        normal_states.extend(body_flow.breaks.iter().cloned());
        normal_states.extend(body_flow.continues.iter().cloned());
        let result = ConcurrencyFlow {
            normal: Self::merge_concurrency_scope_state_values(&normal_states),
            returns: body_flow.returns,
            errors: body_flow.errors,
            ..ConcurrencyFlow::default()
        };
        if let Some(state) = result.normal.as_ref() {
            self.restore_concurrency_scope_state(state);
        }
        self.set_last_concurrency_flow(result.clone());
        result
    }

    /// 检查一个 `while condition` 循环。
    pub(super) fn check_while_statement(
        &mut self,
        condition: &Expression,
        body: &[Statement],
    ) -> ConcurrencyFlow {
        self.push_concurrency_loop();
        self.loop_depth = self.loop_depth.saturating_add(1);
        self.check_condition(condition);
        let condition_state = self.concurrency_scope_state();
        let body_flow = self.check_scoped_body_with_concurrency_flow(body);
        self.pop_concurrency_loop();
        self.loop_depth = self.loop_depth.saturating_sub(1);
        let mut normal_states = vec![condition_state];
        normal_states.extend(body_flow.normal.iter().cloned());
        normal_states.extend(body_flow.breaks.iter().cloned());
        normal_states.extend(body_flow.continues.iter().cloned());
        let result = ConcurrencyFlow {
            normal: Self::merge_concurrency_scope_state_values(&normal_states),
            returns: body_flow.returns,
            errors: body_flow.errors,
            ..ConcurrencyFlow::default()
        };
        if let Some(state) = result.normal.as_ref() {
            self.restore_concurrency_scope_state(state);
        }
        self.set_last_concurrency_flow(result.clone());
        result
    }

    /// 检查 `return` 并统一到当前函数返回类型。
    pub(super) fn check_return_statement(&mut self, value: Option<&Expression>, span: SourceSpan) {
        let Some(expected) = self
            .current_function
            .as_ref()
            .map(|frame| frame.return_type.clone())
        else {
            self.type_error(
                crate::diagnostics::FUNCTION_RETURN_CODE,
                "x04.type.return_outside_function",
                span,
                "return 只能出现在函数体内".to_string(),
            );
            if let Some(value) = value {
                self.check_expression(value);
            }
            return;
        };
        let actual = value
            .map(|expression| self.check_expression(expression))
            .unwrap_or(Type::None);
        if let Some(frame) = self.current_function.as_mut() {
            frame.saw_return = true;
        }
        if matches!(expected, Type::Resource(_)) {
            if actual.is_dynamic()
                && let Some(value) = value
            {
                self.push_runtime_check_with_expected(
                    value.span(),
                    RuntimeCheckKind::ResourceType,
                    expected.clone(),
                );
            }
            if crate::can_assign(&actual, &expected) {
                return;
            }
        }
        if let Err(error) = self.context.unify(&expected, &actual) {
            if matches!(error, crate::unify::UnifyError::RigidTypeParameter { .. }) {
                self.type_error(
                    crate::diagnostics::RIGID_TYPE_PARAMETER_CODE,
                    "x04.type.rigid_type_parameter",
                    span,
                    error.to_string(),
                );
            } else {
                self.type_error_with_params(
                    crate::diagnostics::FUNCTION_RETURN_CODE,
                    "x04.type.return_mismatch",
                    span,
                    error.to_string(),
                    [
                        (
                            "expected".to_owned(),
                            DiagnosticParam::Text(expected.to_string()),
                        ),
                        (
                            "actual".to_owned(),
                            DiagnosticParam::Text(actual.to_string()),
                        ),
                    ],
                );
            }
        }
    }

    /// 检查 `break` 是否处于循环上下文。
    pub(super) fn check_break_statement(&mut self, span: SourceSpan) {
        if self.loop_depth == 0 {
            self.type_error(
                LOOP_CONTROL_CODE,
                "x04.type.break_outside_loop",
                span,
                "break 只能出现在循环体内".to_string(),
            );
        }
    }

    /// 检查 `continue` 是否处于循环上下文。
    pub(super) fn check_continue_statement(&mut self, span: SourceSpan) {
        if self.loop_depth == 0 {
            self.type_error(
                LOOP_CONTROL_CODE,
                "x04.type.continue_outside_loop",
                span,
                "continue 只能出现在循环体内".to_string(),
            );
        }
    }

    /// 检查条件必须是 bool；动态值留下运行时检查标记。
    fn check_condition(&mut self, condition: &Expression) {
        let ty = self.check_expression(condition);
        if ty.is_dynamic() {
            self.push_runtime_check(condition.span(), RuntimeCheckKind::BooleanCondition);
        } else if matches!(ty, Type::Variable(_)) {
            let boolean = Type::scalar(xiao_syntax::ScalarType::Bool);
            if let Err(error) = self.context.unify(&ty, &boolean) {
                if matches!(error, crate::unify::UnifyError::RigidTypeParameter { .. }) {
                    self.type_error(
                        crate::diagnostics::RIGID_TYPE_PARAMETER_CODE,
                        "x04.type.rigid_type_parameter",
                        condition.span(),
                        error.to_string(),
                    );
                } else {
                    self.type_error_with_params(
                        CONDITION_TYPE_CODE,
                        "x04.type.condition_requires_bool",
                        condition.span(),
                        format!("条件必须是 bool，实际为 {}", ty),
                        [(
                            "actual_type".to_owned(),
                            DiagnosticParam::Text(ty.to_string()),
                        )],
                    );
                }
            }
        } else if !ty.is_bool() {
            self.type_error_with_params(
                CONDITION_TYPE_CODE,
                "x04.type.condition_requires_bool",
                condition.span(),
                format!("条件必须是 bool，实际为 {}", ty),
                [(
                    "actual_type".to_owned(),
                    DiagnosticParam::Text(ty.to_string()),
                )],
            );
        }
    }

    /// 顺序检查语句，并把提前退出分流到对应出口。
    pub(super) fn check_statement_sequence(&mut self, body: &[Statement]) -> ConcurrencyFlow {
        let initial = self.concurrency_scope_state();
        let mut normal = Some(initial.clone());
        let mut result = ConcurrencyFlow::default();
        let mut unreachable_state: Option<ConcurrencyScopeState> = None;
        for statement in body {
            let Some(state) = normal.clone() else {
                let state = unreachable_state.clone().unwrap_or_else(|| initial.clone());
                self.restore_concurrency_scope_state(&state);
                self.set_checking_unreachable(true);
                self.check_statement(statement);
                self.set_checking_unreachable(false);
                let _ = self.take_last_concurrency_flow();
                self.restore_concurrency_scope_state(&state);
                continue;
            };
            self.restore_concurrency_scope_state(&state);
            self.check_statement(statement);
            let flow = self
                .take_last_concurrency_flow()
                .unwrap_or_else(|| ConcurrencyFlow {
                    normal: Some(self.concurrency_scope_state()),
                    ..ConcurrencyFlow::default()
                });
            result.returns.extend(flow.returns);
            result.breaks.extend(flow.breaks);
            result.continues.extend(flow.continues);
            result.errors.extend(flow.errors);
            normal = flow.normal;
            if normal.is_none() && unreachable_state.is_none() {
                unreachable_state = Self::merge_concurrency_scope_state_values(
                    &result
                        .returns
                        .iter()
                        .chain(result.breaks.iter())
                        .chain(result.continues.iter())
                        .chain(result.errors.iter())
                        .cloned()
                        .collect::<Vec<_>>(),
                );
            }
        }
        result.normal = normal;
        if let Some(state) = result.normal.as_ref() {
            self.restore_concurrency_scope_state(state);
        } else if let Some(state) = Self::merge_concurrency_scope_state_values(
            &result
                .returns
                .iter()
                .chain(result.breaks.iter())
                .chain(result.continues.iter())
                .chain(result.errors.iter())
                .cloned()
                .collect::<Vec<_>>(),
        ) {
            self.restore_concurrency_scope_state(&state);
        } else {
            self.restore_concurrency_scope_state(&initial);
        }
        result
    }

    /// 在独立作用域中检查一个缩进体，并返回外层绑定的各类出口。
    fn check_scoped_body_with_concurrency_flow(&mut self, body: &[Statement]) -> ConcurrencyFlow {
        self.push_scope();
        self.register_top_level_functions(body);
        let flow = self.check_statement_sequence(body);
        self.pop_scope_with_concurrency_flow(flow)
    }
}

/// 从已知容器类型中提取 `for` 循环元素类型。
fn iterable_element_type(ty: &Type) -> Option<Type> {
    match ty {
        Type::Scalar(xiao_syntax::ScalarType::Str) => {
            Some(Type::scalar(xiao_syntax::ScalarType::Str))
        }
        Type::Array(ArrayType::Homogeneous { element, .. }) => Some((**element).clone()),
        Type::Array(ArrayType::Heterogeneous { elements }) => {
            Some(common_type(elements).unwrap_or(Type::Dynamic))
        }
        Type::Array(ArrayType::Unknown) => Some(Type::Dynamic),
        Type::Tuple(elements) => Some(common_type(elements).unwrap_or(Type::Dynamic)),
        Type::Set(set) => Some(common_type(&set.to_member_types()).unwrap_or(Type::Dynamic)),
        Type::DictTable(dictionary) | Type::DictColumn(dictionary) => Some(
            common_type(
                &dictionary
                    .entries
                    .iter()
                    .map(|entry| (*entry.value).clone())
                    .collect::<Vec<_>>(),
            )
            .unwrap_or(Type::Dynamic),
        ),
        Type::Dynamic => Some(Type::Dynamic),
        _ => None,
    }
}

/// 仅当所有元素类型一致时返回可绑定的循环变量类型。
fn common_type(types: &[Type]) -> Option<Type> {
    let first = types.first()?.clone();
    types.iter().all(|ty| *ty == first).then_some(first)
}
