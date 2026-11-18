//! B3的15个cmath C入口；结构校验只读，数学规则复用Runtime共享内核。

use super::*;
use crate::value::arithmetic::cmath;

/// 只校验载荷结构和句柄种类，不解析lint/lfloat文本或读写挂起错误。
unsafe fn validate_structure(value: &XiaoValue, owned: bool) -> Result<(), i32> {
    let invalid = XiaoAbiStatus::InvalidHandle.code();
    let expected = match value.tag {
        XiaoValueTag::None
        | XiaoValueTag::Int
        | XiaoValueTag::Sint
        | XiaoValueTag::Float
        | XiaoValueTag::Sfloat => return Ok(()),
        XiaoValueTag::Bool => {
            return if unsafe { value.payload.bool_value } <= 1 {
                Ok(())
            } else {
                Err(XiaoAbiStatus::InvalidArgument.code())
            };
        }
        XiaoValueTag::Str | XiaoValueTag::Lint | XiaoValueTag::Lfloat => RuntimeTypeTag::String,
        XiaoValueTag::Table => RuntimeTypeTag::Table,
        XiaoValueTag::Array => RuntimeTypeTag::Array,
        XiaoValueTag::Tuple => RuntimeTypeTag::Tuple,
        XiaoValueTag::DictTable => RuntimeTypeTag::DictTable,
        XiaoValueTag::DictColumn => RuntimeTypeTag::DictColumn,
        XiaoValueTag::Set => RuntimeTypeTag::Set,
        XiaoValueTag::ExternalResource => EXTERNAL_RESOURCE_RUNTIME_TAG,
        XiaoValueTag::Error => {
            let boxed = unsafe { error_ref(value.payload.handle) }.map_err(|_| invalid)?;
            return if boxed.inners.borrow().is_empty() {
                Err(invalid)
            } else {
                Ok(())
            };
        }
        XiaoValueTag::TableDropView => {
            if owned {
                return Err(XiaoAbiStatus::InvalidArgument.code());
            }
            let boxed = unsafe { weak_ref(value.payload.weak_handle) }.map_err(|_| invalid)?;
            return if boxed
                .inners
                .borrow()
                .last()
                .is_some_and(|h| h.type_tag() == RuntimeTypeTag::Table)
            {
                Ok(())
            } else {
                Err(invalid)
            };
        }
        XiaoValueTag::Concurrency => {
            let kind = abi_concurrency::xiao_runtime_concurrency_kind(
                unsafe { value.payload.handle }.cast(),
            );
            return if kind == 0 { Err(invalid) } else { Ok(()) };
        }
        _ => return Err(XiaoAbiStatus::InvalidArgument.code()),
    };
    let boxed = unsafe { strong_ref(value.payload.handle) }.map_err(|_| invalid)?;
    if boxed
        .inners
        .borrow()
        .last()
        .is_some_and(|h| h.type_tag() == expected)
    {
        Ok(())
    } else {
        Err(invalid)
    }
}

/// 统一执行结构、目标标签、数学与拥有输出边界；不在ABI层新增数值转换。
fn execute<const N: usize>(
    inputs: [*const XiaoValue; N],
    out: *mut XiaoValue,
    operation: impl FnOnce([f64; N]) -> RuntimeResult<f64>,
) -> i32 {
    if out.is_null() || inputs.iter().any(|p| p.is_null()) {
        return XiaoAbiStatus::Null.code();
    }
    if inputs.iter().any(|p| *p == out.cast_const()) {
        return XiaoAbiStatus::InvalidArgument.code();
    }
    let values = inputs.map(|p| unsafe { &*p });
    if values.iter().any(|v| !v.tag.is_known()) || !unsafe { (*out).tag }.is_known() {
        return XiaoAbiStatus::InvalidArgument.code();
    }
    for value in values {
        if let Err(status) = unsafe { validate_structure(value, false) } {
            return status;
        }
    }
    if let Err(status) = unsafe { validate_structure(&*out, true) } {
        return status;
    }
    if let Some(value) = values.iter().find(|v| v.tag != XiaoValueTag::Float) {
        let actual = match value.tag {
            XiaoValueTag::None => "none",
            XiaoValueTag::Bool => "bool",
            XiaoValueTag::Int => "int",
            XiaoValueTag::Sint => "sint",
            XiaoValueTag::Lint => "lint",
            XiaoValueTag::Sfloat => "sfloat",
            XiaoValueTag::Lfloat => "lfloat",
            XiaoValueTag::Str => "str",
            XiaoValueTag::Array => "array",
            XiaoValueTag::Tuple => "tuple",
            XiaoValueTag::DictTable | XiaoValueTag::DictColumn => "dict",
            XiaoValueTag::Set => "set",
            XiaoValueTag::Table | XiaoValueTag::TableDropView => "table",
            XiaoValueTag::Error => "Error",
            XiaoValueTag::ExternalResource => "resource",
            XiaoValueTag::Concurrency => "concurrency",
            _ => unreachable!("已校验结构并排除float"),
        };
        return status_from_error(&RuntimeError::type_mismatch("float", actual));
    }
    let values = values.map(|v| unsafe { v.payload.f64_value });
    let result = match operation(values) {
        Ok(value) => XiaoValue::float(value),
        Err(error) => return status_from_error(&error),
    };
    match unsafe { write_value(out, result) } {
        Ok(()) => XiaoAbiStatus::Ok.code(),
        Err(status) => status,
    }
}

macro_rules! unary {
    ($symbol:ident, $kernel:ident) => {
        /// 借读float输入，成功释放替换拥有out；失败保留out。
        /// 1/2/3为结构错误且不碰挂起槽；类型002、域/非有限009走6；成功不清挂起错误。
        #[unsafe(no_mangle)]
        pub extern "C" fn $symbol(value: *const XiaoValue, out: *mut XiaoValue) -> i32 {
            execute([value], out, |[value]| cmath::$kernel(value))
        }
    };
}

macro_rules! binary {
    ($symbol:ident, $kernel:ident, $left:ident, $right:ident) => {
        /// 借读两个float输入，成功释放替换拥有out；失败保留out。
        /// 输入强引用覆盖调用，out与输入不重叠；结构错误不碰挂起槽，数学错误正常传播。
        #[unsafe(no_mangle)]
        pub extern "C" fn $symbol(
            $left: *const XiaoValue,
            $right: *const XiaoValue,
            out: *mut XiaoValue,
        ) -> i32 {
            execute([$left, $right], out, |[left, right]| {
                cmath::$kernel(left, right)
            })
        }
    };
}

unary!(xiao_runtime_cmath_sqrt, sqrt);
unary!(xiao_runtime_cmath_floor, floor);
unary!(xiao_runtime_cmath_ceil, ceil);
unary!(xiao_runtime_cmath_round, round);
unary!(xiao_runtime_cmath_trunc, trunc);
unary!(xiao_runtime_cmath_abs, abs);
binary!(xiao_runtime_cmath_min, min, left, right);
binary!(xiao_runtime_cmath_max, max, left, right);
binary!(xiao_runtime_cmath_pow, pow, base, exponent);
unary!(xiao_runtime_cmath_sin, sin);
unary!(xiao_runtime_cmath_cos, cos);
unary!(xiao_runtime_cmath_tan, tan);
unary!(xiao_runtime_cmath_exp, exp);
unary!(xiao_runtime_cmath_ln, ln);
unary!(xiao_runtime_cmath_log10, log10);

/// C ABI边界与正式参考的真实符号回归。
#[cfg(test)]
#[path = "abi_cmath_tests.rs"]
mod tests;
