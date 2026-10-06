//! Dynamic checks, iteration, and string ABI entry points.

use super::{
    box_strong, expect_strong, mut_bytes, operation_name, runtime_to_value, status, utf8,
    value_to_runtime, write_handle, write_value,
};
use crate::containers::is_hashable;
use crate::errors::RuntimeError;
use crate::memory::RuntimeTypeTag;
use crate::value::{RuntimeValue, StringHandle};
use xiao_runtime_abi::{XiaoAbiBytes, XiaoAbiMutBytes, XiaoAbiStatus, XiaoHandle, XiaoValue};
use xiao_syntax::ScalarType;

/// 执行一个前端登记的动态检查。
#[unsafe(no_mangle)]
pub extern "C" fn xiao_runtime_dynamic_check(kind: XiaoAbiBytes, value: *const XiaoValue) -> i32 {
    if value.is_null() {
        return XiaoAbiStatus::Null.code();
    }
    let kind = match unsafe { operation_name(kind) } {
        Ok(kind) => kind,
        Err(error) => return error,
    };
    let value = match unsafe { value_to_runtime(&*value) } {
        Ok(value) => value,
        Err(error) => return error,
    };
    let result = match kind.as_str() {
        "string_boolean" => match value.convert_to(ScalarType::Bool) {
            Ok(_) => Ok(()),
            Err(_) => Err(RuntimeError::type_mismatch("bool", value.type_name())),
        },
        "iterable" => match value {
            RuntimeValue::Array(_)
            | RuntimeValue::Tuple(_)
            | RuntimeValue::Str(_)
            | RuntimeValue::Set(_)
            | RuntimeValue::DictTable(_)
            | RuntimeValue::DictColumn(_) => Ok(()),
            other => Err(RuntimeError::iterable_required(other.type_name())),
        },
        "selector_step" => match value {
            RuntimeValue::Int(0) | RuntimeValue::Sint(0) => {
                Err(RuntimeError::selector_step("选择器步长不能为 0"))
            }
            RuntimeValue::Int(_) | RuntimeValue::Sint(_) => Ok(()),
            other => Err(RuntimeError::selector_step(format!(
                "选择器步长必须是整数，实际为 {}",
                other.type_name()
            ))),
        },
        "random_count" => match value {
            RuntimeValue::Int(value) if value >= 0 => Ok(()),
            RuntimeValue::Sint(value) if value >= 0 => Ok(()),
            RuntimeValue::Int(_) | RuntimeValue::Sint(_) => {
                Err(RuntimeError::random_count("随机抽取数量不能为负数"))
            }
            other => Err(RuntimeError::random_count(format!(
                "随机抽取数量必须是整数，实际为 {}",
                other.type_name()
            ))),
        },
        "random_seed" => match value {
            RuntimeValue::Int(value) if value >= 0 => Ok(()),
            RuntimeValue::Sint(value) if value >= 0 => Ok(()),
            RuntimeValue::Int(_) | RuntimeValue::Sint(_) => {
                Err(RuntimeError::random_seed("随机种子不能为负数"))
            }
            other => Err(RuntimeError::random_seed(format!(
                "随机种子必须是整数，实际为 {}",
                other.type_name()
            ))),
        },
        "set_hashability" | "set_membership" if !is_hashable(&value) => {
            Err(if kind == "set_hashability" {
                RuntimeError::unhashable_element(value.type_name())
            } else {
                RuntimeError::set_membership_requires_hashable(value.type_name())
            })
        }
        "boolean_condition" => value
            .as_bool()
            .map(|_| ())
            .ok_or_else(|| RuntimeError::type_mismatch("bool", value.type_name())),
        _ => Ok(()),
    };
    status(result).map_or_else(|error| error, |_| XiaoAbiStatus::Ok.code())
}

/// 返回动态可迭代值的元素数量。
#[unsafe(no_mangle)]
pub extern "C" fn xiao_runtime_value_iter_len(value: *const XiaoValue, out: *mut usize) -> i32 {
    if value.is_null() || out.is_null() {
        return XiaoAbiStatus::Null.code();
    }
    let value = match unsafe { value_to_runtime(&*value) } {
        Ok(value) => value,
        Err(error) => return error,
    };
    let length = match &value {
        RuntimeValue::Array(handle) => handle.len(),
        RuntimeValue::Tuple(handle) => handle.len(),
        RuntimeValue::Set(handle) => handle.len(),
        RuntimeValue::DictTable(handle) | RuntimeValue::DictColumn(handle) => handle.len(),
        RuntimeValue::Str(handle) => handle.len(),
        other => {
            let error = RuntimeError::iterable_required(other.type_name());
            return status::<()>(Err(error))
                .map_or_else(|error| error, |_| XiaoAbiStatus::Ok.code());
        }
    };
    unsafe { *out = length };
    XiaoAbiStatus::Ok.code()
}

/// 复制动态可迭代值的指定元素。
#[unsafe(no_mangle)]
pub extern "C" fn xiao_runtime_value_iter_get(
    value: *const XiaoValue,
    index: i64,
    out: *mut XiaoValue,
) -> i32 {
    if value.is_null() || out.is_null() {
        return XiaoAbiStatus::Null.code();
    }
    let value = match unsafe { value_to_runtime(&*value) } {
        Ok(value) => value,
        Err(error) => return error,
    };
    let length = match &value {
        RuntimeValue::Array(handle) => handle.len(),
        RuntimeValue::Tuple(handle) => handle.len(),
        RuntimeValue::Set(handle) => handle.len(),
        RuntimeValue::DictTable(handle) | RuntimeValue::DictColumn(handle) => handle.len(),
        RuntimeValue::Str(handle) => handle.len(),
        other => {
            return status::<()>(Err(RuntimeError::iterable_required(other.type_name())))
                .map_or_else(|error| error, |_| XiaoAbiStatus::Ok.code());
        }
    };
    let normalized = if index < 0 {
        (length as i64).checked_add(index)
    } else {
        Some(index)
    }
    .filter(|index| *index >= 0)
    .and_then(|index| usize::try_from(index).ok())
    .filter(|index| *index < length);
    let Some(index) = normalized else {
        return status::<()>(Err(RuntimeError::selector_bounds(format!(
            "选择器或迭代索引 {} 超出运行时长度 {}",
            index, length
        ))))
        .map_or_else(|error| error, |_| XiaoAbiStatus::Ok.code());
    };
    let item = match &value {
        RuntimeValue::Array(handle) => handle.element(index),
        RuntimeValue::Tuple(handle) => handle.element(index),
        RuntimeValue::Set(handle) => handle.with_elements(|items| items.get(index).cloned()),
        RuntimeValue::DictTable(handle) | RuntimeValue::DictColumn(handle) => {
            handle.with_entries(|entries| entries.get(index).map(|(_, value)| value.clone()))
        }
        RuntimeValue::Str(handle) => handle
            .with_str(|text| {
                text.chars()
                    .nth(index)
                    .map(|character| RuntimeValue::new_string(character.to_string()))
            })
            .and_then(|value| value.transpose()),
        other => Err(RuntimeError::iterable_required(other.type_name())),
    };
    let item = match item {
        Ok(Some(value)) => value,
        Ok(None) => {
            return status::<()>(Err(RuntimeError::selector_bounds(format!(
                "选择器或迭代索引 {} 超出运行时长度 {}",
                index, length
            ))))
            .map_or_else(|error| error, |_| XiaoAbiStatus::Ok.code());
        }
        Err(error) => {
            return status::<()>(Err(error))
                .map_or_else(|error| error, |_| XiaoAbiStatus::Ok.code());
        }
    };
    let item = match runtime_to_value(&item) {
        Ok(value) => value,
        Err(error) => return error,
    };
    unsafe { write_value(out, item) }.map_or_else(|error| error, |_| XiaoAbiStatus::Ok.code())
}

/// 从 UTF-8 字节构造字符串强句柄。
#[unsafe(no_mangle)]
pub extern "C" fn xiao_runtime_string_new(input: XiaoAbiBytes, out: *mut XiaoHandle) -> i32 {
    let text = match unsafe { utf8(input) } {
        Ok(text) => text,
        Err(error) => return error,
    };
    let handle = match status(StringHandle::new(text)) {
        Ok(handle) => box_strong(handle.into_strong_handle()),
        Err(error) => return error,
    };
    unsafe { write_handle(out, handle) }.map_or_else(|error| error, |_| XiaoAbiStatus::Ok.code())
}

/// 查询字符串 Unicode 标量长度。
#[unsafe(no_mangle)]
pub extern "C" fn xiao_runtime_string_len(handle: XiaoHandle, out: *mut usize) -> i32 {
    if out.is_null() {
        return XiaoAbiStatus::Null.code();
    }
    unsafe { *out = 0 };
    let handle = match unsafe { expect_strong(handle, RuntimeTypeTag::String) } {
        Ok(handle) => handle,
        Err(error) => return error,
    };
    let result = status(StringHandle::from_strong_handle(handle)).map(|handle| handle.len());
    match result {
        Ok(length) => {
            unsafe { *out = length };
            XiaoAbiStatus::Ok.code()
        }
        Err(error) => error,
    }
}

/// 复制字符串 UTF-8 字节到调用方缓冲区。
#[unsafe(no_mangle)]
pub extern "C" fn xiao_runtime_string_copy(
    handle: XiaoHandle,
    buffer: XiaoAbiMutBytes,
    written: *mut usize,
) -> i32 {
    if written.is_null() {
        return XiaoAbiStatus::Null.code();
    }
    unsafe { *written = 0 };
    let handle = match unsafe { expect_strong(handle, RuntimeTypeTag::String) } {
        Ok(handle) => match status(StringHandle::from_strong_handle(handle)) {
            Ok(handle) => handle,
            Err(error) => return error,
        },
        Err(error) => return error,
    };
    let text = match status(handle.to_string()) {
        Ok(text) => text,
        Err(error) => return error,
    };
    let bytes = text.as_bytes();
    unsafe { *written = bytes.len() };
    let output = match unsafe { mut_bytes(buffer) } {
        Ok(output) => output,
        Err(error) => return error,
    };
    if output.len() < bytes.len() {
        return XiaoAbiStatus::OutOfBounds.code();
    }
    output[..bytes.len()].copy_from_slice(bytes);
    XiaoAbiStatus::Ok.code()
}
