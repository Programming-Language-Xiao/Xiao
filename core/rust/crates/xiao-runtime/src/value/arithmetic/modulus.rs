//! F-3b独立余数预算；不计算或物化完整商。

use super::common::{self, Family, Number};
use crate::errors::RuntimeResult;
use crate::value::RuntimeValue;
use bigdecimal::{Signed, Zero};
use num_bigint::BigInt;
use std::cmp::Ordering;
use xiao_types::numeric_text::NUMERIC_DIGIT_LIMIT as D;

/// 一次有界模乘；连同最终混合乘法，总次数不得超过64。
fn mod_multiply(
    left: &BigInt,
    right: &BigInt,
    modulus: &BigInt,
    count: &mut usize,
) -> RuntimeResult<BigInt> {
    *count += 1;
    if *count > 64 {
        return Err(common::limit("余数模乘超过64次"));
    }
    let product = common::multiply(left, right, "modular-multiply")?;
    common::check_work("modular-reduction", common::decimal_digits(&product))?;
    common::checked_value("modular-reduction", product % modulus)
}

/// 10^k mod B以平方求模生成，不能先展开10^k。
fn positive_scale(a: &BigInt, b: &BigInt, mut exponent: u32) -> RuntimeResult<BigInt> {
    if *b == BigInt::from(1) {
        return Ok(BigInt::from(0));
    }
    let mut result = BigInt::from(1);
    let mut factor = BigInt::from(10) % b;
    let mut count = 0;
    while exponent != 0 {
        if exponent & 1 != 0 {
            result = mod_multiply(&result, &factor, b, &mut count)?;
        }
        exponent >>= 1;
        if exponent != 0 {
            factor = mod_multiply(&factor, &factor, b, &mut count)?;
        }
    }
    common::check_work(
        "modular-input-remainder",
        common::decimal_digits(a).max(common::decimal_digits(b)),
    )?;
    let reduced = common::checked_value("modular-input-remainder", a % b)?;
    mod_multiply(&reduced, &result, b, &mut count)
}

/// 同号取截断余数，异号非零时取模数减余数；最终符号随除数。
fn correct(z: BigInt, modulus: &BigInt, different: bool) -> RuntimeResult<BigInt> {
    if !different || z.is_zero() {
        return Ok(z);
    }
    common::check_work("remainder-correction", common::decimal_digits(modulus))?;
    common::checked_value("remainder-correction", modulus - z)
}

/// k<0的虚拟分母路径，返回已完成floor符号修正的绝对余数。
fn negative_scale(
    a: &BigInt,
    b: &BigInt,
    db: usize,
    zeros: u32,
    different: bool,
) -> RuntimeResult<BigInt> {
    let order = super::comparison::magnitude(a, 0, b, zeros);
    if order == Ordering::Less && !different {
        return Ok(a.clone());
    }
    let length = db + zeros as usize;
    if order == Ordering::Less && length > D + 1 {
        return Err(common::limit("异号余数差的可证明位数超过32768"));
    }
    common::check_work("remainder-denominator", length)?;
    let modulus = common::align(b, db, zeros)?;
    if order == Ordering::Less {
        // D+1位分母必须实际计算差；这一分支已经完成异号修正。
        common::check_work("remainder-boundary-difference", length)?;
        return common::checked_value("remainder-boundary-difference", modulus - a);
    }
    common::check_work("decimal-remainder", common::decimal_digits(a))?;
    let z = common::checked_value("decimal-remainder", a % &modulus)?;
    correct(z, &modulus, different)
}

/// 商超D不影响余数独立成功；零余数不展开显示scale。
pub(super) fn remainder(left: &Number, right: &Number) -> RuntimeResult<RuntimeValue> {
    if left.is_zero() {
        return common::finish(BigInt::from(0), 0, Family::Decimal);
    }
    let a = left.coefficient.abs();
    let b = right.coefficient.abs();
    let k = right.scale - left.scale;
    let different = left.coefficient.sign() != right.coefficient.sign();
    let magnitude = if k >= 0 {
        correct(positive_scale(&a, &b, k as u32)?, &b, different)?
    } else {
        negative_scale(&a, &b, right.digits, (-k) as u32, different)?
    };
    let coefficient = if right.coefficient.is_negative() {
        -magnitude
    } else {
        magnitude
    };
    common::finish(coefficient, left.scale.max(right.scale), Family::Decimal)
}
