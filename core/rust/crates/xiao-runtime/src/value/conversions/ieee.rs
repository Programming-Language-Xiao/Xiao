//! A4公开转换的精确IEEE拆位；A3私有提升实现保持原样。

use super::{budget, decimal::Decimal};
use crate::errors::{RuntimeError, RuntimeResult};
use crate::value::RuntimeValue;
use num_bigint::BigInt;

/// 新增浮点转换在构造前拒绝非有限来源。
pub(super) fn check_finite(value: &RuntimeValue) -> RuntimeResult<()> {
    match value {
        RuntimeValue::Float(value) if !value.is_finite() => Err(non_finite()),
        RuntimeValue::Sfloat(value) if !value.is_finite() => Err(non_finite()),
        _ => Ok(()),
    }
}

/// 非有限来源复用冻结009，不伪装成文本语法012。
fn non_finite() -> RuntimeError {
    RuntimeError::numeric_overflow("非有限二进制值不能用于此转换")
}

/// binary64按符号、有效位、二进制指数精确恢复。
pub(super) fn from_f64(value: f64) -> RuntimeResult<Decimal> {
    let bits = value.to_bits();
    let exponent = ((bits >> 52) & 0x7ff) as i32;
    if exponent == 0x7ff {
        return Err(non_finite());
    }
    let fraction = bits & ((1_u64 << 52) - 1);
    let (mantissa, exponent) = if exponent == 0 {
        (fraction, -1074)
    } else {
        (fraction | (1_u64 << 52), exponent - 1023 - 52)
    };
    exact(bits >> 63 != 0, mantissa, exponent)
}

/// binary32直接拆位，不通过binary64的文本表示。
pub(super) fn from_f32(value: f32) -> RuntimeResult<Decimal> {
    let bits = value.to_bits();
    let exponent = ((bits >> 23) & 0xff) as i32;
    if exponent == 0xff {
        return Err(non_finite());
    }
    let fraction = u64::from(bits & ((1_u32 << 23) - 1));
    let (mantissa, exponent) = if exponent == 0 {
        (fraction, -149)
    } else {
        (fraction | (1_u64 << 23), exponent - 127 - 23)
    };
    exact(bits >> 31 != 0, mantissa, exponent)
}

/// m*2^e转换为c*10^-s，整数结果scale0，非整数精确规范化，±0归0。
fn exact(negative: bool, mut mantissa: u64, mut exponent: i32) -> RuntimeResult<Decimal> {
    if mantissa == 0 {
        return Ok(Decimal {
            coefficient: BigInt::from(0),
            scale: 0,
            digits: 1,
        });
    }
    if exponent < 0 {
        let twos = mantissa.trailing_zeros().min((-exponent) as u32);
        mantissa >>= twos;
        exponent += twos as i32;
    }
    let (mut coefficient, scale) = if exponent >= 0 {
        budget::check("ieee-integer", 20 + (exponent as usize).div_ceil(3))?;
        (
            budget::checked("ieee-integer", BigInt::from(mantissa) << exponent as u32)?,
            0,
        )
    } else {
        let scale = -exponent;
        budget::check("ieee-five-factor", scale as usize + 1)?;
        let factor = budget::checked("ieee-five-factor", BigInt::from(5).pow(scale as u32))?;
        budget::check("ieee-coefficient", 20 + scale as usize)?;
        (
            budget::checked("ieee-coefficient", BigInt::from(mantissa) * factor)?,
            scale,
        )
    };
    if negative {
        coefficient = -coefficient;
    }
    let digits = budget::digits(&coefficient);
    Ok(Decimal {
        coefficient,
        scale,
        digits,
    })
}
