//! 旧数组、元组、字典、集合与表的稳定 C ABI 符号实现。

use super::*;

/// 构造数组对象。
#[unsafe(no_mangle)]
pub extern "C" fn xiao_runtime_array_new(
    values: *const XiaoValue,
    length: usize,
    out: *mut XiaoHandle,
) -> i32 {
    let values = match unsafe { value_slice(values, length) } {
        Ok(values) => values,
        Err(error) => return error,
    };
    let handle = match status(ArrayHandle::new(values)) {
        Ok(handle) => box_strong(handle.into_strong_handle()),
        Err(error) => return error,
    };
    unsafe { write_handle(out, handle) }.map_or_else(|error| error, |_| XiaoAbiStatus::Ok.code())
}

/// 查询数组长度。
#[unsafe(no_mangle)]
pub extern "C" fn xiao_runtime_array_len(handle: XiaoHandle, out: *mut usize) -> i32 {
    if out.is_null() {
        return XiaoAbiStatus::Null.code();
    }
    unsafe { *out = 0 };
    let handle = match unsafe { expect_strong(handle, RuntimeTypeTag::Array) } {
        Ok(handle) => match status(ArrayHandle::from_strong_handle(handle)) {
            Ok(handle) => handle,
            Err(error) => return error,
        },
        Err(error) => return error,
    };
    unsafe { *out = handle.len() };
    XiaoAbiStatus::Ok.code()
}

/// 复制数组元素。
#[unsafe(no_mangle)]
pub extern "C" fn xiao_runtime_array_get(
    handle: XiaoHandle,
    index: usize,
    out: *mut XiaoValue,
) -> i32 {
    let handle = match unsafe { expect_strong(handle, RuntimeTypeTag::Array) } {
        Ok(handle) => match status(ArrayHandle::from_strong_handle(handle)) {
            Ok(handle) => handle,
            Err(error) => return error,
        },
        Err(error) => return error,
    };
    let value = match status(handle.element(index)) {
        Ok(Some(value)) => value,
        Ok(None) => return XiaoAbiStatus::OutOfBounds.code(),
        Err(error) => return error,
    };
    let value = match runtime_to_value(&value) {
        Ok(value) => value,
        Err(error) => return error,
    };
    unsafe { write_value(out, value) }.map_or_else(|error| error, |_| XiaoAbiStatus::Ok.code())
}

/// 将数组强句柄包装为 ABI 数组值并增加一次引用。
#[unsafe(no_mangle)]
pub extern "C" fn xiao_runtime_value_array(handle: XiaoHandle) -> XiaoValue {
    unsafe { expect_strong(handle, RuntimeTypeTag::Array) }
        .map(|handle| value_from_owned_handle(XiaoValueTag::Array, handle))
        .unwrap_or_else(|_| XiaoValue::none())
}

#[unsafe(no_mangle)]
/// 消费数组强句柄并将其所有权转移到 ABI 数组值，不额外增加引用。
pub extern "C" fn xiao_runtime_value_array_owned(handle: XiaoHandle) -> XiaoValue {
    unsafe { take_strong(handle, RuntimeTypeTag::Array) }
        .map(|handle| value_from_owned_handle(XiaoValueTag::Array, handle))
        .unwrap_or_else(|_| XiaoValue::none())
}

/// 构造元组对象。
#[unsafe(no_mangle)]
pub extern "C" fn xiao_runtime_tuple_new(
    values: *const XiaoValue,
    length: usize,
    out: *mut XiaoHandle,
) -> i32 {
    let values = match unsafe { value_slice(values, length) } {
        Ok(values) => values,
        Err(error) => return error,
    };
    let handle = match status(TupleHandle::new(values)) {
        Ok(handle) => box_strong(handle.into_strong_handle()),
        Err(error) => return error,
    };
    unsafe { write_handle(out, handle) }.map_or_else(|error| error, |_| XiaoAbiStatus::Ok.code())
}

/// 查询元组长度。
#[unsafe(no_mangle)]
pub extern "C" fn xiao_runtime_tuple_len(handle: XiaoHandle, out: *mut usize) -> i32 {
    if out.is_null() {
        return XiaoAbiStatus::Null.code();
    }
    unsafe { *out = 0 };
    let handle = match unsafe { expect_strong(handle, RuntimeTypeTag::Tuple) } {
        Ok(handle) => match status(TupleHandle::from_strong_handle(handle)) {
            Ok(handle) => handle,
            Err(error) => return error,
        },
        Err(error) => return error,
    };
    unsafe { *out = handle.len() };
    XiaoAbiStatus::Ok.code()
}

/// 复制元组元素。
#[unsafe(no_mangle)]
pub extern "C" fn xiao_runtime_tuple_get(
    handle: XiaoHandle,
    index: usize,
    out: *mut XiaoValue,
) -> i32 {
    let handle = match unsafe { expect_strong(handle, RuntimeTypeTag::Tuple) } {
        Ok(handle) => match status(TupleHandle::from_strong_handle(handle)) {
            Ok(handle) => handle,
            Err(error) => return error,
        },
        Err(error) => return error,
    };
    let value = match status(handle.element(index)) {
        Ok(Some(value)) => value,
        Ok(None) => return XiaoAbiStatus::OutOfBounds.code(),
        Err(error) => return error,
    };
    let value = match runtime_to_value(&value) {
        Ok(value) => value,
        Err(error) => return error,
    };
    unsafe { write_value(out, value) }.map_or_else(|error| error, |_| XiaoAbiStatus::Ok.code())
}

/// 将元组强句柄包装为 ABI 元组值并增加一次引用。
#[unsafe(no_mangle)]
pub extern "C" fn xiao_runtime_value_tuple(handle: XiaoHandle) -> XiaoValue {
    unsafe { expect_strong(handle, RuntimeTypeTag::Tuple) }
        .map(|handle| value_from_owned_handle(XiaoValueTag::Tuple, handle))
        .unwrap_or_else(|_| XiaoValue::none())
}

#[unsafe(no_mangle)]
/// 消费元组强句柄并将其所有权转移到 ABI 元组值，不额外增加引用。
pub extern "C" fn xiao_runtime_value_tuple_owned(handle: XiaoHandle) -> XiaoValue {
    unsafe { take_strong(handle, RuntimeTypeTag::Tuple) }
        .map(|handle| value_from_owned_handle(XiaoValueTag::Tuple, handle))
        .unwrap_or_else(|_| XiaoValue::none())
}

/// 构造字典表或字典列对象。
#[unsafe(no_mangle)]
pub extern "C" fn xiao_runtime_dict_new(
    kind: u32,
    keys: *const XiaoAbiBytes,
    values: *const XiaoValue,
    length: usize,
    out: *mut XiaoHandle,
) -> i32 {
    if length != 0 && (keys.is_null() || values.is_null()) {
        return XiaoAbiStatus::Null.code();
    }
    let keys = if length == 0 {
        &[]
    } else {
        unsafe { slice::from_raw_parts(keys, length) }
    };
    let values = match unsafe { value_slice(values, length) } {
        Ok(values) => values,
        Err(error) => return error,
    };
    let kind = match kind {
        0 => DictKind::Table,
        1 => DictKind::Column,
        _ => return XiaoAbiStatus::InvalidArgument.code(),
    };
    let mut entries = Vec::with_capacity(length);
    let mut seen_keys = std::collections::BTreeSet::new();
    for (key, value) in keys.iter().zip(values) {
        let key = match unsafe { utf8(*key) } {
            Ok(key) => key,
            Err(error) => return error,
        };
        if !seen_keys.insert(key.clone()) {
            return XiaoAbiStatus::InvalidArgument.code();
        }
        entries.push((key, value));
    }
    let handle = match status(DictHandle::new(kind, entries)) {
        Ok(handle) => box_strong(handle.into_strong_handle()),
        Err(error) => return error,
    };
    unsafe { write_handle(out, handle) }.map_or_else(|error| error, |_| XiaoAbiStatus::Ok.code())
}

/// 查询字典条目数。
#[unsafe(no_mangle)]
pub extern "C" fn xiao_runtime_dict_len(handle: XiaoHandle, out: *mut usize) -> i32 {
    if out.is_null() {
        return XiaoAbiStatus::Null.code();
    }
    unsafe { *out = 0 };
    let handle = match unsafe { clone_strong(handle) } {
        Ok(handle) => match status(DictHandle::from_strong_handle(handle)) {
            Ok(handle) => handle,
            Err(error) => return error,
        },
        Err(error) => return error,
    };
    unsafe { *out = handle.len() };
    XiaoAbiStatus::Ok.code()
}

/// 按 UTF-8 键复制字典值。
#[unsafe(no_mangle)]
pub extern "C" fn xiao_runtime_dict_get(
    handle: XiaoHandle,
    key: XiaoAbiBytes,
    out: *mut XiaoValue,
) -> i32 {
    let key = match unsafe { utf8(key) } {
        Ok(key) => key,
        Err(error) => return error,
    };
    let handle = match unsafe { clone_strong(handle) } {
        Ok(handle) => match status(DictHandle::from_strong_handle(handle)) {
            Ok(handle) => handle,
            Err(error) => return error,
        },
        Err(error) => return error,
    };
    let value = match status(handle.value(&key)) {
        Ok(Some(value)) => value,
        Ok(None) => return XiaoAbiStatus::OutOfBounds.code(),
        Err(error) => return error,
    };
    let value = match runtime_to_value(&value) {
        Ok(value) => value,
        Err(error) => return error,
    };
    unsafe { write_value(out, value) }.map_or_else(|error| error, |_| XiaoAbiStatus::Ok.code())
}

/// 按 UTF-8 键读取动态成员；只有真实表值允许成员访问。
#[unsafe(no_mangle)]
pub extern "C" fn xiao_runtime_dynamic_member_get(
    value: *const XiaoValue,
    key: XiaoAbiBytes,
    out: *mut XiaoValue,
) -> i32 {
    if value.is_null() || out.is_null() {
        return XiaoAbiStatus::Null.code();
    }
    let key = match unsafe { utf8(key) } {
        Ok(key) => key,
        Err(error) => return error,
    };
    let value = match unsafe { value_to_runtime(&*value) } {
        Ok(value) => value,
        Err(error) => return error,
    };
    let result = match value {
        RuntimeValue::Table(table) => table.get(&key).and_then(|value| {
            value.ok_or_else(|| RuntimeError::invalid_value(format!("表没有成员 {key}")))
        }),
        other => Err(RuntimeError::type_mismatch("table", other.type_name())),
    };
    let value = match status(result) {
        Ok(value) => value,
        Err(error) => return error,
    };
    let value = match runtime_to_value(&value) {
        Ok(value) => value,
        Err(error) => return error,
    };
    unsafe { write_value(out, value) }.map_or_else(|error| error, |_| XiaoAbiStatus::Ok.code())
}

/// 将字典强句柄包装为 ABI 值并增加一次引用。
#[unsafe(no_mangle)]
pub extern "C" fn xiao_runtime_value_dict(handle: XiaoHandle, kind: u32) -> XiaoValue {
    let (tag, expected) = match kind {
        0 => (XiaoValueTag::DictTable, RuntimeTypeTag::DictTable),
        1 => (XiaoValueTag::DictColumn, RuntimeTypeTag::DictColumn),
        _ => return XiaoValue::none(),
    };
    unsafe { expect_strong(handle, expected) }
        .map(|handle| value_from_owned_handle(tag, handle))
        .unwrap_or_else(|_| XiaoValue::none())
}

#[unsafe(no_mangle)]
/// 消费字典强句柄并将其所有权转移到对应的 ABI 字典值，不额外增加引用。
pub extern "C" fn xiao_runtime_value_dict_owned(handle: XiaoHandle, kind: u32) -> XiaoValue {
    let (tag, expected) = match kind {
        0 => (XiaoValueTag::DictTable, RuntimeTypeTag::DictTable),
        1 => (XiaoValueTag::DictColumn, RuntimeTypeTag::DictColumn),
        _ => return XiaoValue::none(),
    };
    unsafe { take_strong(handle, expected) }
        .map(|handle| value_from_owned_handle(tag, handle))
        .unwrap_or_else(|_| XiaoValue::none())
}

/// 构造集合对象。
#[unsafe(no_mangle)]
pub extern "C" fn xiao_runtime_set_new(
    values: *const XiaoValue,
    length: usize,
    out: *mut XiaoHandle,
) -> i32 {
    let values = match unsafe { value_slice(values, length) } {
        Ok(values) => values,
        Err(error) => return error,
    };
    let handle = match status(SetHandle::new(values)) {
        Ok(handle) => box_strong(handle.into_strong_handle()),
        Err(error) => return error,
    };
    unsafe { write_handle(out, handle) }.map_or_else(|error| error, |_| XiaoAbiStatus::Ok.code())
}

/// 查询集合长度。
#[unsafe(no_mangle)]
pub extern "C" fn xiao_runtime_set_len(handle: XiaoHandle, out: *mut usize) -> i32 {
    if out.is_null() {
        return XiaoAbiStatus::Null.code();
    }
    unsafe { *out = 0 };
    let handle = match unsafe { clone_strong(handle) } {
        Ok(handle) => match status(SetHandle::from_strong_handle(handle)) {
            Ok(handle) => handle,
            Err(error) => return error,
        },
        Err(error) => return error,
    };
    unsafe { *out = handle.len() };
    XiaoAbiStatus::Ok.code()
}

/// 判断集合是否包含一个 ABI 值。
#[unsafe(no_mangle)]
pub extern "C" fn xiao_runtime_set_contains(
    handle: XiaoHandle,
    value: *const XiaoValue,
    out: *mut u8,
) -> i32 {
    if value.is_null() || out.is_null() {
        return XiaoAbiStatus::Null.code();
    }
    unsafe { *out = 0 };
    let value = match unsafe { value_to_runtime(&*value) } {
        Ok(value) => value,
        Err(error) => return error,
    };
    let handle = match unsafe { clone_strong(handle) } {
        Ok(handle) => match status(SetHandle::from_strong_handle(handle)) {
            Ok(handle) => handle,
            Err(error) => return error,
        },
        Err(error) => return error,
    };
    match status(handle.contains(&value)) {
        Ok(found) => {
            unsafe { *out = u8::from(found) };
            XiaoAbiStatus::Ok.code()
        }
        Err(error) => error,
    }
}

/// 将集合强句柄包装为 ABI 集合值并增加一次引用。
#[unsafe(no_mangle)]
pub extern "C" fn xiao_runtime_value_set(handle: XiaoHandle) -> XiaoValue {
    unsafe { expect_strong(handle, RuntimeTypeTag::Set) }
        .map(|handle| value_from_owned_handle(XiaoValueTag::Set, handle))
        .unwrap_or_else(|_| XiaoValue::none())
}

#[unsafe(no_mangle)]
/// 消费集合强句柄并将其所有权转移到 ABI 集合值，不额外增加引用。
pub extern "C" fn xiao_runtime_value_set_owned(handle: XiaoHandle) -> XiaoValue {
    unsafe { take_strong(handle, RuntimeTypeTag::Set) }
        .map(|handle| value_from_owned_handle(XiaoValueTag::Set, handle))
        .unwrap_or_else(|_| XiaoValue::none())
}

/// 按字段描述构造表对象。
#[unsafe(no_mangle)]
pub extern "C" fn xiao_runtime_table_new(
    descriptor: *const XiaoTableDescriptor,
    out: *mut XiaoHandle,
) -> i32 {
    let definition = match unsafe { table_definition(descriptor) } {
        Ok(definition) => definition,
        Err(error) => return error,
    };
    let instance = if definition.is_instantiable() {
        TableInstance::new(definition)
    } else {
        TableInstance::singleton(definition)
    };
    let handle = match status(instance) {
        Ok(instance) => box_strong(instance.into_strong_handle()),
        Err(error) => return error,
    };
    unsafe { write_handle(out, handle) }.map_or_else(|error| error, |_| XiaoAbiStatus::Ok.code())
}

/// 按字段名读取表字段。
#[unsafe(no_mangle)]
pub extern "C" fn xiao_runtime_table_get(
    handle: XiaoHandle,
    field: XiaoAbiBytes,
    out: *mut XiaoValue,
) -> i32 {
    let field = match unsafe { utf8(field) } {
        Ok(field) => field,
        Err(error) => return error,
    };
    let handle = match unsafe { clone_strong(handle) } {
        Ok(handle) => match status(TableInstance::from_strong_handle(handle)) {
            Ok(handle) => handle,
            Err(error) => return error,
        },
        Err(error) => return error,
    };
    let value = match status(handle.get_compiled_field(&field)) {
        Ok(Some(value)) => value,
        Ok(None) => return XiaoAbiStatus::OutOfBounds.code(),
        Err(error) => return error,
    };
    let value = match runtime_to_value(&value) {
        Ok(value) => value,
        Err(error) => return error,
    };
    unsafe { write_value(out, value) }.map_or_else(|error| error, |_| XiaoAbiStatus::Ok.code())
}

/// 按字段名写入表字段。
#[unsafe(no_mangle)]
pub extern "C" fn xiao_runtime_table_set(
    handle: XiaoHandle,
    field: XiaoAbiBytes,
    value: *const XiaoValue,
) -> i32 {
    if value.is_null() {
        return XiaoAbiStatus::Null.code();
    }
    let field = match unsafe { utf8(field) } {
        Ok(field) => field,
        Err(error) => return error,
    };
    let value = match unsafe { value_to_runtime(&*value) } {
        Ok(value) => value,
        Err(error) => return error,
    };
    let handle = match unsafe { clone_strong(handle) } {
        Ok(handle) => match status(TableInstance::from_strong_handle(handle)) {
            Ok(handle) => handle,
            Err(error) => return error,
        },
        Err(error) => return error,
    };
    status(handle.set_compiled_field(&field, value))
        .map_or_else(|error| error, |_| XiaoAbiStatus::Ok.code())
}

/// 将表强句柄包装为 ABI 表值并增加一次引用。
#[unsafe(no_mangle)]
pub extern "C" fn xiao_runtime_value_table(handle: XiaoHandle) -> XiaoValue {
    unsafe { expect_strong(handle, RuntimeTypeTag::Table) }
        .map(|handle| value_from_owned_handle(XiaoValueTag::Table, handle))
        .unwrap_or_else(|_| XiaoValue::none())
}

#[unsafe(no_mangle)]
/// 消费表强句柄并将其所有权转移到 ABI 表值，不额外增加引用。
pub extern "C" fn xiao_runtime_value_table_owned(handle: XiaoHandle) -> XiaoValue {
    unsafe { take_strong(handle, RuntimeTypeTag::Table) }
        .map(|handle| value_from_owned_handle(XiaoValueTag::Table, handle))
        .unwrap_or_else(|_| XiaoValue::none())
}
