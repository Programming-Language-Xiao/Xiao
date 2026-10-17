//! Runtime 容器对象：数组、元组、字典表、字典列和集合。
//!
//! 每个容器沿用 `tables` 模块已经确立的两层范式：**私有载荷**持有数据，
//! **公开句柄**只提供窄接口。句柄内部的 `StrongHandle` 与载荷引用都不对外
//! 暴露，外部只能经闭包读取，避免把对象头或载荷借用泄漏到语义层。
//!
//! 本模块只做存储与读取，不含选择器语义：多选、区间、步长和随机选择属于
//! 后续批次，容器本身不解释源码标点。

/// 数组对象与句柄。
pub mod array;
/// 字典表与字典列对象和句柄。
pub mod dict;
/// 集合对象与句柄。
pub mod set;
/// 元组对象与句柄。
pub mod tuple;

/// 重导出数组句柄。
pub use array::ArrayHandle;
/// 重导出字典句柄与形态。
pub use dict::{DictHandle, DictKind};
/// 重导出集合句柄。
pub use set::SetHandle;
/// 重导出元组句柄。
pub use tuple::TupleHandle;

use crate::value::RuntimeValue;

/// 判断一个运行时值能否作为集合元素或字典键。
///
/// 规则必须与类型层的 [`xiao_types::hashability`] 对齐：标量、`none` 和现有内部模块
/// 身份可哈希，资源、容器与表一律不可哈希。正向列出允许集合，新增 Runtime 变体默认
/// 走不可哈希分支，避免排除式匹配漏项。
#[must_use]
pub fn is_hashable(value: &RuntimeValue) -> bool {
    matches!(
        value,
        RuntimeValue::Module(_)
            | RuntimeValue::ModuleFunction(_, _)
            | RuntimeValue::Int(_)
            | RuntimeValue::Sint(_)
            | RuntimeValue::Lint(_)
            | RuntimeValue::Float(_)
            | RuntimeValue::Sfloat(_)
            | RuntimeValue::Lfloat(_)
            | RuntimeValue::Bool(_)
            | RuntimeValue::Str(_)
            | RuntimeValue::None
    )
}

/// 在有序元素序列中做确定性去重，保留首次出现的位置。
pub(super) fn deduplicate(elements: Vec<RuntimeValue>) -> Vec<RuntimeValue> {
    let mut unique: Vec<RuntimeValue> = Vec::with_capacity(elements.len());
    for element in elements {
        if !unique.contains(&element) {
            unique.push(element);
        }
    }
    unique
}

#[cfg(test)]
mod tests {
    use super::is_hashable;
    use crate::value::{ExternalResourceHandle, ExternalResourceKind, RuntimeValue};

    #[test]
    /// 正向允许表保持既有标量/模块身份可哈希，并默认拒绝新资源变体。
    fn hashability_allowlist_rejects_external_resources() {
        for value in [
            RuntimeValue::Module("ascii:module".to_owned()),
            RuntimeValue::ModuleFunction("ascii:module".to_owned(), 1),
            RuntimeValue::Int(1),
            RuntimeValue::Sint(1),
            RuntimeValue::Float(1.0),
            RuntimeValue::Sfloat(1.0),
            RuntimeValue::Lint("1".to_owned()),
            RuntimeValue::Lfloat("1.0".to_owned()),
            RuntimeValue::Bool(true),
            RuntimeValue::None,
        ] {
            assert!(is_hashable(&value), "既有值的可哈希语义漂移：{value:?}");
        }
        let string = RuntimeValue::new_string("hashable").unwrap();
        assert!(is_hashable(&string));
        let resource = RuntimeValue::ExternalResource(
            ExternalResourceHandle::new(ExternalResourceKind::Directory).unwrap(),
        );
        assert!(!is_hashable(&resource));
    }
}
