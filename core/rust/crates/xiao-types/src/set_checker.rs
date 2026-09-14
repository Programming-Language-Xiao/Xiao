//! C2-A 集合静态检查。
//!
//! 该模块只消费语法 AST 并生成 [`crate::Type`] 与结构化诊断。它不创建
//! Runtime 集合、不执行哈希，也不实现集合代数；动态元素只登记后续
//! Runtime 所需的检查标记。

use xiao_source::SourceSpan;
use xiao_syntax::{BinaryOperator, Expression};

use crate::diagnostics::{
    SET_CONSTRUCTOR_ARITY_CODE, SET_DUPLICATE_ELEMENT_CODE, SET_ELEMENT_TYPE_MISMATCH_CODE,
    SET_MEMBERSHIP_TYPE_CODE, SET_UNHASHABLE_ELEMENT_CODE,
};
use crate::numeric::ConstantValue;
use crate::set_types::{Hashability, SetType, hashability};
use crate::types::Type;

use super::{RuntimeCheckKind, TypeChecker};

impl<'source> TypeChecker<'source> {
    /// 检查集合字面量并推导 C2-A 的单一元素类型。
    pub(super) fn check_set_literal(&mut self, elements: &[Expression], _span: SourceSpan) -> Type {
        let mut element_type: Option<Type> = None;
        let mut has_dynamic_type = false;
        let mut has_invalid_element = false;
        let mut constants = Vec::<ConstantValue>::new();

        for element in elements {
            let ty = self.check_expression(element);
            let element_hashability = hashability(&ty);
            match element_hashability {
                Hashability::Hashable => {}
                Hashability::Unhashable => {
                    has_invalid_element = true;
                    self.type_error(
                        SET_UNHASHABLE_ELEMENT_CODE,
                        "x03.type.set_unhashable_element",
                        element.span(),
                        format!("类型 {} 不能作为集合元素", ty),
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

            match ty {
                Type::Dynamic | Type::Variable(_) => {
                    has_dynamic_type = true;
                }
                known => {
                    if let Some(expected) = element_type.as_ref() {
                        if expected != &known {
                            has_invalid_element = true;
                            self.type_error(
                                SET_ELEMENT_TYPE_MISMATCH_CODE,
                                "x03.type.set_element_type_mismatch",
                                element.span(),
                                format!("集合元素类型 {} 与已推断的 {} 不一致", known, expected),
                            );
                        }
                    } else {
                        element_type = Some(known);
                    }
                }
            }

            // 只有编译期已知的常量才参与静态唯一性检查；动态值的哈希和
            // 相等判断必须留给 Runtime，不能依据源码表达式文本猜测。
            if let Some(constant) = self.eval_const(element)
                && !constants.iter().any(|seen| seen == &constant)
            {
                constants.push(constant);
            } else if let Some(constant) = self.eval_const(element) {
                self.type_error(
                    SET_DUPLICATE_ELEMENT_CODE,
                    "x03.type.set_duplicate_element",
                    element.span(),
                    format!("集合中重复的静态元素 {}", format_constant(&constant)),
                );
            }
        }

        let set = if has_dynamic_type || has_invalid_element {
            SetType::Unknown
        } else if let Some(element) = element_type {
            SetType::homogeneous(element)
        } else {
            SetType::Unknown
        };
        Type::Set(set)
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
        arguments: &[Expression],
        span: SourceSpan,
    ) -> Type {
        if !arguments.is_empty() {
            for argument in arguments {
                self.check_expression(argument);
            }
            self.type_error(
                SET_CONSTRUCTOR_ARITY_CODE,
                "x03.type.set_constructor_arity",
                span,
                format!("set() 构造式不接受参数，实际收到 {} 个", arguments.len()),
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
        span: SourceSpan,
    ) -> Type {
        let left_type = self.check_expression(left);
        let right_type = self.check_expression(right);
        match right_type {
            Type::Set(set) => {
                self.check_membership_element(&left_type, set.element_type(), left.span());
                Type::scalar(xiao_syntax::ScalarType::Bool)
            }
            Type::Dynamic | Type::Variable(_) => {
                self.check_membership_element(&left_type, None, left.span());
                self.push_runtime_check(span, RuntimeCheckKind::SetMembership);
                Type::scalar(xiao_syntax::ScalarType::Bool)
            }
            other => {
                self.type_error(
                    SET_MEMBERSHIP_TYPE_CODE,
                    "x03.type.set_membership_requires_set",
                    right.span(),
                    format!(
                        "运算 {} 的右侧必须是集合，实际为 {}",
                        operator.as_str(),
                        other
                    ),
                );
                Type::Dynamic
            }
        }
    }

    /// 验证成员值的可哈希性和已知集合元素类型约束。
    fn check_membership_element(
        &mut self,
        actual: &Type,
        expected: Option<&Type>,
        span: SourceSpan,
    ) {
        match hashability(actual) {
            Hashability::Unhashable => self.type_error(
                SET_UNHASHABLE_ELEMENT_CODE,
                "x03.type.set_unhashable_membership",
                span,
                format!("类型 {} 不能作为集合成员查询值", actual),
            ),
            Hashability::Dynamic => {
                self.push_runtime_check(span, RuntimeCheckKind::SetMembership);
            }
            Hashability::Hashable => {}
        }
        if let Some(expected) = expected
            && hashability(actual) == Hashability::Hashable
            && !actual.is_dynamic()
            && !matches!(actual, Type::Variable(_))
            && !crate::conversion::can_assign(actual, expected)
        {
            self.type_error(
                SET_MEMBERSHIP_TYPE_CODE,
                "x03.type.set_membership_element_mismatch",
                span,
                format!("成员类型 {} 不符合集合元素类型 {}", actual, expected),
            );
        }
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
