//! B2-A1 临时大数适配与确定十进制输出；不实现语言算术或转换矩阵。

use bigdecimal::BigDecimal;
use num_bigint::BigInt;
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

/// 在已完成语法与预算接纳后，精确构造一个临时整数系数。
fn coefficient(view: NumericTextView<'_>) -> RuntimeResult<BigInt> {
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(view.digit_count() + usize::from(view.is_negative()))
        .map_err(|_| allocation_error())?;
    if view.is_negative() {
        bytes.push(b'-');
    }
    bytes.extend(view.coefficient_digits());
    BigInt::parse_bytes(&bytes, 10)
        .ok_or_else(|| RuntimeError::invalid_value("已校验的整数系数无法恢复"))
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
    let integer = parse_integer(text)?;
    let digits = integer.to_str_radix(10);
    let mut result = String::new();
    result
        .try_reserve_exact(digits.len())
        .map_err(|_| allocation_error())?;
    result.push_str(&digits);
    Ok(result)
}

/// 按F-10渲染显示系数与scale；不调用BigDecimal的默认Display。
pub(crate) fn decimal_text(text: &str) -> RuntimeResult<String> {
    let decimal = parse_decimal(text)?;
    let (coefficient, scale) = decimal.as_bigint_and_scale();
    let signed = coefficient.to_str_radix(10);
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
fn allocation_error() -> RuntimeError {
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
