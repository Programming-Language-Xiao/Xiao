//! C2-A/C2-B 集合静态检查。
//!
//! 该模块只消费语法 AST 并生成 [`crate::Type`] 与结构化诊断。它不创建
//! Runtime 集合、不执行哈希，也不实现集合代数；动态元素只登记后续
//! Runtime 所需的检查标记。C2-B 允许静态成员类型并集和动态尾标，
//! 但仍不创建 Runtime 集合对象或执行集合操作。

use xiao_diagnostics::DiagnosticParam;
use xiao_source::SourceSpan;
use xiao_syntax::{BinaryOperator, CallArgument, Expression, Name, SetTypeAnnotation, TypeTerm};

use crate::diagnostics::{
    CONTAINER_TYPE_MISMATCH_CODE, INVALID_DECLARATION_PATH_CODE, SET_CONSTRUCTOR_ARITY_CODE,
    SET_DUPLICATE_ELEMENT_CODE, SET_ELEMENT_TYPE_MISMATCH_CODE, SET_INDEX_UNSUPPORTED_CODE,
    SET_MEMBERSHIP_TYPE_CODE, SET_UNHASHABLE_ELEMENT_CODE,
};
use crate::numeric::ConstantValue;
use crate::set_types::{Hashability, SetType, hashability};
use crate::types::Type;

use super::{RuntimeCheckKind, TypeChecker};

impl<'source> TypeChecker<'source> {
    /// 检查集合字面量并推导 C2-B 的静态成员并集。
    pub(super) fn check_set_literal(&mut self, elements: &[Expression], _span: SourceSpan) -> Type {
        let mut members = Vec::<Type>::new();
        let mut allows_dynamic = false;
        let mut has_invalid_element = false;
        let mut constants = Vec::<(Type, ConstantValue)>::new();

        for element in elements {
            let ty = self.check_expression(element);
            let element_hashability = hashability(&ty);
            match element_hashability {
                Hashability::Hashable => {}
                Hashability::Unhashable => {
                    has_invalid_element = true;
                    self.type_error_with_params(
                        SET_UNHASHABLE_ELEMENT_CODE,
                        "x03.type.set_unhashable_element",
                        element.span(),
                        format!("类型 {} 不能作为集合元素", ty),
                        [(
                            "actual_type".to_owned(),
                            DiagnosticParam::Text(ty.to_string()),
                        )],
                    );
                }
                Hashability::Dynamic => {
                    self.push_runtime_check(element.span(), RuntimeCheckKind::SetHashability);
                }
            }

            // 不可哈希类型已经有专门诊断，不再把它与首个合法元素
            // 比较并制造一个无关的“元素类型不一致”错误。
            if element_hashability == Hashability::Unhashable {
                continue;
            }

            match &ty {
                Type::Dynamic | Type::Variable(_) => allows_dynamic = true,
                known => members.push(known.clone()),
            }

            // 只有编译期已知的常量才参与静态唯一性检查；动态值的哈希和
            // 相等判断必须留给 Runtime，不能依据源码表达式文本猜测。
            if let Some(constant) = self.eval_const(element) {
                if constants
                    .iter()
                    .any(|(seen_type, seen)| seen_type == &ty && same_set_constant(seen, &constant))
                {
                    let value = format_constant(&constant);
                    self.type_error_with_params(
                        SET_DUPLICATE_ELEMENT_CODE,
                        "x03.type.set_duplicate_element",
                        element.span(),
                        format!("集合中重复的静态元素 {}", value),
                        [("element".to_owned(), DiagnosticParam::Text(value))],
                    );
                } else {
                    constants.push((ty, constant));
                }
            }
        }

        let set = if has_invalid_element {
            SetType::Unknown
        } else {
            SetType::heterogeneous_with_dynamic(members, allows_dynamic)
        };
        Type::Set(set)
    }

    /// 检查 `set<T>`/`set<T | U>` 显式声明，并将完整并集写入环境。
    pub(super) fn check_set_declaration(
        &mut self,
        target: Name,
        annotation: &SetTypeAnnotation,
        constraint_path: Option<&xiao_syntax::IndexPath>,
        value: Option<&Expression>,
    ) {
        let key = self.name_key(target);
        let expected_set = declared_set_type(annotation);
        let expected = Type::Set(expected_set.clone());

        if constraint_path.is_some() {
            self.type_error(
                INVALID_DECLARATION_PATH_CODE,
                "x03.type.set_type_path_not_supported",
                target.span,
                "集合类型注解只能用于变量根声明".to_owned(),
            );
        }

        if let Some(value) = value {
            let actual = self.check_expression(value);
            self.check_explicit_set_target(value, &actual, &expected_set);
        }

        if self.environment.contains_current(&key) {
            self.type_error(
                crate::diagnostics::DUPLICATE_DECLARATION_CODE,
                "x02.type.duplicate_declaration",
                target.span,
                format!("名称 {} 在当前作用域中已经声明", self.display_name(target)),
            );
            return;
        }
        if let Err(error) = self
            .environment
            .declare_mutable(key, expected, value.is_some())
        {
            self.environment_error(target.span, error);
        }
    }

    /// 验证显式集合声明的初始化器；静态成员采用严格类型隔离。
    fn check_explicit_set_target(
        &mut self,
        expression: &Expression,
        actual: &Type,
        expected: &SetType,
    ) {
        match actual {
            Type::Set(source) if source.is_unknown() => {}
            Type::Set(source) => {
                for member in source.member_types() {
                    if !expected.contains_type(member) && !expected.allows_dynamic() {
                        self.type_error_with_params(
                            SET_ELEMENT_TYPE_MISMATCH_CODE,
                            "x03.type.set_element_type_mismatch",
                            expression.span(),
                            format!("集合元素类型 {} 不符合显式类型 {}", member, expected),
                            type_params(member, &Type::Set(expected.clone())),
                        );
                    }
                }
                if source.allows_dynamic() {
                    self.push_runtime_check_with_expected(
                        expression.span(),
                        RuntimeCheckKind::SetMembership,
                        Type::Set(expected.clone()),
                    );
                }
            }
            Type::Dynamic => {
                self.push_runtime_check(expression.span(), RuntimeCheckKind::DynamicConversion);
            }
            other => {
                self.type_error_with_params(
                    CONTAINER_TYPE_MISMATCH_CODE,
                    "x03.type.set_initializer_requires_set",
                    expression.span(),
                    format!("集合声明需要集合初始化器，实际为 {}", other),
                    type_params(other, &Type::Set(expected.clone())),
                );
            }
        }
    }

    /// 判断表达式是否是未被反引号包裹的 `set` 构造器名称。
    pub(super) fn is_set_constructor(&self, expression: &Expression) -> bool {
        match expression {
            Expression::Name(_) => self.simple_callee_name(expression).as_deref() == Some("set"),
            Expression::Call { callee, .. } => {
                self.simple_callee_name(callee).as_deref() == Some("set")
            }
            _ => false,
        }
    }

    /// 检查 `set()` 空集合构造式；C2-A 不接受构造参数。
    pub(super) fn check_set_constructor(
        &mut self,
        arguments: &[CallArgument],
        span: SourceSpan,
    ) -> Type {
        if !arguments.is_empty() {
            for argument in arguments {
                self.check_expression(&argument.value);
            }
            self.type_error_with_params(
                SET_CONSTRUCTOR_ARITY_CODE,
                "x03.type.set_constructor_arity",
                span,
                format!("set() 构造式不接受参数，实际收到 {} 个", arguments.len()),
                [
                    (
                        "actual_count".to_owned(),
                        DiagnosticParam::Integer(arguments.len() as i128),
                    ),
                    ("expected_count".to_owned(), DiagnosticParam::Integer(0)),
                ],
            );
        }
        Type::Set(SetType::Unknown)
    }

    /// 检查 `in`/`not in` 集合成员判断并返回 `bool`。
    pub(super) fn check_set_membership(
        &mut self,
        operator: BinaryOperator,
        left: &Expression,
        right: &Expression,
        _span: SourceSpan,
    ) -> Type {
        let left_type = self.check_expression(left);
        let right_type = self.check_expression(right);
        match &right_type {
            Type::Set(set) => {
                self.check_membership_element(&left_type, set, left.span());
                Type::scalar(xiao_syntax::ScalarType::Bool)
            }
            Type::Dynamic | Type::Variable(_) => {
                self.check_membership_element(&left_type, &SetType::Unknown, left.span());
                Type::scalar(xiao_syntax::ScalarType::Bool)
            }
            other => {
                self.type_error_with_params(
                    SET_MEMBERSHIP_TYPE_CODE,
                    "x03.type.set_membership_requires_set",
                    right.span(),
                    format!(
                        "运算 {} 的右侧必须是集合，实际为 {}",
                        operator.as_str(),
                        other
                    ),
                    [
                        (
                            "operator".to_owned(),
                            DiagnosticParam::Text(operator.as_str().to_owned()),
                        ),
                        (
                            "actual_type".to_owned(),
                            DiagnosticParam::Text(other.to_string()),
                        ),
                    ],
                );
                Type::Dynamic
            }
        }
    }

    /// 验证成员值的可哈希性和已知集合元素类型约束。
    fn check_membership_element(&mut self, actual: &Type, expected: &SetType, span: SourceSpan) {
        let mut needs_runtime_check = false;
        match hashability(actual) {
            Hashability::Unhashable => self.type_error_with_params(
                SET_UNHASHABLE_ELEMENT_CODE,
                "x03.type.set_unhashable_membership",
                span,
                format!("类型 {} 不能作为集合成员查询值", actual),
                [(
                    "actual_type".to_owned(),
                    DiagnosticParam::Text(actual.to_string()),
                )],
            ),
            Hashability::Dynamic => {
                needs_runtime_check = true;
            }
            Hashability::Hashable => {}
        }
        if expected.is_unknown() || expected.allows_dynamic() || actual.is_dynamic() {
            if actual.is_dynamic() || expected.is_unknown() {
                needs_runtime_check = true;
            }
            if needs_runtime_check {
                self.push_runtime_check_with_expected(
                    span,
                    RuntimeCheckKind::SetMembership,
                    Type::Set(expected.clone()),
                );
            }
            return;
        }
        if needs_runtime_check {
            self.push_runtime_check_with_expected(
                span,
                RuntimeCheckKind::SetMembership,
                Type::Set(expected.clone()),
            );
        }
        if hashability(actual) == Hashability::Hashable && !expected.contains_type(actual) {
            let expected_text = expected_member_text(expected);
            self.type_error_with_params(
                SET_MEMBERSHIP_TYPE_CODE,
                "x03.type.set_membership_element_mismatch",
                span,
                format!("成员类型 {} 不符合集合元素类型 {}", actual, expected_text),
                [
                    (
                        "actual_type".to_owned(),
                        DiagnosticParam::Text(actual.to_string()),
                    ),
                    (
                        "expected_type".to_owned(),
                        DiagnosticParam::Text(expected_text),
                    ),
                ],
            );
        }
    }

    /// 将显式集合前缀检查为成员类型约束，不复用数组的位置诊断。
    pub(super) fn check_set_element_assignment(
        &mut self,
        actual: &Type,
        expected: &Type,
        span: SourceSpan,
    ) -> bool {
        if crate::conversion::can_assign(actual, expected) {
            true
        } else {
            self.type_error_with_params(
                SET_ELEMENT_TYPE_MISMATCH_CODE,
                "x03.type.set_element_type_mismatch",
                span,
                format!("集合元素类型 {} 不符合显式类型 {}", actual, expected),
                type_params(actual, expected),
            );
            false
        }
    }

    /// 报告集合不可索引；调用方不得为该读取建立选择计划。
    pub(super) fn set_index_error(&mut self, span: SourceSpan) {
        self.type_error(
            SET_INDEX_UNSUPPORTED_CODE,
            "x03.type.set_index_unsupported",
            span,
            "集合没有数字或键名索引，也不能使用高级选择器".to_owned(),
        );
    }
}

/// 为诊断选择兼容的元素类型文本；同构集合沿用 C2-A 的 `int` 形式，
/// 异构集合则展示完整 `set<T | U>` 并集。
fn expected_member_text(expected: &SetType) -> String {
    expected
        .element_type()
        .map_or_else(|| expected.to_string(), ToString::to_string)
}

/// 将语法层的集合类型项降低为规范化的类型并集。
fn declared_set_type(annotation: &SetTypeAnnotation) -> SetType {
    let members = annotation
        .members
        .iter()
        .map(|term| match term {
            TypeTerm::Scalar(scalar) => Type::scalar(*scalar),
            TypeTerm::None => Type::None,
        })
        .collect::<Vec<_>>();
    SetType::heterogeneous(members)
}

/// 保留类型冲突的原始类型文本供后续消息目录插值。
fn type_params(actual: &Type, expected: &Type) -> [(String, DiagnosticParam); 2] {
    [
        (
            "actual_type".to_owned(),
            DiagnosticParam::Text(actual.to_string()),
        ),
        (
            "expected_type".to_owned(),
            DiagnosticParam::Text(expected.to_string()),
        ),
    ]
}

/// 比较同一静态类型的集合常量，不把 `bool` 或不同数值类型隐式合并。
fn same_set_constant(left: &ConstantValue, right: &ConstantValue) -> bool {
    match (left, right) {
        (ConstantValue::BigInteger(left), ConstantValue::BigInteger(right)) => {
            normalized_integer(left) == normalized_integer(right)
        }
        _ => left == right,
    }
}

/// 去掉大整数字面量的前导零，避免源码拼写影响静态唯一性。
fn normalized_integer(value: &str) -> &str {
    let normalized = value.trim_start_matches('0');
    if normalized.is_empty() {
        "0"
    } else {
        normalized
    }
}

/// 生成用于静态重复诊断的常量摘要。
fn format_constant(value: &ConstantValue) -> String {
    match value {
        ConstantValue::Integer(value) => value.to_string(),
        ConstantValue::BigInteger(value) => value.clone(),
        ConstantValue::Float(value) => value.to_string(),
        ConstantValue::Boolean(value) => value.to_string(),
        ConstantValue::String(value) => format!("\"{value}\""),
        ConstantValue::None => "none".to_owned(),
    }
}
