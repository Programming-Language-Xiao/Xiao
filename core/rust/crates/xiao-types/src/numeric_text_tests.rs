//! 冻结语法、预算顺序与规范身份的确定性向量。

use super::*;
use std::collections::hash_map::DefaultHasher;

/// 取得与输入字符串无关的规范身份哈希。
fn hash(view: NumericTextView<'_>) -> u64 {
    let mut state = DefaultHasher::new();
    view.hash(&mut state);
    state.finish()
}

#[test]
/// 显示系数可不同，但同一数学值只有一个借用身份。
fn equal_decimal_spellings_share_identity() {
    let texts = ["1.5", "1.50", "15e-1", "+0001.50_0e+00"];
    let first = validate_numeric_text(texts[0], NumericTextKind::Decimal).unwrap();
    for text in texts {
        let view = validate_numeric_text(text, NumericTextKind::Decimal).unwrap();
        assert_eq!(view, first);
        assert_eq!(hash(view), hash(first));
    }
    assert_eq!(first.display_scale(), 1);
    assert_eq!(
        validate_numeric_text("1.50", NumericTextKind::Decimal)
            .unwrap()
            .display_scale(),
        2
    );
    assert_ne!(
        first,
        validate_numeric_text("1.51", NumericTextKind::Decimal).unwrap()
    );
}

#[test]
/// 语法拒绝不能被未来的系数、指数预算抢先替换。
fn syntax_is_complete_before_numeric_budgets() {
    for text in [
        "",
        "+",
        "-",
        ".",
        "1e",
        "1e+",
        "1_",
        "_1",
        "1__0",
        "1_.0",
        "1._0",
        "1e_0",
        "1e+_0",
        "1e1_",
        " 1",
        "1\n",
        "NaN",
        "Inf",
        "１",
        "\u{feff}1",
        "1\0",
    ] {
        assert_eq!(
            validate_numeric_text(text, NumericTextKind::Decimal).unwrap_err(),
            NumericTextError::InvalidSyntax,
            "{text:?}"
        );
    }
    for text in [".5", "5.", "-5.e+0", "+0_0.5_0e-0_1"] {
        assert!(
            validate_numeric_text(text, NumericTextKind::Decimal).is_ok(),
            "{text}"
        );
    }
    for text in ["1.0", "1e1", "1_0"] {
        assert_eq!(
            validate_numeric_text(text, NumericTextKind::Integer).unwrap_err(),
            NumericTextError::InvalidSyntax
        );
    }
    for text in [
        "1".repeat(NUMERIC_DIGIT_LIMIT + 1) + "x",
        "1e".to_owned() + &"9".repeat(200) + "x",
    ] {
        assert_eq!(
            validate_numeric_text(&text, NumericTextKind::Decimal).unwrap_err(),
            NumericTextError::InvalidSyntax
        );
    }
    let too_long = "x".repeat(NUMERIC_TEXT_LIMIT + 1);
    assert_eq!(
        validate_numeric_text(&too_long, NumericTextKind::Decimal).unwrap_err(),
        NumericTextError::LimitExceeded
    );
}

#[test]
/// 零例外不绕过原文和显式指数；非零显示及规范scale分别受限。
fn zero_and_scales_have_independent_budget_checks() {
    for text in ["0", "-0.00", "0e100000", "0.0e-100000"] {
        let zero = validate_numeric_text(text, NumericTextKind::Decimal).unwrap();
        assert!(zero.is_zero());
        assert!(!zero.is_negative());
        assert_eq!(zero.display_scale(), 0);
    }
    assert!(
        validate_numeric_text(&"0".repeat(NUMERIC_TEXT_LIMIT), NumericTextKind::Integer)
            .unwrap()
            .is_zero()
    );
    for text in [
        "0e100001",
        "0e-100001",
        "1e100001",
        "1.0e-100000",
        "100e100000",
    ] {
        assert_eq!(
            validate_numeric_text(text, NumericTextKind::Decimal).unwrap_err(),
            NumericTextError::LimitExceeded,
            "{text}"
        );
    }
    let left = validate_numeric_text("1e100000", NumericTextKind::Decimal).unwrap();
    let right = validate_numeric_text("10e99999", NumericTextKind::Decimal).unwrap();
    assert_eq!(left, right);
    assert_eq!(hash(left), hash(right));
    assert!(validate_numeric_text("1e-100000", NumericTextKind::Decimal).is_ok());
}

#[test]
/// 最大显示系数可接纳；增加一位尾零也必须拒绝，不能先按规范身份删掉它。
fn coefficient_boundaries_preserve_display_precision() {
    let valid = "1".to_owned() + &"0".repeat(NUMERIC_DIGIT_LIMIT - 1);
    let view = validate_numeric_text(&valid, NumericTextKind::Integer).unwrap();
    assert_eq!(view.digit_count(), NUMERIC_DIGIT_LIMIT);
    assert_eq!(view.canonical_digits().collect::<Vec<_>>(), b"1");
    assert_eq!(view.canonical_scale(), -(NUMERIC_DIGIT_LIMIT as i32 - 1));
    assert_eq!(
        validate_numeric_text(&(valid + "0"), NumericTextKind::Integer).unwrap_err(),
        NumericTextError::LimitExceeded
    );
}

#[test]
/// 有界确定性生成同时覆盖符号、前导零、尾零和小范围指数，无随机依赖。
fn deterministic_numeric_identity_cases() {
    for coefficient in [1, 2, 9, 10, 15, 99, 1201] {
        for exponent in -9..=9 {
            for sign in ["", "-"] {
                let first = format!("{sign}{coefficient}e{exponent}");
                let second = format!("{sign}000{coefficient}000e{}", exponent - 3);
                let left = validate_numeric_text(&first, NumericTextKind::Decimal).unwrap();
                let right = validate_numeric_text(&second, NumericTextKind::Decimal).unwrap();
                assert_eq!(left, right, "{first} {second}");
                assert_eq!(hash(left), hash(right));
            }
        }
    }
}
