//! 精确有限十进制除法；不调用上游默认除法上下文。

use super::common::{self, Family, Number};
use crate::errors::{RuntimeError, RuntimeResult};
use crate::value::RuntimeValue;
use bigdecimal::{Signed, Zero};
use num_bigint::BigInt;
use xiao_types::numeric_text::NUMERIC_DIGIT_LIMIT as D;

/// gcd只处理已接纳系数，欧几里得步数遵守已冻结5D+2上界。
fn gcd(mut left: BigInt, mut right: BigInt) -> RuntimeResult<BigInt> {
    let mut steps = 0;
    while !right.is_zero() {
        steps += 1;
        if steps > 5 * D + 2 {
            return Err(common::limit("gcd逻辑工作量超过上界"));
        }
        common::check_work(
            "gcd-remainder",
            common::decimal_digits(&left).max(common::decimal_digits(&right)),
        )?;
        let remainder = common::checked_value("gcd-remainder", &left % &right)?;
        left = right;
        right = remainder;
    }
    Ok(left)
}

/// 先约分、再精确判定2/5因子，034不得被后续展开预算抢先覆盖。
pub(super) fn divide(left: &Number, right: &Number) -> RuntimeResult<RuntimeValue> {
    if right.is_zero() {
        return Err(RuntimeError::division_by_zero("/"));
    }
    if left.is_zero() {
        return common::finish(BigInt::from(0), 0, Family::Decimal);
    }
    let negative = left.coefficient.sign() != right.coefficient.sign();
    let a = left.coefficient.abs();
    let b = right.coefficient.abs();
    let divisor = gcd(a.clone(), b.clone())?;
    let a = a / &divisor;
    let mut b = b / divisor;
    let twos = b.trailing_zeros().unwrap_or(0);
    b >>= twos;
    let chunk = BigInt::from(7_450_580_596_923_828_125_u64); // 5^27，避免逐个剥除的大量长除。
    let five = BigInt::from(5);
    let one = BigInt::from(1);
    let mut fives = 0_u64;
    while b != one && (&b % &chunk).is_zero() {
        b /= &chunk;
        fives += 27;
    }
    while b != one && (&b % &five).is_zero() {
        b /= &five;
        fives += 1;
    }
    if b != one {
        return Err(RuntimeError::non_terminating_division());
    }
    if twos + fives > (4 * D) as u64 {
        return Err(common::limit("有限因子逻辑工作量超过上界"));
    }
    let k = twos.max(fives);
    let difference = twos.abs_diff(fives);
    common::check_work(
        "finite-division-coefficient",
        common::decimal_digits(&a) + difference as usize,
    )?;
    let mut coefficient = if difference == 0 {
        a
    } else {
        common::check_work("finite-division-factor", difference as usize + 1)?;
        let factor = BigInt::from(if twos > fives { 5 } else { 2 }).pow(difference as u32);
        let factor = common::checked_value("finite-division-factor", factor)?;
        common::checked_value("finite-division-coefficient", a * factor)?
    };
    if negative {
        coefficient = -coefficient;
    }
    let base_scale = left.scale - right.scale;
    let normalized = common::normalize(coefficient, base_scale + k as i32)?;
    let display = base_scale.max(normalized.scale);
    common::check_scales(i64::from(display), i64::from(normalized.scale))?;
    let padding = (display - normalized.scale) as u32;
    if normalized.digits + padding as usize > D {
        return Err(common::limit("精确除法的显示系数超过32768位"));
    }
    let coefficient = common::align(&normalized.coefficient, normalized.digits, padding)?;
    common::finish(coefficient, display, Family::Decimal)
}
