//! A4系数/移位预算和受控结果封装；每个工作值先验上界再构造。

use crate::errors::{RuntimeError, RuntimeResult};
use crate::value::{RuntimeValue, numeric};
use bigdecimal::Zero;
use num_bigint::{BigInt, BigUint};
use xiao_types::numeric_text::{
    NUMERIC_DIGIT_LIMIT as D, NUMERIC_SCALE_LIMIT as H, NUMERIC_WORK_DIGIT_LIMIT as W,
};

/// A4工作或最终预算失败使用冻结033。
pub(super) fn limit(reason: &str) -> RuntimeError {
    RuntimeError::numeric_limit(reason)
}

/// 构造前检查已知十进制位数或可证明上界。
pub(super) fn check(label: &'static str, upper: usize) -> RuntimeResult<()> {
    observe(label, upper, None);
    if upper > W {
        Err(limit("转换整数工作值超过65536位"))
    } else {
        Ok(())
    }
}

/// 非零系数的十进制位数；零计一位。
pub(super) fn digits(value: &BigInt) -> usize {
    value.magnitude().to_str_radix(10).len()
}

/// 物化后记录实际位数，前置上界仍由相应构造点负责。
pub(super) fn checked(label: &'static str, value: BigInt) -> RuntimeResult<BigInt> {
    let actual = digits(&value);
    observe(label, actual, Some(actual));
    if actual > W {
        Err(limit("转换整数工作值超过65536位"))
    } else {
        Ok(value)
    }
}

/// 精确10幂只在已知指数可接纳后构造。
pub(super) fn ten(exponent: u32) -> RuntimeResult<BigInt> {
    check("conversion-ten", exponent as usize + 1)?;
    checked("conversion-ten", BigInt::from(10).pow(exponent))
}

/// 2^k的十进制增长上界取ceil(k/3)，因为2^3<10；这里始终远低于W。
pub(super) fn shift(value: &BigUint, bits: u32, label: &'static str) -> RuntimeResult<BigUint> {
    check(
        label,
        value.to_str_radix(10).len() + bits.div_ceil(3) as usize,
    )?;
    unsigned(label, value << bits)
}

/// 正整数工作值的实际位数检查，不构造带符号副本。
pub(super) fn unsigned(label: &'static str, value: BigUint) -> RuntimeResult<BigUint> {
    let actual = value.to_str_radix(10).len();
    observe(label, actual, Some(actual));
    if actual > W {
        Err(limit("二进制转换工作值超过65536位"))
    } else {
        Ok(value)
    }
}

/// 实际最终整数必须满足D，不以松结果上界代替验收。
pub(super) fn into_lint(coefficient: BigInt) -> RuntimeResult<RuntimeValue> {
    let text = coefficient.to_str_radix(10);
    let length = text.trim_start_matches('-').len();
    observe("conversion-final-integer", length, Some(length));
    if length > D {
        return Err(limit("转换最终整数超过32768位"));
    }
    RuntimeValue::new_lint(&text)
}

/// 以精确系数和显示scale输出，不使用上游默认精度/Display。
pub(super) fn into_lfloat(coefficient: BigInt, scale: i32) -> RuntimeResult<RuntimeValue> {
    if coefficient.is_zero() {
        return RuntimeValue::new_lfloat("0");
    }
    if scale.unsigned_abs() > H as u32 {
        return Err(limit("转换显示scale超过100000"));
    }
    let text = coefficient.to_str_radix(10);
    if text.trim_start_matches('-').len() > D {
        return Err(limit("转换最终系数超过32768位"));
    }
    let exponent = (-scale).to_string();
    let mut encoded = String::new();
    encoded
        .try_reserve_exact(text.len() + exponent.len() + 1)
        .map_err(|_| numeric::allocation_error())?;
    encoded.push_str(&text);
    encoded.push('e');
    encoded.push_str(&exponent);
    RuntimeValue::new_lfloat(&encoded)
}

/// 测试旁观预算，不在生产路径缓存轨迹。
fn observe(label: &'static str, upper: usize, actual: Option<usize>) {
    #[cfg(test)]
    TRACE.with(|trace| {
        if let Some(rows) = trace.borrow_mut().as_mut() {
            rows.push(WorkEvent {
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
#[derive(Debug)]
pub(super) struct WorkEvent {
    /// 预算所在的转换步骤。
    pub label: &'static str,
    /// 构造前上界，实际事件中等于已构造位数。
    pub upper: usize,
    /// 构造后的实际十进制位数；预检事件为None。
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
