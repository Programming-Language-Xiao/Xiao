//! 语句分派、赋值和声明检查。
//!
//! 语句模块只负责静态环境更新和语句级约束，复杂表达式规则由 `expression.rs` 提供。

use xiao_diagnostics::DiagnosticParam;
use xiao_syntax::{
    AssignmentOperator, BinaryOperator, DeclaredType, Expression, FunctionTypeAnnotation,
    ImportStatement, IndexPath, ScalarType, Statement, TypeTerm,
};

use crate::conversion::{
    ConversionKind, can_assign, classify_conversion, is_float, is_integer, is_numeric,
};
use crate::diagnostics::*;
use crate::numeric::binary_scalar_type;
use crate::types::Type;

use super::{RuntimeCheckKind, TypeChecker};

/// 复合赋值检查在推导、诊断和环境写回阶段之间传递的中间状态。
struct ExtendedAssignment<'source> {
    target: &'source Expression,
    name: xiao_syntax::Name,
    key: &'source str,
    value: &'source Expression,
    left_type: &'source Type,
    right_type: &'source Type,
    result_type: Type,
    binary: Option<BinaryOperator>,
    operation_valid: bool,
    dynamic_operation: bool,
    set_dynamic_boundary: bool,
}

impl<'source> TypeChecker<'source> {
    /// 检查一条顶层语句并更新名称环境。
    pub(super) fn check_statement(&mut self, statement: &Statement) {
        match statement {
            Statement::Expression { expression, .. } => {
                self.check_expression(expression);
            }
            Statement::Assignment { target, value, .. } => {
                self.check_simple_assignment(*target, value);
            }
            Statement::ExtendedAssignment {
                target,
                operator,
                value,
                ..
            } => self.check_extended_assignment(target, *operator, value),
            Statement::Declaration {
                target,
                declared_type,
                constraint_path,
                value,
                ..
            } => {
                let key = self.name_key(*target);
                let already_declared = self.environment.contains_current(&key);
                let source = value
                    .as_ref()
                    .and_then(|value| self.concurrency_handle_identity(value));
                self.check_declaration(
                    *target,
                    declared_type,
                    constraint_path.as_ref(),
                    value.as_ref(),
                );
                if !already_declared && self.environment.contains_current(&key) {
                    self.bind_concurrency_handle(
                        key,
                        &declared_type_to_type(declared_type),
                        source,
                    );
                }
            }
            Statement::ConstDeclaration {
                target,
                declared_type,
                value,
                ..
            } => {
                let key = self.name_key(*target);
                let already_declared = self.environment.contains_current(&key);
                let source = self.concurrency_handle_identity(value);
                self.check_const_declaration(*target, declared_type.as_ref(), value);
                if !already_declared && self.environment.contains_current(&key) {
                    self.bind_concurrency_handle(key, &Type::Dynamic, source);
                }
            }
            Statement::Import { import, .. } => self.check_import(import),
            Statement::Function {
                name,
                type_parameters,
                parameters,
                return_type,
                body,
                span,
                ..
            } => self.check_function_statement(
                *name,
                parameters,
                return_type.clone(),
                body,
                *span,
                type_parameters,
            ),
            Statement::Table {
                name,
                kind,
                body,
                span,
                ..
            } => self.check_table_statement(*name, *kind, body, *span),
            Statement::If {
                condition,
                body,
                elif_branches,
                else_body,
                ..
            } => self.check_if_statement(condition, body, elif_branches, else_body.as_deref()),
            Statement::For {
                target,
                iterable,
                body,
                ..
            } => self.check_for_statement(*target, iterable, body),
            Statement::While {
                condition, body, ..
            } => self.check_while_statement(condition, body),
            Statement::Return { value, span, .. } => {
                self.check_return_statement(value.as_ref(), *span)
            }
            Statement::Break { span, .. } => self.check_break_statement(*span),
            Statement::Continue { span, .. } => self.check_continue_statement(*span),
            Statement::Try {
                body,
                catches,
                finally_body,
                span,
                ..
            } => self.check_try_statement(body, catches, finally_body.as_deref(), *span),
            Statement::Raise { value, span, .. } => self.check_raise_statement(value, *span),
        }
    }

    /// 检查普通名称赋值，首次出现时建立单态绑定。
    fn check_simple_assignment(&mut self, target: xiao_syntax::Name, value: &Expression) {
        let imported_function = self.imported_function_signature(value);
        let imported_module = self.imported_namespace_path(value);
        let value_type = self.check_expression(value);
        self.register_array_alias(target, value, &value_type);
        let key = self.name_key(target);
        let handle_source = self.concurrency_handle_identity(value);
        let had_binding = self.environment.lookup(&key).is_some();
        self.reset_concurrency_binding(&key);
        if let Some(binding) = self.environment.lookup(&key).cloned() {
            let existing_type = self.context.instantiate(binding.scheme());
            if value_type.is_dynamic() {
                if matches!(existing_type, Type::Resource(_)) {
                    self.push_runtime_check_with_expected(
                        value.span(),
                        RuntimeCheckKind::ResourceType,
                        existing_type.clone(),
                    );
                } else {
                    self.push_runtime_check(value.span(), RuntimeCheckKind::DynamicConversion);
                }
            }
            if let (Type::Set(source), Type::Set(target)) = (&value_type, &existing_type)
                && source.allows_dynamic()
            {
                // 集合静态成员已经在类型层验证；动态尾标只能由 Runtime
                // 完成成员类型/哈希检查，不能静默当作完全兼容。
                self.push_runtime_check_with_expected(
                    value.span(),
                    RuntimeCheckKind::SetMembership,
                    Type::Set(target.clone()),
                );
            }
            if !self.types_compatible_for_assignment_at(&value_type, &existing_type, target.span) {
                self.type_error_with_params(
                    ASSIGNMENT_TYPE_MISMATCH_CODE,
                    "x02.type.assignment_mismatch",
                    target.span,
                    format!("不能把 {} 赋给已锁定的 {}", value_type, existing_type),
                    [
                        (
                            "actual".to_owned(),
                            DiagnosticParam::Text(value_type.to_string()),
                        ),
                        (
                            "expected".to_owned(),
                            DiagnosticParam::Text(existing_type.to_string()),
                        ),
                    ],
                );
            } else {
                if value_type.is_container() && !binding.container_constraints.is_empty() {
                    self.check_container_assignment_constraints(
                        &value_type,
                        &binding.container_constraints,
                        value.span(),
                    );
                }
                if let Err(error) = self.environment.assign(&key) {
                    self.environment_error(target.span, error);
                } else if let Some(binding) = self.environment.lookup_mut(&key) {
                    binding.set_imported_interfaces(imported_function, imported_module);
                }
                self.bind_concurrency_handle(
                    key.clone(),
                    &value_type,
                    had_binding.then(|| handle_source.clone()).flatten(),
                );
            }
            return;
        }
        let mut binding = crate::environment::Binding::mutable(
            crate::types::TypeScheme::monomorphic(value_type.clone()),
            true,
        );
        binding.set_imported_interfaces(imported_function, imported_module);
        if let Err(error) = self.environment.declare(key, binding) {
            self.environment_error(target.span, error);
        } else {
            self.bind_concurrency_handle(self.name_key(target), &value_type, handle_source);
        }
    }

    /// 检查复合赋值并验证结果仍能写回左值类型槽。
    fn check_extended_assignment(
        &mut self,
        target: &Expression,
        operator: AssignmentOperator,
        value: &Expression,
    ) {
        if matches!(target, Expression::Member { .. }) {
            self.check_table_member_assignment(target, operator, value);
            return;
        }
        if matches!(target, Expression::Selector { .. }) {
            self.check_selector_assignment(target, operator, value);
            return;
        }
        let right_type = self.check_expression(value);
        let Expression::Name(name) = target else {
            self.check_expression(target);
            self.type_error(
                INVALID_OPERANDS_CODE,
                "x02.type.non_name_assignment_target",
                target.span(),
                "P2 只支持标量名称的复合赋值".to_string(),
            );
            return;
        };
        self.check_extended_name_assignment(target, *name, operator, value, right_type);
    }

    /// 检查名称目标的复合赋值并准备最终环境写回。
    fn check_extended_name_assignment(
        &mut self,
        target: &Expression,
        name: xiao_syntax::Name,
        operator: AssignmentOperator,
        value: &Expression,
        right_type: Type,
    ) {
        let key = self.name_key(name);
        let Some(binding) = self.environment.lookup(&key).cloned() else {
            self.undefined_name(name);
            return;
        };
        let initialized = binding.initialized;
        if !initialized {
            self.type_error_with_params(
                UNINITIALIZED_READ_CODE,
                "x02.type.uninitialized_read",
                name.span,
                format!("名称 {} 在复合赋值前不能读取", self.display_name(name)),
                [(
                    "name".to_owned(),
                    DiagnosticParam::Text(self.display_name(name)),
                )],
            );
        }
        let left_type = self.context.instantiate(binding.scheme());
        let binary = super::constant::assignment_binary_operator(operator);
        let (result_type, operation_valid, dynamic_operation, set_dynamic_boundary) =
            self.extended_assignment_result(target, &left_type, &right_type, binary, initialized);
        self.finish_extended_assignment(ExtendedAssignment {
            target,
            name,
            key: &key,
            value,
            left_type: &left_type,
            right_type: &right_type,
            result_type,
            binary,
            operation_valid,
            dynamic_operation,
            set_dynamic_boundary,
        });
    }

    /// 推导复合赋值的结果类型并记录运行时边界。
    fn extended_assignment_result(
        &mut self,
        target: &Expression,
        left_type: &Type,
        right_type: &Type,
        binary: Option<BinaryOperator>,
        mut operation_valid: bool,
    ) -> (Type, bool, bool, bool) {
        let mut dynamic_operation = false;
        let mut set_dynamic_boundary = false;
        let result_type = if let Some(binary) = binary {
            if super::set_operations::should_attempt_set_semantics(binary, left_type, right_type) {
                let analysis =
                    self.check_set_operation_types(binary, left_type, right_type, target.span());
                operation_valid &= analysis.valid;
                dynamic_operation = analysis.result.is_dynamic();
                set_dynamic_boundary = analysis.has_dynamic_boundary;
                analysis.result
            } else {
                match (left_type, right_type) {
                    (Type::Dynamic, _) | (_, Type::Dynamic) => {
                        dynamic_operation = true;
                        self.push_runtime_check(target.span(), RuntimeCheckKind::Arithmetic);
                        Type::Dynamic
                    }
                    (Type::Scalar(left), Type::Scalar(right)) => {
                        match binary_scalar_type(binary, *left, *right) {
                            Ok(scalar) => Type::scalar(scalar),
                            Err(error) => {
                                operation_valid = false;
                                self.numeric_error(target.span(), binary, error);
                                Type::Dynamic
                            }
                        }
                    }
                    _ => {
                        operation_valid = false;
                        self.type_error_with_params(
                            INVALID_OPERANDS_CODE,
                            "x02.type.invalid_compound_operands",
                            target.span(),
                            format!("复合赋值不能作用于 {} 和 {}", left_type, right_type),
                            [
                                (
                                    "left".to_owned(),
                                    DiagnosticParam::Text(left_type.to_string()),
                                ),
                                (
                                    "right".to_owned(),
                                    DiagnosticParam::Text(right_type.to_string()),
                                ),
                            ],
                        );
                        Type::Dynamic
                    }
                }
            }
        } else {
            operation_valid = false;
            self.type_error_with_params(
                INVALID_OPERANDS_CODE,
                "x02.type.invalid_compound_operands",
                target.span(),
                format!("复合赋值不能作用于 {} 和 {}", left_type, right_type),
                [
                    (
                        "left".to_owned(),
                        DiagnosticParam::Text(left_type.to_string()),
                    ),
                    (
                        "right".to_owned(),
                        DiagnosticParam::Text(right_type.to_string()),
                    ),
                ],
            );
            Type::Dynamic
        };
        (
            result_type,
            operation_valid,
            dynamic_operation,
            set_dynamic_boundary,
        )
    }

    /// 完成复合赋值的常量折叠、兼容性检查和绑定更新。
    fn finish_extended_assignment(&mut self, assignment: ExtendedAssignment<'_>) {
        let ExtendedAssignment {
            target,
            name,
            key,
            value,
            left_type,
            right_type,
            result_type,
            binary,
            operation_valid,
            dynamic_operation,
            set_dynamic_boundary,
        } = assignment;
        let constant_known = if operation_valid && !dynamic_operation {
            binary.is_some_and(|binary| {
                self.check_constant_binary_result(
                    binary,
                    target.span(),
                    target,
                    value,
                    &result_type,
                )
            })
        } else {
            false
        };
        if operation_valid
            && !dynamic_operation
            && !constant_known
            && let (Type::Scalar(left), Type::Scalar(right), Type::Scalar(result)) =
                (left_type, right_type, &result_type)
            && let Some(binary) = binary
            && super::constant::binary_requires_runtime_check(binary, *left, *right, *result)
        {
            self.push_runtime_check(
                target.span(),
                if binary == BinaryOperator::Divide
                    || binary == BinaryOperator::FloorDivide
                    || binary == BinaryOperator::Remainder
                {
                    RuntimeCheckKind::Arithmetic
                } else {
                    RuntimeCheckKind::NumericRange
                },
            );
        }
        if operation_valid && set_dynamic_boundary && left_type.is_set() {
            // 集合运算的动态尾标必须在写回锁定左值时再次验证成员类型；
            // `SetOperation` 只描述运算形状本身。
            self.push_runtime_check_with_expected(
                target.span(),
                RuntimeCheckKind::SetMembership,
                left_type.clone(),
            );
        }
        if operation_valid
            && !dynamic_operation
            && !self.types_compatible_for_assignment_at(&result_type, left_type, target.span())
        {
            self.type_error_with_params(
                ASSIGNMENT_TYPE_MISMATCH_CODE,
                "x02.type.compound_result_mismatch",
                target.span(),
                format!("复合赋值结果 {} 不符合 {}", result_type, left_type),
                [
                    (
                        "result".to_owned(),
                        DiagnosticParam::Text(result_type.to_string()),
                    ),
                    (
                        "expected".to_owned(),
                        DiagnosticParam::Text(left_type.to_string()),
                    ),
                ],
            );
        } else if operation_valid {
            if let Err(error) = self.environment.assign(key) {
                self.environment_error(name.span, error);
            }
        }
    }
    /// 检查带显式标量类型的声明和可选初值。
    fn check_declaration(
        &mut self,
        target: xiao_syntax::Name,
        declared_type: &DeclaredType,
        constraint_path: Option<&IndexPath>,
        value: Option<&Expression>,
    ) {
        match declared_type {
            DeclaredType::Set(annotation) => {
                self.check_set_declaration(target, annotation, constraint_path, value);
            }
            DeclaredType::Scalar(scalar) => {
                if !self.try_check_container_declaration(target, *scalar, constraint_path, value) {
                    self.check_scalar_declaration(target, *scalar, value);
                }
            }
            DeclaredType::Resource(resource) => {
                self.reject_resource_constraint_path(constraint_path);
                self.check_resource_declaration(target, *resource, value);
            }
            DeclaredType::Thread(_)
            | DeclaredType::Task(_)
            | DeclaredType::Runtime
            | DeclaredType::Pool
            | DeclaredType::Shared(_)
            | DeclaredType::SharedMut(_)
            | DeclaredType::LockGuard(_) => {
                self.check_concurrency_declaration(target, declared_type, constraint_path, value);
            }
        }
    }

    /// 检查并发专用注解声明；这里只登记静态句柄类型，不执行 Runtime 行为。
    fn check_concurrency_declaration(
        &mut self,
        target: xiao_syntax::Name,
        declared_type: &DeclaredType,
        constraint_path: Option<&IndexPath>,
        value: Option<&Expression>,
    ) {
        if let Some(path) = constraint_path {
            self.type_error(
                INVALID_DECLARATION_PATH_CODE,
                "x03.type.invalid_path",
                path.span(),
                "并发句柄类型不能携带容器路径约束".to_owned(),
            );
        }
        let expected = declared_type_to_type(declared_type);
        if let Some(value) = value {
            let actual = self.check_expression(value);
            if !can_assign(&actual, &expected) {
                self.type_error(
                    ASSIGNMENT_TYPE_MISMATCH_CODE,
                    "x02.type.assignment_mismatch",
                    value.span(),
                    format!("{} 不能赋给 {}", actual, expected),
                );
            }
        }
        let key = self.name_key(target);
        if self.environment.contains_current(&key) {
            self.type_error(
                DUPLICATE_DECLARATION_CODE,
                "x02.type.duplicate_declaration",
                target.span,
                format!("名称 {} 已经声明", self.display_name(target)),
            );
            return;
        }
        self.reset_concurrency_binding(&key);
        if let Err(error) = self
            .environment
            .declare_mutable(key, expected, value.is_some())
        {
            self.environment_error(target.span, error);
        }
    }

    /// 资源注解不提供容器路径约束；变量和表字段使用同一拒绝规则。
    pub(super) fn reject_resource_constraint_path(&mut self, path: Option<&IndexPath>) {
        if let Some(path) = path {
            self.type_error(
                INVALID_DECLARATION_PATH_CODE,
                "x03.type.invalid_path",
                path.span(),
                "资源类型注解不能携带容器路径约束".to_owned(),
            );
        }
    }

    /// 检查资源类型声明和可选初值。
    fn check_resource_declaration(
        &mut self,
        target: xiao_syntax::Name,
        declared_type: xiao_syntax::ResourceKind,
        value: Option<&Expression>,
    ) {
        let key = self.name_key(target);
        let value_type = value.map(|expression| self.check_expression(expression));
        if let Some(value_type) = value_type.as_ref() {
            let expected = Type::Resource(declared_type);
            if value_type.is_dynamic()
                && let Some(value) = value
            {
                self.push_runtime_check_with_expected(
                    value.span(),
                    RuntimeCheckKind::ResourceType,
                    expected.clone(),
                );
            }
            if !crate::can_assign(value_type, &expected) {
                self.type_error(
                    ASSIGNMENT_TYPE_MISMATCH_CODE,
                    "x02.type.assignment_mismatch",
                    target.span,
                    format!("{} 不能赋给 {}", value_type, expected),
                );
            }
        }
        if self.environment.contains_current(&key) {
            self.type_error(
                DUPLICATE_DECLARATION_CODE,
                "x02.type.duplicate_declaration",
                target.span,
                format!("名称 {} 已经声明", target.unquoted_text(self.source)),
            );
            return;
        }
        self.reset_concurrency_binding(&key);
        if let Err(error) =
            self.environment
                .declare_mutable(key, Type::Resource(declared_type), value.is_some())
        {
            self.environment_error(target.span, error);
        }
    }

    /// 检查带显式标量类型的声明和可选初值。
    fn check_scalar_declaration(
        &mut self,
        target: xiao_syntax::Name,
        declared_type: ScalarType,
        value: Option<&Expression>,
    ) {
        let key = self.name_key(target);
        let value_type = value.map(|expression| self.check_expression(expression));
        if let Some(value) = value {
            if let Some(value_type) = &value_type {
                self.check_explicit_target(value, value_type, declared_type, true);
            }
        }
        if self.environment.contains_current(&key) {
            self.type_error(
                DUPLICATE_DECLARATION_CODE,
                "x02.type.duplicate_declaration",
                target.span,
                format!("名称 {} 在当前作用域中已经声明", self.display_name(target)),
            );
            return;
        }
        self.reset_concurrency_binding(&key);
        if let Err(error) =
            self.environment
                .declare_mutable(key, Type::scalar(declared_type), value.is_some())
        {
            self.environment_error(target.span, error);
        }
    }

    /// 检查编译期常量声明、求值和泛化方案。
    fn check_const_declaration(
        &mut self,
        target: xiao_syntax::Name,
        declared_type: Option<&DeclaredType>,
        value: &Expression,
    ) {
        let key = self.name_key(target);
        let diagnostics_before_value = self.diagnostics.len();
        let value_type = self.check_expression(value);
        let forbidden_declared_type = match declared_type {
            Some(DeclaredType::Scalar(declared_type)) => {
                // `const` 不能把运行时检查当成合法初始化；即使源值是
                // Dynamic，也只保留后面的 NON_CONSTANT 诊断。
                self.check_explicit_target(value, &value_type, *declared_type, false);
                false
            }
            Some(DeclaredType::Resource(_))
            | Some(DeclaredType::Set(_))
            | Some(DeclaredType::Thread(_))
            | Some(DeclaredType::Task(_))
            | Some(DeclaredType::Runtime)
            | Some(DeclaredType::Pool)
            | Some(DeclaredType::Shared(_))
            | Some(DeclaredType::SharedMut(_))
            | Some(DeclaredType::LockGuard(_)) => {
                self.type_error(
                    NON_CONSTANT_CODE,
                    "x02.type.non_constant_initializer",
                    value.span(),
                    "资源或集合类型不能用于 const 初始化".to_string(),
                );
                true
            }
            None => false,
        };
        let constant = self.eval_const(value);
        if let Some(constant) = constant {
            let stored = if let Some(DeclaredType::Scalar(target)) = declared_type {
                super::constant::convert_constant(constant.clone(), *target)
            } else if declared_type.is_some() {
                None
            } else {
                Some(constant)
            };
            if let Some(stored) = stored {
                self.constant_values.insert(key.clone(), stored);
            }
        } else if !forbidden_declared_type
            && !self.diagnostics[diagnostics_before_value..]
                .iter()
                .any(|diagnostic| {
                    diagnostic.code() == ARITHMETIC_ERROR_CODE
                        || diagnostic.code() == INVALID_CONVERSION_CODE
                        || diagnostic.code() == NON_CONSTANT_CODE
                })
        {
            self.type_error(
                NON_CONSTANT_CODE,
                "x02.type.non_constant_initializer",
                value.span(),
                "const 的初始化表达式必须能在编译期求值".to_string(),
            );
        }
        if self.environment.contains_current(&key) {
            self.type_error(
                DUPLICATE_DECLARATION_CODE,
                "x02.type.duplicate_declaration",
                target.span,
                format!("名称 {} 在当前作用域中已经声明", self.display_name(target)),
            );
            return;
        }
        self.reset_concurrency_binding(&key);
        // 显式类型必须成为常量绑定的锁定类型，不能因为初始化表达式
        // 的推断类型更宽而丢失声明者的约束（例如 `const sint n = 1`）。
        let binding_type = declared_type
            .map(|declared| match declared {
                DeclaredType::Scalar(scalar) => Type::scalar(*scalar),
                DeclaredType::Resource(resource) => Type::Resource(*resource),
                DeclaredType::Set(_) => Type::Dynamic,
                DeclaredType::Thread(_)
                | DeclaredType::Task(_)
                | DeclaredType::Runtime
                | DeclaredType::Pool
                | DeclaredType::Shared(_)
                | DeclaredType::SharedMut(_)
                | DeclaredType::LockGuard(_) => declared_type_to_type(declared),
            })
            .unwrap_or_else(|| value_type.clone());
        let scheme = self.context.generalize(&self.environment, &binding_type);
        if let Err(error) = self.environment.declare_constant(key, scheme) {
            self.environment_error(target.span, error);
        }
    }

    /// 检查表达式写入显式类型槽时的兼容性和范围。
    pub(super) fn check_explicit_target(
        &mut self,
        expression: &Expression,
        source_type: &Type,
        target: ScalarType,
        allow_runtime_checks: bool,
    ) {
        if source_type.is_dynamic() {
            if allow_runtime_checks {
                self.push_runtime_check(expression.span(), RuntimeCheckKind::DynamicConversion);
            }
            return;
        }
        let target_type = Type::scalar(target);
        let constant = self.eval_const(expression);

        // 恒等和安全加宽可以直接写入目标槽；若目标是较窄的浮点，仍
        // 需要检查已知常量是否会变成无穷大。
        if can_assign(source_type, &target_type) {
            if let (Type::Scalar(source), Some(value)) = (source_type, constant.as_ref())
                && is_numeric(*source)
                && (is_integer(target) || is_float(target))
                && let Err(error) = self.check_constant_target(value, target)
            {
                self.numeric_error(expression.span(), BinaryOperator::Add, error);
            }
            return;
        }

        let conversion = match classify_conversion(source_type, target) {
            Ok(conversion) => conversion,
            Err(error) => {
                self.type_error(
                    INVALID_CONVERSION_CODE,
                    "x02.type.invalid_conversion",
                    expression.span(),
                    error.to_string(),
                );
                return;
            }
        };

        // 常量数值可以安全落入较窄的整数/浮点槽（但不能借此
        // 偷渡隐式的浮点到整数截断）。这条例外只适用于声明初值，
        // 普通变量表达式仍必须显式写出转换。
        if matches!(conversion.kind, ConversionKind::NumericNarrowing)
            && let Some(value) = constant.as_ref()
        {
            match self.check_constant_target(value, target) {
                Ok(()) => return,
                Err(error) => {
                    self.numeric_error(expression.span(), BinaryOperator::Add, error);
                    return;
                }
            }
        }

        // `str -> bool` 的内容检查只有在已经写出显式转换时才会发生；
        // 直接声明仍是非法隐式转换。已知非法拼写应报告转换错误，
        // 而不是同时制造一个无关的运行时检查。
        if conversion.kind == ConversionKind::StrToBool
            && constant.is_some()
            && !self.static_conversion_value_is_valid(expression, target)
        {
            return;
        }

        self.type_error_with_params(
            ASSIGNMENT_TYPE_MISMATCH_CODE,
            "x02.type.implicit_conversion",
            expression.span(),
            format!("不能隐式把 {} 转换为 {}", source_type, target.as_str()),
            [
                (
                    "source".to_owned(),
                    DiagnosticParam::Text(source_type.to_string()),
                ),
                (
                    "target".to_owned(),
                    DiagnosticParam::Text(target.as_str().to_owned()),
                ),
            ],
        );
    }
}

/// 将声明位置的并发专用注解降低为类型层类型。
fn declared_type_to_type(declared: &DeclaredType) -> Type {
    match declared {
        DeclaredType::Scalar(scalar) => Type::scalar(*scalar),
        DeclaredType::Resource(resource) => Type::Resource(*resource),
        DeclaredType::Set(annotation) => Type::Set(crate::SetType::heterogeneous(
            annotation
                .members
                .iter()
                .map(|term| match term {
                    TypeTerm::Scalar(scalar) => Type::scalar(*scalar),
                    TypeTerm::None => Type::None,
                    TypeTerm::Resource(resource) => Type::Resource(*resource),
                    TypeTerm::TypeParameter(_) => Type::Dynamic,
                })
                .collect::<Vec<_>>(),
        )),
        DeclaredType::Thread(element) => Type::thread(annotation_to_type(element)),
        DeclaredType::Task(element) => Type::task(annotation_to_type(element)),
        DeclaredType::Runtime => Type::Runtime,
        DeclaredType::Pool => Type::Pool,
        DeclaredType::Shared(element) => Type::shared(annotation_to_type(element)),
        DeclaredType::SharedMut(element) => Type::shared_mut(annotation_to_type(element)),
        DeclaredType::LockGuard(element) => Type::lock_guard(annotation_to_type(element)),
    }
}

/// 将函数注解的并发类型递归降低为类型层类型。
fn annotation_to_type(annotation: &FunctionTypeAnnotation) -> Type {
    match annotation {
        FunctionTypeAnnotation::Scalar(scalar) => Type::scalar(*scalar),
        FunctionTypeAnnotation::None => Type::None,
        FunctionTypeAnnotation::Resource(resource) => Type::Resource(*resource),
        FunctionTypeAnnotation::Set(_) => Type::set(Type::Dynamic),
        FunctionTypeAnnotation::Array(element) => Type::array(annotation_to_type(element)),
        FunctionTypeAnnotation::Dict(annotation) => {
            Type::DictTable(crate::DictType::open(annotation_to_type(&annotation.value)))
        }
        FunctionTypeAnnotation::TypeParameter(_) => Type::Dynamic,
        FunctionTypeAnnotation::Thread(element) => Type::thread(annotation_to_type(element)),
        FunctionTypeAnnotation::Task(element) => Type::task(annotation_to_type(element)),
        FunctionTypeAnnotation::Runtime => Type::Runtime,
        FunctionTypeAnnotation::Pool => Type::Pool,
        FunctionTypeAnnotation::Shared(element) => Type::shared(annotation_to_type(element)),
        FunctionTypeAnnotation::SharedMut(element) => Type::shared_mut(annotation_to_type(element)),
        FunctionTypeAnnotation::LockGuard(element) => Type::lock_guard(annotation_to_type(element)),
    }
}

impl<'source> TypeChecker<'source> {
    /// 检查导入语句的模块、选择项与局部绑定冲突。
    fn check_import(&mut self, import: &ImportStatement) {
        match import {
            ImportStatement::Modules { imports, .. } => {
                for item in imports {
                    let name = item.alias.unwrap_or(item.path.segments[0]);
                    let mut binding = if item.alias.is_none() {
                        crate::environment::Binding::imported_root()
                    } else {
                        crate::environment::Binding::constant(
                            crate::types::TypeScheme::monomorphic(Type::Dynamic),
                        )
                    };
                    let path = item
                        .path
                        .segments
                        .iter()
                        .map(|name| name.unquoted_text(self.source))
                        .collect::<Vec<_>>()
                        .join(".");
                    if self.module_functions.contains_key(&path)
                        || self
                            .module_functions
                            .keys()
                            .any(|module| module.starts_with(&format!("{path}.")))
                    {
                        let path = if item.alias.is_some() {
                            path
                        } else {
                            item.path.segments[0].unquoted_text(self.source).to_owned()
                        };
                        binding = binding.with_imported_module(path);
                    }
                    if let Err(error) = self.environment.declare_import(
                        self.name_key(name),
                        binding,
                        item.alias.is_none(),
                    ) {
                        self.environment_error(name.span, error);
                    }
                }
            }
            ImportStatement::From {
                module, imports, ..
            } => {
                let module = module
                    .segments
                    .iter()
                    .map(|name| name.unquoted_text(self.source))
                    .collect::<Vec<_>>()
                    .join(".");
                for item in imports {
                    let name = item.alias.unwrap_or(item.name);
                    let binding = self.typed_import_binding(
                        &module,
                        &self.name_key(item.name),
                        crate::environment::Binding::mutable(
                            crate::types::TypeScheme::monomorphic(Type::Dynamic),
                            true,
                        ),
                    );
                    if let Err(error) =
                        self.environment
                            .declare_import(self.name_key(name), binding, false)
                    {
                        self.environment_error(name.span, error);
                    }
                }
            }
        }
    }
}
