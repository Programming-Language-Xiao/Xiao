//! A1数值文本接纳的既有C构造/恢复通路；不增加符号或状态。

use super::*;

/// 分配输入字符串句柄，独立于数值构造器与挂起槽。
fn text_handle(text: &str) -> XiaoHandle {
    let mut handle = std::ptr::null_mut();
    assert_eq!(
        xiao_runtime_string_new(
            XiaoAbiBytes {
                ptr: text.as_ptr(),
                len: text.len()
            },
            &mut handle
        ),
        0
    );
    handle
}

/// 只读检查本线程当前正常传播的可恢复错误。
fn pending_code() -> String {
    PENDING_ERROR.with(
        |pending| match pending.borrow().as_ref().expect("预期挂起错误") {
            PendingError::Recoverable(error) => error.code().to_owned(),
            PendingError::Fatal(_) => panic!("数值接纳不应制造Fatal"),
        },
    )
}

#[test]
/// 既有借用/拥有构造器恢复为同一数值；显示精度各自保留且成功不清挂起槽。
fn numeric_handles_restore_shared_identity_without_mutating_inputs() {
    xiao_runtime_error_clear();
    let handle = text_handle("+0001.50_0e+00");
    let mut borrowed = xiao_runtime_value_lfloat(handle);
    set_pending_runtime_error(RuntimeError::invalid_value("sentinel"));
    let mut owned = xiao_runtime_value_lfloat_owned(text_handle("15e-1"));
    assert_eq!(pending_code(), crate::INVALID_VALUE_CODE);
    let first = unsafe { value_to_runtime(&borrowed) }.unwrap();
    let second = unsafe { value_to_runtime(&owned) }.unwrap();
    assert_eq!(first, second);
    assert_eq!(first.display_text().unwrap(), "1.500");
    assert_eq!(second.display_text().unwrap(), "1.5");
    let original = unsafe { expect_strong(handle, RuntimeTypeTag::String) }.unwrap();
    assert_eq!(
        StringHandle::from_strong_handle(original)
            .unwrap()
            .to_string()
            .unwrap(),
        "+0001.50_0e+00"
    );
    xiao_runtime_value_release(&mut borrowed);
    xiao_runtime_value_release(&mut owned);
    xiao_runtime_release(handle);
    xiao_runtime_error_clear();
}

#[test]
/// 原有无状态返回构造器沿现有挂起槽报告012/033；结构失败保留旧哨兵。
fn constructors_reject_semantic_payloads_through_existing_pending_errors() {
    for (text, expected) in [
        ("bad", crate::INVALID_VALUE_CODE),
        ("0_._0e1", crate::INVALID_VALUE_CODE),
        ("1e100001", crate::NUMERIC_LIMIT_CODE),
    ] {
        xiao_runtime_error_clear();
        let value = xiao_runtime_value_lfloat_owned(text_handle(text));
        assert_eq!(value.tag, XiaoValueTag::None);
        assert_eq!(pending_code(), expected);
    }
    xiao_runtime_error_clear();
    set_pending_runtime_error(RuntimeError::numeric_limit("sentinel"));
    assert_eq!(
        xiao_runtime_value_lint_owned(std::ptr::null_mut()).tag,
        XiaoValueTag::None
    );
    assert_eq!(pending_code(), crate::NUMERIC_LIMIT_CODE);
    xiao_runtime_error_clear();
}

#[test]
/// 外部有效标签也须验证数值载荷，失败状态6且不修改现有out或输入。
fn borrowed_value_restore_preserves_output_on_numeric_failure() {
    xiao_runtime_error_clear();
    let handle = text_handle("1e100001");
    // 模拟外部C直接装配的有效布局；恢复入口必须再次校验，不能信任标签。
    let mut input = value_from_owned_handle(
        XiaoValueTag::Lfloat,
        unsafe { take_strong(handle, RuntimeTypeTag::String) }.unwrap(),
    );
    let mut output = XiaoValue::int(73);
    assert_eq!(
        xiao_runtime_value_cast(
            XiaoAbiBytes {
                ptr: b"str".as_ptr(),
                len: 3
            },
            &input,
            &mut output
        ),
        6
    );
    assert_eq!(pending_code(), crate::NUMERIC_LIMIT_CODE);
    assert_eq!(output.tag, XiaoValueTag::Int);
    assert_eq!(unsafe { output.payload.i64_value }, 73);
    assert_eq!(input.tag, XiaoValueTag::Lfloat);
    xiao_runtime_value_release(&mut input);
    xiao_runtime_value_release(&mut output);
    xiao_runtime_error_clear();
}

#[test]
/// 033/034复用既有状态6；诊断身份不同且失败不释放调用方的旧值。
fn frozen_numeric_errors_reuse_existing_runtime_status() {
    xiao_runtime_error_clear();
    assert_eq!(status_from_error(&RuntimeError::numeric_limit("limit")), 6);
    assert_eq!(pending_code(), crate::NUMERIC_LIMIT_CODE);
    assert_eq!(
        status_from_error(&RuntimeError::non_terminating_division()),
        6
    );
    // 既有挂起槽保留首个错误；不得为验证034而改写正常错误传播规则。
    assert_eq!(pending_code(), crate::NUMERIC_LIMIT_CODE);
    xiao_runtime_error_clear();
    assert_eq!(
        status_from_error(&RuntimeError::non_terminating_division()),
        6
    );
    assert_eq!(pending_code(), crate::NON_TERMINATING_DIVISION_CODE);
    xiao_runtime_error_clear();
}
