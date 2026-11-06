//! A4完整数值矩阵、文本、截断和输入不变；不声称A5语言接线。

use crate::errors::RuntimeResult;
use crate::value::RuntimeValue;
use xiao_syntax::ScalarType;

/// 构造已接纳的lfloat文本标签，不新增字符串到lint的数字parser。
fn decimal(text: &str) -> RuntimeValue {
    RuntimeValue::new_lfloat(text).unwrap()
}

/// 真实字符串句柄用于公开Runtime转换入口测试。
fn text(value: &str) -> RuntimeValue {
    RuntimeValue::new_string(value).unwrap()
}

/// 核对返回标签与文本，不借跨数值族Eq推导类型提升。
fn expect(result: RuntimeResult<RuntimeValue>, target: ScalarType, expected: &str) {
    let result = result.unwrap();
    assert_eq!(result.scalar_type(), Some(target));
    assert_eq!(result.display_text().unwrap(), expected);
}

#[test]
/// 六种数值标量全部36个方向，保留同类型lfloat显示尺度。
fn a4_all_numeric_scalar_directions_use_the_frozen_matrix() {
    let values = [
        RuntimeValue::Int(10),
        RuntimeValue::Sint(10),
        RuntimeValue::new_lint("10").unwrap(),
        RuntimeValue::Float(10.0),
        RuntimeValue::Sfloat(10.0),
        decimal("10.00"),
    ];
    let targets = [
        ScalarType::Int,
        ScalarType::Sint,
        ScalarType::Lint,
        ScalarType::Float,
        ScalarType::Sfloat,
        ScalarType::Lfloat,
    ];
    for value in &values {
        for target in targets {
            let expected = if value.scalar_type() == Some(ScalarType::Lfloat)
                && target == ScalarType::Lfloat
            {
                "10.00"
            } else {
                "10"
            };
            expect(value.convert_to(target), target, expected);
        }
    }
}

#[test]
/// lint窄化检查精确范围，不再因超过i128解析上限而误报012。
fn a4_integer_widths_are_exact_and_range_checked() {
    for value in [i64::MIN, -1, 0, 1, i64::MAX] {
        let integer = RuntimeValue::Int(value);
        let large = integer.convert_to(ScalarType::Lint).unwrap();
        assert_eq!(large.convert_to(ScalarType::Int).unwrap(), integer);
    }
    for value in [i32::MIN, -1, 0, 1, i32::MAX] {
        let integer = RuntimeValue::Sint(value);
        let large = integer.convert_to(ScalarType::Lint).unwrap();
        assert_eq!(large.convert_to(ScalarType::Sint).unwrap(), integer);
    }
    for source in [
        "9223372036854775808",
        "-9223372036854775809",
        "340282366920938463463374607431768211456",
    ] {
        assert_eq!(
            RuntimeValue::new_lint(source)
                .unwrap()
                .convert_to(ScalarType::Int)
                .unwrap_err()
                .code(),
            "X06-RUNTIME-009"
        );
    }
    for source in ["2147483648", "-2147483649"] {
        assert_eq!(
            RuntimeValue::new_lint(source)
                .unwrap()
                .convert_to(ScalarType::Sint)
                .unwrap_err()
                .code(),
            "X06-RUNTIME-009"
        );
    }
    expect(
        RuntimeValue::Lint("-0000".to_owned()).convert_to(ScalarType::Str),
        ScalarType::Str,
        "0",
    );
    expect(
        RuntimeValue::Lint("+00010".to_owned()).convert_to(ScalarType::Str),
        ScalarType::Str,
        "10",
    );
}

#[test]
/// str到整数仍为单码点，数值文本必须显式经过lfloat。
fn a4_strings_preserve_codepoints_and_decimal_syntax() {
    for (source, point) in [
        ("A", 65),
        ("雪", 38634),
        ("😀", 128512),
        ("1", 49),
        ("\0", 0),
    ] {
        for target in [ScalarType::Int, ScalarType::Sint, ScalarType::Lint] {
            expect(text(source).convert_to(target), target, &point.to_string());
        }
    }
    for source in ["", "42", "e\u{301}"] {
        for target in [ScalarType::Int, ScalarType::Sint, ScalarType::Lint] {
            assert_eq!(
                text(source).convert_to(target).unwrap_err().code(),
                "X06-RUNTIME-012"
            );
        }
    }
    for (source, expected) in [
        (".5", "0.5"),
        ("5.", "5"),
        ("1.e2", "1e2"),
        ("+1_234.50_0e-2", "12.34500"),
        ("-0.00", "0"),
    ] {
        expect(
            text(source).convert_to(ScalarType::Lfloat),
            ScalarType::Lfloat,
            expected,
        );
    }
    expect(
        text("42")
            .convert_to(ScalarType::Lfloat)
            .unwrap()
            .convert_to(ScalarType::Lint),
        ScalarType::Lint,
        "42",
    );
    for source in [
        "",
        "+",
        "-",
        ".",
        "1e",
        "1e+",
        "_1",
        "1_",
        "1__0",
        "1_.0",
        "1e+_1",
        " 1",
        "1 ",
        "1 0",
        "\u{feff}1",
        "1\0",
        "NaN",
        "Inf",
        "0x10",
        "１２",
        "−1",
    ] {
        assert_eq!(
            text(source)
                .convert_to(ScalarType::Lfloat)
                .unwrap_err()
                .code(),
            "X06-RUNTIME-012",
            "{source:?}"
        );
    }
    for target in [ScalarType::Float, ScalarType::Sfloat] {
        assert_eq!(
            text("0.1").convert_to(target).unwrap_err().code(),
            "X06-RUNTIME-012"
        );
    }
}

#[test]
/// 同类型及文本往返保留显示精度，新整数来源统一显示scale0。
fn a4_text_round_trips_preserve_display_precision() {
    for source in ["1.50", "1.500", "10.00", "1e100000", "1e-100000", "-0.00"] {
        let number = decimal(source);
        let output = number.convert_to(ScalarType::Str).unwrap();
        let round_trip = output.convert_to(ScalarType::Lfloat).unwrap();
        assert_eq!(number, round_trip);
        assert_eq!(
            number.display_text().unwrap(),
            round_trip.display_text().unwrap()
        );
        assert_eq!(
            number.display_text().unwrap(),
            number
                .convert_to(ScalarType::Lfloat)
                .unwrap()
                .display_text()
                .unwrap()
        );
    }
    for value in [
        RuntimeValue::Int(10),
        RuntimeValue::Sint(10),
        RuntimeValue::new_lint("10").unwrap(),
        RuntimeValue::Float(10.0),
        RuntimeValue::Sfloat(10.0),
    ] {
        expect(
            value.convert_to(ScalarType::Lfloat),
            ScalarType::Lfloat,
            "10",
        );
    }
    expect(
        RuntimeValue::Float(0.1).convert_to(ScalarType::Str),
        ScalarType::Str,
        "0.1",
    );
    expect(
        RuntimeValue::Sfloat(0.1).convert_to(ScalarType::Str),
        ScalarType::Str,
        "0.1",
    );
    expect(
        RuntimeValue::Float(-0.0).convert_to(ScalarType::Str),
        ScalarType::Str,
        "-0",
    );
    expect(
        RuntimeValue::Sfloat(-0.0).convert_to(ScalarType::Str),
        ScalarType::Str,
        "-0",
    );
}

#[test]
/// 小数向零截断，定宽边界允许被截掉的小数部分，超范围仍为009。
fn a4_fractional_integer_conversions_truncate_toward_zero() {
    for (source, expected) in [
        ("1.9", "1"),
        ("-1.9", "-1"),
        ("0.999", "0"),
        ("-0.999", "0"),
        ("1e-100000", "0"),
    ] {
        for target in [ScalarType::Int, ScalarType::Sint, ScalarType::Lint] {
            expect(decimal(source).convert_to(target), target, expected);
        }
    }
    for (source, target, expected) in [
        (
            "9223372036854775807.999",
            ScalarType::Int,
            i64::MAX.to_string(),
        ),
        (
            "-9223372036854775808.999",
            ScalarType::Int,
            i64::MIN.to_string(),
        ),
        ("2147483647.999", ScalarType::Sint, i32::MAX.to_string()),
        ("-2147483648.999", ScalarType::Sint, i32::MIN.to_string()),
    ] {
        expect(decimal(source).convert_to(target), target, &expected);
    }
    for source in ["9223372036854775808", "-9223372036854775809", "1e100000"] {
        assert_eq!(
            decimal(source)
                .convert_to(ScalarType::Int)
                .unwrap_err()
                .code(),
            "X06-RUNTIME-009"
        );
    }
    for source in ["2147483648", "-2147483649"] {
        assert_eq!(
            decimal(source)
                .convert_to(ScalarType::Sint)
                .unwrap_err()
                .code(),
            "X06-RUNTIME-009"
        );
    }
    expect(
        RuntimeValue::Float(-3.9).convert_to(ScalarType::Lint),
        ScalarType::Lint,
        "-3",
    );
    expect(
        RuntimeValue::Sfloat(3.9).convert_to(ScalarType::Lint),
        ScalarType::Lint,
        "3",
    );
    let maximum = RuntimeValue::Float(f64::MAX)
        .convert_to(ScalarType::Lint)
        .unwrap();
    assert_eq!(maximum.display_text().unwrap().len(), 309);
    assert_eq!(
        maximum.convert_to(ScalarType::Float).unwrap(),
        RuntimeValue::Float(f64::MAX)
    );
}

#[test]
/// 有界小数对照独立整数除法，区分保留小数显示与截断结果。
fn a4_truncation_matches_independent_small_integer_reference() {
    for coefficient in -99_i64..=99 {
        for scale in -2_i32..=4 {
            let value = decimal(&format!("{coefficient}e{}", -scale));
            let expected = if scale >= 0 {
                coefficient / 10_i64.pow(scale as u32)
            } else {
                coefficient * 10_i64.pow((-scale) as u32)
            };
            let before = value.display_text().unwrap();
            for target in [ScalarType::Int, ScalarType::Sint, ScalarType::Lint] {
                expect(value.convert_to(target), target, &expected.to_string());
            }
            assert_eq!(value.display_text().unwrap(), before);
        }
    }
}
