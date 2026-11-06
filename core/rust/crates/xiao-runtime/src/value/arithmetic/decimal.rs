//! A3十进制入口、加减乘；输入/非有限/算子域按裁定顺序检查。

use super::Operation;
use super::common::{self, Family, Number};
use crate::errors::{RuntimeError, RuntimeResult};
use crate::value::RuntimeValue;
use num_bigint::BigInt;
use std::cmp::Ordering;

/// 先完整接纳两侧文本，再拒绝任一非有限输入，之后才恢复精确系数。
fn inputs(left: &RuntimeValue, right: &RuntimeValue) -> RuntimeResult<(Number, Number)> {
    common::validate_input(left)?;
    common::validate_input(right)?;
    super::float_lift::check_finite(left)?;
    super::float_lift::check_finite(right)?;
    Ok((Number::decimal(left)?, Number::decimal(right)?))
}

/// 已通过动态类型检查的高精度混合算子；结果保持lfloat族。
pub(super) fn binary(
    op: Operation,
    left: &RuntimeValue,
    right: &RuntimeValue,
) -> RuntimeResult<RuntimeValue> {
    let (left, right) = inputs(left, right)?;
    if matches!(
        op,
        Operation::Divide | Operation::FloorDivide | Operation::Remainder
    ) && right.is_zero()
    {
        return Err(RuntimeError::division_by_zero(op.name()));
    }
    match op {
        Operation::Add | Operation::Subtract => add_subtract(&left, &right, op),
        Operation::Multiply => multiply(&left, &right),
        Operation::Divide => super::exact_division::divide(&left, &right),
        Operation::FloorDivide => super::quotient::floor(&left, &right),
        Operation::Remainder => super::modulus::remainder(&left, &right),
        Operation::Power => super::decimal_power::power(&left, &right),
    }
}

/// 零不抬高显示scale，强消去结果只按实际位数检查D。
fn add_subtract(left: &Number, right: &Number, op: Operation) -> RuntimeResult<RuntimeValue> {
    if left.is_zero() {
        let coefficient = if op == Operation::Subtract {
            -&right.coefficient
        } else {
            right.coefficient.clone()
        };
        return common::finish(coefficient, right.scale, Family::Decimal);
    }
    if right.is_zero() {
        return common::finish(left.coefficient.clone(), left.scale, Family::Decimal);
    }
    let scale = left.scale.max(right.scale);
    let left_zeros = (scale - left.scale) as u32;
    let right_zeros = (scale - right.scale) as u32;
    common::check_work(
        "decimal-add-subtract",
        (left.digits + left_zeros as usize).max(right.digits + right_zeros as usize) + 1,
    )?;
    let a = common::align(&left.coefficient, left.digits, left_zeros)?;
    let b = common::align(&right.coefficient, right.digits, right_zeros)?;
    let value = if op == Operation::Add { a + b } else { a - b };
    common::finish(
        common::checked_value("decimal-add-subtract", value)?,
        scale,
        Family::Decimal,
    )
}

/// 非零乘法先检查显示scale和工作预算，规范scale再按实际乘积验收。
fn multiply(left: &Number, right: &Number) -> RuntimeResult<RuntimeValue> {
    if left.is_zero() || right.is_zero() {
        return common::finish(BigInt::from(0), 0, Family::Decimal);
    }
    let scale = i64::from(left.scale) + i64::from(right.scale);
    common::check_scales(scale, scale)?;
    let value = common::multiply(&left.coefficient, &right.coefficient, "decimal-multiply")?;
    common::finish(value, scale as i32, Family::Decimal)
}

/// 数值顺序通过虚拟补零比较，避免为极端scale差创建大整数。
pub(super) fn compare(left: &RuntimeValue, right: &RuntimeValue) -> RuntimeResult<Ordering> {
    let (left, right) = inputs(left, right)?;
    Ok(super::comparison::numbers(&left, &right))
}
