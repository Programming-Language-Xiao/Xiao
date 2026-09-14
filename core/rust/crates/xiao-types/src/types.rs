//! 类型表示与 Hindley–Milner 类型方案。
//!
//! 本模块只描述类型，不保存源码、变量值或运行时对象。类型检查器、统一器
//! 和后端都通过这些可复制/可比较的值通信，从而避免把宿主语言布局泄漏到
//! Xiao 的语义接口中。

use std::collections::BTreeSet;
use std::fmt::{self, Display, Formatter};

use xiao_syntax::ScalarType;

use crate::containers::{ArrayType, DictType};

/// HM 类型变量的稳定编号。
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct TypeVarId(u32);

impl TypeVarId {
    /// 从原始编号创建类型变量身份。
    #[must_use]
    pub const fn new(raw: u32) -> Self {
        Self(raw)
    }

    /// 返回类型变量的原始编号。
    #[must_use]
    pub const fn get(self) -> u32 {
        self.0
    }
}

/// P2 及后续阶段共享的类型表示。
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum Type {
    /// 一个 Xiao 标量类型。
    Scalar(ScalarType),
    /// `none`/无返回值类型。
    None,
    /// 尚未统一的 HM 类型变量。
    Variable(TypeVarId),
    /// 函数类型；函数语法在 F1 才开放，但统一器现在即可处理它。
    Function {
        /// 参数类型，按声明顺序排列。
        parameters: Vec<Type>,
        /// 返回类型。
        return_type: Box<Type>,
    },
    /// 数组类型；可以是同构、异构或未知形状。
    Array(ArrayType),
    /// 元组类型。
    Tuple(Vec<Type>),
    /// 无序字典表类型。
    DictTable(DictType),
    /// 保持顺序的字典列类型。
    DictColumn(DictType),
    /// 动态值边界或错误恢复类型。
    Dynamic,
}

impl Type {
    /// 创建标量类型。
    #[must_use]
    pub const fn scalar(scalar: ScalarType) -> Self {
        Self::Scalar(scalar)
    }

    /// 从标量类型构造类型值的别名，便于泛型 API 书写。
    #[must_use]
    pub const fn from_scalar(scalar: ScalarType) -> Self {
        Self::Scalar(scalar)
    }

    /// 如果类型是标量，返回其标量枚举。
    #[must_use]
    pub const fn as_scalar(&self) -> Option<ScalarType> {
        match self {
            Self::Scalar(scalar) => Some(*scalar),
            _ => None,
        }
    }

    /// 创建类型变量。
    #[must_use]
    pub const fn variable(id: TypeVarId) -> Self {
        Self::Variable(id)
    }

    /// 创建长度未知的同构数组类型。
    #[must_use]
    pub fn array(element: Type) -> Self {
        Self::Array(ArrayType::homogeneous(element))
    }

    /// 创建异构数组类型。
    #[must_use]
    pub fn array_literal(elements: impl Into<Vec<Type>>) -> Self {
        Self::Array(ArrayType::heterogeneous(elements))
    }

    /// 判断是否为数组、元组或字典容器。
    #[must_use]
    pub const fn is_container(&self) -> bool {
        matches!(
            self,
            Self::Array(_) | Self::Tuple(_) | Self::DictTable(_) | Self::DictColumn(_)
        )
    }

    /// 判断是否为动态边界类型。
    #[must_use]
    pub const fn is_dynamic(&self) -> bool {
        matches!(self, Self::Dynamic)
    }

    /// 判断是否为布尔类型。
    #[must_use]
    pub const fn is_bool(&self) -> bool {
        matches!(self, Self::Scalar(ScalarType::Bool))
    }

    /// 判断是否为整数族标量。
    #[must_use]
    pub const fn is_integer(&self) -> bool {
        matches!(
            self,
            Self::Scalar(ScalarType::Sint | ScalarType::Int | ScalarType::Lint)
        )
    }

    /// 判断是否为浮点族标量。
    #[must_use]
    pub const fn is_float(&self) -> bool {
        matches!(
            self,
            Self::Scalar(ScalarType::Sfloat | ScalarType::Float | ScalarType::Lfloat)
        )
    }

    /// 判断是否为任意数值标量。
    #[must_use]
    pub const fn is_numeric(&self) -> bool {
        self.is_integer() || self.is_float()
    }

    /// 收集类型中自由出现的变量。
    pub fn collect_free_vars(&self, output: &mut BTreeSet<TypeVarId>) {
        match self {
            Self::Variable(id) => {
                output.insert(*id);
            }
            Self::Function {
                parameters,
                return_type,
            } => {
                for parameter in parameters {
                    parameter.collect_free_vars(output);
                }
                return_type.collect_free_vars(output);
            }
            Self::Tuple(items) => {
                for item in items {
                    item.collect_free_vars(output);
                }
            }
            Self::Array(array) => match array {
                ArrayType::Homogeneous { element, .. } => element.collect_free_vars(output),
                ArrayType::Heterogeneous { elements } => {
                    for element in elements {
                        element.collect_free_vars(output);
                    }
                }
                ArrayType::Unknown => {}
            },
            Self::DictTable(dictionary) | Self::DictColumn(dictionary) => {
                for entry in &dictionary.entries {
                    entry.value.collect_free_vars(output);
                }
            }
            Self::Scalar(_) | Self::None | Self::Dynamic => {}
        }
    }

    /// 返回类型中自由变量的集合。
    #[must_use]
    pub fn free_vars(&self) -> BTreeSet<TypeVarId> {
        let mut vars = BTreeSet::new();
        self.collect_free_vars(&mut vars);
        vars
    }
}

impl Display for Type {
    /// 以稳定的 Xiao 风格文本格式化类型。
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Scalar(scalar) => formatter.write_str(scalar.as_str()),
            Self::None => formatter.write_str("none"),
            Self::Variable(id) => write!(formatter, "'t{}", id.get()),
            Self::Function {
                parameters,
                return_type,
            } => {
                formatter.write_str("(")?;
                for (index, parameter) in parameters.iter().enumerate() {
                    if index > 0 {
                        formatter.write_str(", ")?;
                    }
                    parameter.fmt(formatter)?;
                }
                write!(formatter, ") -> {return_type}")
            }
            Self::Tuple(items) => {
                formatter.write_str("(")?;
                for (index, item) in items.iter().enumerate() {
                    if index > 0 {
                        formatter.write_str(", ")?;
                    }
                    item.fmt(formatter)?;
                }
                formatter.write_str(")")
            }
            Self::Array(array) => array.fmt(formatter),
            Self::DictTable(dictionary) => write_dictionary(formatter, dictionary, false),
            Self::DictColumn(dictionary) => write_dictionary(formatter, dictionary, true),
            Self::Dynamic => formatter.write_str("dynamic"),
        }
    }
}

/// 格式化两种字典类型的稳定摘要。
fn write_dictionary(
    formatter: &mut Formatter<'_>,
    dictionary: &DictType,
    ordered: bool,
) -> fmt::Result {
    formatter.write_str(if ordered { "<" } else { "{" })?;
    for (index, entry) in dictionary.entries.iter().enumerate() {
        if index > 0 {
            formatter.write_str(", ")?;
        }
        write!(formatter, "{} = {}", entry.key, entry.value)?;
    }
    formatter.write_str(if ordered { ">" } else { "}" })
}

/// 一个可被环境保存的 HM 类型方案。
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct TypeScheme {
    /// 被泛化、实例化时需要替换的变量。
    pub quantified: Vec<TypeVarId>,
    /// 方案主体类型。
    pub ty: Type,
}

impl TypeScheme {
    /// 创建单态方案。
    #[must_use]
    pub fn monomorphic(ty: Type) -> Self {
        Self {
            quantified: Vec::new(),
            ty,
        }
    }

    /// 创建指定量化变量的方案。
    #[must_use]
    pub fn quantified(quantified: impl Into<Vec<TypeVarId>>, ty: Type) -> Self {
        Self {
            quantified: quantified.into(),
            ty,
        }
    }

    /// 判断方案是否包含量化变量。
    #[must_use]
    pub fn is_polymorphic(&self) -> bool {
        !self.quantified.is_empty()
    }
}

/// `Scheme` 是类型方案的简短别名，便于实现 HM 算法的调用方使用。
pub type Scheme = TypeScheme;

#[cfg(test)]
/// 覆盖类型展示和自由变量收集的单元测试。
mod tests {
    use super::{Type, TypeScheme, TypeVarId};
    use xiao_syntax::ScalarType;

    #[test]
    /// 确认标量、变量和函数类型的展示及自由变量收集稳定。
    fn displays_and_collects_type_variables() {
        let variable = Type::variable(TypeVarId::new(3));
        let function = Type::Function {
            parameters: vec![variable.clone(), Type::scalar(ScalarType::Int)],
            return_type: Box::new(variable.clone()),
        };
        assert_eq!(function.to_string(), "('t3, int) -> 't3");
        assert_eq!(function.free_vars().len(), 1);
        assert!(TypeScheme::quantified(vec![TypeVarId::new(3)], function).is_polymorphic());
    }
}
