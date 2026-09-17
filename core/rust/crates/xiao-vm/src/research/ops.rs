//! 值运算的薄包装。
//!
//! 语义核经这里调用 Runtime 的算子表，不散落裸 `RuntimeValue` 调用。这样算子
//! 矩阵只有一处引用点：将来 Runtime 换实现、或某个机型需要不同的快速路径，
//! 都只改这一个文件。这里不做任何隐式宽度提升——那由后端在降低时插入显式转换。

use xiao_bytecode::research::{ArithOp, CompareOp, PathStep};
use xiao_runtime::{
    ArrayHandle, DictHandle, DictKind, RuntimeError, RuntimeResult, RuntimeValue, SetHandle,
    TupleHandle,
};

/// 执行一条三地址算术指令。
pub fn apply_arith(
    op: ArithOp,
    left: &RuntimeValue,
    right: &RuntimeValue,
) -> RuntimeResult<RuntimeValue> {
    match op {
        ArithOp::Add => left.add(right),
        ArithOp::Subtract => left.subtract(right),
        ArithOp::Multiply => left.multiply(right),
        ArithOp::Divide => left.divide(right),
        ArithOp::FloorDivide => left.floor_divide(right),
        ArithOp::Remainder => left.remainder(right),
        ArithOp::Power => left.power(right),
    }
}

/// 构造数组。
pub fn new_array(elements: Vec<RuntimeValue>) -> RuntimeResult<RuntimeValue> {
    Ok(RuntimeValue::Array(ArrayHandle::new(elements)?))
}

/// 构造元组。
pub fn new_tuple(elements: Vec<RuntimeValue>) -> RuntimeResult<RuntimeValue> {
    Ok(RuntimeValue::Tuple(TupleHandle::new(elements)?))
}

/// 构造无序字典表。
pub fn new_dict_table(entries: Vec<(String, RuntimeValue)>) -> RuntimeResult<RuntimeValue> {
    Ok(RuntimeValue::DictTable(DictHandle::new(
        DictKind::Table,
        entries,
    )?))
}

/// 构造字典列。
pub fn new_dict_column(entries: Vec<(String, RuntimeValue)>) -> RuntimeResult<RuntimeValue> {
    Ok(RuntimeValue::DictColumn(DictHandle::new(
        DictKind::Column,
        entries,
    )?))
}

/// 构造集合；不可哈希元素由 Runtime 拒绝。
pub fn new_set(elements: Vec<RuntimeValue>) -> RuntimeResult<RuntimeValue> {
    Ok(RuntimeValue::Set(SetHandle::new(elements)?))
}

/// 按精确路径读取容器元素。
///
/// 负索引一律经 [`xiao_types::normalize_index`] 归一化——与类型检查器共用同一
/// 套语义，不在这里重写一份。越界与键缺失使用稳定错误身份。
pub fn index_get(source: &RuntimeValue, path: &[PathStep]) -> RuntimeResult<RuntimeValue> {
    let [step] = path else {
        return Err(RuntimeError::invalid_value("嵌套精确索引尚未实现"));
    };
    match (source, step) {
        (RuntimeValue::Array(handle), PathStep::Index(raw)) => {
            let index = resolve_index(*raw, handle.len(), "array")?;
            handle.element(index)?.ok_or_else(missing_element)
        }
        (RuntimeValue::Tuple(handle), PathStep::Index(raw)) => {
            let index = resolve_index(*raw, handle.len(), "tuple")?;
            handle.element(index)?.ok_or_else(missing_element)
        }
        (RuntimeValue::Str(handle), PathStep::Index(raw)) => {
            let length = handle.len();
            let index = resolve_index(*raw, length, "str")?;
            let text = handle.with_str(|text| text.chars().nth(index))?;
            match text {
                Some(character) => RuntimeValue::new_string(character.to_string()),
                None => Err(missing_element()),
            }
        }
        (
            RuntimeValue::DictTable(handle) | RuntimeValue::DictColumn(handle),
            PathStep::Key(key),
        ) => handle
            .value(key)?
            .ok_or_else(|| RuntimeError::key_not_found(handle.kind().as_str(), key.clone())),
        (RuntimeValue::DictColumn(handle), PathStep::Index(raw)) => {
            let length = handle.len();
            let index = resolve_index(*raw, length, "dict_column")?;
            let entry = handle
                .with_entries(|entries| entries.get(index).map(|(_, value)| value.clone()))?;
            entry.ok_or_else(missing_element)
        }
        _ => Err(RuntimeError::type_mismatch(
            "可精确索引的容器",
            source.type_name(),
        )),
    }
}

/// 按容器长度归一化一个有符号索引。
fn resolve_index(raw: i128, length: usize, container: &str) -> RuntimeResult<usize> {
    xiao_types::normalize_index(raw, length)
        .ok_or_else(|| RuntimeError::index_out_of_bounds(container, length, raw))
}

/// 归一化通过、但元素仍取不到时使用的错误。
///
/// 只有容器在读取过程中变得不可读才会走到这里，因此按「对象已释放」报告，
/// 而不是伪造一个越界位置。
fn missing_element() -> RuntimeError {
    RuntimeError::use_after_release()
}

/// 执行一条三地址比较指令，结果恒为布尔。
pub fn apply_compare(
    op: CompareOp,
    left: &RuntimeValue,
    right: &RuntimeValue,
) -> RuntimeResult<RuntimeValue> {
    let result = match op {
        CompareOp::Less => left.less(right)?,
        CompareOp::LessEqual => left.less_equal(right)?,
        CompareOp::Greater => left.greater(right)?,
        CompareOp::GreaterEqual => left.greater_equal(right)?,
        CompareOp::Equal => left.equals(right),
        CompareOp::NotEqual => left.not_equals(right),
    };
    Ok(RuntimeValue::Bool(result))
}

#[cfg(test)]
/// 容器构造与精确索引的运行时语义。
///
/// 这些用例直接调用算子而非从源码构造：静态检查器会提前拒绝常量越界与缺失键，
/// 因此运行时容器错误在字面量程序里不可达，只能在这一层验证。
mod tests {
    use super::{index_get, new_array, new_dict_column, new_dict_table, new_set, new_tuple};
    use xiao_bytecode::research::PathStep;
    use xiao_runtime::{
        CONTAINER_HASHABILITY_CODE, CONTAINER_INDEX_CODE, CONTAINER_KEY_CODE, RuntimeValue,
    };

    /// 构造一个两元素整数数组。
    fn pair() -> RuntimeValue {
        new_array(vec![RuntimeValue::Int(1), RuntimeValue::Int(2)]).expect("数组应分配")
    }

    #[test]
    /// 正索引与负索引都按同一套归一化语义取到元素。
    fn array_index_supports_negative_positions() {
        let array = pair();
        assert_eq!(
            index_get(&array, &[PathStep::Index(0)]),
            Ok(RuntimeValue::Int(1))
        );
        assert_eq!(
            index_get(&array, &[PathStep::Index(-1)]),
            Ok(RuntimeValue::Int(2))
        );
        assert_eq!(
            index_get(&array, &[PathStep::Index(-2)]),
            Ok(RuntimeValue::Int(1))
        );
    }

    #[test]
    /// 越界使用稳定错误身份，不返回空值也不截断。
    fn array_index_out_of_bounds_is_stable() {
        let array = pair();
        let error = index_get(&array, &[PathStep::Index(2)]).expect_err("应越界");
        assert_eq!(error.code(), CONTAINER_INDEX_CODE);
        let error = index_get(&array, &[PathStep::Index(-3)]).expect_err("负索引也应越界");
        assert_eq!(error.code(), CONTAINER_INDEX_CODE);
    }

    #[test]
    /// 字典表按键读取，键缺失使用稳定错误身份。
    fn dict_key_lookup_is_stable() {
        let dict = new_dict_table(vec![("a".to_owned(), RuntimeValue::Int(7))]).expect("应分配");
        assert_eq!(
            index_get(&dict, &[PathStep::Key("a".to_owned())]),
            Ok(RuntimeValue::Int(7))
        );
        let error = index_get(&dict, &[PathStep::Key("b".to_owned())]).expect_err("键应缺失");
        assert_eq!(error.code(), CONTAINER_KEY_CODE);
    }

    #[test]
    /// 字典列同时支持键与数字索引。
    fn dict_column_supports_key_and_position() {
        let column = new_dict_column(vec![
            ("a".to_owned(), RuntimeValue::Int(7)),
            ("b".to_owned(), RuntimeValue::Int(8)),
        ])
        .expect("应分配");
        assert_eq!(
            index_get(&column, &[PathStep::Key("b".to_owned())]),
            Ok(RuntimeValue::Int(8))
        );
        assert_eq!(
            index_get(&column, &[PathStep::Index(-1)]),
            Ok(RuntimeValue::Int(8))
        );
    }

    #[test]
    /// 字符串按字符位置索引，返回单字符字符串。
    fn string_index_returns_single_character() {
        let text = RuntimeValue::new_string("小雪").expect("应分配");
        let indexed = index_get(&text, &[PathStep::Index(-1)]).expect("应取到字符");
        assert_eq!(indexed.type_name(), "str");
        assert!(index_get(&text, &[PathStep::Index(2)]).is_err());
    }

    #[test]
    /// 元组支持位置索引；不可索引的容器形态给出类型错误。
    fn tuple_index_and_unsupported_source() {
        let tuple = new_tuple(vec![RuntimeValue::Int(1)]).expect("应分配");
        assert_eq!(
            index_get(&tuple, &[PathStep::Index(0)]),
            Ok(RuntimeValue::Int(1))
        );
        let set = new_set(vec![RuntimeValue::Int(1)]).expect("应分配");
        assert!(
            index_get(&set, &[PathStep::Index(0)]).is_err(),
            "集合不可索引"
        );
    }

    #[test]
    /// 嵌套路径尚未实现，必须明确报错而不是静默取第一段。
    fn nested_path_is_explicitly_unsupported() {
        let array = pair();
        assert!(index_get(&array, &[PathStep::Index(0), PathStep::Index(1)]).is_err());
    }

    #[test]
    /// 不可哈希元素进入集合的稳定错误身份。
    fn set_hashability_error_identity() {
        let nested = pair();
        let error = new_set(vec![nested]).expect_err("数组不可作为集合元素");
        assert_eq!(error.code(), CONTAINER_HASHABILITY_CODE);
    }
}
