//! A3非负整数值指数与0/±1快路径，不窄化巨大合法指数。

use super::common::{self, Family, Number};
use crate::errors::{RuntimeError, RuntimeResult};
use crate::value::RuntimeValue;
use bigdecimal::{Signed, ToPrimitive};
use num_bigint::BigInt;
use std::cmp::Ordering;
use xiao_types::numeric_text::NUMERIC_DIGIT_LIMIT as D;

/// 规范指数只允许非负整数值；1.0合法，负数与1.5均为009。
pub(super) fn power(base: &Number, exponent: &Number) -> RuntimeResult<RuntimeValue> {
    let normalized = common::normalize(exponent.coefficient.clone(), exponent.scale)?;
    if normalized.coefficient.is_negative() || normalized.scale > 0 {
        return Err(RuntimeError::numeric_overflow(
            "lfloat幂只接受非负整数值指数",
        ));
    }
    if normalized.is_zero() {
        return common::finish(BigInt::from(1), 0, Family::Decimal);
    }
    if normalized.scale == 0 && normalized.coefficient == BigInt::from(1) {
        return common::finish(base.coefficient.clone(), base.scale, Family::Decimal);
    }
    if base.is_zero() {
        return common::finish(BigInt::from(0), 0, Family::Decimal);
    }
    let normalized_base = common::normalize(base.coefficient.clone(), base.scale)?;
    if normalized_base.scale == 0 && normalized_base.coefficient.abs() == BigInt::from(1) {
        let odd = normalized.scale == 0 && (&normalized.coefficient % 2_u8) != BigInt::from(0);
        let result = if normalized_base.coefficient.is_negative() && odd {
            -1
        } else {
            1
        };
        return common::finish(BigInt::from(result), 0, Family::Decimal);
    }
    let maximum = Number {
        coefficient: BigInt::from(1_000_000_u32),
        scale: 0,
        digits: 7,
    };
    if super::comparison::numbers(&normalized, &maximum) == Ordering::Greater {
        return Err(common::limit("非平凡幂指数超过1000000"));
    }
    let n = normalized
        .coefficient
        .to_u32()
        .ok_or_else(|| common::limit("幂指数无法有界恢复"))?
        * 10_u32.pow((-normalized.scale) as u32);
    let scale = i64::from(base.scale) * i64::from(n);
    common::check_scales(scale, i64::from(base.canonical_scale()) * i64::from(n))?;
    let lower = (base.digits - 1) as u64 * u64::from(n) + 1;
    if lower > D as u64 {
        return Err(common::limit("幂显示系数的可证明位数下界超过32768"));
    }
    let result = super::power::coefficient(&base.coefficient, n)?;
    common::finish(result, scale as i32, Family::Decimal)
}
