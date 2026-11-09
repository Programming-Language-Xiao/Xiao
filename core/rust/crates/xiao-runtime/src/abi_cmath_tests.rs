//! B3 C ABI的真实调用、所有权、结构失败与完整MPFR向量。

use super::*;
use std::ptr;

/// 一元C ABI函数指针，调用真实导出符号。
type Unary = extern "C" fn(*const XiaoValue, *mut XiaoValue) -> i32;
/// 二元C ABI函数指针，调用真实导出符号。
type Binary = extern "C" fn(*const XiaoValue, *const XiaoValue, *mut XiaoValue) -> i32;

#[derive(Clone, Copy)]
/// 测试条目按元数保存真实C入口，不复制数学实现。
enum Entry {
    Unary(Unary),
    Binary(Binary),
}

impl Entry {
    /// 按条目元数分派调用，用同一矩阵覆盖全部符号。
    fn call(self, a: *const XiaoValue, b: *const XiaoValue, out: *mut XiaoValue) -> i32 {
        match self {
            Self::Unary(f) => f(a, out),
            Self::Binary(f) => f(a, b, out),
        }
    }
}

/// 独立列出15个符号及可逐位判定的基本样本。
fn entries() -> [(&'static str, Entry, f64, f64, f64); 15] {
    [
        ("sqrt", Entry::Unary(xiao_runtime_cmath_sqrt), 4.0, 0.0, 2.0),
        (
            "floor",
            Entry::Unary(xiao_runtime_cmath_floor),
            1.5,
            0.0,
            1.0,
        ),
        ("ceil", Entry::Unary(xiao_runtime_cmath_ceil), 1.5, 0.0, 2.0),
        (
            "round",
            Entry::Unary(xiao_runtime_cmath_round),
            2.5,
            0.0,
            2.0,
        ),
        (
            "trunc",
            Entry::Unary(xiao_runtime_cmath_trunc),
            -1.5,
            0.0,
            -1.0,
        ),
        ("abs", Entry::Unary(xiao_runtime_cmath_abs), -0.0, 0.0, 0.0),
        (
            "min",
            Entry::Binary(xiao_runtime_cmath_min),
            -0.0,
            0.0,
            -0.0,
        ),
        ("max", Entry::Binary(xiao_runtime_cmath_max), -0.0, 0.0, 0.0),
        ("pow", Entry::Binary(xiao_runtime_cmath_pow), 2.0, 3.0, 8.0),
        ("sin", Entry::Unary(xiao_runtime_cmath_sin), -0.0, 0.0, -0.0),
        ("cos", Entry::Unary(xiao_runtime_cmath_cos), 0.0, 0.0, 1.0),
        ("tan", Entry::Unary(xiao_runtime_cmath_tan), -0.0, 0.0, -0.0),
        ("exp", Entry::Unary(xiao_runtime_cmath_exp), 0.0, 0.0, 1.0),
        ("ln", Entry::Unary(xiao_runtime_cmath_ln), 1.0, 0.0, 0.0),
        (
            "log10",
            Entry::Unary(xiao_runtime_cmath_log10),
            1.0,
            0.0,
            0.0,
        ),
    ]
}

/// 构造带堆载荷的拥有输出，验证失败保留及成功释放。
fn owned() -> XiaoValue {
    runtime_to_value(&RuntimeValue::new_string("cmath-output-sentinel").unwrap()).unwrap()
}

/// 读取完整挂起错误身份与消息，发现隐式清空或替换。
fn pending() -> (String, String) {
    PENDING_ERROR.with(|slot| match slot.borrow().as_ref().unwrap() {
        PendingError::Recoverable(error) => (error.code().into(), error.message().into()),
        _ => panic!("应为可恢复挂起错误"),
    })
}

/// 为每项结构失败预置可区分的挂起错误。
fn sentinel() {
    set_pending_runtime_error(RuntimeError::invalid_value("cmath-pending-sentinel"));
}

/// 同时核对输出标签、句柄身份与内容仍为原值。
fn assert_unchanged(out: &XiaoValue, handle: XiaoHandle) {
    assert_eq!(out.tag, XiaoValueTag::Str);
    assert_eq!(unsafe { out.payload.handle }, handle);
    assert_eq!(
        unsafe { value_to_runtime(out) }
            .unwrap()
            .display_text()
            .unwrap(),
        "cmath-output-sentinel"
    );
}

#[test]
/// 成功结果逐位准确，释放旧拥有值且保留已有挂起错误。
fn all_entries_replace_owned_output_and_keep_pending_on_success() {
    for (name, entry, a, b, expected) in entries() {
        let a = XiaoValue::float(a);
        let b = XiaoValue::float(b);
        let mut out = owned();
        let old = unsafe { strong_ref(out.payload.handle) }
            .unwrap()
            .inners
            .borrow()
            .last()
            .unwrap()
            .downgrade();
        sentinel();
        let before = pending();
        assert_eq!(entry.call(&a, &b, &mut out), 0, "{name}");
        assert_eq!(out.tag, XiaoValueTag::Float);
        assert_eq!(
            unsafe { out.payload.f64_value }.to_bits(),
            expected.to_bits(),
            "{name}"
        );
        assert!(old.upgrade().is_err(), "{name}没有释放旧out载荷");
        assert_eq!(pending(), before, "{name}成功清除了挂起错误");
        xiao_runtime_value_release(&mut out);
    }
    xiao_runtime_error_clear();
}

#[test]
/// 逐项验证每个必需输入与out空指针返回状态1而不改状态。
fn all_entries_reject_required_null_pointers_without_mutating_state() {
    for (name, entry, a, b, _) in entries() {
        let a = XiaoValue::float(a);
        let b = XiaoValue::float(b);
        let mut out = owned();
        let handle = unsafe { out.payload.handle };
        sentinel();
        let before = pending();
        assert_eq!(entry.call(ptr::null(), &b, &mut out), 1, "{name}");
        if matches!(entry, Entry::Binary(_)) {
            assert_eq!(entry.call(&a, ptr::null(), &mut out), 1, "{name}");
        }
        assert_eq!(entry.call(&a, &b, ptr::null_mut()), 1, "{name}");
        assert_unchanged(&out, handle);
        assert_eq!(pending(), before);
        xiao_runtime_value_release(&mut out);
    }
    xiao_runtime_error_clear();
}

#[test]
/// 未知标签、别名输出及非法布尔载荷必须在数学调用前拒绝。
fn all_entries_reject_unknown_tags_aliases_and_malformed_boolean() {
    for (name, entry, a, b, _) in entries() {
        let mut a = XiaoValue::float(a);
        let b = XiaoValue::float(b);
        let unknown = XiaoValue {
            tag: XiaoValueTag::from_raw(999),
            payload: XiaoValuePayload { raw: 0 },
        };
        let bad_bool = XiaoValue {
            tag: XiaoValueTag::Bool,
            payload: XiaoValuePayload { bool_value: 2 },
        };
        let mut out = owned();
        let handle = unsafe { out.payload.handle };
        sentinel();
        let before = pending();
        for invalid in [&unknown, &bad_bool] {
            assert_eq!(entry.call(invalid, &b, &mut out), 2, "{name}");
            if matches!(entry, Entry::Binary(_)) {
                assert_eq!(entry.call(&a, invalid, &mut out), 2, "{name}");
            }
        }
        let pointer = &mut a as *mut XiaoValue;
        assert_eq!(entry.call(pointer, &b, pointer), 2, "{name}");
        let mut bad_out = XiaoValue {
            tag: XiaoValueTag::from_raw(999),
            payload: XiaoValuePayload { raw: 73 },
        };
        assert_eq!(entry.call(&a, &b, &mut bad_out), 2, "{name}");
        assert_eq!(unsafe { bad_out.payload.raw }, 73);
        assert_unchanged(&out, handle);
        assert_eq!(pending(), before);
        xiao_runtime_value_release(&mut out);
    }
    xiao_runtime_error_clear();
}

#[test]
/// 空或错类句柄使用状态3，输出与挂起错误均保留。
fn all_entries_reject_missing_or_mismatched_handles_without_pending_changes() {
    for (name, entry, a, b, _) in entries() {
        let a = XiaoValue::float(a);
        let b = XiaoValue::float(b);
        let mut owner = owned();
        let wrong_kind = XiaoValue {
            tag: XiaoValueTag::Array,
            payload: XiaoValuePayload {
                handle: unsafe { owner.payload.handle },
            },
        };
        let mut out = owned();
        let handle = unsafe { out.payload.handle };
        sentinel();
        let before = pending();
        for tag in [
            XiaoValueTag::Str,
            XiaoValueTag::Lint,
            XiaoValueTag::Lfloat,
            XiaoValueTag::Array,
            XiaoValueTag::Tuple,
            XiaoValueTag::DictTable,
            XiaoValueTag::DictColumn,
            XiaoValueTag::Set,
            XiaoValueTag::Table,
            XiaoValueTag::ExternalResource,
            XiaoValueTag::Error,
            XiaoValueTag::TableDropView,
            XiaoValueTag::Concurrency,
        ] {
            let invalid = XiaoValue {
                tag,
                payload: XiaoValuePayload { raw: 0 },
            };
            assert_eq!(entry.call(&invalid, &b, &mut out), 3, "{name} tag={tag:?}");
            if matches!(entry, Entry::Binary(_)) {
                assert_eq!(entry.call(&a, &invalid, &mut out), 3, "{name}");
            }
        }
        assert_eq!(entry.call(&wrong_kind, &b, &mut out), 3, "{name}");
        assert_unchanged(&out, handle);
        assert_eq!(pending(), before);
        xiao_runtime_value_release(&mut out);
        xiao_runtime_value_release(&mut owner);
    }
    xiao_runtime_error_clear();
}

#[test]
/// 已知错误类型返回002，不先解析数值文本或执行数学。
fn all_entries_report_known_wrong_types_before_parsing_numeric_payloads() {
    for (name, entry, a, b, _) in entries() {
        let a = XiaoValue::float(a);
        let b = XiaoValue::float(b);
        for mut wrong in [
            XiaoValue::none(),
            XiaoValue::bool(false),
            XiaoValue::int(4),
            XiaoValue::sint(4),
            XiaoValue::sfloat(4.0),
            owned(),
        ] {
            let mut out = owned();
            let handle = unsafe { out.payload.handle };
            assert_eq!(entry.call(&wrong, &b, &mut out), 6, "{name}");
            assert_eq!(pending().0, "X06-RUNTIME-002");
            if matches!(entry, Entry::Binary(_)) {
                assert_eq!(entry.call(&a, &wrong, &mut out), 6, "{name}");
                assert_eq!(pending().0, "X06-RUNTIME-002");
            }
            assert_unchanged(&out, handle);
            xiao_runtime_value_release(&mut out);
            xiao_runtime_value_release(&mut wrong);
        }
        for tag in [XiaoValueTag::Lint, XiaoValueTag::Lfloat] {
            let mut wrong = owned();
            wrong.tag = tag;
            let mut out = owned();
            let handle = unsafe { out.payload.handle };
            assert_eq!(entry.call(&wrong, &b, &mut out), 6, "{name}");
            assert_eq!(
                pending().0,
                "X06-RUNTIME-002",
                "不可在类型失败前解析错误数值文本"
            );
            assert_unchanged(&out, handle);
            xiao_runtime_value_release(&mut wrong);
            xiao_runtime_value_release(&mut out);
        }
    }
    xiao_runtime_error_clear();
}

#[test]
/// 全部入口拒绝NaN/Inf并保持原拥有输出。
fn all_entries_reject_nonfinite_inputs_and_keep_owned_output_on_error() {
    for (name, entry, a, b, _) in entries() {
        let a = XiaoValue::float(a);
        let b = XiaoValue::float(b);
        let mut out = owned();
        let handle = unsafe { out.payload.handle };
        for number in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            let invalid = XiaoValue::float(number);
            assert_eq!(entry.call(&invalid, &b, &mut out), 6, "{name}");
            assert_eq!(pending().0, "X06-RUNTIME-009");
            if matches!(entry, Entry::Binary(_)) {
                assert_eq!(entry.call(&a, &invalid, &mut out), 6, "{name}");
                assert_eq!(pending().0, "X06-RUNTIME-009");
            }
            assert_unchanged(&out, handle);
        }
        xiao_runtime_value_release(&mut out);
    }
    xiao_runtime_error_clear();
}

#[test]
/// 二元复合错误先检查类型002，再检查非有限数学009。
fn binary_type_errors_precede_nonfinite_math_checks() {
    for (name, entry, _, _, _) in entries()
        .into_iter()
        .filter(|e| matches!(e.1, Entry::Binary(_)))
    {
        for (a, b) in [
            (XiaoValue::float(f64::INFINITY), XiaoValue::int(0)),
            (XiaoValue::int(0), XiaoValue::float(f64::NAN)),
        ] {
            let mut out = owned();
            let handle = unsafe { out.payload.handle };
            assert_eq!(entry.call(&a, &b, &mut out), 6, "{name}");
            assert_eq!(pending().0, "X06-RUNTIME-002");
            assert_unchanged(&out, handle);
            xiao_runtime_value_release(&mut out);
        }
    }
    xiao_runtime_error_clear();
}

#[test]
/// 非法out结构必须保持原载荷及既有挂起错误。
fn malformed_output_structure_preserves_pending_before_math() {
    for (name, entry, a, b, _) in entries() {
        let a = XiaoValue::float(a);
        let b = XiaoValue::float(b);
        let mut bad_handle = XiaoValue {
            tag: XiaoValueTag::Str,
            payload: XiaoValuePayload { raw: 0 },
        };
        let mut weak_output = XiaoValue {
            tag: XiaoValueTag::TableDropView,
            payload: XiaoValuePayload { raw: 0 },
        };
        sentinel();
        let before = pending();
        assert_eq!(entry.call(&a, &b, &mut bad_handle), 3, "{name}");
        assert_eq!(bad_handle.tag, XiaoValueTag::Str);
        assert!(unsafe { bad_handle.payload.handle }.is_null());
        assert_eq!(entry.call(&a, &b, &mut weak_output), 2, "{name}");
        assert_eq!(weak_output.tag, XiaoValueTag::TableDropView);
        assert_eq!(pending(), before);
    }
    xiao_runtime_error_clear();
}

#[test]
/// 纯float成功路径不触发字符串载荷分配失败点。
fn float_math_does_not_require_payload_allocation() {
    for (name, entry, a, b, _) in entries() {
        let a = XiaoValue::float(a);
        let b = XiaoValue::float(b);
        let mut out = XiaoValue::none();
        let failure = crate::allocation_testing::StringAllocationFailure::after(0);
        assert_eq!(entry.call(&a, &b, &mut out), 0, "{name}");
        assert!(!failure.was_triggered());
    }
}

#[test]
/// 逐条调用C ABI并核对MPFR参考的错误、零符号及ULP。
fn all_abi_results_match_formal_reference_errors_signs_and_ulp() {
    let reference = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../../docs/DevDocs/assets/20o-b2-b/entry-20261108-b3/b1/reference-vectors.tsv"
    ));
    let mut counts = std::collections::BTreeMap::new();
    for line in reference.lines().filter(|line| !line.starts_with('#')) {
        let row: Vec<_> = line.split('\t').collect();
        let name = row[1];
        let entry = entries()
            .into_iter()
            .find(|entry| entry.0 == name)
            .unwrap()
            .1;
        let parse = |text| f64::from_bits(u64::from_str_radix(text, 16).unwrap());
        let a = XiaoValue::float(parse(row[2]));
        let b = XiaoValue::float(if row[3] == "-" { 0.0 } else { parse(row[3]) });
        let mut out = XiaoValue::int(73);
        let status = entry.call(&a, &b, &mut out);
        if row[4] == "X06-RUNTIME-009" {
            assert_eq!(status, 6, "{} {name}", row[0]);
            assert_eq!(pending().0, "X06-RUNTIME-009");
            assert_eq!(out.tag, XiaoValueTag::Int);
            assert_eq!(unsafe { out.payload.i64_value }, 73);
        } else {
            assert_eq!(status, 0, "{} {name}", row[0]);
            assert_eq!(out.tag, XiaoValueTag::Float);
            let expected = parse(row[4]);
            let actual = unsafe { out.payload.f64_value };
            assert!(actual.is_finite());
            assert_eq!(actual == 0.0, expected == 0.0, "{} {name}", row[0]);
            assert_eq!(
                actual.is_sign_negative(),
                expected.is_sign_negative(),
                "{} {name}",
                row[0]
            );
            let exact = matches!(
                name,
                "sqrt" | "floor" | "ceil" | "round" | "trunc" | "abs" | "min" | "max"
            ) || expected == 0.0;
            assert!(
                actual.to_bits().abs_diff(expected.to_bits()) <= if exact { 0 } else { 4 },
                "{} {name}",
                row[0]
            );
        }
        *counts.entry(name).or_insert(0usize) += 1;
    }
    assert_eq!(counts.len(), 15);
    assert_eq!(counts.values().sum::<usize>(), 17243);
    for (name, cases) in counts {
        println!("B3-CMATH-ABI function={name} cases={cases} result=success");
    }
    xiao_runtime_error_clear();
}
