//! 语言二元运算的共同提升入口；实际数学仍调用A1～A4内核。

use super::{RuntimeValue, numeric};
use crate::errors::{RuntimeError, RuntimeResult};
use std::borrow::Cow;
use xiao_syntax::{BinaryOperator, ScalarType};
use xiao_types::numeric_text::NumericTextKind;

impl RuntimeValue {
    /// 采用冻结的语言提升矩阵执行运算，不改变集合/哈希使用的结构Eq。
    #[inline(always)]
    pub fn language_binary(&self, operator: BinaryOperator, other: &Self) -> RuntimeResult<Self> {
        if std::mem::discriminant(self) == std::mem::discriminant(other)
            && !matches!(
                (self, operator),
                (Self::Int(_) | Self::Sint(_), BinaryOperator::Divide)
                    | (
                        Self::Float(_) | Self::Sfloat(_),
                        BinaryOperator::FloorDivide | BinaryOperator::Remainder
                    )
            )
        {
            return perform(self, operator, other);
        }
        promoted(self, operator, other)
    }
}

/// 只在不同数值族或整数字面量语言除法时恢复共同类型。
fn promoted(
    left: &RuntimeValue,
    operator: BinaryOperator,
    right: &RuntimeValue,
) -> RuntimeResult<RuntimeValue> {
    if let (Some(a), Some(b)) = (left.scalar_type(), right.scalar_type())
        && xiao_types::is_numeric(a)
        && xiao_types::is_numeric(b)
    {
        let target = xiao_types::binary_operand_scalar(operator, a, b)
            .map_err(|_| {
                RuntimeError::type_mismatch(
                    format!("{}的数值操作数", operator.as_str()),
                    format!("{} / {}", a.as_str(), b.as_str()),
                )
            })?
            .expect("两个数值标量一定具有共同操作数类型");
        // 已求值的两个输入按顺序完整接纳，再做非有限/域/结果检查。
        validate(left)?;
        validate(right)?;
        let left = cast(left, target)?;
        let right = cast(right, target)?;
        return perform(&left, operator, &right);
    }
    perform(left, operator, right)
}

/// 保留已处于目标类型的借用值，避免为每次比较重建高精度载荷。
fn cast(value: &RuntimeValue, target: ScalarType) -> RuntimeResult<Cow<'_, RuntimeValue>> {
    if value.scalar_type() == Some(target) {
        Ok(Cow::Borrowed(value))
    } else {
        value.convert_to(target).map(Cow::Owned)
    }
}

/// 输入预算复用A1门面，不能让左侧非有限先覆盖右侧超预算。
fn validate(value: &RuntimeValue) -> RuntimeResult<()> {
    match value {
        RuntimeValue::Lint(text) => {
            numeric::validated_text(text, NumericTextKind::Integer).map(|_| ())
        }
        RuntimeValue::Lfloat(text) => {
            numeric::validated_text(text, NumericTextKind::Decimal).map(|_| ())
        }
        _ => Ok(()),
    }
}

/// 共用实际算子核；比较也不通过f64或原始显示文本求值。
#[inline(always)]
fn perform(
    left: &RuntimeValue,
    operator: BinaryOperator,
    right: &RuntimeValue,
) -> RuntimeResult<RuntimeValue> {
    use BinaryOperator::*;
    match operator {
        Add => left.add(right),
        Subtract => left.subtract(right),
        Multiply => left.multiply(right),
        Divide => left.divide(right),
        FloorDivide => left.floor_divide(right),
        Remainder => left.remainder(right),
        Power => left.power(right),
        Equal => Ok(RuntimeValue::Bool(left.equals(right))),
        NotEqual => Ok(RuntimeValue::Bool(left.not_equals(right))),
        Less => left.less(right).map(RuntimeValue::Bool),
        LessEqual => left.less_equal(right).map(RuntimeValue::Bool),
        Greater => left.greater(right).map(RuntimeValue::Bool),
        GreaterEqual => left.greater_equal(right).map(RuntimeValue::Bool),
        _ => Err(RuntimeError::invalid_value("此语言入口不接受该运算符")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    /// 语言数值相等采用提升，结构Eq/Hash保持原标签合同。
    fn language_promotion_does_not_change_structural_equality() {
        let integer = RuntimeValue::Int(1);
        let large = RuntimeValue::new_lint("1").unwrap();
        assert!(!integer.equals(&large));
        assert_eq!(
            integer
                .language_binary(BinaryOperator::Equal, &large)
                .unwrap(),
            RuntimeValue::Bool(true)
        );
        assert_eq!(
            RuntimeValue::Int(9007199254740993)
                .language_binary(
                    BinaryOperator::Equal,
                    &RuntimeValue::Float(9007199254740992.0)
                )
                .unwrap(),
            RuntimeValue::Bool(true)
        );
        assert_eq!(
            RuntimeValue::new_lint("9007199254740993")
                .unwrap()
                .language_binary(
                    BinaryOperator::Equal,
                    &RuntimeValue::Float(9007199254740992.0)
                )
                .unwrap(),
            RuntimeValue::Bool(false)
        );
    }

    #[test]
    /// 动态数值入口也执行F-4商余准入，原始定宽核继续要求同宽。
    fn language_dynamic_arithmetic_uses_frozen_promotions() {
        assert!(RuntimeValue::Int(1).add(&RuntimeValue::Sint(2)).is_err());
        assert_eq!(
            RuntimeValue::Int(1)
                .language_binary(BinaryOperator::Add, &RuntimeValue::Sint(2))
                .unwrap(),
            RuntimeValue::Int(3)
        );
        let value = RuntimeValue::new_lint("7")
            .unwrap()
            .language_binary(BinaryOperator::FloorDivide, &RuntimeValue::Float(2.0))
            .unwrap();
        assert!(matches!(value, RuntimeValue::Lfloat(_)));
        assert_eq!(value.display_text().unwrap(), "3");
        assert_eq!(
            RuntimeValue::Float(7.0)
                .language_binary(BinaryOperator::FloorDivide, &RuntimeValue::Int(2))
                .unwrap_err()
                .code(),
            "X06-RUNTIME-002"
        );
        assert_eq!(
            RuntimeValue::Int(1)
                .language_binary(BinaryOperator::Divide, &RuntimeValue::Int(2))
                .unwrap(),
            RuntimeValue::Float(0.5)
        );
    }

    #[test]
    /// 两侧输入先接纳，再进行非有限提升或数学域检查。
    fn language_input_budget_precedes_non_finite_promotion() {
        let bad = RuntimeValue::Lint("9".repeat(32769));
        assert_eq!(
            RuntimeValue::Float(f64::INFINITY)
                .language_binary(BinaryOperator::Add, &bad)
                .unwrap_err()
                .code(),
            "X06-RUNTIME-033"
        );
        let zero = RuntimeValue::new_lint("0").unwrap();
        assert_eq!(
            RuntimeValue::Float(f64::INFINITY)
                .language_binary(BinaryOperator::FloorDivide, &zero)
                .unwrap_err()
                .code(),
            "X06-RUNTIME-009"
        );
    }
}
