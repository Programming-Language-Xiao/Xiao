//! B3-1：MPFR 4.2.2/GMP 6.3.0 离线区间参考；生产构建不包含向量。

use super::*;
use crate::value::RuntimeValue;
use std::collections::BTreeMap;

const VECTORS: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../../docs/DevDocs/assets/20o-b2-b/entry-20261108-b3/b1/reference-vectors.tsv"
));

struct Vector<'a> {
    id: &'a str,
    function: &'a str,
    a: f64,
    b: f64,
    expected: &'a str,
}

fn vectors() -> impl Iterator<Item = Vector<'static>> {
    VECTORS
        .lines()
        .filter(|line| !line.starts_with('#'))
        .map(|line| {
            let columns: Vec<_> = line.split('\t').collect();
            assert_eq!(columns.len(), 9, "参考行格式错误：{line}");
            assert!(matches!(columns[5], "mpfr_interval" | "contract_special"));
            assert_eq!(columns[7], columns[8], "参考区间未收敛：{line}");
            let from_bits = |column: &str| f64::from_bits(u64::from_str_radix(column, 16).unwrap());
            Vector {
                id: columns[0],
                function: columns[1],
                a: from_bits(columns[2]),
                b: if columns[3] == "-" {
                    0.0
                } else {
                    from_bits(columns[3])
                },
                expected: columns[4],
            }
        })
}

fn call(vector: &Vector<'_>) -> RuntimeResult<f64> {
    let (a, b) = (vector.a, vector.b);
    match vector.function {
        "sqrt" => sqrt(a),
        "floor" => floor(a),
        "ceil" => ceil(a),
        "round" => round(a),
        "trunc" => trunc(a),
        "abs" => abs(a),
        "min" => min(a, b),
        "max" => max(a, b),
        "pow" => pow(a, b),
        "sin" => sin(a),
        "cos" => cos(a),
        "tan" => tan(a),
        "exp" => exp(a),
        "ln" => ln(a),
        "log10" => log10(a),
        other => panic!("未登记的参考函数：{other}"),
    }
}

/// 先精确核对错误、零/非零和符号，再核对ULP；禁止用容差放过flush或符号错误。
fn verify(vector: &Vector<'_>, tolerance: u64) -> u64 {
    let actual = call(vector);
    let context = || {
        format!(
            "{} {} a={:016x} b={:016x}",
            vector.id,
            vector.function,
            vector.a.to_bits(),
            vector.b.to_bits()
        )
    };
    if vector.expected == "X06-RUNTIME-009" {
        assert_eq!(
            actual.unwrap_err().code(),
            "X06-RUNTIME-009",
            "{}",
            context()
        );
        return 0;
    }
    let expected_bits = u64::from_str_radix(vector.expected, 16).expect("不能把不可验证向量当黄金");
    let expected = f64::from_bits(expected_bits);
    let actual = actual.unwrap_or_else(|error| panic!("{}: {error:?}", context()));
    assert!(actual.is_finite() && expected.is_finite(), "{}", context());
    assert_eq!(
        actual == 0.0,
        expected == 0.0,
        "零/非零不一致：{}",
        context()
    );
    assert_eq!(
        actual.is_sign_negative(),
        expected.is_sign_negative(),
        "符号不一致：{}",
        context()
    );
    if expected == 0.0 {
        assert_eq!(
            actual.to_bits(),
            expected_bits,
            "零位模式不一致：{}",
            context()
        );
    }
    // 符号已相同；正数或负数的同号位模式距离即ULP距离。
    let ulp = actual.to_bits().abs_diff(expected_bits);
    assert!(
        ulp <= tolerance,
        "{} actual={:016x} expected={expected_bits:016x} ulp={ulp} limit={tolerance}",
        context(),
        actual.to_bits()
    );
    ulp
}

fn group(functions: &[&str], tolerance: u64) {
    let mut counts = BTreeMap::new();
    let mut maxima = BTreeMap::new();
    for vector in vectors().filter(|v| functions.contains(&v.function)) {
        let ulp = verify(&vector, tolerance);
        *counts.entry(vector.function).or_insert(0usize) += 1;
        let maximum = maxima.entry(vector.function).or_insert(0u64);
        *maximum = (*maximum).max(ulp);
    }
    for function in functions {
        assert!(counts.get(function).copied().unwrap_or_default() >= 800);
        println!(
            "B3-CMATH function={function} cases={} max_ulp={} limit={tolerance} result=success",
            counts[function], maxima[function]
        );
    }
}

#[test]
fn mpfr_sqrt_is_correctly_rounded() {
    group(&["sqrt"], 0);
}

#[test]
fn mpfr_rounding_and_abs_match_exact_bits() {
    group(&["floor", "ceil", "round", "trunc", "abs"], 0);
}

#[test]
fn mpfr_min_max_match_exact_bits_and_zero_signs() {
    group(&["min", "max"], 0);
}

#[test]
fn mpfr_transcendentals_preserve_domains_signs_and_subnormals() {
    group(&["sin", "cos", "tan", "exp", "ln", "log10"], 4);
}

fn outcome(result: RuntimeResult<f64>) -> String {
    match result {
        Ok(value) => format!("{:016x}", value.to_bits()),
        Err(error) => error.code().to_owned(),
    }
}

#[test]
fn mpfr_pow_vectors_and_public_float_share_kernel() {
    group(&["pow"], 4);
    // 调用已接线的公开Runtime算子；旧宿主结果由冻结的B3-1差表单独保留。
    for vector in vectors().filter(|v| v.function == "pow") {
        let public = RuntimeValue::Float(vector.a)
            .power(&RuntimeValue::Float(vector.b))
            .map(|value| {
                if let RuntimeValue::Float(value) = value {
                    value
                } else {
                    panic!("float幂返回类型变化")
                }
            });
        let public = outcome(public);
        let kernel = outcome(call(&vector));
        assert_eq!(public, kernel, "{}：公开float幂与共享内核不一致", vector.id);
        println!(
            "B3-POW-SHARED\t{}\t{:016x}\t{:016x}\t{}\t{}\t{}",
            vector.id,
            vector.a.to_bits(),
            vector.b.to_bits(),
            public,
            kernel,
            vector.expected
        );
    }
}
