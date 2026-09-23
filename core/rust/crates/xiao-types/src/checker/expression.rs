//! 表达式类型检查、运算推导和调用检查。
//!
//! 表达式模块负责递归类型化并保留原有诊断/运行时检查顺序；
//! 容器、函数和表的专门规则仍由各自子模块提供。

use xiao_diagnostics::error_kind_of;
use xiao_source::SourceSpan;
use xiao_syntax::{
    BinaryOperator, CallArgument, Expression, LiteralKind, ScalarType, UnaryOperator,
};

use crate::conversion::{ConversionKind, can_assign, classify_conversion, is_numeric};
use crate::diagnostics::*;
use crate::numeric::{
    binary_scalar_type, is_decimal_integer, parse_float_literal, parse_integer_literal,
};
use crate::types::Type;

use super::{RuntimeCheckKind, TypeChecker, TypedNode};

impl<'source> TypeChecker<'source> {
    /// 递归检查表达式并记录其旁路类型。
    pub(super) fn check_expression(&mut self, expression: &Expression) -> Type {
        let ty = match expression {
            Expression::Literal { kind, span } => self.literal_type(*kind, *span),
            Expression::Name(name) => self.check_name(*name),
            Expression::ArrayLiteral { .. }
            | Expression::TupleLiteral { .. }
            | Expression::DictTableLiteral { .. }
            | Expression::DictColumnLiteral { .. }
            | Expression::SetLiteral { .. } => self.check_container_expression(expression),
            Expression::Group { expression, .. } => self.check_expression(expression),
            Expression::Unary {
                operator,
                operand,
                span,
            } => self.check_unary(*operator, operand, *span),
            Expression::Binary {
                operator,
                left,
                right,
                span,
            } => self.check_binary(*operator, left, right, *span),
            Expression::Call {
                callee,
                arguments,
                span,
            } => self.check_call(callee, arguments, *span),
            Expression::NewCall {
                callee,
                arguments,
                span,
            } => self.check_new_call(callee, arguments, *span),
            Expression::Member {
                object,
                member,
                span,
            } => self.check_table_member_expression(object, *member, *span),
            Expression::Cast {
                expression,
                target,
                span,
            } => self.check_cast(expression, *target, *span),
            Expression::Selector {
                source,
                step,
                selector,
                span,
            } => self.check_container_selector(source, step.as_deref(), selector, *span),
        };
        self.nodes.push(TypedNode {
            span: expression.span(),
            ty: ty.clone(),
        });
        ty
    }

    /// 将字面量类别和文本映射为静态标量类型。
    fn literal_type(&mut self, kind: LiteralKind, span: SourceSpan) -> Type {
        match kind {
            LiteralKind::Integer => {
                let text = self.source.slice(span);
                match parse_integer_literal(text) {
                    Ok(value) if (i64::MIN as i128..=i64::MAX as i128).contains(&value) => {
                        Type::scalar(ScalarType::Int)
                    }
                    Ok(_) => Type::scalar(ScalarType::Lint),
                    Err(_error) if is_decimal_integer(text) => Type::scalar(ScalarType::Lint),
                    Err(error) => {
                        self.numeric_error(span, BinaryOperator::Add, error);
                        Type::Dynamic
                    }
                }
            }
            LiteralKind::Float => match parse_float_literal(self.source.slice(span)) {
                Ok(_) => Type::scalar(ScalarType::Float),
                Err(error) => {
                    self.numeric_error(span, BinaryOperator::Add, error);
                    Type::Dynamic
                }
            },
            LiteralKind::String => Type::scalar(ScalarType::Str),
            LiteralKind::Boolean => Type::scalar(ScalarType::Bool),
            LiteralKind::None => Type::None,
        }
    }

    /// 查找名称、检查初始化状态并实例化其类型方案。
    fn check_name(&mut self, name: xiao_syntax::Name) -> Type {
        let key = self.name_key(name);
        let Some(binding) = self.environment.lookup(&key).cloned() else {
            self.undefined_name(name);
            return Type::Dynamic;
        };
        if !binding.initialized {
            self.type_error(
                UNINITIALIZED_READ_CODE,
                "x02.type.uninitialized_read",
                name.span,
                format!("名称 {} 在赋值前不能读取", self.display_name(name)),
            );
        }
        let instantiated = self.context.instantiate(binding.scheme());
        self.context.apply(&instantiated)
    }

    /// 检查一元运算的操作数和结果类型。
    fn check_unary(
        &mut self,
        operator: UnaryOperator,
        operand: &Expression,
        span: SourceSpan,
    ) -> Type {
        let operand_type = self.check_expression(operand);
        if operand_type.is_dynamic() {
            self.push_runtime_check(span, RuntimeCheckKind::Arithmetic);
            return if operator == UnaryOperator::Not {
                Type::scalar(ScalarType::Bool)
            } else {
                Type::Dynamic
            };
        }
        if let Type::Variable(variable) = &operand_type {
            if operator == UnaryOperator::Not {
                let boolean = Type::scalar(ScalarType::Bool);
                if self
                    .context
                    .unify(&Type::Variable(*variable), &boolean)
                    .is_ok()
                {
                    return boolean;
                }
            } else {
                // 一元正负只能约束“数值族”；具体宽度由返回值、赋值或
                // 调用点继续统一。暂不把未知参数错误地降成 dynamic。
                return Type::Variable(*variable);
            }
        }
        match operator {
            UnaryOperator::Not if operand_type.is_bool() => Type::scalar(ScalarType::Bool),
            UnaryOperator::Plus | UnaryOperator::Minus if operand_type.is_numeric() => {
                let result = operand_type.clone();
                if let Type::Scalar(scalar) = result {
                    if let Some(value) = self.eval_const(operand) {
                        match super::constant::eval_const_unary(operator, value) {
                            Some(Err(error)) => {
                                self.numeric_error(span, BinaryOperator::Add, error)
                            }
                            Some(Ok(value)) => {
                                if let Err(error) = self.check_constant_target(&value, scalar) {
                                    self.numeric_error(span, BinaryOperator::Add, error);
                                }
                            }
                            None => {}
                        }
                    } else if matches!(
                        scalar,
                        ScalarType::Sint | ScalarType::Int | ScalarType::Sfloat | ScalarType::Float
                    ) {
                        self.push_runtime_check(span, RuntimeCheckKind::NumericRange);
                    }
                }
                result
            }
            _ => {
                self.type_error(
                    INVALID_OPERANDS_CODE,
                    "x02.type.invalid_unary_operand",
                    span,
                    format!("一元运算 {} 不能作用于 {}", operator.as_str(), operand_type),
                );
                Type::Dynamic
            }
        }
    }

    /// 检查二元运算、数值提升和编译期算术错误。
    fn check_binary(
        &mut self,
        operator: BinaryOperator,
        left: &Expression,
        right: &Expression,
        span: SourceSpan,
    ) -> Type {
        if matches!(operator, BinaryOperator::In | BinaryOperator::NotIn) {
            return self.check_set_membership(operator, left, right, span);
        }
        let left_type = self.check_expression(left);
        let right_type = self.check_expression(right);
        if super::set_operations::should_attempt_set_semantics(operator, &left_type, &right_type) {
            return self
                .check_set_operation_types(operator, &left_type, &right_type, span)
                .result;
        }
        if let Some(result) = self.infer_binary_with_variables(operator, &left_type, &right_type) {
            return result;
        }
        let mut operation_valid = false;
        let result = match (&left_type, &right_type) {
            (Type::Dynamic, _) | (_, Type::Dynamic) => {
                self.push_runtime_check(span, RuntimeCheckKind::Arithmetic);
                Type::Dynamic
            }
            (Type::Scalar(left), Type::Scalar(right)) => {
                match binary_scalar_type(operator, *left, *right) {
                    Ok(result) => {
                        operation_valid = true;
                        Type::scalar(result)
                    }
                    Err(error) => {
                        self.numeric_error(span, operator, error);
                        Type::Dynamic
                    }
                }
            }
            _ => {
                self.type_error(
                    INVALID_OPERANDS_CODE,
                    "x02.type.invalid_binary_operands",
                    span,
                    format!(
                        "运算 {} 不能作用于 {} 和 {}",
                        operator.as_str(),
                        left_type,
                        right_type
                    ),
                );
                Type::Dynamic
            }
        };
        let constant_known = if operation_valid {
            self.check_constant_binary_result(operator, span, left, right, &result)
        } else {
            false
        };
        if operation_valid
            && !constant_known
            && let (Type::Scalar(left), Type::Scalar(right), Type::Scalar(result)) =
                (&left_type, &right_type, &result)
            && super::constant::binary_requires_runtime_check(operator, *left, *right, *result)
        {
            self.push_runtime_check(
                span,
                if operator == BinaryOperator::Divide
                    || operator == BinaryOperator::FloorDivide
                    || operator == BinaryOperator::Remainder
                {
                    RuntimeCheckKind::Arithmetic
                } else {
                    RuntimeCheckKind::NumericRange
                },
            );
        }
        result
    }

    /// 在函数签名仍含类型变量时建立最小的二元运算约束。
    ///
    /// 函数体通常先于调用点检查；如果这里把未知参数立即当成非法
    /// 操作数，合法的 `def f(x) -> int` 也无法由返回值或后续调用完成
    /// 推断。该辅助只处理可局部确定的标量约束，其余情况仍交给统一器
    /// 或最终的“无法推断”诊断，不把未知值静默改成动态类型。
    fn infer_binary_with_variables(
        &mut self,
        operator: BinaryOperator,
        left: &Type,
        right: &Type,
    ) -> Option<Type> {
        let left = self.context.apply(left);
        let right = self.context.apply(right);
        let left_variable = matches!(left, Type::Variable(_));
        let right_variable = matches!(right, Type::Variable(_));
        if !left_variable && !right_variable {
            return None;
        }

        let is_comparison = matches!(
            operator,
            BinaryOperator::Equal
                | BinaryOperator::NotEqual
                | BinaryOperator::Less
                | BinaryOperator::LessEqual
                | BinaryOperator::Greater
                | BinaryOperator::GreaterEqual
                | BinaryOperator::Is
                | BinaryOperator::IsNot
        );
        if matches!(operator, BinaryOperator::And | BinaryOperator::Or) {
            return self.infer_variable_boolean(&left, &right, left_variable, right_variable);
        }
        if left_variable && right_variable {
            return self.infer_binary_with_two_variables(operator, &left, &right, is_comparison);
        }

        let (variable, known, variable_on_left) = if left_variable {
            (&left, &right, true)
        } else {
            (&right, &left, false)
        };
        let Type::Scalar(known_scalar) = known else {
            return is_comparison.then_some(Type::scalar(ScalarType::Bool));
        };
        let candidate = Self::binary_variable_candidate(
            operator,
            *known_scalar,
            variable_on_left,
            is_comparison,
        )?;
        let candidate_type = Type::scalar(candidate);
        self.context.unify(variable, &candidate_type).ok()?;
        let resolved_left = self.context.apply(&left);
        let resolved_right = self.context.apply(&right);
        if is_comparison {
            return Some(Type::scalar(ScalarType::Bool));
        }
        match (&resolved_left, &resolved_right) {
            (Type::Scalar(left), Type::Scalar(right)) => {
                binary_scalar_type(operator, *left, *right)
                    .ok()
                    .map(Type::scalar)
            }
            _ => Some(self.context.apply(&candidate_type)),
        }
    }

    /// 将参与布尔逻辑运算的类型变量统一为 `bool`。
    fn infer_variable_boolean(
        &mut self,
        left: &Type,
        right: &Type,
        left_variable: bool,
        right_variable: bool,
    ) -> Option<Type> {
        let boolean = Type::scalar(ScalarType::Bool);
        let valid_left = !left_variable && left == &boolean;
        let valid_right = !right_variable && right == &boolean;
        if (valid_left || left_variable) && (valid_right || right_variable) {
            if left_variable {
                self.context.unify(left, &boolean).ok()?;
            }
            if right_variable {
                self.context.unify(right, &boolean).ok()?;
            }
            Some(boolean)
        } else {
            None
        }
    }

    /// 推导两个类型变量参与二元运算时的共同结果类型。
    fn infer_binary_with_two_variables(
        &mut self,
        operator: BinaryOperator,
        left: &Type,
        right: &Type,
        is_comparison: bool,
    ) -> Option<Type> {
        if is_comparison {
            self.context.unify(left, right).ok()?;
            return Some(Type::scalar(ScalarType::Bool));
        }
        if matches!(
            operator,
            BinaryOperator::Add
                | BinaryOperator::Subtract
                | BinaryOperator::Multiply
                | BinaryOperator::Divide
                | BinaryOperator::FloorDivide
                | BinaryOperator::Remainder
                | BinaryOperator::Power
        ) {
            self.context.unify(left, right).ok()?;
            Some(self.context.apply(left))
        } else {
            None
        }
    }

    /// 根据已知标量和运算方向推导另一侧类型变量的候选标量。
    fn binary_variable_candidate(
        operator: BinaryOperator,
        known_scalar: ScalarType,
        variable_on_left: bool,
        is_comparison: bool,
    ) -> Option<ScalarType> {
        match operator {
            BinaryOperator::Add => {
                if known_scalar == ScalarType::Str
                    || is_numeric(known_scalar)
                    || (!variable_on_left && known_scalar == ScalarType::Bool)
                {
                    if known_scalar == ScalarType::Bool && variable_on_left {
                        None
                    } else if known_scalar == ScalarType::Bool {
                        Some(ScalarType::Int)
                    } else {
                        Some(known_scalar)
                    }
                } else {
                    None
                }
            }
            BinaryOperator::Subtract
            | BinaryOperator::Multiply
            | BinaryOperator::Divide
            | BinaryOperator::FloorDivide
            | BinaryOperator::Remainder
            | BinaryOperator::Power => {
                if is_numeric(known_scalar) {
                    Some(known_scalar)
                } else if !variable_on_left
                    && known_scalar == ScalarType::Bool
                    && matches!(operator, BinaryOperator::Subtract)
                {
                    Some(ScalarType::Int)
                } else {
                    None
                }
            }
            _ if is_comparison => Some(known_scalar),
            _ => None,
        }
    }
    /// 检查赋值兼容性；含 HM 类型变量时先统一，再使用固定转换矩阵。
    pub(super) fn types_compatible_for_assignment(&mut self, source: &Type, target: &Type) -> bool {
        let source = self.context.apply(source);
        let target = self.context.apply(target);
        if !source.free_vars().is_empty() || !target.free_vars().is_empty() {
            self.context.unify(&source, &target).is_ok()
        } else {
            can_assign(&source, &target)
        }
    }

    /// 检查常量二元运算的求值错误及结果宽度；动态表达式留给后端
    /// 的运行时检查，不在这里假装已经完成求值。
    pub(super) fn check_constant_binary_result(
        &mut self,
        operator: BinaryOperator,
        span: SourceSpan,
        left: &Expression,
        right: &Expression,
        result_type: &Type,
    ) -> bool {
        let (Some(left), Some(right)) = (self.eval_const(left), self.eval_const(right)) else {
            return false;
        };
        let Some(result) = super::constant::eval_const_binary(operator, left, right) else {
            return false;
        };
        match result {
            Err(error) => {
                self.numeric_error(span, operator, error);
                true
            }
            Ok(value) => {
                if let Type::Scalar(target) = result_type
                    && let Err(error) = self.check_constant_target(&value, *target)
                {
                    self.numeric_error(span, operator, error);
                }
                true
            }
        }
    }

    /// 检查标量转换、预留内建函数和 HM 函数调用。
    fn check_call(
        &mut self,
        callee: &Expression,
        arguments: &[CallArgument],
        span: SourceSpan,
    ) -> Type {
        if let Some(result) = self.check_error_constructor_call(callee, arguments, span) {
            return result;
        }
        if super::constant::is_random_seed_callee(callee, self.source) {
            return self.check_random_seed_call(arguments, span);
        }
        if let Some(result) = self.check_table_method_call(callee, arguments, span) {
            return result;
        }
        if self.is_set_constructor(callee) {
            return self.check_set_constructor(arguments, span);
        }
        if self
            .function_callee_key(callee)
            .is_some_and(|key| self.function_signatures.contains_key(&key))
        {
            self.check_expression(callee);
            return self
                .check_known_function_call(callee, arguments, span)
                .unwrap_or(Type::Dynamic);
        }
        if let Some(target) = self.scalar_callee(callee) {
            return self.check_scalar_call(target, arguments, span);
        }
        if self.simple_callee_name(callee).as_deref() == Some("input") {
            for argument in arguments {
                self.check_expression(&argument.value);
            }
            return Type::scalar(ScalarType::Str);
        }
        if self.simple_callee_name(callee).as_deref() == Some("print") {
            for argument in arguments {
                self.check_expression(&argument.value);
            }
            return Type::None;
        }
        self.check_function_value_call(callee, arguments, span)
    }

    /// 检查内建错误对象构造调用，并处理不可捕获的 `FatalError`。
    fn check_error_constructor_call(
        &mut self,
        callee: &Expression,
        arguments: &[CallArgument],
        span: SourceSpan,
    ) -> Option<Type> {
        let name = self.simple_callee_name(callee)?;
        if name == "FatalError" {
            for argument in arguments {
                self.check_expression(&argument.value);
            }
            self.type_error(
                CATCH_FATAL_CODE,
                "x07.type.fatal_constructor",
                span,
                "FatalError 不能构造为可恢复错误对象".to_string(),
            );
            return Some(Type::Dynamic);
        }
        if error_kind_of(&name).is_some() {
            for argument in arguments {
                self.check_expression(&argument.value);
            }
            return Some(Type::Dynamic);
        }
        None
    }

    /// 检查标量转换构造器调用并记录必要的运行时转换检查。
    fn check_scalar_call(
        &mut self,
        target: ScalarType,
        arguments: &[CallArgument],
        span: SourceSpan,
    ) -> Type {
        if arguments.len() != 1 {
            self.type_error(
                INVALID_OPERANDS_CODE,
                "x02.type.conversion_arity",
                span,
                "标量转换函数必须接收一个参数".to_string(),
            );
            return Type::Dynamic;
        }
        let source_diagnostics = self.diagnostics.len();
        let source_type = self.check_expression(&arguments[0].value);
        if !self.static_conversion_value_is_valid(&arguments[0].value, target) {
            return Type::Dynamic;
        }
        if !self.static_target_range_is_valid(&arguments[0].value, target) {
            return Type::Dynamic;
        }
        let constant_known = self.eval_const(&arguments[0].value).is_some();
        match classify_conversion(&source_type, target) {
            Ok(conversion) => {
                if conversion.requires_runtime_check
                    && !constant_known
                    && !self.has_errors_since(source_diagnostics)
                {
                    self.push_runtime_check(
                        arguments[0].value.span(),
                        match conversion.kind {
                            ConversionKind::StrToBool => RuntimeCheckKind::StringBoolean,
                            ConversionKind::RuntimeChecked => RuntimeCheckKind::DynamicConversion,
                            _ => RuntimeCheckKind::NumericRange,
                        },
                    );
                }
                Type::scalar(target)
            }
            Err(error) => {
                self.type_error(
                    INVALID_CONVERSION_CODE,
                    "x02.type.invalid_conversion",
                    arguments[0].value.span(),
                    error.to_string(),
                );
                Type::Dynamic
            }
        }
    }

    /// 检查函数值调用的参数数量、参数类型和返回类型。
    fn check_function_value_call(
        &mut self,
        callee: &Expression,
        arguments: &[CallArgument],
        span: SourceSpan,
    ) -> Type {
        let callee_type = self.check_expression(callee);
        let argument_types = arguments
            .iter()
            .map(|argument| self.check_expression(&argument.value))
            .collect::<Vec<_>>();
        if let Type::Function {
            parameters,
            return_type,
        } = callee_type
        {
            if parameters.len() != argument_types.len() {
                self.type_error(
                    INVALID_OPERANDS_CODE,
                    "x02.type.call_arity",
                    span,
                    format!(
                        "函数需要 {} 个参数，实际得到 {}",
                        parameters.len(),
                        argument_types.len()
                    ),
                );
            } else {
                for (expected, actual) in parameters.iter().zip(argument_types.iter()) {
                    if let Err(error) = self.context.unify(expected, actual) {
                        self.unification_error(span, error);
                    }
                }
            }
            *return_type
        } else {
            Type::Dynamic
        }
    }
    /// 检查构造调用；P2 暂只报告尚未开放的表构造语义。
    fn check_new_call(
        &mut self,
        callee: &Expression,
        arguments: &[CallArgument],
        span: SourceSpan,
    ) -> Type {
        self.check_table_new_call(callee, arguments, span)
    }

    /// 检查 `as` 显式转换并记录必要的运行时检查。
    fn check_cast(
        &mut self,
        expression: &Expression,
        target: ScalarType,
        span: SourceSpan,
    ) -> Type {
        let source_diagnostics = self.diagnostics.len();
        let source_type = self.check_expression(expression);
        if !self.static_conversion_value_is_valid(expression, target) {
            return Type::Dynamic;
        }
        if !self.static_target_range_is_valid(expression, target) {
            return Type::Dynamic;
        }
        let constant_known = self.eval_const(expression).is_some();
        match classify_conversion(&source_type, target) {
            Ok(conversion) => {
                if conversion.requires_runtime_check
                    && !constant_known
                    && !self.has_errors_since(source_diagnostics)
                {
                    self.push_runtime_check(
                        span,
                        match conversion.kind {
                            ConversionKind::StrToBool => RuntimeCheckKind::StringBoolean,
                            ConversionKind::RuntimeChecked => RuntimeCheckKind::DynamicConversion,
                            _ => RuntimeCheckKind::NumericRange,
                        },
                    );
                }
                Type::scalar(target)
            }
            Err(error) => {
                self.type_error(
                    INVALID_CONVERSION_CODE,
                    "x02.type.invalid_conversion",
                    span,
                    error.to_string(),
                );
                Type::Dynamic
            }
        }
    }
}
