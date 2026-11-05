//! A1精确适配、显示往返、错误身份与集合行为，不实现算术运算。

use super::*;
use crate::{NUMERIC_LIMIT_CODE, SetHandle};
use std::collections::{HashMap, HashSet};

#[test]
/// F-10显示向量逐字匹配，默认BigDecimal格式阈值不得影响结果。
fn frozen_decimal_display_vectors_round_trip() {
    for (text, expected) in [
        ("1.50", "1.50"),
        ("1.5", "1.5"),
        ("15e-1", "1.5"),
        ("+0001.50_0e+00", "1.500"),
        ("0.00", "0"),
        ("-0.00", "0"),
        ("10", "10"),
        ("1e100000", "1e100000"),
        ("1e-65535", "1e-65535"),
        (".5", "0.5"),
        ("5.", "5"),
        ("100e1", "100e1"),
        ("-1e-65534", "-1e-65534"),
    ] {
        let value = crate::RuntimeValue::new_lfloat(text).unwrap();
        assert_eq!(value.display_text().unwrap(), expected, "{text}");
        let restored = crate::RuntimeValue::new_lfloat(expected).unwrap();
        assert_eq!(restored, value);
        assert_eq!(restored.display_text().unwrap(), expected);
    }
    let largest_plain = crate::RuntimeValue::new_lfloat("1e-65534")
        .unwrap()
        .display_text()
        .unwrap();
    assert_eq!(largest_plain.len(), 65_536);
    assert_eq!(largest_plain, "0.".to_owned() + &"0".repeat(65_533) + "1");
    assert_eq!(
        crate::RuntimeValue::new_lfloat(&largest_plain).unwrap(),
        crate::RuntimeValue::new_lfloat("1e-65534").unwrap()
    );
}

#[test]
/// 大数系数不经过i128/f64；负号、前导零、显示尾零及极大scale精确保留。
fn temporary_upstream_values_are_exact_and_bounded() {
    let digits = "9".repeat(32_768);
    assert_eq!(parse_integer(&digits).unwrap().to_str_radix(10), digits);
    assert_eq!(
        crate::RuntimeValue::new_lint(&("+000".to_owned() + &digits))
            .unwrap()
            .display_text()
            .unwrap(),
        digits
    );
    let decimal = parse_decimal("-0001.50_0e-99997").unwrap();
    let (coefficient, scale) = decimal.as_bigint_and_scale();
    assert_eq!(coefficient.to_str_radix(10), "-1500");
    assert_eq!(scale, 100_000);
    assert_eq!(integer_text("-00000").unwrap(), "0");
    assert_eq!(decimal_text("0.0e-100000").unwrap(), "0");
}

#[test]
/// 012、033、011是不同场景；后续坏语法仍先于系数/指数超限。
fn recoverable_errors_preserve_the_frozen_order() {
    let oversized = "1".repeat(32_769);
    for text in [oversized.as_str(), "1e100001", "100e100000", "0e100001"] {
        let error = crate::RuntimeValue::new_lfloat(text).unwrap_err();
        assert_eq!(error.code(), NUMERIC_LIMIT_CODE);
        assert_eq!(error.kind(), RuntimeErrorKind::Resource);
    }
    for text in [" 1", "1_", "0_._0e1", "NaN", "Inf", "1e999999x", "１"] {
        let error = crate::RuntimeValue::new_lfloat(text).unwrap_err();
        assert_eq!(error.code(), crate::INVALID_VALUE_CODE);
        assert_eq!(error.kind(), RuntimeErrorKind::Type);
    }
    assert_eq!(
        crate::RuntimeValue::new_lfloat(&(oversized + "x"))
            .unwrap_err()
            .code(),
        crate::INVALID_VALUE_CODE
    );
    assert_eq!(
        crate::RuntimeValue::new_lfloat(&"x".repeat(65_537))
            .unwrap_err()
            .code(),
        NUMERIC_LIMIT_CODE
    );
    assert_eq!(
        crate::RuntimeValue::new_lint(&"0".repeat(65_536))
            .unwrap()
            .display_text()
            .unwrap(),
        "0"
    );
    assert_eq!(
        crate::RuntimeValue::new_lint(&"0".repeat(65_537))
            .unwrap_err()
            .code(),
        NUMERIC_LIMIT_CODE
    );
    assert_eq!(allocation_error().code(), ALLOCATION_CODE);
    assert_ne!(allocation_error().code(), NUMERIC_LIMIT_CODE);
}

#[test]
#[expect(
    clippy::mutable_key_type,
    reason = "此测试只以校验后的不可变数值变体作键，核对RuntimeValue的Hash/Eq契约"
)]
/// 同值异显示在实际Xiao集合去重/查询；Rust映射仅检查底层Hash契约。
fn numeric_identity_drives_sets_and_rust_hash_maps() {
    let values: Vec<_> = ["1.5", "1.50", "15e-1", "+0001.50_0e+00"]
        .into_iter()
        .map(|text| crate::RuntimeValue::new_lfloat(text).unwrap())
        .collect();
    let set = SetHandle::new(values.clone()).unwrap();
    assert_eq!(set.len(), 1);
    assert!(
        set.contains(&crate::RuntimeValue::new_lfloat("150e-2").unwrap())
            .unwrap()
    );
    assert!(
        !set.contains(&crate::RuntimeValue::new_lfloat("1.51").unwrap())
            .unwrap()
    );
    let hashes: HashSet<_> = values.iter().cloned().collect();
    assert_eq!(hashes.len(), 1);
    let mut mapping = HashMap::new();
    for (index, value) in values.into_iter().enumerate() {
        mapping.insert(value, index);
    }
    assert_eq!(mapping.len(), 1);
    assert_eq!(
        mapping.get(&crate::RuntimeValue::new_lfloat("1500e-3").unwrap()),
        Some(&3)
    );
    assert_ne!(
        crate::RuntimeValue::new_lint("10").unwrap(),
        crate::RuntimeValue::new_lfloat("10").unwrap()
    );
}

#[test]
#[expect(
    clippy::mutable_key_type,
    reason = "此测试只构造不可变的数值标量，不使用RuntimeValue的可变句柄变体"
)]
/// 前导零和显示尾零的确定性生成，不添加rand或proptest。
fn deterministic_spelling_cases_have_one_runtime_identity() {
    for coefficient in 1..=96 {
        for exponent in [-12, -1, 0, 1, 12] {
            let left =
                crate::RuntimeValue::new_lfloat(&format!("{coefficient}e{exponent}")).unwrap();
            let right =
                crate::RuntimeValue::new_lfloat(&format!("000{coefficient}000e{}", exponent - 3))
                    .unwrap();
            assert_eq!(left, right);
            let values: HashSet<_> = [left, right].into_iter().collect();
            assert_eq!(values.len(), 1);
        }
    }
}
