//! 算子私有的IEEE位模式精确提升；不开放A4公开转换入口。

use super::common::{self, Number};
use crate::errors::{RuntimeError, RuntimeResult};
use crate::value::RuntimeValue;
use num_bigint::BigInt;

/// 在两侧输入预算完成后、算子域检查前统一拒绝非有限值。
pub(super) fn check_finite(value: &RuntimeValue) -> RuntimeResult<()> {
    let finite = match value {
        RuntimeValue::Float(value) => value.is_finite(),
        RuntimeValue::Sfloat(value) => value.is_finite(),
        _ => true,
    };
    if finite {
        Ok(())
    } else {
        Err(RuntimeError::numeric_overflow(
            "高精度混合运算不接受非有限输入",
        ))
    }
}

/// binary64只拆符号/指数/有效位，不经显示文本或浮点近似运算。
pub(super) fn f64_number(value: f64) -> RuntimeResult<Number> {
    let bits = value.to_bits();
    let exponent = ((bits >> 52) & 0x7ff) as i32;
    if exponent == 0x7ff {
        return Err(RuntimeError::numeric_overflow(
            "非有限float不能提升为lfloat",
        ));
    }
    let fraction = bits & ((1_u64 << 52) - 1);
    let (mantissa, exponent) = if exponent == 0 {
        (fraction, -1074)
    } else {
        (fraction | (1_u64 << 52), exponent - 1023 - 52)
    };
    exact(bits >> 63 != 0, mantissa, exponent)
}

/// binary32独立拆位，不把最短显示文本当作原始数值。
pub(super) fn f32_number(value: f32) -> RuntimeResult<Number> {
    let bits = value.to_bits();
    let exponent = ((bits >> 23) & 0xff) as i32;
    if exponent == 0xff {
        return Err(RuntimeError::numeric_overflow(
            "非有限sfloat不能提升为lfloat",
        ));
    }
    let fraction = u64::from(bits & ((1_u32 << 23) - 1));
    let (mantissa, exponent) = if exponent == 0 {
        (fraction, -149)
    } else {
        (fraction | (1_u64 << 23), exponent - 127 - 23)
    };
    exact(bits >> 31 != 0, mantissa, exponent)
}

/// m*2^e精确改写为c*10^-s；整数结果scale0，二进制正负零统一0。
fn exact(negative: bool, mut mantissa: u64, mut exponent: i32) -> RuntimeResult<Number> {
    if mantissa == 0 {
        return Ok(Number {
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
        // 最大binary64只有309位；保守二进制位数上界仍远低于W。
        common::check_work("float-lift-integer", 20 + exponent as usize)?;
        (
            common::checked_value(
                "float-lift-integer",
                BigInt::from(mantissa) << exponent as u32,
            )?,
            0,
        )
    } else {
        let scale = -exponent;
        common::check_work("float-lift-factor", scale as usize + 1)?;
        let factor = common::checked_value("float-lift-factor", BigInt::from(5).pow(scale as u32))?;
        (
            common::multiply(&BigInt::from(mantissa), &factor, "float-lift-coefficient")?,
            scale,
        )
    };
    if negative {
        coefficient = -coefficient;
    }
    let digits = common::decimal_digits(&coefficient);
    Ok(Number {
        coefficient,
        scale,
        digits,
    })
}
