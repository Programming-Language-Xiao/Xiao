//! A4向零截断及定宽/大整数目标预算，先判断再补零或长除。

use super::{budget, decimal::Decimal};
use crate::errors::{RuntimeError, RuntimeResult};
use crate::value::{RuntimeValue, numeric};
use bigdecimal::{ToPrimitive, Zero};
use num_bigint::BigInt;
use xiao_syntax::ScalarType;
use xiao_types::numeric_text::{NUMERIC_DIGIT_LIMIT as D, NumericTextKind};

/// 定宽范围失败固定009；不因宿主i128解析失败而误报012。
fn range_error() -> RuntimeError {
    RuntimeError::numeric_overflow("截断后的数值超出目标整数范围")
}

/// 恢复并截断合法数值；输入预算先于结果范围/资源检查。
pub(super) fn convert(value: &RuntimeValue, target: ScalarType) -> RuntimeResult<RuntimeValue> {
    if target == ScalarType::Lint
        && let RuntimeValue::Lfloat(text) = value
    {
        return truncate_text(text);
    }
    let decimal = Decimal::from_value(value)?;
    let limit = match target {
        ScalarType::Int => 19,
        ScalarType::Sint => 10,
        ScalarType::Lint => D,
        _ => unreachable!("调用方仅指定整数目标"),
    };
    let coefficient = truncate(decimal, limit, target == ScalarType::Lint)?;
    match target {
        ScalarType::Int => coefficient
            .to_i64()
            .map(RuntimeValue::Int)
            .ok_or_else(range_error),
        ScalarType::Sint => coefficient
            .to_i32()
            .map(RuntimeValue::Sint)
            .ok_or_else(range_error),
        ScalarType::Lint => budget::into_lint(coefficient),
        _ => unreachable!("调用方仅指定整数目标"),
    }
}

/// 十进制文本向零截断直接取有效整数前缀或补零，不经二进制大整数往返。
///
/// 输入先完整接纳，输出位数d-s在分配前检查D；不足一个整数位时归一为0。
fn truncate_text(text: &str) -> RuntimeResult<RuntimeValue> {
    let view = numeric::validated_text(text, NumericTextKind::Decimal)?;
    let scale = view.display_scale();
    if view.is_zero() || scale >= view.digit_count() as i32 {
        return RuntimeValue::new_lint("0");
    }
    let length = (view.digit_count() as i64 - i64::from(scale)) as usize;
    if length > D {
        return Err(budget::limit("截断后lint超过32768位"));
    }
    let mut result = String::new();
    result
        .try_reserve_exact(length + usize::from(view.is_negative()))
        .map_err(|_| numeric::allocation_error())?;
    if view.is_negative() {
        result.push('-');
    }
    result.extend(view.coefficient_digits().take(length).map(char::from));
    if scale < 0 {
        result.extend(std::iter::repeat_n('0', (-scale) as usize));
    }
    RuntimeValue::new_lint(&result)
}

/// 显式向零截断，已知最终位数超限时不物化巨型整数。
fn truncate(value: Decimal, limit: usize, large_target: bool) -> RuntimeResult<BigInt> {
    if value.coefficient.is_zero() || value.scale >= value.digits as i32 {
        return Ok(BigInt::from(0));
    }
    let result_digits = (value.digits as i64 - i64::from(value.scale)) as usize;
    if result_digits > limit {
        return Err(if large_target {
            budget::limit("截断后lint超过32768位")
        } else {
            range_error()
        });
    }
    if value.scale == 0 {
        return Ok(value.coefficient);
    }
    if value.scale > 0 {
        let denominator = budget::ten(value.scale as u32)?;
        budget::check("truncate-division", value.digits)?;
        budget::checked("truncate-division", value.coefficient / denominator)
    } else {
        budget::check("truncate-padding", result_digits)?;
        let factor = budget::ten((-value.scale) as u32)?;
        budget::checked("truncate-padding", value.coefficient * factor)
    }
}
