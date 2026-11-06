//! A2内核的独立边界、顺序与工作模型证据。

use super::*;
use num_bigint::BigInt;
use xiao_types::numeric_text::{NUMERIC_DIGIT_LIMIT as D, NUMERIC_WORK_DIGIT_LIMIT as W};

/// 构造合法lint输入，测试不引入str到lint的语言数字解析。
fn lint(text: &str) -> RuntimeValue {
    RuntimeValue::new_lint(text).unwrap()
}

/// 通过真实Runtime入口核对返回族与精确显示。
fn expect(value: RuntimeResult<RuntimeValue>, text: &str, decimal: bool) {
    let value = value.unwrap();
    assert_eq!(matches!(value, RuntimeValue::Lfloat(_)), decimal);
    assert_eq!(value.display_text().unwrap(), text);
}

#[test]
/// 超过宿主i128的加减乘与顺序比较仍精确。
fn arithmetic_integer_values_do_not_narrow_to_host_widths() {
    let large = lint("340282366920938463463374607431768211456");
    expect(
        large.add(&lint("1")),
        "340282366920938463463374607431768211457",
        false,
    );
    expect(
        large.subtract(&lint("1")),
        "340282366920938463463374607431768211455",
        false,
    );
    expect(
        large.multiply(&RuntimeValue::Int(-2)),
        "-680564733841876926926749214863536422912",
        false,
    );
    assert!(
        large
            .greater(&lint("340282366920938463463374607431768211455"))
            .unwrap()
    );
    assert!(large.less_equal(&large).unwrap());
    expect(
        large.negate(),
        "-340282366920938463463374607431768211456",
        false,
    );
    expect(lint("-0").negate(), "0", false);
}

#[test]
/// 小输入以有界整数数学独立核对四符号floor配对及余数恒等式。
fn arithmetic_integer_floor_pairs_cover_all_signs() {
    for a in -31_i64..=31 {
        for b in -7_i64..=7 {
            if b == 0 {
                continue;
            }
            let mut q = a / b;
            let trunc_r = a % b;
            if trunc_r != 0 && (trunc_r < 0) != (b < 0) {
                q -= 1;
            }
            let r = a - q * b;
            expect(
                lint(&a.to_string()).floor_divide(&lint(&b.to_string())),
                &q.to_string(),
                false,
            );
            expect(
                lint(&a.to_string()).remainder(&lint(&b.to_string())),
                &r.to_string(),
                false,
            );
        }
    }
    expect(lint("1").divide(&lint("8")), "0.125", true);
    expect(lint("100").divide(&RuntimeValue::Sint(4)), "25", true);
    expect(lint("-3").divide(&lint("2")), "-1.5", true);
}

#[test]
/// 非终止判断不得被后续工作/结果超限抢先替换。
fn arithmetic_integer_division_errors_keep_the_frozen_order() {
    assert_eq!(
        lint("1").divide(&lint("3")).unwrap_err().code(),
        "X06-RUNTIME-034"
    );
    for result in [
        lint("1").divide(&lint("0")),
        lint("1").floor_divide(&lint("0")),
        lint("1").remainder(&lint("0")),
    ] {
        assert_eq!(result.unwrap_err().code(), "X06-RUNTIME-013");
    }
    let finite_denominator = BigInt::from(2).pow(100_000);
    assert_eq!(
        lint("1")
            .divide(&lint(&finite_denominator.to_str_radix(10)))
            .unwrap_err()
            .code(),
        "X06-RUNTIME-033"
    );
    let non_terminating = finite_denominator * 3_u8;
    assert_eq!(
        lint("1")
            .divide(&lint(&non_terminating.to_str_radix(10)))
            .unwrap_err()
            .code(),
        "X06-RUNTIME-034"
    );
}

#[test]
/// 最终系数实际位数与工作上界分别检查，强消去不能按松结果上界拒绝。
fn arithmetic_integer_limits_and_cancellation_are_atomic() {
    let maximum = "9".repeat(D);
    let before = lint(&maximum);
    let one_less = lint(&("9".repeat(D - 1) + "8"));
    expect(before.subtract(&one_less), "1", false);
    assert_eq!(
        before.add(&lint("1")).unwrap_err().code(),
        "X06-RUNTIME-033"
    );
    assert_eq!(
        before.multiply(&lint("10")).unwrap_err().code(),
        "X06-RUNTIME-033"
    );
    assert_eq!(before.display_text().unwrap(), maximum);
    common::trace_start();
    let _ = before.multiply(&before).unwrap_err();
    let events = common::trace_finish();
    eprintln!("A2最大乘法工作记录：{events:?}");
    assert!(events.iter().any(|event| event.label == "integer-multiply"
        && event.upper == W
        && event.actual.is_none()));
    assert!(
        events
            .iter()
            .filter_map(|event| event.actual)
            .all(|digits| digits <= W)
    );
}

#[test]
/// 非平凡幂按真实结果验收，单位底数不先窄化巨大合法指数。
fn arithmetic_integer_power_bounds_preserve_fast_paths() {
    expect(lint("0").power(&lint("0")), "1", false);
    expect(lint("-1").power(&lint(&"9".repeat(D))), "-1", false);
    expect(lint("1").power(&lint(&"9".repeat(D))), "1", false);
    expect(lint("0").power(&lint(&"9".repeat(D))), "0", false);
    assert_eq!(
        lint("2").power(&lint("-1")).unwrap_err().code(),
        "X06-RUNTIME-009"
    );
    assert_eq!(
        lint("2").power(&lint("1000001")).unwrap_err().code(),
        "X06-RUNTIME-033"
    );
    let expected = BigInt::from(2).pow(108_852).to_str_radix(10);
    assert_eq!(expected.len(), D);
    expect(lint("2").power(&lint("108852")), &expected, false);
    assert_eq!(
        lint("2").power(&lint("108853")).unwrap_err().code(),
        "X06-RUNTIME-033"
    );
    common::trace_start();
    expect(lint("7").power(&lint("1")), "7", false);
    assert!(
        common::trace_finish()
            .iter()
            .all(|event| !event.label.starts_with("power-"))
    );
}

#[test]
/// 类型先于预算，输入接纳先于幂快路径；bool奇偶不受i128限制。
fn arithmetic_integer_input_order_and_bool_parity_are_stable() {
    let invalid = RuntimeValue::Lint("9".repeat(D + 1));
    assert_eq!(
        invalid.add(&RuntimeValue::Bool(false)).unwrap_err().code(),
        "X06-RUNTIME-002"
    );
    assert_eq!(
        invalid.power(&lint("0")).unwrap_err().code(),
        "X06-RUNTIME-033"
    );
    let odd = lint(&"9".repeat(D));
    let even = lint(&("9".repeat(D - 1) + "8"));
    assert_eq!(
        RuntimeValue::Bool(false).add(&odd).unwrap(),
        RuntimeValue::Bool(true)
    );
    assert_eq!(
        RuntimeValue::Bool(true).subtract(&odd).unwrap(),
        RuntimeValue::Bool(false)
    );
    assert_eq!(
        RuntimeValue::Bool(true).add(&even).unwrap(),
        RuntimeValue::Bool(true)
    );
    assert!(odd.add(&RuntimeValue::Bool(true)).is_err());
}
