//! 算子私有的精确系数、预算与结果封装；不改变公开转换矩阵。

use crate::errors::{RuntimeError, RuntimeResult};
use crate::value::{RuntimeValue, numeric};
use bigdecimal::Zero;
use num_bigint::BigInt;
use xiao_types::numeric_text::{
    NUMERIC_DIGIT_LIMIT as D, NUMERIC_SCALE_LIMIT as H, NUMERIC_WORK_DIGIT_LIMIT as W,
    NumericTextKind,
};

/// A2/A3结果族，保持整数结果为lint，十进制结果为lfloat。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum Family {
    Integer,
    Decimal,
}

/// 一次调用内的临时十进制系数与显示scale，不跨调用缓存。
#[derive(Clone, Debug)]
pub(super) struct Number {
    /// 去前导零、保留显示尾零的有符号整数系数。
    pub coefficient: BigInt,
    /// 显示scale，数值为coefficient乘10的负scale次方。
    pub scale: i32,
    /// 非零绝对系数的十进制位数；零计一位。
    pub digits: usize,
}

impl Number {
    /// 恢复已完成接纳的整数族，不经过i128或f64。
    pub fn integer(value: &RuntimeValue) -> RuntimeResult<Self> {
        let coefficient = match value {
            RuntimeValue::Lint(text) => numeric::parse_integer(text)?,
            RuntimeValue::Int(value) => BigInt::from(*value),
            RuntimeValue::Sint(value) => BigInt::from(*value),
            _ => return Err(RuntimeError::type_mismatch("integer", value.type_name())),
        };
        let digits = match value {
            RuntimeValue::Lint(text) => {
                numeric::validated_text(text, NumericTextKind::Integer)?.digit_count()
            }
            _ => decimal_digits(&coefficient),
        };
        Ok(Self {
            coefficient,
            scale: 0,
            digits,
        })
    }

    /// 数学零统一为一位系数和scale0。
    pub fn is_zero(&self) -> bool {
        self.coefficient.is_zero()
    }

    /// 恢复lfloat显示系数和scale，或精确提升算子内部的定宽数值。
    pub fn decimal(value: &RuntimeValue) -> RuntimeResult<Self> {
        match value {
            RuntimeValue::Lfloat(text) => {
                let parsed = numeric::parse_decimal(text)?;
                let (coefficient, scale) = parsed.into_bigint_and_scale();
                let digits = numeric::validated_text(text, NumericTextKind::Decimal)?.digit_count();
                Ok(Self {
                    coefficient,
                    scale: scale as i32,
                    digits,
                })
            }
            RuntimeValue::Float(value) => super::float_lift::f64_number(*value),
            RuntimeValue::Sfloat(value) => super::float_lift::f32_number(*value),
            _ => Self::integer(value),
        }
    }

    /// 规范scale仅去系数尾零，用于幂的构造前结果scale检查。
    pub fn canonical_scale(&self) -> i32 {
        if self.is_zero() {
            return 0;
        }
        let text = self.coefficient.to_str_radix(10);
        self.scale - text.bytes().rev().take_while(|byte| *byte == b'0').count() as i32
    }
}

/// 入口类型已检查后，按实参顺序完成文本预算和完整语法。
pub(super) fn validate_input(value: &RuntimeValue) -> RuntimeResult<()> {
    match value {
        RuntimeValue::Lint(text) => {
            numeric::validated_text(text, NumericTextKind::Integer).map(|_| ())
        }
        RuntimeValue::Lfloat(text) => {
            numeric::validated_text(text, NumericTextKind::Decimal).map(|_| ())
        }
        _ => Ok(()),
    }
}

/// 非零整数绝对值位数，零也计一位。
pub(super) fn decimal_digits(value: &BigInt) -> usize {
    value.to_str_radix(10).trim_start_matches('-').len()
}

/// 预算失败只使用已冻结033，不混同实际分配011。
pub(super) fn limit(reason: &str) -> RuntimeError {
    RuntimeError::numeric_limit(reason)
}

/// 构造前检查可证明的整数工作位数上界。
pub(super) fn check_work(label: &'static str, upper: usize) -> RuntimeResult<()> {
    observe(label, upper, None);
    if upper > W {
        Err(limit("整数工作值超过65536位"))
    } else {
        Ok(())
    }
}

/// 记录并检查实际工作位数，调用点仍必须先有构造前上界。
pub(super) fn checked_value(label: &'static str, value: BigInt) -> RuntimeResult<BigInt> {
    let actual = decimal_digits(&value);
    checked_digits(label, actual)?;
    Ok(value)
}

/// 已构造系数的精确位数；十进制字节系数与二进制大整数共用同一W门槛。
pub(super) fn checked_digits(label: &'static str, actual: usize) -> RuntimeResult<()> {
    observe(label, actual, Some(actual));
    if actual > W {
        Err(limit("整数工作值超过65536位"))
    } else {
        Ok(())
    }
}

/// 有界整数乘法；平方求幂等路径都使用同一工作预算。
pub(super) fn multiply(
    left: &BigInt,
    right: &BigInt,
    label: &'static str,
) -> RuntimeResult<BigInt> {
    check_work(label, decimal_digits(left) + decimal_digits(right))?;
    checked_value(label, left * right)
}

/// 有界10的幂，调用前先核对最终整数长度，禁止先展开巨型指数。
pub(super) fn power_ten(exponent: u32) -> RuntimeResult<BigInt> {
    let digits = exponent as usize + 1;
    check_work("power-ten", digits)?;
    let value = BigInt::from(10).pow(exponent);
    // 10^k恰有k+1位；直接使用精确构造式，避免为计位再做基数转换。
    observe("power-ten", digits, Some(digits));
    Ok(value)
}

/// 已知补零长度下的精确对齐，零不扩张。
pub(super) fn align(value: &BigInt, digits: usize, zeros: u32) -> RuntimeResult<BigInt> {
    if value.is_zero() {
        return Ok(BigInt::from(0));
    }
    let exact_digits = digits + zeros as usize;
    check_work("alignment", exact_digits)?;
    if zeros == 0 {
        return Ok(value.clone());
    }
    let factor = power_ten(zeros)?;
    // 非零c乘10^k恰好增加k位，不能用d(c)+d(factor)松界抢拒边界。
    let aligned = value * factor;
    // 非零系数补k个十进制零的实际长度就是d+k，符号不参与位数。
    observe("alignment", exact_digits, Some(exact_digits));
    Ok(aligned)
}

/// 一次整数长除后用有界乘减恢复余数，不重复长除法。
pub(super) fn div_rem(numerator: &BigInt, denominator: &BigInt) -> RuntimeResult<(BigInt, BigInt)> {
    let bound = decimal_digits(numerator);
    check_work("div-rem-input", bound.max(decimal_digits(denominator)))?;
    let quotient = checked_value("quotient", numerator / denominator)?;
    // |q*b|<=|a|，这一精确上界避免不必要的D+1拒绝。
    check_work("division-product", bound)?;
    let product = checked_value("division-product", &quotient * denominator)?;
    check_work("division-remainder", bound)?;
    let remainder = checked_value("division-remainder", numerator - product)?;
    Ok((quotient, remainder))
}

/// 仅在数值结果实际可表示后输出，显示尾零不能为了预算而删除。
pub(super) fn finish(
    coefficient: BigInt,
    scale: i32,
    family: Family,
) -> RuntimeResult<RuntimeValue> {
    if coefficient.is_zero() {
        return finish_digits(&[], false, 0, family);
    }
    let text = coefficient.to_str_radix(10);
    finish_digits(
        text.trim_start_matches('-').as_bytes(),
        text.starts_with('-'),
        scale,
        family,
    )
}

/// 规范十进制系数直接输出；不经基数往返，D/H与零例外和大数路径一致。
pub(super) fn finish_digits(
    coefficient: &[u8],
    negative: bool,
    scale: i32,
    family: Family,
) -> RuntimeResult<RuntimeValue> {
    if coefficient.is_empty() || coefficient == b"0" {
        return match family {
            Family::Integer => RuntimeValue::new_lint("0"),
            Family::Decimal => RuntimeValue::new_lfloat("0"),
        };
    }
    let digits = coefficient.len();
    observe("final-coefficient", digits, Some(digits));
    if digits > D {
        return Err(limit("最终显示系数超过32768位"));
    }
    let trailing = coefficient
        .iter()
        .rev()
        .take_while(|byte| **byte == b'0')
        .count() as i64;
    check_scales(i64::from(scale), i64::from(scale) - trailing)?;
    let mut text = String::new();
    text.try_reserve_exact(digits + usize::from(negative))
        .map_err(|_| numeric::allocation_error())?;
    if negative {
        text.push('-');
    }
    text.extend(coefficient.iter().copied().map(char::from));
    match family {
        Family::Integer => RuntimeValue::new_lint(&text),
        Family::Decimal => {
            let exponent = (-i64::from(scale)).to_string();
            let mut encoded = String::new();
            encoded
                .try_reserve_exact(text.len() + exponent.len() + 1)
                .map_err(|_| numeric::allocation_error())?;
            encoded.push_str(&text);
            encoded.push('e');
            encoded.push_str(&exponent);
            RuntimeValue::new_lfloat(&encoded)
        }
    }
}

/// 已知非零结果的显示/规范scale同时接纳。
pub(super) fn check_scales(display: i64, canonical: i64) -> RuntimeResult<()> {
    if display.abs() > i64::from(H) || canonical.abs() > i64::from(H) {
        Err(limit("结果显示或规范scale超过100000"))
    } else {
        Ok(())
    }
}

/// 去显示系数尾零用于除法规范结果；不用于偷偷缩减最终显示精度。
pub(super) fn normalize(coefficient: BigInt, scale: i32) -> RuntimeResult<Number> {
    if coefficient.is_zero() {
        return Ok(Number {
            coefficient,
            scale: 0,
            digits: 1,
        });
    }
    let text = coefficient.to_str_radix(10);
    let zeros = text.bytes().rev().take_while(|byte| *byte == b'0').count();
    let coefficient = if zeros == 0 {
        coefficient
    } else {
        coefficient / power_ten(zeros as u32)?
    };
    let digits = decimal_digits(&coefficient);
    Ok(Number {
        coefficient,
        scale: scale - zeros as i32,
        digits,
    })
}

/// 单元测试旁观工作上界与实际位数；生产路径不保存轨迹或共享缓存。
fn observe(label: &'static str, upper: usize, actual: Option<usize>) {
    #[cfg(test)]
    TRACE.with(|trace| {
        if let Some(events) = trace.borrow_mut().as_mut() {
            events.push(WorkEvent {
                label,
                upper,
                actual,
            });
        }
    });
    #[cfg(not(test))]
    let _ = (label, upper, actual);
}

#[cfg(test)]
#[derive(Clone, Debug)]
pub(super) struct WorkEvent {
    /// 产生此预算判定或实际工作值的算法步骤。
    pub label: &'static str,
    /// 构造前上界；实际事件中为已计算的实际位数。
    pub upper: usize,
    /// 仅实际构造后的事件携带位数，预检事件为None。
    pub actual: Option<usize>,
}

#[cfg(test)]
thread_local! { static TRACE: std::cell::RefCell<Option<Vec<WorkEvent>>> = const { std::cell::RefCell::new(None) }; }

#[cfg(test)]
pub(super) fn trace_start() {
    TRACE.with(|trace| *trace.borrow_mut() = Some(Vec::new()));
}

#[cfg(test)]
pub(super) fn trace_finish() -> Vec<WorkEvent> {
    TRACE.with(|trace| trace.borrow_mut().take().unwrap_or_default())
}
