//! C0 容器类型、路径约束与空数组物化计划。
//!
//! 本模块只保存静态结构，不创建运行时容器对象。数组、元组和两种字典
//! 容器通过值类型传递；路径约束使用语义化的数字/键名段，避免让类型层
//! 依赖语法层的选择器实现。

use std::fmt::{self, Display, Formatter};

use crate::types::Type;

/// 一个数组的静态形状。
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum ArrayType {
    /// 元素类型统一的数组；长度为空表示运行时长度未知。
    Homogeneous {
        /// 所有元素的类型。
        element: Box<Type>,
        /// 已知的固定长度。
        length: Option<usize>,
    },
    /// 从字面量推导出的异构数组，元素位置类型固定。
    Heterogeneous {
        /// 按索引保存每个元素的类型。
        elements: Vec<Type>,
    },
    /// 尚未知道元素形状的数组边界。
    Unknown,
}

impl ArrayType {
    /// 创建长度未知的同构数组。
    #[must_use]
    pub fn homogeneous(element: Type) -> Self {
        Self::Homogeneous {
            element: Box::new(element),
            length: None,
        }
    }

    /// 创建固定长度的同构数组。
    #[must_use]
    pub fn homogeneous_with_length(element: Type, length: usize) -> Self {
        Self::Homogeneous {
            element: Box::new(element),
            length: Some(length),
        }
    }

    /// 创建异构数组形状。
    #[must_use]
    pub fn heterogeneous(elements: impl Into<Vec<Type>>) -> Self {
        Self::Heterogeneous {
            elements: elements.into(),
        }
    }

    /// 返回静态长度；未知长度返回 `None`。
    #[must_use]
    pub fn length(&self) -> Option<usize> {
        match self {
            Self::Homogeneous { length, .. } => *length,
            Self::Heterogeneous { elements } => Some(elements.len()),
            Self::Unknown => None,
        }
    }

    /// 返回指定非负索引的静态元素类型。
    #[must_use]
    pub fn element_at(&self, index: usize) -> Option<&Type> {
        match self {
            Self::Homogeneous { element, length } => {
                if length.is_some_and(|length| index >= length) {
                    None
                } else {
                    Some(element)
                }
            }
            Self::Heterogeneous { elements } => elements.get(index),
            Self::Unknown => None,
        }
    }

    /// 返回同构数组的元素类型；异构和未知数组返回 `None`。
    #[must_use]
    pub fn homogeneous_element(&self) -> Option<&Type> {
        match self {
            Self::Homogeneous { element, .. } => Some(element),
            Self::Heterogeneous { .. } | Self::Unknown => None,
        }
    }
}

/// 字典条目的静态键值类型。
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct DictEntryType {
    /// 规范化后的键文本。
    pub key: String,
    /// 键对应的值类型。
    pub value: Box<Type>,
}

/// 字典表或字典列的静态结构。
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct DictType {
    /// 按源码顺序保存条目；无序字典表只是不把顺序暴露为语义。
    pub entries: Vec<DictEntryType>,
}

impl DictType {
    /// 创建一个字典类型。
    #[must_use]
    pub fn new(entries: impl Into<Vec<DictEntryType>>) -> Self {
        Self {
            entries: entries.into(),
        }
    }

    /// 按规范化键查找条目类型。
    #[must_use]
    pub fn value_type(&self, key: &str) -> Option<&Type> {
        self.entries
            .iter()
            .find(|entry| entry.key == key)
            .map(|entry| entry.value.as_ref())
    }

    /// 返回字典中的键数量。
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// 判断字典是否为空。
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// 容器路径中的语义段；不包含语法层源码区间。
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ContainerPathSegment {
    /// 数组、元组或字典列的数字索引。
    Index(usize),
    /// 字典表/字典列的规范化键名。
    Key(String),
}

/// 一条从容器根开始的静态元素类型约束。
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct PathConstraint {
    /// 从根到目标元素的路径；空路径表示根容器约束。
    pub path: Vec<ContainerPathSegment>,
    /// 目标槽位必须满足的类型；如果槽位本身是已知容器，类型约束递归
    /// 作用于该容器的直接元素，和 Xiao 的父路径声明语义一致。
    pub ty: Box<Type>,
}

/// 可合并的路径约束集合。
#[derive(Clone, Debug, Default, Eq, Hash, PartialEq)]
pub struct PathConstraintTree {
    constraints: Vec<PathConstraint>,
}

impl PathConstraintTree {
    /// 创建空约束集合。
    #[must_use]
    pub const fn new() -> Self {
        Self {
            constraints: Vec::new(),
        }
    }

    /// 插入一条约束；同一路径后插入的约束覆盖前一条。
    pub fn insert(&mut self, path: Vec<ContainerPathSegment>, ty: Type) {
        if let Some(existing) = self
            .constraints
            .iter_mut()
            .find(|constraint| constraint.path == path)
        {
            *existing.ty = ty;
        } else {
            self.constraints.push(PathConstraint {
                path,
                ty: Box::new(ty),
            });
        }
    }

    /// 按插入顺序返回约束视图。
    #[must_use]
    pub fn constraints(&self) -> &[PathConstraint] {
        &self.constraints
    }

    /// 查找精确路径的约束。
    #[must_use]
    pub fn get(&self, path: &[ContainerPathSegment]) -> Option<&Type> {
        self.constraints
            .iter()
            .find(|constraint| constraint.path == path)
            .map(|constraint| constraint.ty.as_ref())
    }

    /// 查找一条路径当前生效的最近约束。
    ///
    /// 精确路径优先；如果没有精确项，则沿父路径向上继承最近的约束。这个
    /// 查询使 `str list[2]` 可以作为内嵌数组的默认元素类型，再由
    /// `int list[2/0]` 对更具体的位置覆盖。
    #[must_use]
    pub fn effective_for(&self, path: &[ContainerPathSegment]) -> Option<&Type> {
        self.constraints
            .iter()
            .filter(|constraint| {
                constraint.path.len() <= path.len()
                    && constraint
                        .path
                        .iter()
                        .zip(path)
                        .all(|(left, right)| left == right)
            })
            .max_by_key(|constraint| constraint.path.len())
            .map(|constraint| constraint.ty.as_ref())
    }

    /// 判断约束集合是否为空。
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.constraints.is_empty()
    }
}

/// 空数组路径声明生成的静态物化计划。
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct ContainerMaterializationPlan {
    /// 计划所属的绑定名称。
    pub binding: String,
    /// 需要物化的路径约束。
    pub constraints: PathConstraintTree,
    /// 最深路径要求的各层最小长度。
    pub minimum_lengths: Vec<usize>,
}

impl ContainerMaterializationPlan {
    /// 创建一个空数组物化计划。
    #[must_use]
    pub fn new(binding: impl Into<String>, constraints: PathConstraintTree) -> Self {
        let mut minimum_lengths = Vec::new();
        for constraint in constraints.constraints() {
            for (depth, segment) in constraint.path.iter().enumerate() {
                if let ContainerPathSegment::Index(index) = segment {
                    let required = index.saturating_add(1);
                    if minimum_lengths.len() <= depth {
                        minimum_lengths.resize(depth + 1, 0);
                    }
                    minimum_lengths[depth] = minimum_lengths[depth].max(required);
                }
            }
        }
        Self {
            binding: binding.into(),
            constraints,
            minimum_lengths,
        }
    }
}

impl Display for ArrayType {
    /// 以稳定的 Xiao 类型文本格式化数组形状。
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Homogeneous { element, .. } => write!(formatter, "[{element}]"),
            Self::Heterogeneous { elements } => {
                formatter.write_str("[")?;
                for (index, element) in elements.iter().enumerate() {
                    if index > 0 {
                        formatter.write_str(", ")?;
                    }
                    element.fmt(formatter)?;
                }
                formatter.write_str("]")
            }
            Self::Unknown => formatter.write_str("[dynamic]"),
        }
    }
}

#[cfg(test)]
/// 覆盖数组形状、字典查找和路径物化计划。
mod tests {
    use super::{
        ArrayType, ContainerMaterializationPlan, ContainerPathSegment, PathConstraintTree,
    };
    use crate::types::Type;
    use xiao_syntax::ScalarType;

    #[test]
    /// 验证异构数组可以按位置提供类型，路径计划能计算最小长度。
    fn models_shapes_and_materialization() {
        let array = ArrayType::heterogeneous(vec![
            Type::scalar(ScalarType::Int),
            Type::scalar(ScalarType::Str),
        ]);
        assert_eq!(array.length(), Some(2));
        assert_eq!(array.element_at(1), Some(&Type::scalar(ScalarType::Str)));

        let mut constraints = PathConstraintTree::new();
        constraints.insert(
            vec![
                ContainerPathSegment::Index(3),
                ContainerPathSegment::Index(2),
            ],
            Type::scalar(ScalarType::Int),
        );
        let plan = ContainerMaterializationPlan::new("list", constraints);
        assert_eq!(plan.minimum_lengths, vec![4, 3]);
    }

    #[test]
    /// 确认子路径覆盖父路径，但兄弟路径仍继承父约束。
    fn resolves_most_specific_constraint() {
        let mut constraints = PathConstraintTree::new();
        constraints.insert(
            vec![ContainerPathSegment::Index(2)],
            Type::scalar(ScalarType::Str),
        );
        constraints.insert(
            vec![
                ContainerPathSegment::Index(2),
                ContainerPathSegment::Index(0),
            ],
            Type::scalar(ScalarType::Int),
        );
        assert_eq!(
            constraints.effective_for(&[
                ContainerPathSegment::Index(2),
                ContainerPathSegment::Index(0)
            ]),
            Some(&Type::scalar(ScalarType::Int))
        );
        assert_eq!(
            constraints.effective_for(&[
                ContainerPathSegment::Index(2),
                ContainerPathSegment::Index(1)
            ]),
            Some(&Type::scalar(ScalarType::Str))
        );
    }
}
