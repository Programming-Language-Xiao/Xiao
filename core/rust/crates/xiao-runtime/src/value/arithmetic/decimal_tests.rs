//! A3内核语义；小范围参考仅用独立i128有理数运算。

use super::*;

/// 构造合法十进制文本标签，不经过A4公开转换。
fn decimal(text: &str) -> RuntimeValue {
    RuntimeValue::new_lfloat(text).unwrap()
}

/// 核对实际入口的返回族与保留显示精度的文本。
fn expect(result: RuntimeResult<RuntimeValue>, expected: &str) {
    let value = result.unwrap();
    assert!(matches!(value, RuntimeValue::Lfloat(_)));
    assert_eq!(value.display_text().unwrap(), expected);
}

/// 仅供有界独立参考，按明确c/s直接插入小数点，不调用产品渲染器。
fn render(coefficient: i128, scale: i32) -> String {
    if coefficient == 0 {
        return "0".to_owned();
    }
    if scale < 0 {
        return format!("{coefficient}e{}", -scale);
    }
    let sign = if coefficient < 0 { "-" } else { "" };
    let digits = coefficient.abs().to_string();
    let scale = scale as usize;
    if scale == 0 {
        format!("{sign}{digits}")
    } else if scale < digits.len() {
        format!(
            "{sign}{}.{}",
            &digits[..digits.len() - scale],
            &digits[digits.len() - scale..]
        )
    } else {
        format!("{sign}0.{}{digits}", "0".repeat(scale - digits.len()))
    }
}

#[test]
/// 显示scale与数值身份分离，零不提高另一输入尺度。
fn arithmetic_decimal_display_scales_and_identities_are_preserved() {
    expect(decimal("1.50").add(&decimal("1.5")), "3.00");
    expect(decimal("1.50").subtract(&decimal("1.5")), "0");
    expect(decimal("1.50").multiply(&decimal("2.0")), "3.000");
    expect(decimal("1.00").divide(&RuntimeValue::Int(2)), "0.50");
    expect(decimal("1.50").remainder(&decimal("1.00")), "0.50");
    expect(decimal("0.00").add(&decimal("1e100000")), "1e100000");
    expect(decimal("0").subtract(&decimal("1e100000")), "-1e100000");
    expect(decimal("-0").negate(), "0");
    expect(decimal("1.500").negate(), "-1.500");
    let values = [decimal("1.5"), decimal("1.50"), decimal("15e-1")];
    let set = crate::SetHandle::new(values.to_vec()).unwrap();
    assert_eq!(set.len(), 1);
    assert!(set.contains(&decimal("1.5000")).unwrap());
    assert!(values[0].less_equal(&values[1]).unwrap());
    assert!(values[2].greater_equal(&values[0]).unwrap());
}

#[test]
/// 原始显示文本的字典顺序无关，虚拟补零比较不用展开极端scale差。
fn arithmetic_decimal_order_is_numeric_and_exact() {
    for (a, b) in [
        ("9", "10"),
        ("1e-100000", "1e100000"),
        ("-10", "-9"),
        ("0", "0.00001"),
        ("-1e-100000", "0"),
    ] {
        assert!(decimal(a).less(&decimal(b)).unwrap(), "{a} < {b}");
        assert!(decimal(b).greater(&decimal(a)).unwrap(), "{b} > {a}");
    }
    common::trace_start();
    assert!(decimal("1e100000").greater(&decimal("1e-100000")).unwrap());
    assert!(
        common::trace_finish()
            .iter()
            .all(|event| event.label != "alignment" && event.label != "power-ten")
    );
    assert!(
        RuntimeValue::new_lint("9007199254740993")
            .unwrap()
            .greater(&RuntimeValue::Float(9007199254740992.0))
            .unwrap()
    );
    assert!(decimal("0.1").less(&RuntimeValue::Float(0.1)).unwrap());
}

#[test]
/// 四符号、不同scale的加减乘/floor余数对照独立有界整数参考。
fn arithmetic_decimal_small_rational_reference_covers_signs_and_scales() {
    for a in -9_i128..=9 {
        for b in -5_i128..=5 {
            for sa in [-1_i32, 0, 1, 2] {
                for sb in [-1_i32, 0, 1, 2] {
                    let left = decimal(&format!("{a}e{}", -sa));
                    let right = decimal(&format!("{b}e{}", -sb));
                    let scale = if a == 0 {
                        if b == 0 { 0 } else { sb }
                    } else if b == 0 {
                        sa
                    } else {
                        sa.max(sb)
                    };
                    let aligned_a = if a == 0 {
                        0
                    } else {
                        a * 10_i128.pow((scale - sa) as u32)
                    };
                    let aligned_b = if b == 0 {
                        0
                    } else {
                        b * 10_i128.pow((scale - sb) as u32)
                    };
                    expect(left.add(&right), &render(aligned_a + aligned_b, scale));
                    expect(left.subtract(&right), &render(aligned_a - aligned_b, scale));
                    expect(left.multiply(&right), &render(a * b, sa + sb));
                    if b == 0 {
                        continue;
                    }
                    let s = sa.max(sb);
                    let ax = a * 10_i128.pow((s - sa) as u32);
                    let bx = b * 10_i128.pow((s - sb) as u32);
                    let mut q = ax / bx;
                    let trunc = ax % bx;
                    if trunc != 0 && (trunc < 0) != (bx < 0) {
                        q -= 1;
                    }
                    expect(left.floor_divide(&right), &q.to_string());
                    expect(left.remainder(&right), &render(ax - bx * q, s));
                    assert_eq!(left.less(&right).unwrap(), ax < bx);
                }
            }
        }
    }
}

#[test]
/// 小有理数通过整数长除的独立参考判断有限/非终止与必要显示尺度。
fn arithmetic_decimal_finite_division_matches_independent_small_reference() {
    for a in -13_i128..=13 {
        for b in -13_i128..=13 {
            if b == 0 {
                continue;
            }
            for (sa, sb) in [(0_i32, 0_i32), (2, 1), (-2, 1), (1, -2)] {
                let left = decimal(&format!("{a}e{}", -sa));
                let right = decimal(&format!("{b}e{}", -sb));
                let mut numerator = a;
                let mut places = 0_i32;
                while numerator % b != 0 && places < 12 {
                    numerator *= 10;
                    places += 1;
                }
                if numerator % b != 0 {
                    assert_eq!(left.divide(&right).unwrap_err().code(), "X06-RUNTIME-034");
                    continue;
                }
                let mut coefficient = numerator / b;
                let mut scale = sa - sb + places;
                if coefficient != 0 {
                    while coefficient % 10 == 0 {
                        coefficient /= 10;
                        scale -= 1;
                    }
                    let display = (sa - sb).max(scale);
                    coefficient *= 10_i128.pow((display - scale) as u32);
                    scale = display;
                }
                expect(left.divide(&right), &render(coefficient, scale));
            }
        }
    }
}

#[test]
/// 浮点混合按实际位模式提升，私有提升不改变公开convert_to矩阵。
fn arithmetic_decimal_mixed_binary_inputs_are_lifted_exactly() {
    expect(
        decimal("0").add(&RuntimeValue::Float(0.1)),
        "0.1000000000000000055511151231257827021181583404541015625",
    );
    expect(
        decimal("0").add(&RuntimeValue::Sfloat(0.1)),
        "0.100000001490116119384765625",
    );
    expect(decimal("0").add(&RuntimeValue::Float(10.0)), "10");
    expect(decimal("0").add(&RuntimeValue::Sfloat(-0.0)), "0");
    expect(
        RuntimeValue::new_lint("7")
            .unwrap()
            .floor_divide(&RuntimeValue::Float(2.0)),
        "3",
    );
    expect(
        RuntimeValue::Sfloat(-7.5).remainder(&RuntimeValue::new_lint("2").unwrap()),
        "0.5",
    );
    expect(RuntimeValue::Float(0.5).divide(&decimal("0.125")), "4");
    let number = super::float_lift::f64_number(f64::from_bits(1)).unwrap();
    assert_eq!(number.scale, 1074);
    assert_eq!(number.coefficient, num_bigint::BigInt::from(5).pow(1074));
    let number = super::float_lift::f32_number(f32::from_bits(1)).unwrap();
    assert_eq!(number.scale, 149);
    assert_eq!(number.coefficient, num_bigint::BigInt::from(5).pow(149));
    assert!(
        RuntimeValue::Float(1.0)
            .floor_divide(&RuntimeValue::Float(1.0))
            .is_err()
    );
    assert!(
        RuntimeValue::Float(0.1)
            .convert_to(xiao_syntax::ScalarType::Lfloat)
            .is_err()
    );
}

#[test]
/// 指数先验整数域再快路径，n=1保显示尺度，其它单位底数规范化。
fn arithmetic_decimal_power_preserves_frozen_scales_and_domains() {
    for (base, exponent, expected) in [
        ("0", "0", "1"),
        ("1.00", "1.0", "1.00"),
        ("1.00", "2.0", "1"),
        ("-1.00", "3.0", "-1"),
        ("2.0", "2.0", "4.00"),
        ("-2.0", "3", "-8.000"),
    ] {
        expect(decimal(base).power(&decimal(exponent)), expected);
    }
    for exponent in ["-1", "1.5", "-0.5"] {
        assert_eq!(
            decimal("0").power(&decimal(exponent)).unwrap_err().code(),
            "X06-RUNTIME-009"
        );
    }
    for (base, expected) in [("0", "0"), ("1.00", "1"), ("-1.00", "1")] {
        expect(decimal(base).power(&decimal("1e100000")), expected);
    }
    expect(decimal("-1.00").power(&decimal(&"9".repeat(32_768))), "-1");
    assert_eq!(
        decimal("2").power(&decimal("1000001")).unwrap_err().code(),
        "X06-RUNTIME-033"
    );
    common::trace_start();
    expect(decimal("7.00").power(&decimal("1")), "7.00");
    assert!(
        common::trace_finish()
            .iter()
            .all(|event| !event.label.starts_with("power-"))
    );
}
