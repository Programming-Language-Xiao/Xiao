//! 十进制加减以独立i128与原成熟大数路径对照；覆盖显示尺度、混合族和工作预算。
use super::*;

/// 有界有理数的独立十进制显示参考，不使用产品系数运算。
fn reference_text(value: i128, scale: i32) -> String {
    if value == 0 {
        return "0".into();
    }
    let sign = if value < 0 { "-" } else { "" };
    let digits = value.abs().to_string();
    if scale < 0 {
        return format!("{value}e{}", -scale);
    }
    if scale == 0 {
        return value.to_string();
    }
    let scale = scale as usize;
    if scale < digits.len() {
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
/// 进借位、符号翻转、零与混合整数/十进制对照独立i128参考。
fn text_add_subtract_matches_independent_rational_reference() {
    for a in [-23_i128, -10, -1, 0, 1, 10, 23] {
        for sa in [-5_i32, 0, 5] {
            let left = if sa == 0 {
                RuntimeValue::new_lint(&a.to_string()).unwrap()
            } else {
                RuntimeValue::new_lfloat(&format!("{a}e{}", -sa)).unwrap()
            };
            for b in [-23_i128, -10, -1, 0, 1, 10, 23] {
                for sb in [-5_i32, 0, 5] {
                    let right = if sb == 0 {
                        RuntimeValue::Int(b as i64)
                    } else {
                        RuntimeValue::new_lfloat(&format!("{b}e{}", -sb)).unwrap()
                    };
                    for op in [Operation::Add, Operation::Subtract] {
                        let scale = if a == 0 {
                            sb
                        } else if b == 0 {
                            sa
                        } else {
                            sa.max(sb)
                        };
                        let ac = if a == 0 {
                            0
                        } else {
                            a * 10_i128.pow((scale - sa) as u32)
                        };
                        let bc = if b == 0 {
                            0
                        } else {
                            b * 10_i128.pow((scale - sb) as u32)
                        };
                        let expected = if op == Operation::Add {
                            ac + bc
                        } else {
                            ac - bc
                        };
                        let actual = binary(op, &left, &right).unwrap().unwrap();
                        assert_eq!(
                            actual.display_text().unwrap(),
                            reference_text(expected, scale)
                        );
                        assert_eq!(
                            actual.type_name(),
                            if sa == 0 && sb == 0 { "lint" } else { "lfloat" }
                        );
                    }
                }
            }
        }
    }
}

#[test]
/// 成熟大数路径作为不同算法参考，极端scale与尾零不得因快路改变错误身份。
fn text_add_subtract_matches_bigint_path_and_input_state() {
    let mut values: Vec<_> = [
        "-12.3400",
        "-1e5",
        "-1e-5",
        "-0.00",
        "0",
        "0.0010",
        "1.50",
        "9.99",
        "1000",
        "1e100000",
        "1e-100000",
    ]
    .into_iter()
    .map(|text| RuntimeValue::new_lfloat(text).unwrap())
    .collect();
    values.extend([
        RuntimeValue::new_lint("-999999999999999999999999999999999").unwrap(),
        RuntimeValue::Int(i64::MIN),
        RuntimeValue::Sint(i32::MAX),
    ]);
    for left in &values {
        for right in &values {
            for op in [Operation::Add, Operation::Subtract] {
                let before = (left.display_text().unwrap(), right.display_text().unwrap());
                let actual = binary(op, left, right).unwrap();
                let expected = if matches!(left, RuntimeValue::Lfloat(_))
                    || matches!(right, RuntimeValue::Lfloat(_))
                {
                    super::super::decimal::binary(op, left, right)
                } else {
                    super::super::integer::binary(op, left, right)
                };
                match (actual, expected) {
                    (Ok(actual), Ok(expected)) => {
                        assert_eq!(actual.type_name(), expected.type_name());
                        assert_eq!(
                            actual.display_text().unwrap(),
                            expected.display_text().unwrap()
                        );
                    }
                    (Err(actual), Err(expected)) => {
                        assert_eq!(actual.code(), expected.code());
                        assert_eq!(actual.kind(), expected.kind());
                    }
                    (actual, expected) => panic!("不同算法结果不一致：{actual:?} / {expected:?}"),
                }
                assert_eq!(
                    before,
                    (left.display_text().unwrap(), right.display_text().unwrap())
                );
            }
        }
    }
}

#[test]
/// 最大系数进位超D、D+1强消去与W+1预检均沿原冻结门槛。
fn text_add_subtract_keeps_exact_result_and_work_limits() {
    use xiao_types::numeric_text::NUMERIC_DIGIT_LIMIT as D;
    let maximum = RuntimeValue::new_lint(&"9".repeat(D)).unwrap();
    assert_eq!(
        maximum.add(&RuntimeValue::Int(1)).unwrap_err().code(),
        "X06-RUNTIME-033"
    );
    let power = RuntimeValue::new_lfloat(&format!("1e{D}")).unwrap();
    assert_eq!(
        power.subtract(&maximum).unwrap().display_text().unwrap(),
        "1"
    );
    let large = RuntimeValue::new_lfloat("1e32768").unwrap();
    let tiny = RuntimeValue::new_lfloat("1e-32767").unwrap();
    common::trace_start();
    assert_eq!(large.add(&tiny).unwrap_err().code(), "X06-RUNTIME-033");
    assert!(
        common::trace_finish()
            .iter()
            .all(|event| event.actual.is_none())
    );
}
