//! F-3b十进制floor商；先判精确数量级，再做至多一组有界对齐。

use super::common::{self, Family, Number};
use crate::errors::RuntimeResult;
use crate::value::RuntimeValue;
use bigdecimal::{Signed, Zero};
use num_bigint::BigInt;
use xiao_types::numeric_text::NUMERIC_DIGIT_LIMIT as D;

/// 非零分母已在入口检查，floor负数进位后仍须按实际系数检查D。
pub(super) fn floor(left: &Number, right: &Number) -> RuntimeResult<RuntimeValue> {
    if left.is_zero() {
        return common::finish(BigInt::from(0), 0, Family::Decimal);
    }
    let a = left.coefficient.abs();
    let b = right.coefficient.abs();
    let different = left.coefficient.sign() != right.coefficient.sign();
    let k = right.scale - left.scale;
    let ell = super::comparison::ratio_order(&a, left.digits, &b, right.digits) + k;
    if ell < 0 {
        return common::finish(
            BigInt::from(if different { -1 } else { 0 }),
            0,
            Family::Decimal,
        );
    }
    if ell as usize + 1 > D {
        return Err(common::limit("floor商的精确数量级超过32768位"));
    }
    common::check_work("floor-numerator", left.digits + k.max(0) as usize)?;
    common::check_work("floor-denominator", right.digits + (-k).max(0) as usize)?;
    let numerator = common::align(&a, left.digits, k.max(0) as u32)?;
    let denominator = common::align(&b, right.digits, (-k).max(0) as u32)?;
    let (mut quotient, remainder) = common::div_rem(&numerator, &denominator)?;
    if different {
        if !remainder.is_zero() {
            common::check_work("floor-correction", common::decimal_digits(&quotient) + 1)?;
            quotient = common::checked_value("floor-correction", quotient + 1_u8)?;
        }
        quotient = -quotient;
    }
    common::finish(quotient, 0, Family::Decimal)
}
