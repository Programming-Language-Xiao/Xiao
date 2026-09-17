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
/// 规则必须与类型层的 [`xiao_types::hashability`] 对齐：标量与 `none` 可哈希，
/// 容器与表一律不可哈希（它们可变，运行时不追踪其内部变化）。两侧不一致会让
/// 静态能通过的程序在运行时被拒。
#[must_use]
pub fn is_hashable(value: &RuntimeValue) -> bool {
    !matches!(
        value,
        RuntimeValue::Table(_)
            | RuntimeValue::Array(_)
            | RuntimeValue::Tuple(_)
            | RuntimeValue::DictTable(_)
            | RuntimeValue::DictColumn(_)
            | RuntimeValue::Set(_)
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
