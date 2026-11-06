//! A3接纳/工作模型、错误顺序、强消去和输入不变证据。

use super::*;
use xiao_types::numeric_text::{NUMERIC_DIGIT_LIMIT as D, NUMERIC_WORK_DIGIT_LIMIT as W};

/// 通过已接纳的十进制标签调用内核。
fn decimal(text: &str) -> RuntimeValue {
    RuntimeValue::new_lfloat(text).unwrap()
}

/// 精确核对lfloat显示，测试不宣称语言后端已完成接线。
fn expect(result: RuntimeResult<RuntimeValue>, expected: &str) {
    let result = result.unwrap();
    assert!(matches!(result, RuntimeValue::Lfloat(_)));
    assert_eq!(result.display_text().unwrap(), expected);
}

#[test]
/// 一元负号共享输入接纳，零不能隐藏原文/指数超限或坏语法。
fn arithmetic_high_precision_negation_checks_admission_before_zero() {
    for value in [
        RuntimeValue::Lfloat("0e100001".to_owned()),
        RuntimeValue::Lint("0".repeat(65_537)),
        RuntimeValue::Lint("9".repeat(D + 1)),
    ] {
        assert_eq!(value.negate().unwrap_err().code(), "X06-RUNTIME-033");
    }
    assert_eq!(
        RuntimeValue::Lfloat("0ex".to_owned())
            .negate()
            .unwrap_err()
            .code(),
        "X06-RUNTIME-012"
    );
    let text = "9".repeat(D);
    let integer = RuntimeValue::new_lint(&text).unwrap();
    assert_eq!(
        integer.negate().unwrap().display_text().unwrap(),
        "-".to_owned() + &text
    );
    assert_eq!(integer.display_text().unwrap(), text);
    expect(decimal("-1.500").negate(), "1.500");
}

#[test]
/// 构造前W与最终实际D分别核对，合法强消去不被松结果上界误拒。
fn arithmetic_decimal_work_limits_and_cancellation_keep_inputs_unchanged() {
    let text = "9".repeat(D);
    let maximum = decimal(&text);
    expect(maximum.subtract(&decimal(&("9".repeat(D - 1) + "8"))), "1");
    assert_eq!(
        maximum.add(&decimal("1")).unwrap_err().code(),
        "X06-RUNTIME-033"
    );
    assert_eq!(
        maximum.multiply(&decimal("10")).unwrap_err().code(),
        "X06-RUNTIME-033"
    );
    assert_eq!(maximum.display_text().unwrap(), text);
    common::trace_start();
    assert_eq!(
        decimal("1e32767")
            .add(&decimal("1e-32767"))
            .unwrap_err()
            .code(),
        "X06-RUNTIME-033"
    );
    let events = common::trace_finish();
    eprintln!("A3对齐W边界工作记录：{events:?}");
    assert!(
        events
            .iter()
            .any(|event| event.label == "decimal-add-subtract"
                && event.upper == W
                && event.actual.is_none())
    );
    assert!(
        events
            .iter()
            .filter_map(|event| event.actual)
            .all(|digits| digits <= W)
    );
    common::trace_start();
    assert_eq!(
        decimal("1e32768")
            .add(&decimal("1e-32767"))
            .unwrap_err()
            .code(),
        "X06-RUNTIME-033"
    );
    let events = common::trace_finish();
    eprintln!("A3对齐W+1拒绝记录：{events:?}");
    assert!(
        events
            .iter()
            .any(|event| event.upper == W + 1 && event.actual.is_none())
    );
    assert!(events.iter().all(|event| event.actual.is_none()));
}

#[test]
/// 显示与规范scale分别受H约束，不能为通过预算偷删显示尾零。
fn arithmetic_decimal_scale_limits_precede_expensive_construction() {
    common::trace_start();
    assert_eq!(
        decimal("1e-100000")
            .multiply(&decimal("0.1"))
            .unwrap_err()
            .code(),
        "X06-RUNTIME-033"
    );
    assert!(common::trace_finish().is_empty());
    assert_eq!(
        decimal("2e100000")
            .multiply(&decimal("5"))
            .unwrap_err()
            .code(),
        "X06-RUNTIME-033"
    );
    assert_eq!(
        decimal("1.00")
            .multiply(&decimal(&("9".repeat(D - 1))))
            .unwrap_err()
            .code(),
        "X06-RUNTIME-033"
    );
    common::trace_start();
    assert_eq!(
        decimal("1e-50001").power(&decimal("2")).unwrap_err().code(),
        "X06-RUNTIME-033"
    );
    assert!(
        common::trace_finish()
            .iter()
            .all(|event| !event.label.starts_with("power-"))
    );
    expect(decimal("1e-50000").power(&decimal("2")), "1e-100000");
}

#[test]
/// 034先于展开或scale预算，合法零分子仍先检查输入与非零分母。
fn arithmetic_decimal_exact_division_keeps_failure_identities_and_order() {
    assert_eq!(
        decimal("1e100000")
            .divide(&decimal("3e-100000"))
            .unwrap_err()
            .code(),
        "X06-RUNTIME-034"
    );
    assert_eq!(
        decimal("1e100000")
            .divide(&decimal("1e-100000"))
            .unwrap_err()
            .code(),
        "X06-RUNTIME-033"
    );
    for op in [
        Operation::Divide,
        Operation::FloorDivide,
        Operation::Remainder,
    ] {
        assert_eq!(
            binary(op, &decimal("0"), &decimal("0"))
                .unwrap()
                .unwrap_err()
                .code(),
            "X06-RUNTIME-013"
        );
    }
    let invalid = RuntimeValue::Lfloat("9".repeat(D + 1));
    assert_eq!(
        decimal("0").divide(&invalid).unwrap_err().code(),
        "X06-RUNTIME-033"
    );
    assert_eq!(
        invalid.add(&RuntimeValue::Bool(false)).unwrap_err().code(),
        "X06-RUNTIME-002"
    );
    assert_eq!(
        RuntimeValue::Lfloat("1x".to_owned())
            .add(&decimal("1"))
            .unwrap_err()
            .code(),
        "X06-RUNTIME-012"
    );
}

#[test]
/// 实际答复确认的复合顺序：两侧预算→非有限→算子域。
fn arithmetic_decimal_non_finite_precedes_operator_domain_after_inputs() {
    let infinity = RuntimeValue::Float(f64::INFINITY);
    let zero = RuntimeValue::new_lint("0").unwrap();
    assert_eq!(
        infinity.floor_divide(&zero).unwrap_err().code(),
        "X06-RUNTIME-009"
    );
    assert_eq!(
        infinity.remainder(&zero).unwrap_err().code(),
        "X06-RUNTIME-009"
    );
    assert_eq!(
        RuntimeValue::Float(f64::NAN)
            .power(&decimal("0"))
            .unwrap_err()
            .code(),
        "X06-RUNTIME-009"
    );
    let invalid = RuntimeValue::Lint("9".repeat(D + 1));
    assert_eq!(
        infinity.divide(&invalid).unwrap_err().code(),
        "X06-RUNTIME-033"
    );
    assert_eq!(
        invalid.divide(&infinity).unwrap_err().code(),
        "X06-RUNTIME-033"
    );
    assert_eq!(
        decimal("1").divide(&zero).unwrap_err().code(),
        "X06-RUNTIME-013"
    );
}

#[test]
/// 巨大商拒绝与小余数成功独立，不允许展开巨大对齐或商。
fn arithmetic_decimal_remainder_does_not_materialize_the_quotient() {
    let left = decimal("1e100000");
    let right = decimal("1e-100000");
    common::trace_start();
    assert_eq!(
        left.floor_divide(&right).unwrap_err().code(),
        "X06-RUNTIME-033"
    );
    assert!(common::trace_finish().is_empty());
    common::trace_start();
    expect(left.remainder(&right), "0");
    let events = common::trace_finish();
    assert!(
        events
            .iter()
            .all(|event| event.label != "alignment" && event.label != "quotient")
    );
    common::trace_start();
    expect(left.remainder(&decimal("3e-100000")), "1e-100000");
    let events = common::trace_finish();
    assert!(
        events
            .iter()
            .all(|event| event.label != "alignment" && event.label != "quotient")
    );
    assert!(
        events
            .iter()
            .filter(|event| event.label == "modular-multiply" && event.actual.is_some())
            .count()
            <= 64
    );
    expect(
        decimal("-1e100000").remainder(&decimal("3e-100000")),
        "2e-100000",
    );
    expect(
        decimal("1e100000").remainder(&decimal("-3e-100000")),
        "-2e-100000",
    );
    expect(
        decimal("-1e-100000").floor_divide(&decimal("1e100000")),
        "-1",
    );
    assert_eq!(
        decimal("-1e-100000")
            .remainder(&decimal("1e100000"))
            .unwrap_err()
            .code(),
        "X06-RUNTIME-033"
    );
    expect(
        decimal("1e-100000").remainder(&decimal("1e100000")),
        "1e-100000",
    );
    assert_eq!(left.display_text().unwrap(), "1e100000");
    assert_eq!(right.display_text().unwrap(), "1e-100000");
}

#[test]
/// D+1位的虚拟分母必须实际求差，不能按松上界拒绝可表示余数。
fn arithmetic_decimal_remainder_d_plus_one_boundary_is_checked_exactly() {
    let numerator = decimal(&format!("-{}", "9".repeat(D)));
    common::trace_start();
    expect(numerator.remainder(&decimal(&format!("1e{D}"))), "1");
    let events = common::trace_finish();
    eprintln!("A3余数D+1强消去工作记录：{events:?}");
    assert!(
        events
            .iter()
            .any(|event| event.label == "alignment" && event.actual == Some(D + 1))
    );
    assert!(
        events
            .iter()
            .any(|event| event.label == "remainder-boundary-difference" && event.actual == Some(1))
    );
    assert_eq!(
        numerator
            .remainder(&decimal(&format!("2e{D}")))
            .unwrap_err()
            .code(),
        "X06-RUNTIME-033"
    );
    common::trace_start();
    assert_eq!(
        numerator
            .remainder(&decimal(&format!("1e{}", D + 1)))
            .unwrap_err()
            .code(),
        "X06-RUNTIME-033"
    );
    assert!(
        common::trace_finish()
            .iter()
            .all(|event| event.label != "alignment")
    );
    expect(decimal("-9.9").floor_divide(&decimal("1")), "-10");
}

#[test]
/// 最大合法系数的非零余数实际触达W模乘，仍不创建商或巨型展开。
fn arithmetic_decimal_dense_modulus_reaches_the_work_boundary() {
    let a = decimal(&("9".repeat(D - 1) + "8"));
    let b = decimal(&format!("{}e-{}", "9".repeat(D), D - 1));
    common::trace_start();
    expect(a.remainder(&b), &("8.".to_owned() + &"9".repeat(D - 1)));
    let events = common::trace_finish();
    eprintln!("A3最大非零模乘工作记录：{events:?}");
    assert!(events.iter().any(|event| event.label == "modular-multiply"
        && event.upper == W
        && event.actual.is_none()));
    assert!(
        events
            .iter()
            .filter_map(|event| event.actual)
            .all(|digits| digits <= W)
    );
    assert!(
        events
            .iter()
            .all(|event| event.label != "quotient" && event.label != "alignment")
    );
}

#[test]
/// 2因子有限除法的最终系数D与D+1，以及幂scale H与H+1独立验收。
fn arithmetic_decimal_finite_division_and_power_exact_limits() {
    let denominator = num_bigint::BigInt::from(2).pow(46_880);
    let expected = num_bigint::BigInt::from(5).pow(46_880).to_str_radix(10);
    assert_eq!(expected.len(), D);
    let result = decimal("1")
        .divide(&decimal(&denominator.to_str_radix(10)))
        .unwrap();
    let view = common::Number::decimal(&result).unwrap();
    assert_eq!(view.digits, D);
    assert_eq!(view.scale, 46_880);
    assert_eq!(view.coefficient.to_str_radix(10), expected);
    let denominator = denominator * 2_u8;
    assert_eq!(
        decimal("1")
            .divide(&decimal(&denominator.to_str_radix(10)))
            .unwrap_err()
            .code(),
        "X06-RUNTIME-033"
    );
    expect(
        decimal(&"9".repeat(D)).divide(&decimal("10")),
        &("9".repeat(D - 1) + ".9"),
    );
    expect(decimal("0.1").power(&decimal("100000")), "1e-100000");
    assert_eq!(
        decimal("0.1").power(&decimal("100001")).unwrap_err().code(),
        "X06-RUNTIME-033"
    );
}
