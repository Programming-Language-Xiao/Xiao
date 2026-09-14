//! C2-A 集合类型与可哈希能力模型。
//!
//! 本模块只描述集合的静态元素约束和可哈希判定，不创建运行时集合对象，
//! 也不规定集合的遍历顺序。集合元素在 C2-A 中默认要求单一静态类型；
//! 异构集合和运行时插入规则由后续 C2 阶段扩展。

use std::fmt::{self, Display, Formatter};

use crate::types::Type;

/// C2-A 集合的静态元素类型描述。
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum SetType {
    /// 已知所有元素都满足同一个类型约束。
    Homogeneous {
        /// 集合元素类型。
        element: Box<Type>,
    },
    /// 空集合或元素类型尚未由上下文确定。
    Unknown,
}

impl SetType {
    /// 创建指定元素类型的集合描述。
    #[must_use]
    pub fn homogeneous(element: Type) -> Self {
        Self::Homogeneous {
            element: Box::new(element),
        }
    }

    /// 创建尚未确定元素类型的集合描述。
    #[must_use]
    pub const fn unknown() -> Self {
        Self::Unknown
    }

    /// 返回已知的元素类型；未知集合返回 `None`。
    #[must_use]
    pub fn element_type(&self) -> Option<&Type> {
        match self {
            Self::Homogeneous { element } => Some(element),
            Self::Unknown => None,
        }
    }

    /// 判断集合是否仍处于未知元素类型状态。
    #[must_use]
    pub const fn is_unknown(&self) -> bool {
        matches!(self, Self::Unknown)
    }
}

impl Display for SetType {
    /// 以稳定的 Xiao 类型文本格式化集合。
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Homogeneous { element } => write!(formatter, "set<{element}>"),
            Self::Unknown => formatter.write_str("set"),
        }
    }
}

/// 一个类型在集合中作为元素时的可哈希判定。
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Hashability {
    /// 编译期已知值可以稳定参与集合哈希。
    Hashable,
    /// 编译期已知类型不能作为集合元素。
    Unhashable,
    /// 类型或值需要 Runtime 才能判定。
    Dynamic,
}

/// 判断一个静态类型是否满足 Python 风格的集合可哈希要求。
///
/// Xiao 的可变数组、字典、元组、普通集合和函数均不可哈希。C2-A 暂不
/// 对元组执行递归可哈希证明；后续 C2 阶段可以在不改变此接口的前提下
/// 放宽该规则。包含动态值时将判定延后到 Runtime。`bool` 是独立标量
/// 类型，但与其他不可变标量一样可哈希。
#[must_use]
pub fn hashability(ty: &Type) -> Hashability {
    match ty {
        Type::Scalar(_) | Type::None => Hashability::Hashable,
        Type::Dynamic | Type::Variable(_) => Hashability::Dynamic,
        Type::Tuple(_)
        | Type::Array(_)
        | Type::DictTable(_)
        | Type::DictColumn(_)
        | Type::Set(_)
        | Type::Function { .. } => Hashability::Unhashable,
    }
}

/// 检查两个集合类型能否进行赋值或统一。
///
/// 未知元素类型是保守的开放约束；一旦两侧都知道元素类型，则递归使用
/// 普通 Xiao 赋值规则。该函数放在集合模块中，避免转换矩阵承载集合细节。
#[must_use]
pub fn can_assign_set(source: &SetType, target: &SetType) -> bool {
    match (source.element_type(), target.element_type()) {
        (_, None) | (None, _) => true,
        (Some(source), Some(target)) => crate::conversion::can_assign(source, target),
    }
}

#[cfg(test)]
/// 覆盖集合类型展示和可哈希能力的单元测试。
mod tests {
    use super::{Hashability, SetType, hashability};
    use crate::types::Type;
    use xiao_syntax::ScalarType;

    #[test]
    /// 标量可哈希；C2-A 暂时拒绝元组和数组作为集合元素。
    fn classifies_hashability() {
        assert_eq!(
            hashability(&Type::scalar(ScalarType::Bool)),
            Hashability::Hashable
        );
        assert_eq!(
            hashability(&Type::Tuple(vec![Type::scalar(ScalarType::Int)])),
            Hashability::Unhashable
        );
        assert!(matches!(hashability(&Type::Dynamic), Hashability::Dynamic));
    }

    #[test]
    /// 集合类型格式化保持未知和同构两种稳定形式。
    fn formats_set_types() {
        assert_eq!(SetType::unknown().to_string(), "set");
        assert_eq!(
            SetType::homogeneous(Type::scalar(ScalarType::Str)).to_string(),
            "set<str>"
        );
    }
}
