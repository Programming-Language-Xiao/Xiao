#![allow(dead_code)]

//! B2-B私有cmath底座。
//!
//! 这里只提供经过边界检查的纯Rust f64内核，不注册intrinsic、不暴露C ABI，
//! 也不切换现有 float `**` 调用点。B3会在冻结的全链路批次一次性接线。

use crate::errors::{RuntimeError, RuntimeResult};

/// B2-B规划中的15项函数名称，供私有测试和后续接线共享。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum Function {
    Sqrt,
    Floor,
    Ceil,
    Round,
    Trunc,
    Abs,
    Min,
    Max,
    Pow,
    Sin,
    Cos,
    Tan,
    Exp,
    Ln,
    Log10,
}

/// 域错误、非有限输入和非有限结果沿已冻结009边界。
fn domain(function: Function, message: &'static str) -> RuntimeError {
    RuntimeError::numeric_overflow(format!("cmath::{function:?}: {message}"))
}

/// 先拒绝所有非有限输入；不能让libm的NaN传播绕过语言错误身份。
fn finite(function: Function, value: f64) -> RuntimeResult<f64> {
    if value.is_finite() {
        Ok(value)
    } else {
        Err(domain(function, "输入必须是有限值"))
    }
}

/// 检查有限结果，允许正常子正规数与有符号零。
fn result(function: Function, value: f64) -> RuntimeResult<f64> {
    if value.is_finite() {
        Ok(value)
    } else {
        Err(domain(function, "结果不是有限值"))
    }
}

/// 私有sqrt边界，负零由libm保留。
pub(super) fn sqrt(value: f64) -> RuntimeResult<f64> {
    let value = finite(Function::Sqrt, value)?;
    if value < 0.0 {
        return Err(domain(Function::Sqrt, "定义域要求x>=0"));
    }
    result(Function::Sqrt, libm::sqrt(value))
}

/// 私有floor边界。
pub(super) fn floor(value: f64) -> RuntimeResult<f64> {
    let value = finite(Function::Floor, value)?;
    result(Function::Floor, libm::floor(value))
}

/// 私有ceil边界。
pub(super) fn ceil(value: f64) -> RuntimeResult<f64> {
    let value = finite(Function::Ceil, value)?;
    result(Function::Ceil, libm::ceil(value))
}

/// 最近偶数舍入；不使用宿主默认round的远离零规则。
pub(super) fn round(value: f64) -> RuntimeResult<f64> {
    let value = finite(Function::Round, value)?;
    result(Function::Round, libm::roundeven(value))
}

/// 向零截断。
pub(super) fn trunc(value: f64) -> RuntimeResult<f64> {
    let value = finite(Function::Trunc, value)?;
    result(Function::Trunc, libm::trunc(value))
}

/// 逐位绝对值；负零按IEEE绝对值归为正零。
pub(super) fn abs(value: f64) -> RuntimeResult<f64> {
    let value = finite(Function::Abs, value)?;
    result(Function::Abs, libm::fabs(value))
}

/// 异号零平局取-0；同号零保持输入顺序无关的零符号。
pub(super) fn min(left: f64, right: f64) -> RuntimeResult<f64> {
    let left = finite(Function::Min, left)?;
    let right = finite(Function::Min, right)?;
    if left == right {
        if left == 0.0 && right == 0.0 {
            return Ok(if left.is_sign_negative() || right.is_sign_negative() {
                -0.0
            } else {
                0.0
            });
        }
        return Ok(left);
    }
    Ok(left.min(right))
}

/// 异号零平局取+0；同号零保留零。
pub(super) fn max(left: f64, right: f64) -> RuntimeResult<f64> {
    let left = finite(Function::Max, left)?;
    let right = finite(Function::Max, right)?;
    if left == right {
        if left == 0.0 && right == 0.0 {
            return Ok(if left.is_sign_positive() || right.is_sign_positive() {
                0.0
            } else {
                -0.0
            });
        }
        return Ok(left);
    }
    Ok(left.max(right))
}

/// 共享pow域边界；B3将float **和cmath.pow接到同一函数。
pub(super) fn pow(base: f64, exponent: f64) -> RuntimeResult<f64> {
    let base = finite(Function::Pow, base)?;
    let exponent = finite(Function::Pow, exponent)?;
    if exponent == 0.0 {
        return Ok(1.0);
    }
    if base == 0.0 && exponent < 0.0 {
        return Err(domain(Function::Pow, "零不能取负幂"));
    }
    if base < 0.0 && exponent.fract() != 0.0 {
        return Err(domain(Function::Pow, "负底数要求整数指数"));
    }
    if base == 0.0 && exponent.fract() == 0.0 && exponent % 2.0 != 0.0 && base.is_sign_negative() {
        return Ok(-0.0);
    }
    result(Function::Pow, libm::pow(base, exponent))
}

/// 普通超越函数统一有限输入/输出边界。
pub(super) fn sin(value: f64) -> RuntimeResult<f64> {
    result(Function::Sin, libm::sin(finite(Function::Sin, value)?))
}

/// 普通超越函数统一有限输入/输出边界。
pub(super) fn cos(value: f64) -> RuntimeResult<f64> {
    result(Function::Cos, libm::cos(finite(Function::Cos, value)?))
}

/// tan不使用epsilon判极点；只有实际非有限结果才报009。
pub(super) fn tan(value: f64) -> RuntimeResult<f64> {
    result(Function::Tan, libm::tan(finite(Function::Tan, value)?))
}

/// exp在正常IEEE下溢时允许有符号零，溢出结果报009。
pub(super) fn exp(value: f64) -> RuntimeResult<f64> {
    result(Function::Exp, libm::exp(finite(Function::Exp, value)?))
}

/// ln的定义域为x>0。
pub(super) fn ln(value: f64) -> RuntimeResult<f64> {
    let value = finite(Function::Ln, value)?;
    if value <= 0.0 {
        return Err(domain(Function::Ln, "定义域要求x>0"));
    }
    result(Function::Ln, libm::log(value))
}

/// log10的定义域为x>0。
pub(super) fn log10(value: f64) -> RuntimeResult<f64> {
    let value = finite(Function::Log10, value)?;
    if value <= 0.0 {
        return Err(domain(Function::Log10, "定义域要求x>0"));
    }
    result(Function::Log10, libm::log10(value))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_fifteen_functions_have_private_finite_boundaries() {
        assert_eq!(sqrt(4.0).unwrap().to_bits(), 2.0_f64.to_bits());
        assert_eq!(floor(1.75).unwrap(), 1.0);
        assert_eq!(ceil(1.25).unwrap(), 2.0);
        assert_eq!(round(2.5).unwrap(), 2.0);
        assert_eq!(trunc(-1.75).unwrap(), -1.0);
        assert_eq!(abs(-2.0).unwrap(), 2.0);
        assert_eq!(min(-0.0, 0.0).unwrap().to_bits(), (-0.0_f64).to_bits());
        assert_eq!(max(-0.0, 0.0).unwrap().to_bits(), 0.0_f64.to_bits());
        assert_eq!(min(-2.0, -2.0).unwrap(), -2.0);
        assert_eq!(max(2.0, 2.0).unwrap(), 2.0);
        assert_eq!(pow(2.0, 3.0).unwrap(), 8.0);
        assert!((sin(0.5).unwrap() - 0.4794255386).abs() < 1e-10);
        assert!((cos(0.5).unwrap() - 0.8775825619).abs() < 1e-10);
        assert!((tan(0.5).unwrap() - 0.5463024898).abs() < 1e-10);
        assert!((exp(1.0).unwrap() - std::f64::consts::E).abs() < 1e-10);
        assert!((ln(2.0).unwrap() - std::f64::consts::LN_2).abs() < 1e-10);
        assert!((log10(100.0).unwrap() - 2.0).abs() < 1e-10);
    }

    #[test]
    fn independent_reference_vectors_stay_within_four_ulp() {
        fn ordered(value: f64) -> i64 {
            let bits = value.to_bits() as i64;
            if bits < 0 { i64::MIN - bits } else { bits }
        }
        fn ulp(left: f64, right: f64) -> u64 {
            ordered(left).abs_diff(ordered(right))
        }
        let vectors = [
            (Function::Sin, sin(0.5).unwrap(), 0.479425538604203_f64),
            (Function::Cos, cos(0.5).unwrap(), 0.8775825618903728_f64),
            (Function::Tan, tan(0.5).unwrap(), 0.5463024898437905_f64),
            (Function::Exp, exp(1.0).unwrap(), std::f64::consts::E),
            (Function::Ln, ln(2.0).unwrap(), std::f64::consts::LN_2),
            (Function::Log10, log10(100.0).unwrap(), 2.0_f64),
        ];
        for (function, actual, reference) in vectors {
            assert!(
                ulp(actual, reference) <= 4,
                "{function:?}: {actual:?} vs {reference:?}"
            );
        }
    }

    #[test]
    fn all_fifteen_functions_reject_nonfinite_inputs() {
        let results = [
            sqrt(f64::NAN),
            floor(f64::NAN),
            ceil(f64::NAN),
            round(f64::NAN),
            trunc(f64::NAN),
            abs(f64::NAN),
            min(f64::NAN, 0.0),
            max(0.0, f64::INFINITY),
            pow(1.0, f64::NAN),
            sin(f64::NAN),
            cos(f64::INFINITY),
            tan(f64::NAN),
            exp(f64::NAN),
            ln(f64::NAN),
            log10(f64::INFINITY),
        ];
        assert!(
            results
                .into_iter()
                .all(|result| result.unwrap_err().code() == "X06-RUNTIME-009")
        );
    }

    #[test]
    fn domains_nonfinite_and_signed_zero_follow_frozen_boundary() {
        for result in [
            sqrt(-1.0),
            ln(0.0),
            log10(-1.0),
            pow(-1.0, 0.5),
            pow(0.0, -1.0),
        ] {
            assert_eq!(result.unwrap_err().code(), "X06-RUNTIME-009");
        }
        for result in [sqrt(f64::NAN), sin(f64::INFINITY), exp(f64::NAN)] {
            assert_eq!(result.unwrap_err().code(), "X06-RUNTIME-009");
        }
        assert_eq!(sqrt(-0.0).unwrap().to_bits(), (-0.0_f64).to_bits());
        assert_eq!(round(-0.5).unwrap().to_bits(), (-0.0_f64).to_bits());
        assert_eq!(floor(-0.0).unwrap().to_bits(), (-0.0_f64).to_bits());
        assert_eq!(ceil(-0.0).unwrap().to_bits(), (-0.0_f64).to_bits());
        assert_eq!(trunc(-0.0).unwrap().to_bits(), (-0.0_f64).to_bits());
        assert_eq!(sin(-0.0).unwrap().to_bits(), (-0.0_f64).to_bits());
        assert_eq!(tan(-0.0).unwrap().to_bits(), (-0.0_f64).to_bits());
        assert_eq!(pow(-0.0, 3.0).unwrap().to_bits(), (-0.0_f64).to_bits());
    }

    #[test]
    fn pow_reference_vector_stays_within_four_ulp() {
        let actual = pow(2.0, 0.5).unwrap();
        let reference = std::f64::consts::SQRT_2;
        let ordered = |value: f64| {
            let bits = value.to_bits() as i64;
            if bits < 0 { i64::MIN - bits } else { bits }
        };
        assert!(ordered(actual).abs_diff(ordered(reference)) <= 4);
    }
}
