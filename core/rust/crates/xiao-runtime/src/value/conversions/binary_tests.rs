//! IEEE位模式黄金和独立二进制参考，不用被测转换生成预期值。

use crate::value::RuntimeValue;
use num_bigint::BigInt;
use xiao_syntax::ScalarType;

/// 从有理数m*2^e构造精确十进制测试输入，黄金位模式单独给出。
fn dyadic(mantissa: BigInt, exponent: i32) -> RuntimeValue {
    let (coefficient, scale) = if exponent >= 0 {
        (mantissa << exponent as u32, 0)
    } else {
        (
            mantissa * BigInt::from(5).pow((-exponent) as u32),
            -exponent,
        )
    };
    RuntimeValue::new_lfloat(&format!("{coefficient}e{}", -scale)).unwrap()
}

/// 直接核对目标IEEE位模式，包括符号零；不以近似相等放过舍入差异。
fn expect_bits(value: &RuntimeValue, target: ScalarType, bits: u64) {
    match value.convert_to(target).unwrap() {
        RuntimeValue::Float(value) => assert_eq!(value.to_bits(), bits),
        RuntimeValue::Sfloat(value) => assert_eq!(u64::from(value.to_bits()), bits),
        other => panic!("转换结果类型不符：{}", other.type_name()),
    }
}

#[test]
/// 实际二进制0.1承接为完整精确十进制，整数来源的scale保持0。
fn a4_binary_inputs_keep_actual_values_and_normalize_zero() {
    let f64_value = RuntimeValue::Float(0.1)
        .convert_to(ScalarType::Lfloat)
        .unwrap();
    assert_eq!(
        f64_value.display_text().unwrap(),
        "0.1000000000000000055511151231257827021181583404541015625"
    );
    let f32_value = RuntimeValue::Sfloat(0.1)
        .convert_to(ScalarType::Lfloat)
        .unwrap();
    assert_eq!(
        f32_value.display_text().unwrap(),
        "0.100000001490116119384765625"
    );
    for value in [
        RuntimeValue::Float(-0.0),
        RuntimeValue::Float(0.0),
        RuntimeValue::Sfloat(-0.0),
        RuntimeValue::Sfloat(0.0),
    ] {
        let decimal = value.convert_to(ScalarType::Lfloat).unwrap();
        assert_eq!(decimal.display_text().unwrap(), "0");
        expect_bits(&decimal, ScalarType::Float, 0);
        expect_bits(&decimal, ScalarType::Sfloat, 0);
    }
    for (source, target, bits) in [
        ("0.1", ScalarType::Float, 0x3fb9_9999_9999_999a),
        ("-0.1", ScalarType::Float, 0xbfb9_9999_9999_999a),
        ("0.1", ScalarType::Sfloat, 0x3dcc_cccd),
        ("-0.1", ScalarType::Sfloat, 0xbdcc_cccd),
    ] {
        expect_bits(&RuntimeValue::new_lfloat(source).unwrap(), target, bits);
    }
}

#[test]
/// 偶数/奇数有效位的中点均直接舍入一次，负数完全对称。
fn a4_binary_rounding_uses_nearest_even_at_halfway_points() {
    for (target, precision, one, sign) in [
        (
            ScalarType::Float,
            53_u32,
            0x3ff0_0000_0000_0000_u64,
            1_u64 << 63,
        ),
        (ScalarType::Sfloat, 24, 0x3f80_0000, 1_u64 << 31),
    ] {
        let power = BigInt::from(1_u8) << precision;
        for (offset, expected) in [(1_u8, one), (3, one + 2), (5, one + 2), (7, one + 4)] {
            let mantissa = &power + offset;
            expect_bits(
                &dyadic(mantissa.clone(), -(precision as i32)),
                target,
                expected,
            );
            expect_bits(
                &dyadic(-mantissa, -(precision as i32)),
                target,
                sign | expected,
            );
        }
    }
    for (text, target, bits) in [
        (
            "9007199254740993",
            ScalarType::Float,
            0x4340_0000_0000_0000_u64,
        ),
        ("9007199254740995", ScalarType::Float, 0x4340_0000_0000_0002),
        (
            "-9007199254740993",
            ScalarType::Float,
            0xc340_0000_0000_0000,
        ),
        ("16777217", ScalarType::Sfloat, 0x4b80_0000),
        ("16777219", ScalarType::Sfloat, 0x4b80_0002),
    ] {
        expect_bits(&RuntimeValue::new_lint(text).unwrap(), target, bits);
    }
}

#[test]
/// MAX到溢出中点之间仍可舍入回有限值，中点自身按非有限结果拒绝。
fn a4_binary_overflow_uses_the_rounded_result() {
    for (target, precision, emax, maximum_bits, sign) in [
        (
            ScalarType::Float,
            53_u32,
            1023_u32,
            0x7fef_ffff_ffff_ffff_u64,
            1_u64 << 63,
        ),
        (ScalarType::Sfloat, 24, 127, 0x7f7f_ffff, 1_u64 << 31),
    ] {
        let maximum = ((BigInt::from(1_u8) << precision) - 1_u8) << (emax - (precision - 1));
        let midpoint =
            (BigInt::from(1_u8) << (emax + 1)) - (BigInt::from(1_u8) << (emax - precision));
        for coefficient in [maximum.clone(), maximum + 1_u8, &midpoint - 1_u8] {
            expect_bits(&dyadic(coefficient.clone(), 0), target, maximum_bits);
            expect_bits(&dyadic(-coefficient, 0), target, sign | maximum_bits);
        }
        for coefficient in [midpoint.clone(), midpoint + 1_u8] {
            for coefficient in [coefficient.clone(), -coefficient] {
                assert_eq!(
                    dyadic(coefficient, 0)
                        .convert_to(target)
                        .unwrap_err()
                        .code(),
                    "X06-RUNTIME-009"
                );
            }
        }
    }
}

#[test]
/// 最小子正规、半步、正常/子正规交界逐位核对，不把正常下溢报成009。
fn a4_binary_subnormal_and_signed_zero_boundaries_are_exact() {
    for (target, precision, half_exponent, minimum_normal, sign) in [
        (
            ScalarType::Float,
            53_u32,
            -1075_i32,
            1_u64 << 52,
            1_u64 << 63,
        ),
        (ScalarType::Sfloat, 24, -150, 1_u64 << 23, 1_u64 << 31),
    ] {
        for (mantissa, expected) in [(1_u8, 0_u64), (2, 1), (3, 2), (4, 2), (5, 2), (7, 4)] {
            expect_bits(
                &dyadic(BigInt::from(mantissa), half_exponent),
                target,
                expected,
            );
            expect_bits(
                &dyadic(-BigInt::from(mantissa), half_exponent),
                target,
                sign | expected,
            );
        }
        let mantissa = (BigInt::from(1_u8) << precision) - 1_u8;
        let midpoint = dyadic(mantissa.clone(), half_exponent);
        expect_bits(&midpoint, target, minimum_normal);
        expect_bits(
            &dyadic(-mantissa.clone(), half_exponent),
            target,
            sign | minimum_normal,
        );
        // 在精确中点的十进制系数上减1，偏移远小于半个最小子正规间距。
        let coefficient = mantissa * BigInt::from(5).pow((-half_exponent) as u32) - 1_u8;
        let below = RuntimeValue::new_lfloat(&format!("{coefficient}e{half_exponent}")).unwrap();
        expect_bits(&below, target, minimum_normal - 1);
    }
    for target in [ScalarType::Float, ScalarType::Sfloat] {
        expect_bits(&RuntimeValue::new_lfloat("1e-100000").unwrap(), target, 0);
        let sign = if target == ScalarType::Float {
            1_u64 << 63
        } else {
            1_u64 << 31
        };
        expect_bits(
            &RuntimeValue::new_lfloat("-1e-100000").unwrap(),
            target,
            sign,
        );
    }
}

#[test]
/// 这个精确值经f64中转会两次舍入；直接binary32必须选择上邻值。
fn a4_binary32_does_not_round_through_binary64() {
    let mantissa = (BigInt::from(1_u8) << 100_u32) + (BigInt::from(1_u8) << 76_u32) + 1_u8;
    let value = dyadic(mantissa.clone(), -100);
    expect_bits(&value, ScalarType::Sfloat, 0x3f80_0001);
    let intermediate = value.convert_to(ScalarType::Float).unwrap();
    expect_bits(&intermediate, ScalarType::Sfloat, 0x3f80_0000);
    expect_bits(&dyadic(-mantissa, -100), ScalarType::Sfloat, 0xbf80_0001);
}

#[test]
/// 固定种子的位模式生成覆盖两宽度；黄金为原始IEEE位，无随机依赖。
fn a4_binary_round_trips_match_independent_ieee_bit_patterns() {
    let mut state = 0x4d59_5df4_d0f3_3173_u64;
    for _ in 0..768 {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        let value = f64::from_bits(state);
        if value.is_finite() {
            let decimal = RuntimeValue::Float(value)
                .convert_to(ScalarType::Lfloat)
                .unwrap();
            expect_bits(
                &decimal,
                ScalarType::Float,
                if value == 0.0 { 0 } else { state },
            );
            let narrowed = value as f32;
            if narrowed.is_finite() {
                expect_bits(&decimal, ScalarType::Sfloat, u64::from(narrowed.to_bits()));
            } else {
                assert_eq!(
                    decimal.convert_to(ScalarType::Sfloat).unwrap_err().code(),
                    "X06-RUNTIME-009"
                );
            }
        }
        let bits = (state >> 11) as u32;
        let value = f32::from_bits(bits);
        if value.is_finite() {
            let decimal = RuntimeValue::Sfloat(value)
                .convert_to(ScalarType::Lfloat)
                .unwrap();
            expect_bits(
                &decimal,
                ScalarType::Sfloat,
                if value == 0.0 { 0 } else { u64::from(bits) },
            );
        }
    }
}
