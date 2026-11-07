#![allow(dead_code)]

//! B2-B私有显式精度除法契约。
//!
//! B1只建立参数、舍入模式和最终一次舍入的边界；不连接公开函数、类型/IR、VM/LLVM、
//! intrinsic或C ABI。公开入口在B2批次一次性接线。

use super::common::{self, Family, Number};
use crate::errors::{RuntimeError, RuntimeResult};
use crate::value::RuntimeValue;
use bigdecimal::{Signed, Zero};
use num_bigint::BigInt;
use xiao_types::numeric_text::{NUMERIC_DIGIT_LIMIT as D, NUMERIC_SCALE_LIMIT as H};

/// F-1确认的七种最终舍入模式。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum Rounding {
    HalfEven,
    HalfUp,
    HalfDown,
    TowardZero,
    AwayFromZero,
    Floor,
    Ceiling,
}

/// 一次显式除法请求；precision表示最多有效数字。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct Request {
    pub precision: usize,
    pub rounding: Rounding,
}

fn invalid(message: &'static str) -> RuntimeError {
    RuntimeError::invalid_value(message)
}

/// 解析源级模式名；不接受别名或本地化名称。
fn parse_rounding(value: &str) -> RuntimeResult<Rounding> {
    match value {
        "half_even" => Ok(Rounding::HalfEven),
        "half_up" => Ok(Rounding::HalfUp),
        "half_down" => Ok(Rounding::HalfDown),
        "toward_zero" => Ok(Rounding::TowardZero),
        "away_from_zero" => Ok(Rounding::AwayFromZero),
        "floor" => Ok(Rounding::Floor),
        "ceiling" => Ok(Rounding::Ceiling),
        _ => Err(invalid("rounding必须是冻结的七种名称之一")),
    }
}

/// 精度和舍入参数域检查；最终舍入由B2实现批次一次执行。
pub(super) fn request(precision: i64, rounding: &str) -> RuntimeResult<Request> {
    if !(1..=32_768).contains(&precision) {
        return Err(invalid("precision必须在1到32768之间"));
    }
    Ok(Request {
        precision: precision as usize,
        rounding: parse_rounding(rounding)?,
    })
}

/// 计算显式精度除法。
///
/// 这里先在整数商余上生成一个保护位，再只按最终商的被舍弃部分执行一次舍入。
/// 结果不补齐无意义的尾零；尾零会在输出前折回 `scale`。该函数只接收已经通过
/// `Number` 接纳检查的私有数值，不负责公开入口、挂起错误或 ABI 状态。
pub(super) fn divide(
    left: &Number,
    right: &Number,
    request: Request,
) -> RuntimeResult<RuntimeValue> {
    if right.is_zero() {
        return Err(RuntimeError::division_by_zero("/"));
    }
    if left.is_zero() {
        return common::finish(BigInt::from(0), 0, Family::Decimal);
    }

    let negative = left.coefficient.is_negative() != right.coefficient.is_negative();
    let numerator = left.coefficient.abs();
    let denominator = right.coefficient.abs();
    let numerator_digits = common::decimal_digits(&numerator);
    let denominator_digits = common::decimal_digits(&denominator);
    let order = super::comparison::ratio_order(
        &numerator,
        numerator_digits,
        &denominator,
        denominator_digits,
    );

    // One extra decimal digit plus the exact remainder is enough to distinguish every
    // rounding mode, including an exact half. The input scales cancel from this
    // alignment distance, so a large exponent does not force a second unbounded scan.
    let shift = request.precision as i64 - i64::from(order);
    let (scaled_numerator, scaled_denominator) = scale_pair(
        &numerator,
        &denominator,
        shift,
        numerator_digits,
        denominator_digits,
    )?;
    let (guarded, remainder) = common::div_rem(&scaled_numerator, &scaled_denominator)?;
    let (truncated, discarded_digits) = (&guarded / 10_u8, (&guarded % 10_u8).to_u32_digits());
    let discarded_digit = discarded_digits.1.first().copied().unwrap_or(0) as u8;
    let increment = should_increment(
        request.rounding,
        negative,
        &truncated,
        discarded_digit,
        !remainder.is_zero(),
    );
    let mut coefficient = truncated;
    if increment {
        coefficient += 1_u8;
    }
    if negative {
        coefficient = -coefficient;
    }

    // The p-digit coefficient is paired with the decimal order of the exact value.
    // The input scales are represented here; they do not disappear merely because the
    // quotient was computed from an aligned integer pair.
    let scale = i64::from(request.precision as i32 - 1)
        - i64::from(order)
        - (i64::from(right.scale) - i64::from(left.scale));
    if scale.abs() > i64::from(H) {
        return Err(common::limit("显式除法结果scale超过100000"));
    }
    common::check_work(
        "explicit-division-result",
        common::decimal_digits(&coefficient),
    )?;
    let normalized = common::normalize(coefficient, scale as i32)?;
    // A discarded part is allowed to round to a shorter exact-looking value, but the
    // representation still has to satisfy both display and canonical scale bounds.
    common::check_scales(
        i64::from(normalized.scale),
        i64::from(normalized.canonical_scale()),
    )?;
    if normalized.digits > D {
        return Err(common::limit("显式除法结果系数超过32768位"));
    }
    common::finish(normalized.coefficient, normalized.scale, Family::Decimal)
}

/// 按十进制位移构造商余输入；构造前先做工作上界检查，避免为非法超限输入
/// 分配大整数。
fn scale_pair(
    numerator: &BigInt,
    denominator: &BigInt,
    shift: i64,
    numerator_digits: usize,
    denominator_digits: usize,
) -> RuntimeResult<(BigInt, BigInt)> {
    if shift >= 0 {
        let shift =
            usize::try_from(shift).map_err(|_| common::limit("显式除法对齐位数超出范围"))?;
        let width = numerator_digits
            .checked_add(shift)
            .ok_or_else(|| common::limit("显式除法工作值超出范围"))?;
        common::check_work("explicit-division-numerator", width)?;
        let factor = common::power_ten(
            u32::try_from(shift).map_err(|_| common::limit("显式除法对齐位数超出范围"))?,
        )?;
        let scaled = common::checked_value("explicit-division-numerator", numerator * factor)?;
        Ok((scaled, denominator.clone()))
    } else {
        let shift =
            usize::try_from(-shift).map_err(|_| common::limit("显式除法对齐位数超出范围"))?;
        let width = denominator_digits
            .checked_add(shift)
            .ok_or_else(|| common::limit("显式除法工作值超出范围"))?;
        common::check_work("explicit-division-denominator", width)?;
        let factor = common::power_ten(
            u32::try_from(shift).map_err(|_| common::limit("显式除法对齐位数超出范围"))?,
        )?;
        let scaled = common::checked_value("explicit-division-denominator", denominator * factor)?;
        Ok((numerator.clone(), scaled))
    }
}

fn should_increment(
    rounding: Rounding,
    negative: bool,
    truncated: &BigInt,
    discarded_digit: u8,
    remainder_nonzero: bool,
) -> bool {
    let discarded = discarded_digit != 0 || remainder_nonzero;
    if !discarded {
        return false;
    }
    let above_half = discarded_digit > 5 || (discarded_digit == 5 && remainder_nonzero);
    let exact_half = discarded_digit == 5 && !remainder_nonzero;
    match rounding {
        Rounding::HalfEven => above_half || (exact_half && (truncated % 2_u8 != BigInt::from(0))),
        Rounding::HalfUp => above_half || exact_half,
        Rounding::HalfDown => above_half,
        Rounding::TowardZero => false,
        Rounding::AwayFromZero => true,
        Rounding::Floor => negative,
        Rounding::Ceiling => !negative,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_exact_precision_bounds_and_all_rounding_modes() {
        for mode in [
            "half_even",
            "half_up",
            "half_down",
            "toward_zero",
            "away_from_zero",
            "floor",
            "ceiling",
        ] {
            assert!(request(1, mode).is_ok());
            assert!(request(32_768, mode).is_ok());
        }
    }

    #[test]
    fn rejects_precision_and_rounding_before_any_division_work() {
        for precision in [0, -1, 32_769, i64::MAX] {
            assert_eq!(
                request(precision, "half_even").unwrap_err().code(),
                "X06-RUNTIME-012"
            );
        }
        assert_eq!(
            request(3, "half_even_alias").unwrap_err().code(),
            "X06-RUNTIME-012"
        );
    }

    fn decimal(text: &str) -> Number {
        Number::decimal(&RuntimeValue::new_lfloat(text).unwrap()).unwrap()
    }

    fn result(left: &str, right: &str, precision: i64, mode: &str) -> String {
        divide(
            &decimal(left),
            &decimal(right),
            request(precision, mode).unwrap(),
        )
        .unwrap()
        .display_text()
        .unwrap()
    }

    #[test]
    fn rounds_the_final_quotient_once_for_all_modes() {
        let cases = [
            ("half_even", "2", "-2", "2", "-2"),
            ("half_up", "3", "-3", "2", "-2"),
            ("half_down", "2", "-2", "2", "-2"),
            ("toward_zero", "2", "-2", "2", "-2"),
            ("away_from_zero", "3", "-3", "3", "-3"),
            ("floor", "2", "-3", "2", "-3"),
            ("ceiling", "3", "-2", "3", "-2"),
        ];
        for (mode, positive_half, negative_half, positive_tail, negative_tail) in cases {
            assert_eq!(result("2.5", "1", 1, mode), positive_half, "{mode}");
            assert_eq!(result("-2.5", "1", 1, mode), negative_half, "{mode}");
            assert_eq!(result("2.1", "1", 1, mode), positive_tail, "{mode}");
            assert_eq!(result("-2.1", "1", 1, mode), negative_tail, "{mode}");
        }
    }

    #[test]
    fn keeps_exact_short_results_and_rounds_carries() {
        assert_eq!(result("1.00", "2", 10, "half_even"), "0.5");
        assert_eq!(result("1", "3", 3, "half_even"), "0.333");
        assert_eq!(result("9.99", "1", 2, "half_even"), "1e1");
        assert_eq!(result("3", "8", 2, "half_even"), "0.38");
    }

    #[test]
    fn division_by_zero_and_work_limits_are_stable() {
        assert_eq!(
            divide(
                &decimal("1"),
                &decimal("0"),
                request(3, "half_even").unwrap()
            )
            .unwrap_err()
            .code(),
            "X06-RUNTIME-013"
        );
        let numerator = format!("1{}", "0".repeat(D - 1));
        let denominator = "9".repeat(D);
        assert_eq!(
            divide(
                &decimal(&numerator),
                &decimal(&denominator),
                request(32_768, "half_even").unwrap()
            )
            .unwrap_err()
            .code(),
            "X06-RUNTIME-033"
        );
    }
}
