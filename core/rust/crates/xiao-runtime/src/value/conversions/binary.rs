//! 十进制有理数直接舍入为binary32/64；不经过宿主f64中转或默认精度。

use super::{budget, decimal::Decimal};
use crate::errors::{RuntimeError, RuntimeResult};
use crate::value::RuntimeValue;
use bigdecimal::{Signed, ToPrimitive, Zero};
use num_bigint::BigUint;
use std::cmp::Ordering;
use xiao_syntax::ScalarType;

/// IEEE目标的精确常量；十进制裁剪只用于确定无争议的溢出/零。
#[derive(Clone, Copy)]
struct Format {
    /// 显式小数有效位数，精度还包括一个隐含首位。
    fraction: u32,
    /// 正常数无偏最小指数。
    minimum: i32,
    /// 正常数无偏最大指数。
    maximum: i32,
    /// IEEE指数偏移。
    bias: i32,
    /// 符号位的位置。
    sign_bit: u32,
    /// 大于此十进制数量级必溢出；边界仍精确求值。
    decimal_max: i32,
    /// 小于此十进制数量级必舍入为零；边界仍精确求值。
    decimal_min: i32,
}

/// binary64的目标常量。
const F64: Format = Format {
    fraction: 52,
    minimum: -1022,
    maximum: 1023,
    bias: 1023,
    sign_bit: 63,
    decimal_max: 308,
    decimal_min: -324,
};
/// binary32的目标常量。
const F32: Format = Format {
    fraction: 23,
    minimum: -126,
    maximum: 127,
    bias: 127,
    sign_bit: 31,
    decimal_max: 38,
    decimal_min: -46,
};

/// 目标范围失败为009，按正确舍入结果而不是数学值是否略大于MAX判定。
fn overflow() -> RuntimeError {
    RuntimeError::numeric_overflow("正确舍入结果超出目标二进制浮点范围")
}

/// 只接收lint/lfloat，输入接纳必须先于结果域检查。
pub(super) fn convert(value: &RuntimeValue, target: ScalarType) -> RuntimeResult<RuntimeValue> {
    let value = Decimal::from_value(value)?;
    match target {
        ScalarType::Float => {
            encode(value, F64).map(|bits| RuntimeValue::Float(f64::from_bits(bits)))
        }
        ScalarType::Sfloat => {
            encode(value, F32).map(|bits| RuntimeValue::Sfloat(f32::from_bits(bits as u32)))
        }
        _ => unreachable!("调用方只指定二进制浮点目标"),
    }
}

/// 不展开巨型十进制指数；剩余边界输入的分子/分母至多约3.4万位。
fn rational(value: &Decimal) -> RuntimeResult<(BigUint, BigUint)> {
    let coefficient = value.coefficient.magnitude();
    if value.scale >= 0 {
        let denominator = budget::ten(value.scale as u32)?.into_parts().1;
        Ok((coefficient.clone(), denominator))
    } else {
        budget::check(
            "binary-decimal-padding",
            value.digits + (-value.scale) as usize,
        )?;
        let factor = budget::ten((-value.scale) as u32)?.into_parts().1;
        let numerator = budget::unsigned("binary-decimal-padding", coefficient * factor)?;
        Ok((numerator, BigUint::from(1_u8)))
    }
}

/// floor(log2(N/D))由二进制位数和一次精确比较得到，不使用浮点log。
fn exponent(numerator: &BigUint, denominator: &BigUint) -> RuntimeResult<i32> {
    let guess = numerator.bits() as i32 - denominator.bits() as i32;
    let order = if guess >= 0 {
        numerator.cmp(&budget::shift(
            denominator,
            guess as u32,
            "binary-order-denominator",
        )?)
    } else {
        budget::shift(numerator, (-guess) as u32, "binary-order-numerator")?.cmp(denominator)
    };
    Ok(guess - i32::from(order == Ordering::Less))
}

/// 一次整数长除后用q*D恢复余数，最近偶数只在最后舍入一次。
fn rounded_significand(
    numerator: &BigUint,
    denominator: &BigUint,
    quantum: i32,
) -> RuntimeResult<u64> {
    let (numerator, denominator) = if quantum < 0 {
        (
            budget::shift(numerator, (-quantum) as u32, "binary-scale-numerator")?,
            denominator.clone(),
        )
    } else {
        (
            numerator.clone(),
            budget::shift(denominator, quantum as u32, "binary-scale-denominator")?,
        )
    };
    let bound = numerator
        .to_str_radix(10)
        .len()
        .max(denominator.to_str_radix(10).len());
    budget::check("binary-quotient", bound)?;
    let quotient = budget::unsigned("binary-quotient", &numerator / &denominator)?;
    // q*D<=N，因此用N的精确位数上界，不用系数位数和误拒。
    budget::check("binary-product", numerator.to_str_radix(10).len())?;
    let product = budget::unsigned("binary-product", &quotient * &denominator)?;
    budget::check("binary-remainder", numerator.to_str_radix(10).len())?;
    let remainder = budget::unsigned("binary-remainder", numerator - product)?;
    let twice = budget::shift(&remainder, 1, "binary-half-comparison")?;
    let mut significand = quotient.to_u64().ok_or_else(overflow)?;
    if twice > denominator || (twice == denominator && significand & 1 != 0) {
        significand += 1;
    }
    Ok(significand)
}

/// 正常/子正规数共用有理数舍入；非零负来源舍入到零仍保留负号。
fn encode(value: Decimal, format: Format) -> RuntimeResult<u64> {
    if value.coefficient.is_zero() {
        return Ok(0);
    }
    let sign = u64::from(value.coefficient.is_negative()) << format.sign_bit;
    let decimal_order = value.digits as i32 - value.scale - 1;
    if decimal_order > format.decimal_max {
        return Err(overflow());
    }
    if decimal_order < format.decimal_min {
        return Ok(sign);
    }
    let (numerator, denominator) = rational(&value)?;
    let mut exponent = exponent(&numerator, &denominator)?;
    if exponent > format.maximum {
        return Err(overflow());
    }
    let quantum = exponent.max(format.minimum) - format.fraction as i32;
    let mut significand = rounded_significand(&numerator, &denominator, quantum)?;
    if exponent < format.minimum {
        // q可进位为最小正常数，其IEEE编码恰好等于1<<fraction。
        return Ok(sign | significand);
    }
    if significand == (1_u64 << (format.fraction + 1)) {
        significand >>= 1;
        exponent += 1;
        if exponent > format.maximum {
            return Err(overflow());
        }
    }
    let biased = (exponent + format.bias) as u64;
    Ok(sign | (biased << format.fraction) | (significand - (1_u64 << format.fraction)))
}
