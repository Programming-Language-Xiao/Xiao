//! A4文本/系数/scale预算、错误顺序与输入不变；非性能证据。

use super::budget;
use crate::value::RuntimeValue;
use xiao_syntax::ScalarType;
use xiao_types::numeric_text::{NUMERIC_DIGIT_LIMIT as D, NUMERIC_WORK_DIGIT_LIMIT as W};

/// 经既有接纳构造lfloat输入。
fn decimal(text: &str) -> RuntimeValue {
    RuntimeValue::new_lfloat(text).unwrap()
}

#[test]
/// 原文先于语法、语法先于系数预算，零快路径不能跳过显式指数检查。
fn a4_input_admission_precedes_range_checks_and_zero_shortcuts() {
    let bad = RuntimeValue::Lfloat("0e100001".to_owned());
    for target in [
        ScalarType::Lfloat,
        ScalarType::Lint,
        ScalarType::Int,
        ScalarType::Sint,
        ScalarType::Float,
        ScalarType::Sfloat,
        ScalarType::Str,
    ] {
        assert_eq!(
            bad.convert_to(target).unwrap_err().code(),
            "X06-RUNTIME-033"
        );
    }
    let oversized = RuntimeValue::Lint("9".repeat(D + 1));
    for target in [
        ScalarType::Int,
        ScalarType::Sint,
        ScalarType::Float,
        ScalarType::Sfloat,
        ScalarType::Lfloat,
        ScalarType::Str,
    ] {
        assert_eq!(
            oversized.convert_to(target).unwrap_err().code(),
            "X06-RUNTIME-033"
        );
    }
    for (source, code) in [
        ("x".repeat(65_537), "X06-RUNTIME-033"),
        ("9".repeat(D + 1) + "x", "X06-RUNTIME-012"),
        ("9".repeat(D + 1), "X06-RUNTIME-033"),
        ("1e999999x".to_owned(), "X06-RUNTIME-012"),
    ] {
        let text = RuntimeValue::new_string(source.clone()).unwrap();
        assert_eq!(
            text.convert_to(ScalarType::Lfloat).unwrap_err().code(),
            code
        );
        assert_eq!(text.display_text().unwrap(), source);
    }
    let zero = RuntimeValue::new_string("0".repeat(65_536))
        .unwrap()
        .convert_to(ScalarType::Lfloat)
        .unwrap();
    assert_eq!(zero.display_text().unwrap(), "0");
}

#[test]
/// 大整数目标D/D+1与定宽范围009分别验收，超限前不补巨型零。
fn a4_integer_result_limits_are_checked_before_materialization() {
    let maximum = decimal("1e32767");
    let integer = maximum.convert_to(ScalarType::Lint).unwrap();
    assert_eq!(
        integer.display_text().unwrap(),
        "1".to_owned() + &"0".repeat(D - 1)
    );
    assert_eq!(integer.convert_to(ScalarType::Lfloat).unwrap(), maximum);
    assert_eq!(
        integer
            .convert_to(ScalarType::Lfloat)
            .unwrap()
            .display_text()
            .unwrap(),
        integer.display_text().unwrap()
    );
    budget::trace_start();
    assert_eq!(
        decimal("1e32768")
            .convert_to(ScalarType::Lint)
            .unwrap_err()
            .code(),
        "X06-RUNTIME-033"
    );
    assert!(budget::trace_finish().is_empty());
    budget::trace_start();
    assert_eq!(
        decimal("1e100000")
            .convert_to(ScalarType::Int)
            .unwrap_err()
            .code(),
        "X06-RUNTIME-009"
    );
    assert!(budget::trace_finish().is_empty());
    let coefficient = "9".repeat(D);
    let value = decimal(&coefficient);
    assert_eq!(
        value
            .convert_to(ScalarType::Lint)
            .unwrap()
            .display_text()
            .unwrap(),
        coefficient
    );
    let too_large = decimal(&(coefficient.clone() + "e1"));
    assert_eq!(
        too_large.convert_to(ScalarType::Lint).unwrap_err().code(),
        "X06-RUNTIME-033"
    );
    assert_eq!(
        too_large.convert_to(ScalarType::Int).unwrap_err().code(),
        "X06-RUNTIME-009"
    );
    assert_eq!(value.display_text().unwrap(), coefficient);
}

#[test]
/// 先移除小数再判定边界，最大显示系数仍可截断为合法定宽整数。
fn a4_large_fractional_coefficients_can_truncate_to_bounded_integers() {
    let coefficient = "1".to_owned() + &"0".repeat(D - 1);
    let value = decimal(&format!("{coefficient}e-{}", D - 19));
    budget::trace_start();
    assert_eq!(
        value.convert_to(ScalarType::Int).unwrap(),
        RuntimeValue::Int(1_000_000_000_000_000_000)
    );
    let events = budget::trace_finish();
    eprintln!("A4最大系数截断工作记录：{events:?}");
    assert!(
        events
            .iter()
            .any(|event| event.label == "truncate-division" && event.actual == Some(19))
    );
    assert!(events.iter().all(|event| event.upper <= W));
    assert!(
        events
            .iter()
            .filter_map(|event| event.actual)
            .all(|actual| actual <= W)
    );
}

#[test]
/// 巨型scale的确定溢出/下溢可直接判定；边界仍执行精确商余且工作值有界。
fn a4_binary_limits_clip_only_unambiguous_extremes() {
    for target in [ScalarType::Float, ScalarType::Sfloat] {
        budget::trace_start();
        assert_eq!(
            decimal("1e100000").convert_to(target).unwrap_err().code(),
            "X06-RUNTIME-009"
        );
        assert!(budget::trace_finish().is_empty());
        budget::trace_start();
        let zero = decimal("-1e-100000").convert_to(target).unwrap();
        assert_eq!(zero.display_text().unwrap(), "-0");
        assert!(budget::trace_finish().is_empty());
    }
    let text = format!("{}e-{}", "9".repeat(D), D + 323);
    let value = decimal(&text);
    let before = value.display_text().unwrap();
    budget::trace_start();
    let RuntimeValue::Float(output) = value.convert_to(ScalarType::Float).unwrap() else {
        panic!("目标应为float")
    };
    assert_eq!(output.to_bits(), 2);
    let events = budget::trace_finish();
    eprintln!("A4最大系数边界舍入工作记录：{events:?}");
    assert!(events.iter().any(|event| event.label == "binary-quotient"));
    assert!(events.iter().all(|event| event.upper <= W));
    assert!(
        events
            .iter()
            .filter_map(|event| event.actual)
            .all(|actual| actual <= W)
    );
    assert_eq!(value.display_text().unwrap(), before);
}

#[test]
/// 新增二进制来源转换拒绝NaN/Inf；有限原值和显示身份在失败后不变。
fn a4_non_finite_and_failed_conversions_preserve_the_frozen_errors() {
    for value in [
        RuntimeValue::Float(f64::NAN),
        RuntimeValue::Float(f64::INFINITY),
        RuntimeValue::Float(f64::NEG_INFINITY),
        RuntimeValue::Sfloat(f32::NAN),
        RuntimeValue::Sfloat(f32::INFINITY),
        RuntimeValue::Sfloat(f32::NEG_INFINITY),
    ] {
        for target in [ScalarType::Lint, ScalarType::Lfloat, ScalarType::Str] {
            assert_eq!(
                value.convert_to(target).unwrap_err().code(),
                "X06-RUNTIME-009"
            );
        }
    }
    let value = decimal("1.5000e100000");
    let before = value.display_text().unwrap();
    for target in [
        ScalarType::Float,
        ScalarType::Sfloat,
        ScalarType::Int,
        ScalarType::Lint,
    ] {
        assert!(value.convert_to(target).is_err());
    }
    assert_eq!(value.display_text().unwrap(), before);
    assert_ne!(
        crate::value::numeric::allocation_error().code(),
        "X06-RUNTIME-033"
    );
    assert_eq!(
        crate::value::numeric::allocation_error().code(),
        "X06-RUNTIME-011"
    );
}
