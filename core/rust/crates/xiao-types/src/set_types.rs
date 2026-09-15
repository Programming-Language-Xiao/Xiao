//! C2 集合类型与可哈希能力模型。
//!
//! 本模块只描述集合的静态元素约束和可哈希判定，不创建运行时集合对象，
//! 也不规定集合的遍历顺序。C2-B 在这里表达静态成员类型并集和动态尾标，
//! C2-C 额外表达集合运算产生的静态结果；仍不创建运行时集合对象，也不实现
//! 集合代数的执行或增删操作。

use std::fmt::{self, Display, Formatter};

use crate::types::Type;

/// C2-A 集合的静态元素类型描述。
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum SetType {
    /// 已静态证明不包含任何成员的集合结果。
    ///
    /// 该状态只用于类型运算结果；它与 [`Self::Unknown`] 不同，后者表示
    /// 尚未知道集合的元素约束（例如无上下文的 `set()`）。
    Empty,
    /// 已知所有元素都满足同一个类型约束。
    Homogeneous {
        /// 集合元素类型。
        element: Box<Type>,
    },
    /// 已知静态成员类型的并集；`allows_dynamic` 表示至少有一个成员的
    /// 类型只能在 Runtime 确定。成员按稳定顺序去重保存。
    Heterogeneous {
        /// 集合中已知的静态成员类型。
        members: Vec<Type>,
        /// 是否允许尚未静态确定类型的成员。
        allows_dynamic: bool,
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

    /// 创建由多个静态成员类型组成的集合描述。
    ///
    /// 成员会按稳定的 Xiao 类型文本排序并去重；只有一个静态成员时
    /// 会规范化为 [`Self::Homogeneous`]，以保持 C2-A 类型展示兼容。
    #[must_use]
    pub fn heterogeneous(members: impl Into<Vec<Type>>) -> Self {
        Self::heterogeneous_with_dynamic(members, false)
    }

    /// 创建静态成员并集，并显式记录动态类型尾标。
    #[must_use]
    pub fn heterogeneous_with_dynamic(members: impl Into<Vec<Type>>, allows_dynamic: bool) -> Self {
        let mut members = members.into();
        normalize_members(&mut members);
        if !allows_dynamic {
            if let [element] = members.as_slice() {
                return Self::homogeneous(element.clone());
            }
            if members.is_empty() {
                return Self::Unknown;
            }
        }
        Self::Heterogeneous {
            members,
            allows_dynamic,
        }
    }

    /// 创建一个已静态证明为空的集合类型。
    #[must_use]
    pub const fn empty() -> Self {
        Self::Empty
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
            Self::Empty | Self::Heterogeneous { .. } | Self::Unknown => None,
        }
    }

    /// 返回已知静态成员类型；同构集合也以单项切片语义返回。
    ///
    /// 为避免在公共 API 中返回临时分配，该方法返回迭代器；调用方若需
    /// 持有类型值，可使用 [`Self::to_member_types`]。
    pub fn member_types(&self) -> impl Iterator<Item = &Type> {
        match self {
            Self::Homogeneous { element } => std::slice::from_ref(element.as_ref()).iter(),
            Self::Heterogeneous { members, .. } => members.iter(),
            Self::Empty | Self::Unknown => [].iter(),
        }
    }

    /// 将已知静态成员类型复制为一个稳定顺序的向量。
    #[must_use]
    pub fn to_member_types(&self) -> Vec<Type> {
        self.member_types().cloned().collect()
    }

    /// 返回异构集合的规范化成员切片；同构/未知集合返回空切片。
    #[must_use]
    pub fn members(&self) -> &[Type] {
        match self {
            Self::Heterogeneous { members, .. } => members,
            Self::Empty | Self::Homogeneous { .. } | Self::Unknown => &[],
        }
    }

    /// 判断集合是否允许动态类型成员。
    #[must_use]
    pub const fn allows_dynamic(&self) -> bool {
        matches!(
            self,
            Self::Heterogeneous {
                allows_dynamic: true,
                ..
            }
        )
    }

    /// 判断集合是否已静态证明为空。
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        matches!(self, Self::Empty)
    }

    /// 判断集合是否包含指定静态成员类型。
    #[must_use]
    pub fn contains_type(&self, candidate: &Type) -> bool {
        self.member_types().any(|member| member == candidate)
    }

    /// 判断集合是否已经记录至少一个静态成员类型。
    #[must_use]
    pub fn has_known_members(&self) -> bool {
        self.member_types().next().is_some()
    }

    /// 判断集合是否仍处于未知元素类型状态。
    #[must_use]
    pub const fn is_unknown(&self) -> bool {
        matches!(self, Self::Unknown)
    }

    /// 判断集合是否带有需要 Runtime 继续确认的边界。
    ///
    /// `Unknown` 表示完全未知，异构集合的动态尾标表示只知道部分成员；
    /// 两者都会使集合运算或比较需要旁路检查。
    #[must_use]
    pub const fn has_dynamic_boundary(&self) -> bool {
        self.is_unknown() || self.allows_dynamic()
    }

    /// 计算两个集合类型的静态并集结果。
    #[must_use]
    pub fn union(&self, other: &Self) -> Self {
        if self.is_empty() {
            return other.clone();
        }
        if other.is_empty() {
            return self.clone();
        }
        if self.is_unknown() && other.is_unknown() {
            return Self::Unknown;
        }
        let mut members = self.to_member_types();
        members.extend(other.member_types().cloned());
        Self::from_operation_members(
            members,
            self.has_dynamic_boundary() || other.has_dynamic_boundary(),
        )
    }

    /// 计算两个集合类型的静态交集结果。
    #[must_use]
    pub fn intersection(&self, other: &Self) -> Self {
        if self.is_empty() || other.is_empty() {
            return Self::Empty;
        }
        if self.is_unknown() && other.is_unknown() {
            return Self::Unknown;
        }
        if self.is_unknown() {
            return Self::from_operation_members(other.to_member_types(), true);
        }
        if other.is_unknown() {
            return Self::from_operation_members(self.to_member_types(), true);
        }
        let right = other.to_member_types();
        let members = self
            .member_types()
            .filter(|member| right.iter().any(|candidate| candidate == *member))
            .cloned()
            .collect::<Vec<_>>();
        Self::from_operation_members(
            members,
            self.has_dynamic_boundary() || other.has_dynamic_boundary(),
        )
    }

    /// 计算两个集合类型的静态差集结果。
    ///
    /// 类型层保留左侧所有已知成员；右侧存在动态边界时只追加动态尾标，
    /// 不把无法静态证明的成员误删。
    #[must_use]
    pub fn difference(&self, other: &Self) -> Self {
        if self.is_empty() {
            return Self::Empty;
        }
        if self.is_unknown() {
            return Self::Unknown;
        }
        Self::from_operation_members(
            self.to_member_types(),
            self.has_dynamic_boundary() || other.has_dynamic_boundary(),
        )
    }

    /// 计算两个集合类型的静态对称差结果。
    ///
    /// 对成员类型而言，对称差的可证明结果是两侧成员类型的并集；
    /// 具体值的排除与唯一性由后续 Runtime 阶段执行。
    #[must_use]
    pub fn symmetric_difference(&self, other: &Self) -> Self {
        self.union(other)
    }

    /// 用集合运算结果的成员列表和动态边界构造规范化类型。
    fn from_operation_members(members: Vec<Type>, allows_dynamic: bool) -> Self {
        let mut members = members;
        normalize_members(&mut members);
        if members.is_empty() {
            if allows_dynamic {
                Self::Heterogeneous {
                    members,
                    allows_dynamic: true,
                }
            } else {
                Self::Empty
            }
        } else {
            Self::heterogeneous_with_dynamic(members, allows_dynamic)
        }
    }
}

impl Display for SetType {
    /// 以稳定的 Xiao 类型文本格式化集合。
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Homogeneous { element } => write!(formatter, "set<{element}>"),
            Self::Heterogeneous {
                members,
                allows_dynamic,
            } => {
                formatter.write_str("set<")?;
                for (index, member) in members.iter().enumerate() {
                    if index > 0 {
                        formatter.write_str(" | ")?;
                    }
                    member.fmt(formatter)?;
                }
                if *allows_dynamic {
                    if !members.is_empty() {
                        formatter.write_str(" | ")?;
                    }
                    formatter.write_str("dynamic")?;
                }
                formatter.write_str(">")
            }
            Self::Empty => formatter.write_str("set<never>"),
            Self::Unknown => formatter.write_str("set"),
        }
    }
}

/// 规范化集合成员类型的排序和去重。
fn normalize_members(members: &mut Vec<Type>) {
    members.sort_by_cached_key(ToString::to_string);
    members.dedup();
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
/// C2-A 只证明标量和 `none` 可哈希，拒绝数组、字典、普通集合和函数。
/// 元组的递归可哈希证明尚未实现，因此本阶段也会拒绝；这不改变总体
/// 规范中“所有成员可哈希的元组可哈希”的规则。动态类型将判定延后到
/// Runtime。`bool` 是独立标量类型，与其他不可变标量一样可哈希。
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
        | Type::Function { .. }
        | Type::Table(_) => Hashability::Unhashable,
    }
}

/// 检查两个集合类型能否进行赋值或统一。
///
/// 未知元素类型是保守的开放约束；一旦两侧都知道元素类型，则递归使用
/// 普通 Xiao 赋值规则。该函数放在集合模块中，避免转换矩阵承载集合细节。
#[must_use]
pub fn can_assign_set(source: &SetType, target: &SetType) -> bool {
    if source.is_empty() {
        return true;
    }
    if target.is_empty() {
        return false;
    }
    if source.is_unknown() || target.is_unknown() {
        // `set()` 的未知元素约束可以在声明上下文中被具体化；未知目标
        // 则表示尚未施加更窄的约束。
        return true;
    }
    // 动态尾标表示“兼容性延后到 Runtime”，而不是静态拒绝；调用方
    // 必须同时登记相应的 RuntimeCheckKind，不能把它当成已经证明。
    source.member_types().all(|source_member| {
        target
            .member_types()
            .any(|target_member| source_member == target_member)
            || target.allows_dynamic()
    })
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
        assert_eq!(
            SetType::heterogeneous(vec![
                Type::scalar(ScalarType::Str),
                Type::scalar(ScalarType::Int),
            ])
            .to_string(),
            "set<int | str>"
        );
        assert_eq!(
            SetType::heterogeneous_with_dynamic(vec![Type::scalar(ScalarType::Int)], true)
                .to_string(),
            "set<int | dynamic>"
        );
    }

    #[test]
    /// 成员规范化去重且保留动态尾标；不同标量宽度不会被合并。
    fn normalizes_heterogeneous_members() {
        let set = SetType::heterogeneous_with_dynamic(
            vec![
                Type::scalar(ScalarType::Int),
                Type::scalar(ScalarType::Str),
                Type::scalar(ScalarType::Int),
                Type::scalar(ScalarType::Sint),
            ],
            true,
        );
        assert_eq!(set.to_member_types().len(), 3);
        assert!(set.allows_dynamic());
        assert!(set.contains_type(&Type::scalar(ScalarType::Sint)));
    }

    #[test]
    /// 集合赋值按成员静态类型保持不变性，不能借数值加宽混淆哈希身份。
    fn keeps_set_member_types_invariant() {
        let int_set = SetType::homogeneous(Type::scalar(ScalarType::Int));
        let sint_set = SetType::homogeneous(Type::scalar(ScalarType::Sint));
        let union = SetType::heterogeneous(vec![
            Type::scalar(ScalarType::Int),
            Type::scalar(ScalarType::Str),
        ]);
        assert!(!super::can_assign_set(&sint_set, &int_set));
        assert!(super::can_assign_set(&int_set, &union));
    }

    #[test]
    /// 集合运算保持成员并集、交集、左侧差集和动态边界规则。
    fn computes_static_set_operation_results() {
        let ints = SetType::homogeneous(Type::scalar(ScalarType::Int));
        let mixed = SetType::heterogeneous(vec![
            Type::scalar(ScalarType::Int),
            Type::scalar(ScalarType::Str),
        ]);
        let bools = SetType::homogeneous(Type::scalar(ScalarType::Bool));
        assert_eq!(ints.union(&bools).to_string(), "set<bool | int>");
        assert_eq!(mixed.intersection(&ints).to_string(), "set<int>");
        assert_eq!(mixed.difference(&ints).to_string(), "set<int | str>");
        assert_eq!(ints.intersection(&bools), SetType::empty());
        assert_eq!(
            SetType::unknown().union(&ints).to_string(),
            "set<int | dynamic>"
        );
    }

    #[test]
    /// 静态空集合可赋给任意成员约束，但未知集合不能伪装成空集合。
    fn distinguishes_empty_from_unknown_for_assignment() {
        assert!(super::can_assign_set(
            &SetType::empty(),
            &SetType::homogeneous(Type::scalar(ScalarType::Int))
        ));
        assert!(!super::can_assign_set(
            &SetType::unknown(),
            &SetType::empty()
        ));
        assert!(!super::can_assign_set(
            &SetType::homogeneous(Type::scalar(ScalarType::Int)),
            &SetType::empty()
        ));
    }
}
