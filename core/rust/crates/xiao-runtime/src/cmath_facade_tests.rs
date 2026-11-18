//! B4-2B 内部 Rust 门面：逐项比对共享内核及正式参考，不经过 C ABI 状态槽。

use super::*;
use crate::value::arithmetic::cmath as kernel;

#[derive(Clone, Copy)]
/// 测试用函数指针形态，避免以相同参数列表误调用一元/二元门面。
enum Entry {
    Unary(fn(f64) -> RuntimeResult<f64>),
    Binary(fn(f64, f64) -> RuntimeResult<f64>),
}

impl Entry {
    /// 按真实元数调用门面或共享内核。
    fn call(self, a: f64, b: f64) -> RuntimeResult<f64> {
        match self {
            Self::Unary(f) => f(a),
            Self::Binary(f) => f(a, b),
        }
    }
}

/// 独立列出公开门面和私有内核的配对，逐项发现接错函数。
fn entries() -> [(&'static str, Entry, Entry); 15] {
    use Entry::{Binary as B, Unary as U};
    [
        ("sqrt", U(cmath_sqrt), U(kernel::sqrt)),
        ("floor", U(cmath_floor), U(kernel::floor)),
        ("ceil", U(cmath_ceil), U(kernel::ceil)),
        ("round", U(cmath_round), U(kernel::round)),
        ("trunc", U(cmath_trunc), U(kernel::trunc)),
        ("abs", U(cmath_abs), U(kernel::abs)),
        ("min", B(cmath_min), B(kernel::min)),
        ("max", B(cmath_max), B(kernel::max)),
        ("pow", B(cmath_pow), B(kernel::pow)),
        ("sin", U(cmath_sin), U(kernel::sin)),
        ("cos", U(cmath_cos), U(kernel::cos)),
        ("tan", U(cmath_tan), U(kernel::tan)),
        ("exp", U(cmath_exp), U(kernel::exp)),
        ("ln", U(cmath_ln), U(kernel::ln)),
        ("log10", U(cmath_log10), U(kernel::log10)),
    ]
}

/// 只比较精确 IEEE 位模式或稳定错误码，不受错误实例编号影响。
fn identity(result: RuntimeResult<f64>) -> Result<u64, String> {
    result
        .map(f64::to_bits)
        .map_err(|error| error.code().into())
}

#[test]
/// 正式离线向量逐条经过公开 Rust 门面；与私有内核逐位或逐错误码一致。
fn rust_facades_match_all_shared_kernel_reference_inputs() {
    let vectors = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../../tests/fixtures/docs/cmath/reference-vectors.tsv"
    ));
    let mut counts = [0usize; 15];
    let entries = entries();
    for line in vectors.lines().filter(|line| !line.starts_with('#')) {
        let cols: Vec<_> = line.split('\t').collect();
        let (index, (_, public, private)) = entries
            .iter()
            .enumerate()
            .find(|(_, (name, _, _))| *name == cols[1])
            .unwrap();
        let number = |column: &str| f64::from_bits(u64::from_str_radix(column, 16).unwrap());
        let a = number(cols[2]);
        let b = if cols[3] == "-" { 0.0 } else { number(cols[3]) };
        assert_eq!(
            identity(public.call(a, b)),
            identity(private.call(a, b)),
            "{}",
            cols[0]
        );
        counts[index] += 1;
    }
    assert_eq!(counts.iter().sum::<usize>(), 17_243);
    assert!(counts.iter().all(|count| *count >= 800));
    println!("B4-2B-RUNTIME reference_inputs=17243 facades=15 kernel_identity=exact");
}

#[test]
/// 每个必填输入独立注入 NaN/±Inf，所有门面均沿 009 返回。
fn rust_facades_reject_each_non_finite_operand() {
    for (name, public, _) in entries() {
        for non_finite in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            assert_eq!(
                public.call(non_finite, 1.0).unwrap_err().code(),
                NUMERIC_OVERFLOW_CODE,
                "{name}"
            );
            if matches!(public, Entry::Binary(_)) {
                assert_eq!(
                    public.call(1.0, non_finite).unwrap_err().code(),
                    NUMERIC_OVERFLOW_CODE,
                    "{name}"
                );
            }
        }
    }
}

#[test]
/// 成功和域错误既不建立也不替换 ABI 挂起错误，错误仍直接通过 Result 返回。
fn rust_facades_leave_abi_pending_error_untouched() {
    for (_, public, _) in entries() {
        abi::xiao_runtime_error_clear();
        let _ = public.call(1.0, 2.0);
        let _ = public.call(f64::NAN, 2.0);
        assert_eq!(abi::xiao_runtime_error_class(), 0);
        let sentinel = RuntimeError::invalid_value("b4-2b-runtime-sentinel");
        let id = sentinel.error_id();
        abi::set_pending_runtime_error(sentinel);
        let _ = public.call(1.0, 2.0);
        let _ = public.call(f64::NAN, 2.0);
        let mut snapshot = std::mem::MaybeUninit::uninit();
        assert_eq!(abi::xiao_runtime_error_snapshot(snapshot.as_mut_ptr()), 0);
        let snapshot = unsafe { snapshot.assume_init() };
        assert_eq!(snapshot.error_id, id);
        assert_eq!(snapshot.stack_depth, 0);
        abi::xiao_runtime_error_clear();
    }
}
