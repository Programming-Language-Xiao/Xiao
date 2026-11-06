//! B2-A4显式转换内核；不改变定宽旧规则或A3算子私有提升。

mod binary;
mod budget;
mod decimal;
mod ieee;
mod integer;

use crate::errors::RuntimeResult;
use crate::value::RuntimeValue;
use crate::value::numeric;
use xiao_syntax::ScalarType;

/// 接管已冻结的高精度转换和浮点显示；其余组合仍交既有转换矩阵。
#[inline(always)]
pub(super) fn convert(
    value: &RuntimeValue,
    source: ScalarType,
    target: ScalarType,
) -> Option<RuntimeResult<RuntimeValue>> {
    use ScalarType::{Float, Lfloat, Lint, Sfloat, Str};
    if !matches!(source, Lint | Lfloat)
        && target != Lfloat
        && !(matches!(source, Float | Sfloat) && matches!(target, Lint | Str))
    {
        return None;
    }
    extended_convert(value, source, target)
}

/// 高精度分支独立保留，定宽热路径只内联标签判定。
fn extended_convert(
    value: &RuntimeValue,
    source: ScalarType,
    target: ScalarType,
) -> Option<RuntimeResult<RuntimeValue>> {
    use ScalarType::{Float, Int, Lfloat, Lint, Sfloat, Sint, Str};
    let result = match (source, target) {
        (Str, Lfloat) => {
            let RuntimeValue::Str(text) = value else {
                unreachable!("来源标签已由RuntimeValue确认")
            };
            text.with_str(RuntimeValue::new_lfloat)
                .and_then(|result| result)
        }
        (Lint, Str) => {
            let RuntimeValue::Lint(text) = value else {
                unreachable!("来源标签已由RuntimeValue确认")
            };
            numeric::integer_text(text).and_then(RuntimeValue::new_string)
        }
        (Lfloat, Str) => {
            let RuntimeValue::Lfloat(text) = value else {
                unreachable!("来源标签已由RuntimeValue确认")
            };
            numeric::decimal_text(text).and_then(RuntimeValue::new_string)
        }
        (Float | Sfloat, Str) => ieee::check_finite(value)
            .and_then(|()| value.display_text())
            .and_then(RuntimeValue::new_string),
        (Lint, Lfloat) => {
            let RuntimeValue::Lint(text) = value else {
                unreachable!("来源标签已由RuntimeValue确认")
            };
            // 整数文本到十进制文本保持系数与scale0，无需先转成二进制大整数。
            numeric::validated_text(text, xiao_types::numeric_text::NumericTextKind::Integer)
                .and_then(|_| RuntimeValue::new_lfloat(text))
        }
        (Int | Sint | Float | Sfloat, Lfloat) => {
            decimal::Decimal::from_value(value).and_then(decimal::Decimal::into_lfloat)
        }
        (Lint | Lfloat | Float | Sfloat, Lint) => integer::convert(value, target),
        (Lint | Lfloat, Int | Sint) => integer::convert(value, target),
        (Lint | Lfloat, Float | Sfloat) => binary::convert(value, target),
        _ => return None,
    };
    Some(result)
}

#[cfg(test)]
mod binary_tests;
#[cfg(test)]
mod resource_tests;
#[cfg(test)]
mod tests;
