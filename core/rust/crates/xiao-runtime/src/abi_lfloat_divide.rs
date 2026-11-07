//! B2 的四参显式十进制除法 C 入口；结构错误不访问挂起错误槽。

use super::*;

/// 只读验证字符串承载的结构并借用原始文本，不提前执行数值接纳。
unsafe fn raw_string(value: &XiaoValue) -> Result<StringHandle, i32> {
    let boxed = unsafe { strong_ref(value.payload.handle) }
        .map_err(|_| XiaoAbiStatus::InvalidHandle.code())?;
    let inner = boxed.inners.borrow();
    let handle = inner.last().ok_or(XiaoAbiStatus::InvalidHandle.code())?;
    if handle.type_tag() != RuntimeTypeTag::String {
        return Err(XiaoAbiStatus::InvalidHandle.code());
    }
    let cloned = handle
        .try_clone()
        .map_err(|_| XiaoAbiStatus::InvalidHandle.code())?;
    StringHandle::from_strong_handle(cloned).map_err(|_| XiaoAbiStatus::InvalidHandle.code())
}

/// 四个输入槽借读且调用期间保持强引用；out 已初始化、拥有并与输入不重叠。
///
/// 0 成功释放替换 out；1 空指针、2 非法标签、3 坏句柄不动 out 和挂起槽；
/// 6 语义失败保留 out 并沿既有错误通道传播；成功不清除既有挂起错误。
#[unsafe(no_mangle)]
pub extern "C" fn xiao_runtime_lfloat_divide(
    numerator: *const XiaoValue,
    denominator: *const XiaoValue,
    precision: *const XiaoValue,
    rounding: *const XiaoValue,
    out: *mut XiaoValue,
) -> i32 {
    let pointers = [numerator, denominator, precision, rounding];
    if out.is_null() || pointers.iter().any(|ptr| ptr.is_null()) {
        return XiaoAbiStatus::Null.code();
    }
    // 调用方保证 out 不与输入重叠；精确同址也在释放前拒绝。
    if pointers.iter().any(|ptr| *ptr == out.cast_const()) {
        return XiaoAbiStatus::InvalidArgument.code();
    }
    let values = pointers.map(|ptr| unsafe { &*ptr });
    if values.iter().any(|value| !value.tag.is_known()) || !unsafe { (*out).tag }.is_known() {
        return XiaoAbiStatus::InvalidArgument.code();
    }
    let mut strings: [Option<StringHandle>; 4] = std::array::from_fn(|_| None);
    for (index, value) in values.iter().enumerate() {
        if matches!(
            value.tag,
            XiaoValueTag::Str | XiaoValueTag::Lint | XiaoValueTag::Lfloat
        ) {
            strings[index] = match unsafe { raw_string(value) } {
                Ok(value) => Some(value),
                Err(status) => return status,
            };
        }
    }
    let expected = [
        XiaoValueTag::Lfloat,
        XiaoValueTag::Lfloat,
        XiaoValueTag::Int,
        XiaoValueTag::Str,
    ];
    if values
        .iter()
        .zip(expected)
        .any(|(value, expected)| value.tag != expected)
    {
        return status_from_error(&RuntimeError::type_mismatch(
            "lfloat, lfloat, int, str",
            "lfloat_divide 实参标签",
        ));
    }
    // 借用视图保持到数学调用完成；语义校验前不复制数值原文。
    let numerator = strings[0].as_ref().expect("已验证numerator");
    let denominator = strings[1].as_ref().expect("已验证denominator");
    let rounding = strings[3].as_ref().expect("已验证rounding");
    let precision = unsafe { values[2].payload.i64_value };
    let result = numerator
        .with_str(|n| {
            denominator
                .with_str(|d| {
                    rounding
                        .with_str(|r| crate::value::lfloat_divide_text(n, d, precision, r))
                        .and_then(std::convert::identity)
                })
                .and_then(std::convert::identity)
        })
        .and_then(std::convert::identity);
    let value = match result {
        Ok(value) => match runtime_to_value(&value) {
            Ok(value) => value,
            Err(status) => return status,
        },
        Err(error) => return status_from_error(&error),
    };
    match unsafe { write_value(out, value) } {
        Ok(()) => 0,
        Err(status) => status,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 创建和销毁测试拥有槽，所有错误路径都保留旧输出。
    fn input(text: &str) -> XiaoValue {
        runtime_to_value(&RuntimeValue::new_lfloat(text).unwrap()).unwrap()
    }
    fn mode(text: &str) -> XiaoValue {
        runtime_to_value(&RuntimeValue::new_string(text).unwrap()).unwrap()
    }
    fn code() -> String {
        PENDING_ERROR.with(|slot| match slot.borrow().as_ref().unwrap() {
            PendingError::Recoverable(e) => e.code().into(),
            _ => panic!("可恢复错误"),
        })
    }

    #[test]
    /// 外部C原始数值载荷按参数域、左输入、右输入、除零与结果预算的顺序验收。
    fn external_raw_text_keeps_error_order_and_ownership() {
        fn raw(text: &str) -> XiaoValue {
            let mut value = runtime_to_value(&RuntimeValue::new_string(text).unwrap()).unwrap();
            value.tag = XiaoValueTag::Lfloat;
            value
        }
        for (left, right, precision, rounding, expected) in [
            ("bad", "0", 0, "bad", "X06-RUNTIME-012"),
            ("1e100001", "bad", 1, "half_even", "X06-RUNTIME-033"),
            ("bad", "1e100001", 1, "half_even", "X06-RUNTIME-012"),
            ("0", "bad", 1, "half_even", "X06-RUNTIME-012"),
            ("0", "1e100001", 1, "half_even", "X06-RUNTIME-033"),
            ("1e100000", "1e-100000", 1, "half_even", "X06-RUNTIME-033"),
        ] {
            xiao_runtime_error_clear();
            let mut a = raw(left);
            let mut b = raw(right);
            let mut m = mode(rounding);
            let p = XiaoValue::int(precision);
            let mut out = mode("unchanged");
            assert_eq!(xiao_runtime_lfloat_divide(&a, &b, &p, &m, &mut out), 6);
            assert_eq!(code(), expected);
            assert_eq!(
                unsafe { value_to_runtime(&out) }
                    .unwrap()
                    .display_text()
                    .unwrap(),
                "unchanged"
            );
            for value in [&mut a, &mut b, &mut m, &mut out] {
                xiao_runtime_value_release(value);
            }
        }
        xiao_runtime_error_clear();
    }

    #[test]
    fn replaces_owned_output_and_preserves_pending_on_success() {
        let mut a = input("1.00");
        let mut b = input("2");
        let p = XiaoValue::int(32768);
        let mut m = mode("half_even");
        let mut out = mode("old");
        set_pending_runtime_error(RuntimeError::invalid_value("sentinel"));
        assert_eq!(xiao_runtime_lfloat_divide(&a, &b, &p, &m, &mut out), 0);
        assert_eq!(
            unsafe { value_to_runtime(&out) }
                .unwrap()
                .display_text()
                .unwrap(),
            "0.5"
        );
        assert_eq!(code(), "X06-RUNTIME-012");
        for value in [&mut a, &mut b, &mut m, &mut out] {
            xiao_runtime_value_release(value);
        }
        xiao_runtime_error_clear();
    }

    #[test]
    fn parameter_errors_precede_zero_and_all_failures_preserve_output() {
        for (precision, rounding, denominator, expected) in [
            (0, "invalid", "0", "X06-RUNTIME-012"),
            (1, "invalid", "0", "X06-RUNTIME-012"),
            (1, "half_even", "0", "X06-RUNTIME-013"),
        ] {
            xiao_runtime_error_clear();
            let mut a = input("1");
            let mut b = input(denominator);
            let p = XiaoValue::int(precision);
            let mut m = mode(rounding);
            let mut out = XiaoValue::int(73);
            assert_eq!(xiao_runtime_lfloat_divide(&a, &b, &p, &m, &mut out), 6);
            assert_eq!(unsafe { out.payload.i64_value }, 73);
            assert_eq!(code(), expected);
            for value in [&mut a, &mut b, &mut m] {
                xiao_runtime_value_release(value);
            }
        }
        xiao_runtime_error_clear();
    }

    #[test]
    fn structural_failures_do_not_touch_pending_or_output() {
        let mut a = input("1");
        let mut b = input("3");
        let p = XiaoValue::int(3);
        let mut m = mode("half_even");
        let mut out = XiaoValue::int(73);
        set_pending_runtime_error(RuntimeError::invalid_value("sentinel"));
        assert_eq!(
            xiao_runtime_lfloat_divide(std::ptr::null(), &b, &p, &m, &mut out),
            1
        );
        assert_eq!(
            xiao_runtime_lfloat_divide(&a, &b, &p, &m, std::ptr::null_mut()),
            1
        );
        let mut unknown = XiaoValue::none();
        unknown.tag = XiaoValueTag::from_raw(999);
        assert_eq!(
            xiao_runtime_lfloat_divide(&unknown, &b, &p, &m, &mut out),
            2
        );
        let mut invalid = XiaoValue::none();
        invalid.tag = XiaoValueTag::Lfloat;
        assert_eq!(
            xiao_runtime_lfloat_divide(&invalid, &b, &p, &m, &mut out),
            3
        );
        assert_eq!(unsafe { out.payload.i64_value }, 73);
        assert_eq!(code(), "X06-RUNTIME-012");
        xiao_runtime_error_clear();
        assert_eq!(
            xiao_runtime_lfloat_divide(&a, &b, &XiaoValue::sint(3), &m, &mut out),
            6
        );
        assert_eq!(code(), "X06-RUNTIME-002");
        xiao_runtime_error_clear();
        let failure = crate::allocation_testing::StringAllocationFailure::after(0);
        assert_eq!(xiao_runtime_lfloat_divide(&a, &b, &p, &m, &mut out), 6);
        assert!(failure.was_triggered());
        assert_eq!(code(), "X06-RUNTIME-011");
        assert_eq!(unsafe { out.payload.i64_value }, 73);
        drop(failure);
        for value in [&mut a, &mut b, &mut m] {
            xiao_runtime_value_release(value);
        }
        xiao_runtime_error_clear();
    }
}
