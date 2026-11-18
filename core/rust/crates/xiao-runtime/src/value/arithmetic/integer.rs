//! A2整数精确内核；整数族混合只在涉及lint时进入这里。

use super::Operation;
use super::common::{self, Family, Number};
use crate::errors::{RuntimeError, RuntimeResult};
use crate::value::RuntimeValue;
use bigdecimal::{Signed, Zero};
use std::cmp::Ordering;

/// 整数输入在运算前先按实参顺序完成同一接纳。
pub(super) fn binary(
    op: Operation,
    left: &RuntimeValue,
    right: &RuntimeValue,
) -> RuntimeResult<RuntimeValue> {
    common::validate_input(left)?;
    common::validate_input(right)?;
    let left = Number::integer(left)?;
    let right = Number::integer(right)?;
    if matches!(
        op,
        Operation::Divide | Operation::FloorDivide | Operation::Remainder
    ) && right.is_zero()
    {
        return Err(RuntimeError::division_by_zero(op.name()));
    }
    match op {
        Operation::Add | Operation::Subtract => {
            common::check_work("integer-add-subtract", left.digits.max(right.digits) + 1)?;
            let value = if op == Operation::Add {
                &left.coefficient + &right.coefficient
            } else {
                &left.coefficient - &right.coefficient
            };
            common::finish(
                common::checked_value("integer-add-subtract", value)?,
                0,
                Family::Integer,
            )
        }
        Operation::Multiply => {
            let value =
                common::multiply(&left.coefficient, &right.coefficient, "integer-multiply")?;
            common::finish(value, 0, Family::Integer)
        }
        Operation::Divide => super::exact_division::divide(&left, &right),
        Operation::FloorDivide | Operation::Remainder => {
            let different_signs = left.coefficient.sign() != right.coefficient.sign();
            let a = left.coefficient.abs();
            let b = right.coefficient.abs();
            if op == Operation::FloorDivide {
                let (mut quotient, remainder) = common::div_rem(&a, &b)?;
                if different_signs {
                    if !remainder.is_zero() {
                        quotient += 1;
                    }
                    quotient = -quotient;
                }
                common::finish(quotient, 0, Family::Integer)
            } else {
                common::check_work("integer-remainder", left.digits.max(right.digits))?;
                let remainder = common::checked_value("integer-remainder", &a % &b)?;
                let magnitude = if different_signs && !remainder.is_zero() {
                    &b - remainder
                } else {
                    remainder
                };
                let value = if right.coefficient.sign() == num_bigint::Sign::Minus {
                    -magnitude
                } else {
                    magnitude
                };
                common::finish(value, 0, Family::Integer)
            }
        }
        Operation::Power => super::power::integer(&left, &right),
    }
}

/// 有界系数精确比较，不经过文本字典序或浮点。
pub(super) fn compare(left: &RuntimeValue, right: &RuntimeValue) -> RuntimeResult<Ordering> {
    common::validate_input(left)?;
    common::validate_input(right)?;
    Ok(Number::integer(left)?
        .coefficient
        .cmp(&Number::integer(right)?.coefficient))
}

/// 大lint的bool奇偶只读取最低有效数字，不窄化宿主整数。
pub(super) fn adjust_bool(value: bool, text: &str) -> RuntimeResult<bool> {
    super::super::numeric::validated_text(
        text,
        xiao_types::numeric_text::NumericTextKind::Integer,
    )?;
    let odd = text
        .as_bytes()
        .last()
        .is_some_and(|byte| (byte - b'0') % 2 == 1);
    Ok(value ^ odd)
}
