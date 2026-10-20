//! Dynamic checks, iteration, and string ABI entry points.

use super::{
    box_strong, expect_strong, mut_bytes, operation_name, runtime_to_value, status, utf8,
    value_from_owned_handle, value_to_runtime, write_handle, write_value,
};
use crate::containers::{ArrayHandle, is_hashable};
use crate::errors::RuntimeError;
use crate::memory::RuntimeTypeTag;
use crate::value::{RuntimeValue, StringHandle};
use xiao_runtime_abi::{
    XiaoAbiBytes, XiaoAbiMutBytes, XiaoAbiStatus, XiaoHandle, XiaoValue, XiaoValueTag,
};
use xiao_syntax::RandomMode;
use xiao_syntax::ScalarType;
use xiao_types::{SeededRandom, sample_indices};

thread_local! {
    /// 同一原生执行会话共享随机状态；仅显式 seed 重置，选择操作持续推进。
    static RANDOM: std::cell::RefCell<SeededRandom> = const { std::cell::RefCell::new(SeededRandom::new(0)) };
}

/// 与 VM 共用确定性随机源和种子规则，不把纯动态检查偷换成重置操作。
#[unsafe(no_mangle)]
pub extern "C" fn xiao_runtime_random_seed(value: *const XiaoValue) -> i32 {
    if value.is_null() {
        return XiaoAbiStatus::Null.code();
    }
    let value = match unsafe { value_to_runtime(&*value) } {
        Ok(value) => value,
        Err(error) => return error,
    };
    let seed = match value {
        RuntimeValue::Int(value) if value >= 0 => Ok(value as u128),
        RuntimeValue::Sint(value) if value >= 0 => Ok(value as u128),
        RuntimeValue::Lint(value) => value
            .parse::<u128>()
            .map_err(|_| RuntimeError::random_seed("种子无效")),
        _ => Err(RuntimeError::random_seed("种子必须是非负整数")),
    };
    match status(seed) {
        Ok(seed) => {
            RANDOM.with(|random| random.borrow_mut().reseed(seed));
            XiaoAbiStatus::Ok.code()
        }
        Err(error) => error,
    }
}

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
        kind if kind.starts_with("resource_type:")
            || kind.starts_with("resource_array:")
            || kind.starts_with("resource_dictionary:") =>
        {
            let (check, expected) = kind.split_once(':').expect("资源检查已包含分隔符");
            if crate::resources::resource_type_matches(&value, check, expected) {
                Ok(())
            } else {
                let expected = match check {
                    "resource_array" => format!("[{expected}]"),
                    "resource_dictionary" => format!("字典值 {expected}"),
                    _ => expected.to_owned(),
                };
                Err(RuntimeError::type_mismatch(expected, value.type_name()))
            }
        }
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
        "selector_bounds" => match value {
            RuntimeValue::Array(_)
            | RuntimeValue::Tuple(_)
            | RuntimeValue::DictColumn(_)
            | RuntimeValue::Str(_) => Ok(()),
            RuntimeValue::Int(value) if value >= 0 => Ok(()),
            RuntimeValue::Sint(value) if value >= 0 => Ok(()),
            RuntimeValue::Lint(value) if value.parse::<i128>().is_ok_and(|value| value >= 0) => {
                Ok(())
            }
            _ => Err(RuntimeError::selector_bounds("选择器下标必须是非负整数")),
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

/// 按动态整数位置或字符串键读取字典/有序容器值。
#[unsafe(no_mangle)]
pub extern "C" fn xiao_runtime_value_index_get_dynamic(
    value: *const XiaoValue,
    index: *const XiaoValue,
    out: *mut XiaoValue,
) -> i32 {
    if value.is_null() || index.is_null() || out.is_null() {
        return XiaoAbiStatus::Null.code();
    }
    let source = match unsafe { value_to_runtime(&*value) } {
        Ok(value) => value,
        Err(error) => return error,
    };
    let index = match unsafe { value_to_runtime(&*index) } {
        Ok(value) => value,
        Err(error) => return error,
    };
    let result = match index {
        RuntimeValue::Str(key) => {
            let key = match key.to_string() {
                Ok(key) => key,
                Err(error) => {
                    return status::<()>(Err(error))
                        .map_or_else(|error| error, |_| XiaoAbiStatus::Ok.code());
                }
            };
            match source {
                RuntimeValue::DictTable(handle) | RuntimeValue::DictColumn(handle) => {
                    handle.value(&key).and_then(|value| {
                        value
                            .ok_or_else(|| RuntimeError::key_not_found(handle.kind().as_str(), key))
                    })
                }
                other => Err(RuntimeError::type_mismatch("字典", other.type_name())),
            }
        }
        other => {
            let raw = match other {
                RuntimeValue::Int(value) => i128::from(value),
                RuntimeValue::Sint(value) => i128::from(value),
                RuntimeValue::Lint(value) => match value.parse::<i128>() {
                    Ok(value) => value,
                    Err(_) => {
                        return status::<()>(Err(RuntimeError::type_mismatch("int", "lint")))
                            .map_or_else(|error| error, |_| XiaoAbiStatus::Ok.code());
                    }
                },
                other => {
                    return status::<()>(Err(RuntimeError::type_mismatch(
                        "int 或 str",
                        other.type_name(),
                    )))
                    .map_or_else(|error| error, |_| XiaoAbiStatus::Ok.code());
                }
            };
            dynamic_index_value(&source, raw)
        }
    };
    let result = match status(result) {
        Ok(value) => value,
        Err(error) => return error,
    };
    match runtime_to_value(&result) {
        Ok(value) => unsafe { write_value(out, value) }
            .map_or_else(|error| error, |_| XiaoAbiStatus::Ok.code()),
        Err(error) => error,
    }
}

/// 按动态整数位置或字符串键更新字典/有序容器值。
#[unsafe(no_mangle)]
pub extern "C" fn xiao_runtime_value_index_set_dynamic(
    value: *const XiaoValue,
    index: *const XiaoValue,
    item: *const XiaoValue,
) -> i32 {
    if value.is_null() || index.is_null() || item.is_null() {
        return XiaoAbiStatus::Null.code();
    }
    let source = match unsafe { value_to_runtime(&*value) } {
        Ok(value) => value,
        Err(error) => return error,
    };
    let index = match unsafe { value_to_runtime(&*index) } {
        Ok(value) => value,
        Err(error) => return error,
    };
    let item = match unsafe { value_to_runtime(&*item) } {
        Ok(value) => value,
        Err(error) => return error,
    };
    let result = match index {
        RuntimeValue::Str(key) => {
            let key = match key.to_string() {
                Ok(key) => key,
                Err(error) => {
                    return status::<()>(Err(error))
                        .map_or_else(|error| error, |_| XiaoAbiStatus::Ok.code());
                }
            };
            match source {
                RuntimeValue::DictTable(handle) | RuntimeValue::DictColumn(handle) => {
                    let kind = handle.kind().as_str();
                    handle.with_entries_mut(|entries| {
                        let Some((_, slot)) = entries.iter_mut().find(|(name, _)| name == &key)
                        else {
                            return Err(RuntimeError::key_not_found(kind, key.clone()));
                        };
                        *slot = item.clone();
                        Ok(())
                    })
                }
                other => Err(RuntimeError::type_mismatch("字典", other.type_name())),
            }
        }
        other => {
            let raw = match other {
                RuntimeValue::Int(value) => i128::from(value),
                RuntimeValue::Sint(value) => i128::from(value),
                RuntimeValue::Lint(value) => match value.parse::<i128>() {
                    Ok(value) => value,
                    Err(_) => {
                        return status::<()>(Err(RuntimeError::type_mismatch("int", "lint")))
                            .map_or_else(|error| error, |_| XiaoAbiStatus::Ok.code());
                    }
                },
                other => {
                    return status::<()>(Err(RuntimeError::type_mismatch(
                        "int 或 str",
                        other.type_name(),
                    )))
                    .map_or_else(|error| error, |_| XiaoAbiStatus::Ok.code());
                }
            };
            match source {
                RuntimeValue::Array(handle) => {
                    let index = match xiao_types::normalize_index(raw, handle.len()) {
                        Some(index) => index,
                        None => {
                            return status::<()>(Err(RuntimeError::selector_bounds(format!(
                                "选择器索引 {raw} 超出运行时长度 {}",
                                handle.len()
                            ))))
                            .map_or_else(|error| error, |_| XiaoAbiStatus::Ok.code());
                        }
                    };
                    handle.with_elements_mut(|elements| {
                        let Some(slot) = elements.get_mut(index) else {
                            return Err(RuntimeError::selector_bounds("数组元素在写入时消失"));
                        };
                        *slot = item.clone();
                        Ok(())
                    })
                }
                RuntimeValue::Tuple(handle) => {
                    let index = match xiao_types::normalize_index(raw, handle.len()) {
                        Some(index) => index,
                        None => {
                            return status::<()>(Err(RuntimeError::selector_bounds(format!(
                                "选择器索引 {raw} 超出运行时长度 {}",
                                handle.len()
                            ))))
                            .map_or_else(|error| error, |_| XiaoAbiStatus::Ok.code());
                        }
                    };
                    handle.with_elements_mut(|elements| {
                        let Some(slot) = elements.get_mut(index) else {
                            return Err(RuntimeError::selector_bounds("元组元素在写入时消失"));
                        };
                        *slot = item.clone();
                        Ok(())
                    })
                }
                RuntimeValue::DictColumn(handle) => {
                    let index = match xiao_types::normalize_index(raw, handle.len()) {
                        Some(index) => index,
                        None => {
                            return status::<()>(Err(RuntimeError::selector_bounds(format!(
                                "选择器索引 {raw} 超出运行时长度 {}",
                                handle.len()
                            ))))
                            .map_or_else(|error| error, |_| XiaoAbiStatus::Ok.code());
                        }
                    };
                    handle.with_entries_mut(|entries| {
                        let Some((_, slot)) = entries.get_mut(index) else {
                            return Err(RuntimeError::selector_bounds("字典列元素在写入时消失"));
                        };
                        *slot = item.clone();
                        Ok(())
                    })
                }
                RuntimeValue::DictTable(_) => Err(RuntimeError::type_mismatch("str", "int")),
                other => Err(RuntimeError::type_mismatch("可写容器", other.type_name())),
            }
        }
    };
    status(result).map_or_else(|error| error, |_| XiaoAbiStatus::Ok.code())
}

/// 删除动态字典键；只允许删除已有字符串键。
#[unsafe(no_mangle)]
pub extern "C" fn xiao_runtime_value_dict_remove_dynamic(
    value: *const XiaoValue,
    index: *const XiaoValue,
) -> i32 {
    if value.is_null() || index.is_null() {
        return XiaoAbiStatus::Null.code();
    }
    let source = match unsafe { value_to_runtime(&*value) } {
        Ok(value) => value,
        Err(error) => return error,
    };
    let index = match unsafe { value_to_runtime(&*index) } {
        Ok(value) => value,
        Err(error) => return error,
    };
    let RuntimeValue::Str(key) = index else {
        return status::<()>(Err(RuntimeError::type_mismatch("str", index.type_name())))
            .map_or_else(|error| error, |_| XiaoAbiStatus::Ok.code());
    };
    let key = match key.to_string() {
        Ok(key) => key,
        Err(error) => {
            return status::<()>(Err(error))
                .map_or_else(|error| error, |_| XiaoAbiStatus::Ok.code());
        }
    };
    let (RuntimeValue::DictTable(handle) | RuntimeValue::DictColumn(handle)) = source else {
        return status::<()>(Err(RuntimeError::type_mismatch("字典", source.type_name())))
            .map_or_else(|error| error, |_| XiaoAbiStatus::Ok.code());
    };
    let kind = handle.kind().as_str();
    let result = handle.with_entries_mut(|entries| {
        let Some(position) = entries.iter().position(|(name, _)| name == &key) else {
            return Err(RuntimeError::key_not_found(kind, key.clone()));
        };
        entries.remove(position);
        Ok(())
    });
    status(result).map_or_else(|error| error, |_| XiaoAbiStatus::Ok.code())
}

/// 枚举字典键并返回新数组值。
#[unsafe(no_mangle)]
pub extern "C" fn xiao_runtime_value_dict_keys(
    value: *const XiaoValue,
    out: *mut XiaoValue,
) -> i32 {
    dict_collect_dynamic(value, out, true)
}

/// 枚举字典值并返回新数组值。
#[unsafe(no_mangle)]
pub extern "C" fn xiao_runtime_value_dict_values(
    value: *const XiaoValue,
    out: *mut XiaoValue,
) -> i32 {
    dict_collect_dynamic(value, out, false)
}

/// 通过 Runtime ABI 枚举字典键或值，并把结果写入拥有的数组值槽。
fn dict_collect_dynamic(value: *const XiaoValue, out: *mut XiaoValue, keys: bool) -> i32 {
    if value.is_null() || out.is_null() {
        return XiaoAbiStatus::Null.code();
    }
    let source = match unsafe { value_to_runtime(&*value) } {
        Ok(value) => value,
        Err(error) => return error,
    };
    let (RuntimeValue::DictTable(handle) | RuntimeValue::DictColumn(handle)) = source else {
        return status::<()>(Err(RuntimeError::type_mismatch("字典", source.type_name())))
            .map_or_else(|error| error, |_| XiaoAbiStatus::Ok.code());
    };
    let result = handle.with_entries(|entries| {
        entries
            .iter()
            .map(|(key, value)| {
                if keys {
                    RuntimeValue::new_string(key.clone())
                } else {
                    Ok(value.clone())
                }
            })
            .collect::<crate::errors::RuntimeResult<Vec<_>>>()
    });
    let values = match result {
        Ok(Ok(values)) => values,
        Ok(Err(error)) => {
            return status::<()>(Err(error))
                .map_or_else(|error| error, |_| XiaoAbiStatus::Ok.code());
        }
        Err(error) => {
            return status::<()>(Err(error))
                .map_or_else(|error| error, |_| XiaoAbiStatus::Ok.code());
        }
    };
    let array = match crate::containers::ArrayHandle::new(values) {
        Ok(array) => RuntimeValue::Array(array),
        Err(error) => {
            return status::<()>(Err(error))
                .map_or_else(|error| error, |_| XiaoAbiStatus::Ok.code());
        }
    };
    match runtime_to_value(&array) {
        Ok(value) => unsafe { write_value(out, value) }
            .map_or_else(|error| error, |_| XiaoAbiStatus::Ok.code()),
        Err(error) => error,
    }
}

/// 在 Runtime ABI 内部执行一个有序容器整数位置读取。
fn dynamic_index_value(
    source: &RuntimeValue,
    raw: i128,
) -> crate::errors::RuntimeResult<RuntimeValue> {
    let length = match source {
        RuntimeValue::Array(handle) => handle.len(),
        RuntimeValue::Tuple(handle) => handle.len(),
        RuntimeValue::Str(handle) => handle.len(),
        RuntimeValue::DictColumn(handle) => handle.len(),
        RuntimeValue::DictTable(_) => {
            return Err(RuntimeError::type_mismatch("str", "int"));
        }
        other => return Err(RuntimeError::iterable_required(other.type_name())),
    };
    let index = xiao_types::normalize_index(raw, length).ok_or_else(|| {
        RuntimeError::selector_bounds(format!("选择器索引 {raw} 超出运行时长度 {length}"))
    })?;
    match source {
        RuntimeValue::Array(handle) => handle
            .element(index)?
            .ok_or_else(|| RuntimeError::selector_bounds("数组元素在读取时消失")),
        RuntimeValue::Tuple(handle) => handle
            .element(index)?
            .ok_or_else(|| RuntimeError::selector_bounds("元组元素在读取时消失")),
        RuntimeValue::Str(handle) => {
            let character =
                handle.with_str(|text| text.chars().nth(index).map(|value| value.to_string()))?;
            let Some(character) = character else {
                return Err(RuntimeError::selector_bounds("字符串元素在读取时消失"));
            };
            RuntimeValue::new_string(character)
        }
        RuntimeValue::DictColumn(handle) => handle
            .with_entries(|entries| entries.get(index).map(|(_, value)| value.clone()))?
            .ok_or_else(|| RuntimeError::selector_bounds("字典列元素在读取时消失")),
        _ => unreachable!("dynamic_index_value 已验证来源类型"),
    }
}

/// 在集合迭代开始时复制元素到数组值，冻结本次循环的观察序列。
#[unsafe(no_mangle)]
pub extern "C" fn xiao_runtime_value_iter_snapshot(
    value: *const XiaoValue,
    out: *mut XiaoValue,
) -> i32 {
    if value.is_null() || out.is_null() {
        return XiaoAbiStatus::Null.code();
    }
    let value = match unsafe { value_to_runtime(&*value) } {
        Ok(value) => value,
        Err(error) => return error,
    };
    let RuntimeValue::Set(handle) = value else {
        return status::<()>(Err(RuntimeError::type_mismatch("set", value.type_name())))
            .map_or_else(|error| error, |_| XiaoAbiStatus::Ok.code());
    };
    let elements = match handle.with_elements(|elements| elements.to_vec()) {
        Ok(elements) => elements,
        Err(error) => return status::<()>(Err(error)).map_or_else(|error| error, |_| 0),
    };
    let array = match ArrayHandle::new(elements) {
        Ok(array) => RuntimeValue::Array(array),
        Err(error) => return status::<()>(Err(error)).map_or_else(|error| error, |_| 0),
    };
    let array = match runtime_to_value(&array) {
        Ok(array) => array,
        Err(error) => return error,
    };
    unsafe { write_value(out, array) }.map_or_else(|error| error, |_| XiaoAbiStatus::Ok.code())
}

/// 向 ABI 数组值原地追加一个元素。
#[unsafe(no_mangle)]
pub extern "C" fn xiao_runtime_array_push(array: *const XiaoValue, value: *const XiaoValue) -> i32 {
    if array.is_null() || value.is_null() {
        return XiaoAbiStatus::Null.code();
    }
    let array = match unsafe { value_to_runtime(&*array) } {
        Ok(RuntimeValue::Array(handle)) => handle,
        Ok(other) => {
            return status::<()>(Err(RuntimeError::type_mismatch("array", other.type_name())))
                .map_or_else(|error| error, |_| XiaoAbiStatus::Ok.code());
        }
        Err(error) => return error,
    };
    let value = match unsafe { value_to_runtime(&*value) } {
        Ok(value) => value,
        Err(error) => return error,
    };
    status(array.push(value)).map_or_else(|error| error, |_| XiaoAbiStatus::Ok.code())
}

/// 向 ABI 集合值插入一个元素；重复元素保持无操作。
#[unsafe(no_mangle)]
pub extern "C" fn xiao_runtime_set_add(set: *const XiaoValue, value: *const XiaoValue) -> i32 {
    if set.is_null() || value.is_null() {
        return XiaoAbiStatus::Null.code();
    }
    let set = match unsafe { value_to_runtime(&*set) } {
        Ok(RuntimeValue::Set(handle)) => handle,
        Ok(other) => {
            return status::<()>(Err(RuntimeError::type_mismatch("set", other.type_name())))
                .map_or_else(|error| error, |_| XiaoAbiStatus::Ok.code());
        }
        Err(error) => return error,
    };
    let value = match unsafe { value_to_runtime(&*value) } {
        Ok(value) => value,
        Err(error) => return error,
    };
    status(set.insert(&value)).map_or_else(|error| error, |_| XiaoAbiStatus::Ok.code())
}

/// 从 ABI 集合值删除一个元素；`out` 返回是否实际删除。
#[unsafe(no_mangle)]
pub extern "C" fn xiao_runtime_set_remove(
    set: *const XiaoValue,
    value: *const XiaoValue,
    out: *mut u8,
) -> i32 {
    if set.is_null() || value.is_null() || out.is_null() {
        return XiaoAbiStatus::Null.code();
    }
    unsafe { *out = 0 };
    let set = match unsafe { value_to_runtime(&*set) } {
        Ok(RuntimeValue::Set(handle)) => handle,
        Ok(other) => {
            return status::<()>(Err(RuntimeError::type_mismatch("set", other.type_name())))
                .map_or_else(|error| error, |_| XiaoAbiStatus::Ok.code());
        }
        Err(error) => return error,
    };
    let value = match unsafe { value_to_runtime(&*value) } {
        Ok(value) => value,
        Err(error) => return error,
    };
    match status(set.remove(&value)) {
        Ok(removed) => {
            unsafe { *out = u8::from(removed) };
            XiaoAbiStatus::Ok.code()
        }
        Err(error) => error,
    }
}

/// 按冻结的种子 0 语义执行动态随机选择并返回数组值。
#[unsafe(no_mangle)]
pub extern "C" fn xiao_runtime_value_select(
    value: *const XiaoValue,
    count: i64,
    mode: XiaoAbiBytes,
    out: *mut XiaoValue,
) -> i32 {
    if value.is_null() || out.is_null() {
        return XiaoAbiStatus::Null.code();
    }
    let count = match usize::try_from(count) {
        Ok(count) => count,
        Err(_) => return XiaoAbiStatus::InvalidArgument.code(),
    };
    let mode = match unsafe { utf8(mode) } {
        Ok(mode) => mode,
        Err(error) => return error,
    };
    let source = match unsafe { value_to_runtime(&*value) } {
        Ok(source) => source,
        Err(error) => return error,
    };
    let values = match &source {
        RuntimeValue::Array(handle) => match handle.with_elements(|values| values.to_vec()) {
            Ok(values) => values,
            Err(error) => return status::<()>(Err(error)).map_or_else(|error| error, |_| 0),
        },
        RuntimeValue::Tuple(handle) => match handle.with_elements(|values| values.to_vec()) {
            Ok(values) => values,
            Err(error) => return status::<()>(Err(error)).map_or_else(|error| error, |_| 0),
        },
        RuntimeValue::DictColumn(handle) => match handle.with_entries(|entries| {
            entries
                .iter()
                .map(|(_, value)| value.clone())
                .collect::<Vec<_>>()
        }) {
            Ok(values) => values,
            Err(error) => return status::<()>(Err(error)).map_or_else(|error| error, |_| 0),
        },
        RuntimeValue::Str(handle) => match handle.with_str(|text| {
            text.chars()
                .map(|character| RuntimeValue::new_string(character.to_string()))
                .collect::<Result<Vec<_>, _>>()
        }) {
            Ok(Ok(values)) => values,
            Ok(Err(error)) => return status::<()>(Err(error)).map_or_else(|error| error, |_| 0),
            Err(error) => return status::<()>(Err(error)).map_or_else(|error| error, |_| 0),
        },
        _ => return XiaoAbiStatus::InvalidArgument.code(),
    };
    let mode = match mode.as_str() {
        "without_replacement" => RandomMode::WithoutReplacement,
        "with_replacement" => RandomMode::WithReplacement,
        _ => return XiaoAbiStatus::InvalidArgument.code(),
    };
    let indices = match RANDOM
        .with(|random| sample_indices(values.len(), count, mode, &mut *random.borrow_mut()))
    {
        Ok(indices) => indices,
        Err(error) => {
            let runtime_error = RuntimeError::random_count(error.to_string());
            return status::<()>(Err(runtime_error))
                .map_or_else(|error| error, |_| XiaoAbiStatus::Ok.code());
        }
    };
    let selected = indices
        .into_iter()
        .map(|index| values[index].clone())
        .collect::<Vec<_>>();
    let handle = match status(ArrayHandle::new(selected)) {
        Ok(handle) => handle.into_strong_handle(),
        Err(error) => return error,
    };
    let result = value_from_owned_handle(XiaoValueTag::Array, handle);
    unsafe { write_value(out, result) }.map_or_else(|error| error, |_| XiaoAbiStatus::Ok.code())
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
