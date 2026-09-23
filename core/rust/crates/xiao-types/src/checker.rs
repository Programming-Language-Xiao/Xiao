//! 面向 P2/S0 的静态类型检查器。
//!
//! 检查器只消费 `xiao-syntax` 的公开 AST 和源码区间，输出类型化结果、结构化
//! 诊断及需要后端插入的运行时检查标记；它绝不执行 Xiao 程序或修改原始 AST。

use std::collections::BTreeMap;

use xiao_diagnostics::{Diagnostic, error_kind_of};
use xiao_source::{SourceFile, SourceSpan};
use xiao_syntax::{
    BinaryOperator, CallArgument, Expression, LiteralKind, Program, ScalarType, UnaryOperator,
};

use crate::containers::ContainerMaterializationPlan;
use crate::conversion::{
    ConversionKind, can_assign, classify_conversion, is_float, is_integer, is_numeric,
};
use crate::diagnostics::*;
use crate::environment::TypeEnvironment;
use crate::functions::FunctionSignature;
use crate::numeric::{
    ConstantValue, NumericError, binary_scalar_type, check_float_range,
    check_float_to_integer_range, check_integer_range, is_decimal_integer, parse_float_literal,
    parse_integer_literal,
};
use crate::types::Type;
use crate::unify::TypeContext;

/// 常量求值和跨职责共享的无状态辅助。
#[path = "checker/constant.rs"]
mod constant;
/// C0 容器语义的子模块；保持主检查器只负责语句分派和标量规则。
#[path = "container_checker.rs"]
pub(crate) mod container_checker;
/// 04 条件、循环、返回和入口静态检查。
#[path = "control_checker.rs"]
mod control_checker;
/// 类型诊断与运行时检查标记上报。
#[path = "checker/diagnostic.rs"]
mod diagnostic;
/// 04 函数定义、签名占位和调用参数检查。
#[path = "function_checker.rs"]
mod function_checker;
/// 类型检查结果和运行时检查标记的公开模型。
#[path = "checker/result.rs"]
mod result;
/// C1 有序容器选择、随机种子和选择器左值检查。
#[path = "selector_checker.rs"]
mod selector_checker;
/// C2-A 集合字面量、可哈希检查和成员判断。
#[path = "set_checker.rs"]
mod set_checker;
/// C2-C 集合代数、比较和动态检查计划。
#[path = "set_operations.rs"]
mod set_operations;
/// 语句分派、赋值和声明检查。
#[path = "checker/statement.rs"]
mod statement;
/// 05-C 表声明、成员访问和生命周期静态契约。
#[path = "table_checker.rs"]
mod table_checker;

use self::constant::{
    binary_requires_runtime_check, convert_constant, decode_string, eval_const_binary,
    eval_const_unary, is_random_seed_callee,
};

pub use self::result::{RuntimeCheck, RuntimeCheckKind, TypeCheckResult, TypedNode};
use self::set_operations::should_attempt_set_semantics;
use self::table_checker::TableFrame;

/// P2 静态检查器；生命周期只借用不可变源码。
pub struct TypeChecker<'source> {
    source: &'source SourceFile,
    context: TypeContext,
    environment: TypeEnvironment,
    diagnostics: Vec<Diagnostic>,
    nodes: Vec<TypedNode>,
    runtime_checks: Vec<RuntimeCheck>,
    materialization_plans: Vec<ContainerMaterializationPlan>,
    selection_plans: Vec<crate::selection_model::SelectionPlan>,
    broadcast_assignment_plans: Vec<crate::selection_model::BroadcastAssignmentPlan>,
    random_seed_plans: Vec<crate::selection_model::RandomSeedPlan>,
    constant_values: BTreeMap<String, ConstantValue>,
    function_signatures: BTreeMap<String, FunctionSignature>,
    table_signatures: BTreeMap<String, crate::tables::TableSignature>,
    current_table: Option<TableFrame>,
    current_function: Option<FunctionFrame>,
    loop_depth: usize,
}

/// 正在检查的函数上下文；只存静态返回约束，不持有运行时栈。
#[derive(Clone, Debug)]
struct FunctionFrame {
    /// 返回类型约束。
    return_type: Type,
    /// 是否已经遇到返回语句。
    saw_return: bool,
}

impl<'source> TypeChecker<'source> {
    /// 创建使用全局空作用域的检查器。
    #[must_use]
    pub fn new(source: &'source SourceFile) -> Self {
        Self {
            source,
            context: TypeContext::new(),
            environment: TypeEnvironment::new(),
            diagnostics: Vec::new(),
            nodes: Vec::new(),
            runtime_checks: Vec::new(),
            materialization_plans: Vec::new(),
            selection_plans: Vec::new(),
            broadcast_assignment_plans: Vec::new(),
            random_seed_plans: Vec::new(),
            constant_values: BTreeMap::new(),
            function_signatures: BTreeMap::new(),
            table_signatures: BTreeMap::new(),
            current_table: None,
            current_function: None,
            loop_depth: 0,
        }
    }

    /// 直接检查一个程序；这是不需要保留检查器状态时的便捷入口。
    #[must_use]
    pub fn check(source: &'source SourceFile, program: &Program) -> TypeCheckResult {
        Self::new(source).check_program(program)
    }

    /// 使用当前环境检查整个程序并返回结果。
    #[must_use]
    pub fn check_program(mut self, program: &Program) -> TypeCheckResult {
        self.register_top_level_functions(&program.statements);
        self.register_top_level_tables(&program.statements);
        for statement in &program.statements {
            self.check_statement(statement);
        }
        self.finalize_function_inference();
        // 表达式节点是在约束收集期间记录的；函数调用或函数体返回值可能在
        // 后续语句才把类型变量绑定到具体标量。对外发布结果前统一应用最终
        // 替换，确保 IR 消费者看到的节点类型与函数签名使用同一份结论。
        for node in &mut self.nodes {
            node.ty = self.context.apply(&node.ty);
        }
        TypeCheckResult {
            nodes: self.nodes,
            diagnostics: self.diagnostics,
            runtime_checks: self.runtime_checks,
            environment: self.environment,
            materialization_plans: self.materialization_plans,
            selection_plans: self.selection_plans,
            broadcast_assignment_plans: self.broadcast_assignment_plans,
            random_seed_plans: self.random_seed_plans,
            entry_mode: program.entry_mode,
            function_signatures: self.function_signatures,
            table_signatures: self.table_signatures,
        }
    }

    /// 返回检查器当前使用的源码。
    #[must_use]
    pub const fn source(&self) -> &'source SourceFile {
        self.source
    }

    /// 递归检查表达式并记录其旁路类型。
    fn check_expression(&mut self, expression: &Expression) -> Type {
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
                        match eval_const_unary(operator, value) {
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
        if should_attempt_set_semantics(operator, &left_type, &right_type) {
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
            && binary_requires_runtime_check(operator, *left, *right, *result)
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
            let boolean = Type::scalar(ScalarType::Bool);
            let valid_left = !left_variable && left == boolean;
            let valid_right = !right_variable && right == boolean;
            if (valid_left || left_variable) && (valid_right || right_variable) {
                if left_variable {
                    self.context.unify(&left, &boolean).ok()?;
                }
                if right_variable {
                    self.context.unify(&right, &boolean).ok()?;
                }
                return Some(boolean);
            }
            return None;
        }

        if left_variable && right_variable {
            if is_comparison {
                self.context.unify(&left, &right).ok()?;
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
                self.context.unify(&left, &right).ok()?;
                return Some(self.context.apply(&left));
            }
            return None;
        }

        let (variable, known, variable_on_left) = if left_variable {
            (&left, &right, true)
        } else {
            (&right, &left, false)
        };
        let Type::Scalar(known_scalar) = known else {
            return is_comparison.then_some(Type::scalar(ScalarType::Bool));
        };
        let candidate = match operator {
            BinaryOperator::Add => {
                if *known_scalar == ScalarType::Str
                    || is_numeric(*known_scalar)
                    || (!variable_on_left && *known_scalar == ScalarType::Bool)
                {
                    if *known_scalar == ScalarType::Bool && variable_on_left {
                        None
                    } else if *known_scalar == ScalarType::Bool {
                        Some(ScalarType::Int)
                    } else {
                        Some(*known_scalar)
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
                if is_numeric(*known_scalar) {
                    Some(*known_scalar)
                } else if !variable_on_left
                    && *known_scalar == ScalarType::Bool
                    && matches!(operator, BinaryOperator::Subtract)
                {
                    Some(ScalarType::Int)
                } else {
                    None
                }
            }
            _ if is_comparison => Some(*known_scalar),
            _ => None,
        }?;

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

    /// 检查赋值兼容性；含 HM 类型变量时先统一，再使用固定转换矩阵。
    fn types_compatible_for_assignment(&mut self, source: &Type, target: &Type) -> bool {
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
    fn check_constant_binary_result(
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
        let Some(result) = eval_const_binary(operator, left, right) else {
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
        if self
            .simple_callee_name(callee)
            .as_deref()
            .is_some_and(|name| name == "FatalError")
        {
            for argument in arguments {
                self.check_expression(&argument.value);
            }
            self.type_error(
                CATCH_FATAL_CODE,
                "x07.type.fatal_constructor",
                span,
                "FatalError 不能构造为可恢复错误对象".to_string(),
            );
            return Type::Dynamic;
        }
        if self
            .simple_callee_name(callee)
            .as_deref()
            .is_some_and(|name| error_kind_of(name).is_some())
        {
            for argument in arguments {
                self.check_expression(&argument.value);
            }
            return Type::Dynamic;
        }
        if is_random_seed_callee(callee, self.source) {
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
                                ConversionKind::RuntimeChecked => {
                                    RuntimeCheckKind::DynamicConversion
                                }
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
        } else if self.simple_callee_name(callee).as_deref() == Some("input") {
            for argument in arguments {
                self.check_expression(&argument.value);
            }
            Type::scalar(ScalarType::Str)
        } else if self.simple_callee_name(callee).as_deref() == Some("print") {
            for argument in arguments {
                self.check_expression(&argument.value);
            }
            Type::None
        } else {
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

    /// 识别未被反引号包裹的标量转换构造器。
    fn scalar_callee(&self, callee: &Expression) -> Option<ScalarType> {
        let name = self.simple_callee_name(callee)?;
        ScalarType::from_name(&name)
    }

    /// 检查已知常量是否满足转换的内容约束；动态值留给运行时检查。
    fn static_conversion_value_is_valid(
        &mut self,
        expression: &Expression,
        target: ScalarType,
    ) -> bool {
        let Some(constant) = self.eval_const(expression) else {
            return true;
        };
        if target == ScalarType::Bool {
            if let ConstantValue::String(value) = &constant {
                if !matches!(value.as_str(), "true" | "True" | "false" | "False") {
                    self.type_error(
                        INVALID_CONVERSION_CODE,
                        "x02.type.invalid_string_boolean",
                        expression.span(),
                        "只有 true/True/false/False 可以转换为 bool".to_string(),
                    );
                    return false;
                }
            }
        }
        true
    }

    /// 检查编译期已知数值是否超出显式转换的目标范围。
    fn static_target_range_is_valid(
        &mut self,
        expression: &Expression,
        target: ScalarType,
    ) -> bool {
        let Some(constant) = self.eval_const(expression) else {
            return true;
        };
        let numeric_source = matches!(
            &constant,
            ConstantValue::Integer(_) | ConstantValue::BigInteger(_) | ConstantValue::Float(_)
        );
        if numeric_source && (is_integer(target) || is_float(target)) {
            if let Err(error) = self.check_explicit_constant_target(&constant, target) {
                self.numeric_error(expression.span(), BinaryOperator::Add, error);
                return false;
            }
        }
        true
    }

    /// 验证显式数值转换的常量边界；与隐式初始化不同，浮点到整数
    /// 在这里允许向零截断，只要截断后的结果仍在目标范围内。
    fn check_explicit_constant_target(
        &self,
        value: &ConstantValue,
        target: ScalarType,
    ) -> Result<(), NumericError> {
        match value {
            ConstantValue::Float(value) if is_integer(target) => {
                check_float_to_integer_range(*value, target)
            }
            _ => self.check_constant_target(value, target),
        }
    }

    /// 读取普通名称调用者文本；反引号名称不视为内建函数。
    fn simple_callee_name(&self, callee: &Expression) -> Option<String> {
        let Expression::Name(name) = callee else {
            return None;
        };
        if name.backticked {
            return None;
        }
        Some(name.unquoted_text(self.source).to_owned())
    }

    /// 返回任意名称调用者的规范化环境键；内建函数仍由
    /// [`Self::simple_callee_name`] 单独限制为普通 ASCII 名称。
    fn function_callee_key(&self, callee: &Expression) -> Option<String> {
        let Expression::Name(name) = callee else {
            return None;
        };
        Some(self.name_key(*name))
    }

    /// 纯递归求值一个已知编译期表达式，动态输入返回 `None`。
    fn eval_const(&self, expression: &Expression) -> Option<ConstantValue> {
        match expression {
            Expression::Literal { kind, span } => match kind {
                LiteralKind::Integer => {
                    let text = self.source.slice(*span);
                    parse_integer_literal(text)
                        .map(ConstantValue::Integer)
                        .ok()
                        .or_else(|| {
                            is_decimal_integer(text)
                                .then(|| ConstantValue::BigInteger(text.to_owned()))
                        })
                }
                LiteralKind::Float => parse_float_literal(self.source.slice(*span))
                    .ok()
                    .map(ConstantValue::Float),
                LiteralKind::String => {
                    decode_string(self.source.slice(*span)).map(ConstantValue::String)
                }
                LiteralKind::Boolean => {
                    Some(ConstantValue::Boolean(self.source.slice(*span) == "true"))
                }
                LiteralKind::None => Some(ConstantValue::None),
            },
            Expression::Name(name) => self.constant_values.get(&self.name_key(*name)).cloned(),
            Expression::Group { expression, .. } => self.eval_const(expression),
            Expression::Unary {
                operator, operand, ..
            } => {
                let value = self.eval_const(operand)?;
                match eval_const_unary(*operator, value) {
                    Some(Ok(value)) => Some(value),
                    Some(Err(_)) | None => None,
                }
            }
            Expression::Binary {
                operator,
                left,
                right,
                ..
            } => {
                let left = self.eval_const(left)?;
                let right = self.eval_const(right)?;
                match eval_const_binary(*operator, left, right) {
                    Some(Ok(value)) => Some(value),
                    Some(Err(_)) | None => None,
                }
            }
            Expression::Cast {
                expression, target, ..
            } => {
                let value = self.eval_const(expression)?;
                convert_constant(value, *target)
            }
            Expression::Call {
                callee, arguments, ..
            } => {
                let target = self.scalar_callee(callee)?;
                let value = self.eval_const(&arguments.first()?.value)?;
                convert_constant(value, target)
            }
            _ => None,
        }
    }

    /// 验证常量值能否落入指定标量类型的表示范围。
    fn check_constant_target(
        &self,
        value: &ConstantValue,
        target: ScalarType,
    ) -> Result<(), NumericError> {
        match value {
            ConstantValue::Integer(value) => {
                if is_integer(target) {
                    check_integer_range(*value, target)
                } else if is_float(target) {
                    check_float_range(*value as f64, target)
                } else {
                    Ok(())
                }
            }
            ConstantValue::BigInteger(_) => {
                if target == ScalarType::Lint {
                    Ok(())
                } else {
                    Err(NumericError::Overflow(target))
                }
            }
            ConstantValue::Float(value) => {
                if is_float(target) {
                    check_float_range(*value, target)
                } else if is_integer(target) {
                    Err(NumericError::InvalidLiteral(
                        "浮点值不能隐式转换为整数".to_string(),
                    ))
                } else {
                    Ok(())
                }
            }
            ConstantValue::Boolean(_) if target == ScalarType::Bool => Ok(()),
            ConstantValue::String(_) if target == ScalarType::Str => Ok(()),
            _ => Ok(()),
        }
    }
}

/// 检查一个已解析程序的便捷函数。
#[must_use]
pub fn check(source: &SourceFile, program: &Program) -> TypeCheckResult {
    TypeChecker::check(source, program)
}
