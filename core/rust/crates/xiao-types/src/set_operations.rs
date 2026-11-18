//! C2-C 集合运算的静态分派与检查计划。
//!
//! 本模块把集合运算从主检查器中隔离出来：它只消费已经推导出的 [`Type`]
//! 并生成集合结果类型、稳定诊断和 Runtime 检查标记，不创建集合对象，也不
//! 执行哈希或成员值运算。这样后续 Runtime/IR 可以复用同一套类型规则。

use xiao_diagnostics::DiagnosticParam;
use xiao_source::SourceSpan;
use xiao_syntax::BinaryOperator;

use crate::diagnostics::{SET_COMPARISON_TYPE_CODE, SET_OPERATION_TYPE_CODE};
use crate::types::Type;

use super::{RuntimeCheckKind, TypeChecker};

/// 一次集合二元运算的静态检查结果。
pub(super) struct SetOperationCheck {
    /// 运算结果类型；比较运算固定为 `bool`。
    pub(super) result: Type,
    /// 两侧类型是否满足该集合运算的静态形状要求。
    pub(super) valid: bool,
    /// 是否需要把结果写回一个集合左值时继续检查动态成员。
    pub(super) has_dynamic_boundary: bool,
}

/// 判断运算符是否具有集合代数语义。
pub(super) const fn is_set_operation_operator(operator: BinaryOperator) -> bool {
    matches!(
        operator,
        BinaryOperator::Add
            | BinaryOperator::Subtract
            | BinaryOperator::Intersect
            | BinaryOperator::SymmetricDifference
    )
}

/// 判断运算符是否可以比较两个集合。
pub(super) const fn is_set_comparison_operator(operator: BinaryOperator) -> bool {
    matches!(
        operator,
        BinaryOperator::Equal
            | BinaryOperator::NotEqual
            | BinaryOperator::Less
            | BinaryOperator::LessEqual
            | BinaryOperator::Greater
            | BinaryOperator::GreaterEqual
    )
}

/// 判断当前类型组合是否应转入集合语义分支。
pub(super) fn should_attempt_set_semantics(
    operator: BinaryOperator,
    left: &Type,
    right: &Type,
) -> bool {
    if is_set_comparison_operator(operator) {
        return left.is_set() || right.is_set();
    }
    if !is_set_operation_operator(operator) {
        return false;
    }
    matches!(
        operator,
        BinaryOperator::Intersect | BinaryOperator::SymmetricDifference
    ) || left.is_set()
        || right.is_set()
}

impl<'source> TypeChecker<'source> {
    /// 检查已经求得操作数类型的集合运算，并登记动态检查计划。
    pub(super) fn check_set_operation_types(
        &mut self,
        operator: BinaryOperator,
        left: &Type,
        right: &Type,
        span: SourceSpan,
    ) -> SetOperationCheck {
        if is_set_comparison_operator(operator) {
            return self.check_set_comparison_types(operator, left, right, span);
        }

        let dynamic_left = is_dynamic_boundary(left);
        let dynamic_right = is_dynamic_boundary(right);
        match (left, right) {
            (Type::Set(left), Type::Set(right)) => {
                let result = match operator {
                    BinaryOperator::Add => left.union(right),
                    BinaryOperator::Subtract => left.difference(right),
                    BinaryOperator::Intersect => left.intersection(right),
                    BinaryOperator::SymmetricDifference => left.symmetric_difference(right),
                    _ => unreachable!("调用方只应传入集合代数运算符"),
                };
                let has_dynamic_boundary = left.has_dynamic_boundary()
                    || right.has_dynamic_boundary()
                    || result.has_dynamic_boundary();
                if has_dynamic_boundary {
                    self.push_runtime_check(span, RuntimeCheckKind::SetOperation);
                }
                SetOperationCheck {
                    result: Type::Set(result),
                    valid: true,
                    has_dynamic_boundary,
                }
            }
            (_, _)
                if (dynamic_left || dynamic_right)
                    && is_set_shape_boundary(left)
                    && is_set_shape_boundary(right) =>
            {
                // 动态值只有在另一侧已经确定为集合，或运算符本身只属于
                // 集合代数时才进入此分支；实际集合形状留给 Runtime 验证。
                self.push_runtime_check(span, RuntimeCheckKind::SetOperation);
                SetOperationCheck {
                    result: Type::Dynamic,
                    valid: true,
                    has_dynamic_boundary: true,
                }
            }
            _ => {
                self.report_set_operand_error(operator, left, right, span, false);
                SetOperationCheck {
                    result: Type::Dynamic,
                    valid: false,
                    has_dynamic_boundary: false,
                }
            }
        }
    }

    /// 检查集合比较；结果固定是 `bool`，动态边界登记比较检查。
    fn check_set_comparison_types(
        &mut self,
        operator: BinaryOperator,
        left: &Type,
        right: &Type,
        span: SourceSpan,
    ) -> SetOperationCheck {
        let dynamic_boundary = is_dynamic_boundary(left) || is_dynamic_boundary(right);
        match (left, right) {
            (Type::Set(left), Type::Set(right)) => {
                let dynamic_boundary =
                    dynamic_boundary || left.has_dynamic_boundary() || right.has_dynamic_boundary();
                if dynamic_boundary {
                    self.push_runtime_check(span, RuntimeCheckKind::SetComparison);
                }
                SetOperationCheck {
                    result: Type::scalar(xiao_syntax::ScalarType::Bool),
                    valid: true,
                    has_dynamic_boundary: dynamic_boundary,
                }
            }
            (left, right)
                if (is_dynamic_boundary(left) || is_dynamic_boundary(right))
                    && is_set_shape_boundary(left)
                    && is_set_shape_boundary(right) =>
            {
                self.push_runtime_check(span, RuntimeCheckKind::SetComparison);
                SetOperationCheck {
                    result: Type::scalar(xiao_syntax::ScalarType::Bool),
                    valid: true,
                    has_dynamic_boundary: true,
                }
            }
            _ => {
                self.report_set_operand_error(operator, left, right, span, true);
                SetOperationCheck {
                    result: Type::Dynamic,
                    valid: false,
                    has_dynamic_boundary: false,
                }
            }
        }
    }

    /// 生成集合运算或比较的稳定类型诊断。
    fn report_set_operand_error(
        &mut self,
        operator: BinaryOperator,
        left: &Type,
        right: &Type,
        span: SourceSpan,
        comparison: bool,
    ) {
        let (code, message_id, message) = if comparison {
            (
                SET_COMPARISON_TYPE_CODE,
                "x03.type.set_comparison_requires_sets",
                format!(
                    "集合比较 {} 需要两个集合，实际为 {} 和 {}",
                    operator.as_str(),
                    left,
                    right
                ),
            )
        } else {
            (
                SET_OPERATION_TYPE_CODE,
                "x03.type.set_operation_requires_sets",
                format!(
                    "集合运算 {} 需要两个集合，实际为 {} 和 {}",
                    operator.as_str(),
                    left,
                    right
                ),
            )
        };
        self.type_error_with_params(
            code,
            message_id,
            span,
            message,
            [
                (
                    "operator".to_owned(),
                    DiagnosticParam::Text(operator.as_str().to_owned()),
                ),
                (
                    "left_type".to_owned(),
                    DiagnosticParam::Text(left.to_string()),
                ),
                (
                    "right_type".to_owned(),
                    DiagnosticParam::Text(right.to_string()),
                ),
            ],
        );
    }
}

/// 判断类型是否是可以在 Runtime 才确定集合形状的边界。
fn is_dynamic_boundary(ty: &Type) -> bool {
    match ty {
        Type::Dynamic | Type::Variable(_) => true,
        Type::Set(set) => set.has_dynamic_boundary(),
        _ => false,
    }
}

/// 判断类型是否至少可能在 Runtime 表示一个集合。
fn is_set_shape_boundary(ty: &Type) -> bool {
    matches!(ty, Type::Set(_) | Type::Dynamic | Type::Variable(_))
}
