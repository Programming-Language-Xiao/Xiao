//! A6资源/错误原子性：固定边界，不在默认debug测试中运行性能基准。

use xiao_runtime::RuntimeValue;
use xiao_runtime_abi::*;
use xiao_syntax::{BinaryOperator, ScalarType};
use xiao_types::numeric_text::NUMERIC_DIGIT_LIMIT as D;

/// 可恢复数值身份直接经已签接纳构造。
fn decimal(text: &str) -> RuntimeValue {
    RuntimeValue::new_lfloat(text).unwrap()
}

#[test]
/// 原文→完整语法→系数/scale预算；失败不改输入，零不跳过指数检查。
fn admission_boundaries_and_error_order_preserve_inputs() {
    for (text, code) in [
        ("x".repeat(65537), "X06-RUNTIME-033"),
        ("9".repeat(D + 1) + "x", "X06-RUNTIME-012"),
        ("9".repeat(D + 1), "X06-RUNTIME-033"),
        ("0e100001".into(), "X06-RUNTIME-033"),
        ("1e-100001".into(), "X06-RUNTIME-033"),
    ] {
        let value = RuntimeValue::new_string(text.clone()).unwrap();
        assert_eq!(
            value.convert_to(ScalarType::Lfloat).unwrap_err().code(),
            code
        );
        assert_eq!(value.display_text().unwrap(), text);
    }
    assert_eq!(
        RuntimeValue::new_string("0".repeat(65536))
            .unwrap()
            .convert_to(ScalarType::Lfloat)
            .unwrap()
            .display_text()
            .unwrap(),
        "0"
    );
    assert_eq!(decimal("1e100000").display_text().unwrap(), "1e100000");
}

#[test]
/// 最大系数的真实D+1对齐强消去按实际结果接纳，非平凡平衡乘法触达D。
fn actual_results_accept_strong_cancellation_and_balanced_products() {
    let large = decimal(&format!("1e{D}"));
    let below = decimal(&"9".repeat(D));
    assert_eq!(large.subtract(&below).unwrap().display_text().unwrap(), "1");
    assert_eq!(below.display_text().unwrap(), "9".repeat(D));
    let n = D / 2;
    let value = RuntimeValue::new_lint(&"9".repeat(n)).unwrap();
    let expected = "9".repeat(n - 1) + "8" + &"0".repeat(n - 1) + "1";
    assert_eq!(
        value.multiply(&value).unwrap().display_text().unwrap(),
        expected
    );
    let over = RuntimeValue::new_lint(&"9".repeat(n + 1)).unwrap();
    assert_eq!(over.multiply(&over).unwrap_err().code(), "X06-RUNTIME-033");
}

#[test]
/// 商与余数独立接纳，034先于展开预算；D+1异号余数必须实际求差。
fn quotient_remainder_and_nontermination_have_independent_budgets() {
    let large = decimal("1e100000");
    let tiny = decimal("1e-100000");
    assert_eq!(
        large.floor_divide(&tiny).unwrap_err().code(),
        "X06-RUNTIME-033"
    );
    assert_eq!(large.remainder(&tiny).unwrap().display_text().unwrap(), "0");
    assert_eq!(
        large
            .remainder(&decimal("3e-100000"))
            .unwrap()
            .display_text()
            .unwrap(),
        "1e-100000"
    );
    assert_eq!(
        large.divide(&decimal("3e-100000")).unwrap_err().code(),
        "X06-RUNTIME-034"
    );
    let negative = decimal(&format!("-{}", "9".repeat(D)));
    assert_eq!(
        negative
            .remainder(&decimal(&format!("1e{D}")))
            .unwrap()
            .display_text()
            .unwrap(),
        "1"
    );
    assert_eq!(
        negative
            .remainder(&decimal(&format!("2e{D}")))
            .unwrap_err()
            .code(),
        "X06-RUNTIME-033"
    );
}

#[test]
/// 精确结果位数而非松估计决定幂接纳，单位底数仍先检查指数域。
fn power_and_conversion_limits_do_not_expand_unbounded_values() {
    let two = RuntimeValue::new_lint("2").unwrap();
    assert_eq!(
        two.power(&RuntimeValue::new_lint("108852").unwrap())
            .unwrap()
            .display_text()
            .unwrap()
            .len(),
        D
    );
    assert_eq!(
        two.power(&RuntimeValue::new_lint("108853").unwrap())
            .unwrap_err()
            .code(),
        "X06-RUNTIME-033"
    );
    assert_eq!(
        decimal("1").power(&decimal("-1")).unwrap_err().code(),
        "X06-RUNTIME-009"
    );
    assert_eq!(
        decimal("1e32768")
            .convert_to(ScalarType::Lint)
            .unwrap_err()
            .code(),
        "X06-RUNTIME-033"
    );
    assert_eq!(
        decimal("1e100000")
            .convert_to(ScalarType::Float)
            .unwrap_err()
            .code(),
        "X06-RUNTIME-009"
    );
    assert_eq!(
        decimal("-1e-100000")
            .convert_to(ScalarType::Float)
            .unwrap()
            .display_text()
            .unwrap(),
        "-0"
    );
}

#[test]
/// 动态类型先于输入预算，两侧预算先于非有限，非有限先于除零。
fn mixed_error_priority_is_stable_at_the_language_entry() {
    let invalid = RuntimeValue::Lint("9".repeat(D + 1));
    assert_eq!(
        invalid
            .language_binary(BinaryOperator::Add, &RuntimeValue::Bool(false))
            .unwrap_err()
            .code(),
        "X06-RUNTIME-002"
    );
    let infinite = RuntimeValue::Float(f64::INFINITY);
    assert_eq!(
        infinite
            .language_binary(BinaryOperator::Divide, &invalid)
            .unwrap_err()
            .code(),
        "X06-RUNTIME-033"
    );
    assert_eq!(
        infinite
            .language_binary(
                BinaryOperator::Divide,
                &RuntimeValue::new_lint("0").unwrap()
            )
            .unwrap_err()
            .code(),
        "X06-RUNTIME-009"
    );
    assert_eq!(
        decimal("1").divide(&decimal("0")).unwrap_err().code(),
        "X06-RUNTIME-013"
    );
}

/// 测试持有的拥有ABI值恰好归还一次。
struct Owned(XiaoValue);
impl Drop for Owned {
    fn drop(&mut self) {
        unsafe { xiao_runtime_value_release_strong(&mut self.0) }
    }
}

/// 测试输入UTF-8视图只在同步调用期间借读。
fn bytes(text: &str) -> XiaoAbiBytes {
    XiaoAbiBytes {
        ptr: text.as_ptr(),
        len: text.len(),
    }
}

impl Owned {
    fn text(text: &str, kind: &str) -> Self {
        let mut handle = std::ptr::null_mut();
        assert_eq!(
            unsafe { xiao_runtime_string_new(bytes(text), &mut handle) },
            0
        );
        let value = unsafe {
            match kind {
                "lint" => xiao_runtime_value_lint_owned(handle),
                "lfloat" => xiao_runtime_value_lfloat_owned(handle),
                "str" => xiao_runtime_value_str_owned(handle),
                _ => panic!("未知测试类型"),
            }
        };
        Self(value)
    }
    fn display(&self) -> String {
        let mut text = Owned(XiaoValue::none());
        assert_eq!(
            unsafe { xiao_runtime_value_cast(bytes("str"), &self.0, &mut text.0) },
            0
        );
        let mut buffer = vec![0; 65536];
        let mut written = 0;
        assert_eq!(
            unsafe {
                xiao_runtime_string_copy(
                    text.0.payload.handle,
                    XiaoAbiMutBytes {
                        ptr: buffer.as_mut_ptr(),
                        capacity: buffer.len(),
                    },
                    &mut written,
                )
            },
            0
        );
        String::from_utf8(buffer[..written].to_vec()).unwrap()
    }
}

/// 只在测试边界清除挂起槽，不用该槽模拟error_code。
struct Pending;
impl Drop for Pending {
    fn drop(&mut self) {
        unsafe { xiao_runtime_error_clear() }
    }
}

/// 读取现有快照并立即复制借用code；不修改挂起错误。
fn pending() -> (u64, String) {
    let mut snapshot = unsafe { std::mem::zeroed::<XiaoAbiErrorSnapshot>() };
    assert_eq!(unsafe { xiao_runtime_error_snapshot(&mut snapshot) }, 0);
    let code = if snapshot.code.len == 0 {
        String::new()
    } else {
        String::from_utf8(
            unsafe { std::slice::from_raw_parts(snapshot.code.ptr, snapshot.code.len) }.to_vec(),
        )
        .unwrap()
    };
    (snapshot.error_id, code)
}

#[test]
/// 有效ABI输入的语义失败保持拥有out与两侧输入，不返回部分结果。
fn abi_semantic_failures_preserve_owned_out_and_inputs() {
    let _pending = Pending;
    for (operation, left, right, code) in [
        ("div", "1", "0", "X06-RUNTIME-013"),
        ("div", "1", "3", "X06-RUNTIME-034"),
        ("floor_div", "1e100000", "1e-100000", "X06-RUNTIME-033"),
        ("pow", "2", "-1", "X06-RUNTIME-009"),
    ] {
        unsafe {
            xiao_runtime_error_clear();
        }
        let left = Owned::text(left, "lfloat");
        let right = Owned::text(right, "lfloat");
        let mut out = Owned::text("old-out", "str");
        let left_before = left.display();
        let right_before = right.display();
        let old = unsafe { out.0.payload.handle };
        assert_eq!(
            unsafe { xiao_runtime_value_binary(bytes(operation), &left.0, &right.0, &mut out.0) },
            6
        );
        assert_eq!(out.0.tag, XiaoValueTag::Str);
        assert_eq!(unsafe { out.0.payload.handle }, old);
        assert_eq!(pending().1, code);
        assert_eq!(out.display(), "old-out");
        assert_eq!(left.display(), left_before);
        assert_eq!(right.display(), right_before);
    }
}

#[test]
/// 真正消费定向011注入，失败不改变旧拥有out或借用数值源。
fn controlled_allocation_failure_is_distinct_from_numeric_limit() {
    let _pending = Pending;
    unsafe {
        xiao_runtime_error_clear();
    }
    let input = Owned::text("1.50", "lfloat");
    let mut out = Owned::text("old-out", "str");
    let old = unsafe { out.0.payload.handle };
    let fail = xiao_runtime::allocation_testing::StringAllocationFailure::after(0);
    assert_eq!(
        unsafe { xiao_runtime_value_cast(bytes("str"), &input.0, &mut out.0) },
        6
    );
    assert!(fail.was_triggered());
    assert_eq!(pending().1, "X06-RUNTIME-011");
    assert_eq!(unsafe { out.0.payload.handle }, old);
    assert_eq!(out.display(), "old-out");
    assert_eq!(input.display(), "1.50");
}

#[test]
/// 成功替换旧out、归还其强引用，并保留原有挂起错误身份。
fn successful_abi_replacement_releases_old_out_without_clearing_pending() {
    let _pending = Pending;
    unsafe {
        xiao_runtime_error_clear();
    }
    let left = Owned::text("1.50", "lfloat");
    let right = Owned::text("2", "lfloat");
    let mut out = Owned::text("old-out", "str");
    let weak = unsafe { xiao_runtime_weak(out.0.payload.handle) };
    assert!(!weak.is_null());
    assert_eq!(
        unsafe {
            xiao_runtime_error_raise_type(
                bytes("Error"),
                bytes("A6_ORIGINAL"),
                bytes("original"),
                XiaoAbiErrorLocation::none(),
            )
        },
        0
    );
    let before = pending();
    assert_eq!(
        unsafe { xiao_runtime_value_binary(bytes("add"), &left.0, &right.0, &mut out.0) },
        0
    );
    assert_eq!(pending(), before);
    assert_eq!(out.display(), "3.50");
    let revived = unsafe { xiao_runtime_weak_upgrade(weak) };
    assert!(revived.is_null(), "旧out还持有强引用");
    unsafe {
        xiao_runtime_weak_release(weak);
    }
}
