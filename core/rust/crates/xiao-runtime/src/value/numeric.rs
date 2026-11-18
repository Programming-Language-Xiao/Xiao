//! B2-A共用的临时大数恢复与确定十进制输出；所有临时数据在调用内释放。

use bigdecimal::BigDecimal;
use num_bigint::BigInt;
use std::collections::BTreeMap;
use xiao_types::numeric_text::{
    NUMERIC_TEXT_LIMIT, NumericTextError, NumericTextKind, NumericTextView, validate_numeric_text,
};

use crate::errors::{ALLOCATION_CODE, RuntimeError, RuntimeErrorKind, RuntimeResult};

/// 把共享接纳结果映射到F-12的既有或已冻结错误身份。
pub(crate) fn validated_text(
    text: &str,
    kind: NumericTextKind,
) -> RuntimeResult<NumericTextView<'_>> {
    validate_numeric_text(text, kind).map_err(|error| match error {
        NumericTextError::InvalidSyntax => {
            RuntimeError::invalid_value("数值文本不符合ASCII十进制语法")
        }
        NumericTextError::LimitExceeded => {
            RuntimeError::numeric_limit("数值文本、系数、指数或scale超过预算")
        }
    })
}

/// 已接纳系数的直接文本，去前导零但保留显示尾零；不经二进制大整数往返。
fn coefficient_text(view: NumericTextView<'_>) -> RuntimeResult<String> {
    let mut result = String::new();
    result
        .try_reserve_exact(view.digit_count() + usize::from(view.is_negative()))
        .map_err(|_| allocation_error())?;
    if view.is_negative() {
        result.push('-');
    }
    result.extend(view.coefficient_digits().map(char::from));
    Ok(result)
}

/// 按均衡块构造十进制整数，避免逐块乘小基数的二次前缀扫描。
///
/// 幂表只存在于本次恢复中，结束即释放；最大因子只有半个输入长度加一位，
/// 已接纳输入至多D位，故所有因子和合并值都小于W，不建立跨调用缓存。
fn balanced_coefficient(bytes: &[u8], powers: &mut BTreeMap<usize, BigInt>) -> BigInt {
    if bytes.len() <= 128 {
        return BigInt::parse_bytes(bytes, 10).expect("已接纳系数只含ASCII数字");
    }
    let middle = bytes.len() / 2;
    let left = balanced_coefficient(&bytes[..middle], powers);
    let right = balanced_coefficient(&bytes[middle..], powers);
    let width = bytes.len() - middle;
    let factor = powers
        .entry(width)
        .or_insert_with(|| BigInt::from(10).pow(width as u32));
    left * &*factor + right
}

/// 仅算术或数值转换确需大数时恢复系数，显示与文本构造保持线性。
fn coefficient(view: NumericTextView<'_>) -> RuntimeResult<BigInt> {
    let text = coefficient_text(view)?;
    let digits = text.strip_prefix('-').unwrap_or(&text);
    let magnitude = balanced_coefficient(digits.as_bytes(), &mut BTreeMap::new());
    Ok(if view.is_negative() {
        -magnitude
    } else {
        magnitude
    })
}

/// 恢复临时大整数，不把对象或缓存保存在Runtime值中。
pub(crate) fn parse_integer(text: &str) -> RuntimeResult<BigInt> {
    coefficient(validated_text(text, NumericTextKind::Integer)?)
}

/// 按明确的整数系数与scale构造临时BigDecimal，不使用默认上下文或精度。
pub(crate) fn parse_decimal(text: &str) -> RuntimeResult<BigDecimal> {
    let view = validated_text(text, NumericTextKind::Decimal)?;
    Ok(BigDecimal::new(
        coefficient(view)?,
        i64::from(view.display_scale()),
    ))
}

/// 规范整数符号、前导零与零；不开放str到lint的语言数字解析。
pub(crate) fn integer_text(text: &str) -> RuntimeResult<String> {
    coefficient_text(validated_text(text, NumericTextKind::Integer)?)
}

/// 按F-10渲染显示系数与scale；不调用BigDecimal的默认Display。
pub(crate) fn decimal_text(text: &str) -> RuntimeResult<String> {
    let view = validated_text(text, NumericTextKind::Decimal)?;
    let scale = i64::from(view.display_scale());
    let signed = coefficient_text(view)?;
    let negative = signed.starts_with('-');
    let digits = signed.strip_prefix('-').unwrap_or(&signed);
    let count = digits.len();
    let sign_bytes = usize::from(negative);
    let plain_len = if scale < 0 {
        None
    } else {
        let scale = scale as usize;
        Some(
            sign_bytes
                + if scale == 0 {
                    count
                } else if scale < count {
                    count + 1
                } else {
                    scale + 2
                },
        )
    };
    let mut result = String::new();
    if let Some(length) = plain_len.filter(|&length| length <= NUMERIC_TEXT_LIMIT) {
        result
            .try_reserve_exact(length)
            .map_err(|_| allocation_error())?;
        if negative {
            result.push('-');
        }
        let scale = scale as usize;
        if scale == 0 {
            result.push_str(digits);
        } else if scale < count {
            result.push_str(&digits[..count - scale]);
            result.push('.');
            result.push_str(&digits[count - scale..]);
        } else {
            result.push_str("0.");
            result.extend(std::iter::repeat_n('0', scale - count));
            result.push_str(digits);
        }
    } else {
        let exponent = (-scale).to_string();
        result
            .try_reserve_exact(signed.len() + 1 + exponent.len())
            .map_err(|_| allocation_error())?;
        result.push_str(&signed);
        result.push('e');
        result.push_str(&exponent);
    }
    Ok(result)
}

/// 受控文本缓冲区失败沿现有ResourceError/011，不冒充数值预算033。
pub(crate) fn allocation_error() -> RuntimeError {
    RuntimeError::new(
        RuntimeErrorKind::Resource,
        ALLOCATION_CODE,
        "runtime.allocation",
        "无法分配数值文本缓冲区",
    )
}

#[cfg(test)]
#[path = "numeric_tests.rs"]
mod tests;
