//! 有界平方求幂，保留0/±1与n=0/1快路径。

use super::common::{self, Family, Number};
use crate::errors::{RuntimeError, RuntimeResult};
use crate::value::RuntimeValue;
use bigdecimal::{Signed, ToPrimitive};
use num_bigint::BigInt;
use xiao_types::numeric_text::NUMERIC_DIGIT_LIMIT as D;

/// 系数平方求幂，最多64次受检整数乘法，不做最后多余平方。
pub(super) fn coefficient(base: &BigInt, mut exponent: u32) -> RuntimeResult<BigInt> {
    let mut result = BigInt::from(1);
    let mut factor = base.clone();
    let mut count = 0;
    while exponent != 0 {
        if exponent & 1 != 0 {
            count += 1;
            result = common::multiply(&result, &factor, "power-result")?;
        }
        exponent >>= 1;
        if exponent != 0 {
            count += 1;
            factor = common::multiply(&factor, &factor, "power-square")?;
        }
        if count > 64 {
            return Err(common::limit("幂整数乘法超过64次"));
        }
    }
    Ok(result)
}

/// A2非负整数指数；快路径前已接纳两个输入，不把巨大合法指数窄化。
pub(super) fn integer(base: &Number, exponent: &Number) -> RuntimeResult<RuntimeValue> {
    if exponent.coefficient.sign() == num_bigint::Sign::Minus {
        return Err(RuntimeError::numeric_overflow("lint幂不接受负指数"));
    }
    if exponent.is_zero() {
        return common::finish(BigInt::from(1), 0, Family::Integer);
    }
    if exponent.coefficient == BigInt::from(1) {
        return common::finish(base.coefficient.clone(), 0, Family::Integer);
    }
    if base.is_zero() {
        return common::finish(BigInt::from(0), 0, Family::Integer);
    }
    if base.coefficient.abs() == BigInt::from(1) {
        let odd = (&exponent.coefficient % 2_u8) != BigInt::from(0);
        let result = if base.coefficient.sign() == num_bigint::Sign::Minus && odd {
            -1
        } else {
            1
        };
        return common::finish(BigInt::from(result), 0, Family::Integer);
    }
    if exponent.coefficient > BigInt::from(1_000_000_u32) {
        return Err(common::limit("非平凡幂指数超过1000000"));
    }
    let n = exponent
        .coefficient
        .to_u32()
        .ok_or_else(|| common::limit("幂指数无法有界恢复"))?;
    let lower = (base.digits - 1) as u64 * u64::from(n) + 1;
    if lower > D as u64 {
        return Err(common::limit("幂结果的可证明位数下界超过32768"));
    }
    let result = coefficient(&base.coefficient, n)?;
    common::finish(result, 0, Family::Integer)
}
