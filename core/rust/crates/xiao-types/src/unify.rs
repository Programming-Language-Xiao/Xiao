//! HM 类型变量统一、occurs-check、泛化与实例化。
//!
//! 统一器不读取 AST，也不产生诊断文本；调用方可以把 [`UnifyError`] 映射到
//! 自己的源码位置和错误编号。这样函数类型等后续语法加入时无需改动语法层。

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::{self, Display, Formatter};

use crate::containers::{ArrayType, DictEntryType, DictType};
use crate::environment::TypeEnvironment;
use crate::set_types::SetType;
use crate::types::{Type, TypeScheme, TypeVarId};

/// 类型统一失败的结构化原因。
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum UnifyError {
    /// 两个具体类型不相容。
    Mismatch {
        /// 左侧类型。
        left: Type,
        /// 右侧类型。
        right: Type,
    },
    /// 把变量统一为包含自身的类型会形成无限类型。
    OccursCheck {
        /// 待绑定变量。
        variable: TypeVarId,
        /// 包含该变量的候选类型。
        ty: Type,
    },
    /// 两个函数或元组的成员数量不同。
    ArityMismatch {
        /// 左侧数量。
        left: usize,
        /// 右侧数量。
        right: usize,
    },
}

impl Display for UnifyError {
    /// 生成适合开发者日志的稳定说明。
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Mismatch { left, right } => write!(formatter, "cannot unify {left} with {right}"),
            Self::OccursCheck { variable, ty } => {
                write!(
                    formatter,
                    "type variable 't{} occurs in {ty}",
                    variable.get()
                )
            }
            Self::ArityMismatch { left, right } => {
                write!(formatter, "type arity mismatch: {left} versus {right}")
            }
        }
    }
}

impl std::error::Error for UnifyError {}

/// 类型变量到类型的有限替换映射。
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Substitution {
    entries: BTreeMap<TypeVarId, Type>,
}

impl Substitution {
    /// 创建空替换。
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// 返回替换中的绑定数量。
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// 判断替换是否为空。
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// 查看变量的直接绑定。
    #[must_use]
    pub fn get(&self, variable: TypeVarId) -> Option<&Type> {
        self.entries.get(&variable)
    }

    /// 插入一条已经通过 occurs-check 的绑定。
    pub fn insert(&mut self, variable: TypeVarId, ty: Type) {
        self.entries.insert(variable, ty);
    }

    /// 对类型递归应用替换，并防止恶意环导致无限递归。
    #[must_use]
    pub fn apply(&self, ty: &Type) -> Type {
        self.apply_with_seen(ty, &mut BTreeSet::new())
    }

    /// 递归应用替换并用访问集阻断非法循环。
    fn apply_with_seen(&self, ty: &Type, seen: &mut BTreeSet<TypeVarId>) -> Type {
        match ty {
            Type::Variable(variable) => {
                let Some(replacement) = self.entries.get(variable) else {
                    return ty.clone();
                };
                if !seen.insert(*variable) {
                    return ty.clone();
                }
                let result = self.apply_with_seen(replacement, seen);
                seen.remove(variable);
                result
            }
            Type::Function {
                parameters,
                return_type,
            } => Type::Function {
                parameters: parameters
                    .iter()
                    .map(|parameter| self.apply_with_seen(parameter, seen))
                    .collect(),
                return_type: Box::new(self.apply_with_seen(return_type, seen)),
            },
            Type::Tuple(items) => Type::Tuple(
                items
                    .iter()
                    .map(|item| self.apply_with_seen(item, seen))
                    .collect(),
            ),
            Type::Array(array) => Type::Array(apply_array(array, self, seen)),
            Type::DictTable(dictionary) => {
                Type::DictTable(apply_dictionary(dictionary, self, seen))
            }
            Type::DictColumn(dictionary) => {
                Type::DictColumn(apply_dictionary(dictionary, self, seen))
            }
            Type::Set(set) => Type::Set(apply_set(set, self, seen)),
            Type::Scalar(_) | Type::None | Type::Dynamic => ty.clone(),
        }
    }

    /// 判断类型中是否出现变量。
    #[must_use]
    pub fn occurs(&self, variable: TypeVarId, ty: &Type) -> bool {
        self.apply(ty).free_vars().contains(&variable)
    }

    /// 统一两个类型并把结果写入当前替换。
    pub fn unify(&mut self, left: &Type, right: &Type) -> Result<Type, UnifyError> {
        let left = self.apply(left);
        let right = self.apply(right);
        if left == right {
            return Ok(left);
        }
        match (&left, &right) {
            (Type::Dynamic, other) | (other, Type::Dynamic) => Ok(other.clone()),
            (Type::Variable(variable), candidate) => self.bind(*variable, candidate.clone()),
            (candidate, Type::Variable(variable)) => self.bind(*variable, candidate.clone()),
            (
                Type::Function {
                    parameters: left_parameters,
                    return_type: left_return,
                },
                Type::Function {
                    parameters: right_parameters,
                    return_type: right_return,
                },
            ) => {
                if left_parameters.len() != right_parameters.len() {
                    return Err(UnifyError::ArityMismatch {
                        left: left_parameters.len(),
                        right: right_parameters.len(),
                    });
                }
                let mut unified_parameters = Vec::with_capacity(left_parameters.len());
                for (left_parameter, right_parameter) in
                    left_parameters.iter().zip(right_parameters)
                {
                    unified_parameters.push(self.unify(left_parameter, right_parameter)?);
                }
                let unified_return = self.unify(left_return, right_return)?;
                Ok(Type::Function {
                    parameters: unified_parameters,
                    return_type: Box::new(unified_return),
                })
            }
            (Type::Tuple(left_items), Type::Tuple(right_items)) => {
                if left_items.len() != right_items.len() {
                    return Err(UnifyError::ArityMismatch {
                        left: left_items.len(),
                        right: right_items.len(),
                    });
                }
                let mut items = Vec::with_capacity(left_items.len());
                for (left_item, right_item) in left_items.iter().zip(right_items) {
                    items.push(self.unify(left_item, right_item)?);
                }
                Ok(Type::Tuple(items))
            }
            (Type::Array(left), Type::Array(right)) => unify_arrays(self, left, right),
            (Type::DictTable(left), Type::DictTable(right)) => {
                unify_dictionaries(self, left, right, false).map(Type::DictTable)
            }
            (Type::DictColumn(left), Type::DictColumn(right)) => {
                unify_dictionaries(self, left, right, true).map(Type::DictColumn)
            }
            (Type::Set(left), Type::Set(right)) => unify_sets(self, left, right),
            _ => Err(UnifyError::Mismatch { left, right }),
        }
    }

    /// 按顺序统一一组类型约束；第一条失败即返回其结构化原因。
    pub fn unify_all<'types>(
        &mut self,
        constraints: impl IntoIterator<Item = (&'types Type, &'types Type)>,
    ) -> Result<(), UnifyError> {
        for (left, right) in constraints {
            self.unify(left, right)?;
        }
        Ok(())
    }

    /// 执行单变量绑定并进行 occurs-check。
    fn bind(&mut self, variable: TypeVarId, ty: Type) -> Result<Type, UnifyError> {
        if ty == Type::Variable(variable) {
            return Ok(ty);
        }
        if self.occurs(variable, &ty) {
            return Err(UnifyError::OccursCheck { variable, ty });
        }
        self.entries.insert(variable, ty.clone());
        Ok(ty)
    }
}

/// HM 算法上下文，负责分配新变量并持有当前替换。
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct TypeContext {
    next_variable: u32,
    substitution: Substitution,
}

impl TypeContext {
    /// 创建从编号零开始的上下文。
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// 分配一个新的类型变量身份。
    pub fn fresh_variable(&mut self) -> TypeVarId {
        let variable = TypeVarId::new(self.next_variable);
        self.next_variable = self.next_variable.saturating_add(1);
        variable
    }

    /// 分配一个新的变量类型。
    pub fn fresh_type(&mut self) -> Type {
        Type::variable(self.fresh_variable())
    }

    /// 返回当前替换的只读视图。
    #[must_use]
    pub const fn substitution(&self) -> &Substitution {
        &self.substitution
    }

    /// 将类型应用替换后的规范化结果返回；是 [`Self::apply`] 的语义别名。
    #[must_use]
    pub fn resolve(&self, ty: &Type) -> Type {
        self.apply(ty)
    }

    /// 对类型应用当前替换。
    #[must_use]
    pub fn apply(&self, ty: &Type) -> Type {
        self.substitution.apply(ty)
    }

    /// 使用当前替换统一两个类型。
    pub fn unify(&mut self, left: &Type, right: &Type) -> Result<Type, UnifyError> {
        self.substitution.unify(left, right)
    }

    /// 对外暴露 occurs-check 查询，供泛型推断器构造约束时复用。
    #[must_use]
    pub fn occurs_check(&self, variable: TypeVarId, ty: &Type) -> bool {
        self.substitution.occurs(variable, ty)
    }

    /// 对类型进行 HM 泛化，排除环境中已经自由出现的变量。
    #[must_use]
    pub fn generalize(&self, environment: &TypeEnvironment, ty: &Type) -> TypeScheme {
        let applied = self.apply(ty);
        let mut variables = applied.free_vars();
        let environment_variables = environment.free_type_vars();
        variables.retain(|variable| !environment_variables.contains(variable));
        TypeScheme::quantified(variables.into_iter().collect::<Vec<_>>(), applied)
    }

    /// 对方案实例化，为每个量化变量分配新变量。
    pub fn instantiate(&mut self, scheme: &TypeScheme) -> Type {
        let mut replacements = BTreeMap::new();
        for variable in &scheme.quantified {
            replacements.insert(*variable, self.fresh_type());
        }
        substitute_quantified(&scheme.ty, &replacements)
    }
}

/// 递归替换方案中的量化变量。
fn substitute_quantified(ty: &Type, replacements: &BTreeMap<TypeVarId, Type>) -> Type {
    match ty {
        Type::Variable(variable) => replacements
            .get(variable)
            .cloned()
            .unwrap_or_else(|| ty.clone()),
        Type::Function {
            parameters,
            return_type,
        } => Type::Function {
            parameters: parameters
                .iter()
                .map(|parameter| substitute_quantified(parameter, replacements))
                .collect(),
            return_type: Box::new(substitute_quantified(return_type, replacements)),
        },
        Type::Tuple(items) => Type::Tuple(
            items
                .iter()
                .map(|item| substitute_quantified(item, replacements))
                .collect(),
        ),
        Type::Array(array) => Type::Array(substitute_array(array, replacements)),
        Type::DictTable(dictionary) => {
            Type::DictTable(substitute_dictionary(dictionary, replacements))
        }
        Type::DictColumn(dictionary) => {
            Type::DictColumn(substitute_dictionary(dictionary, replacements))
        }
        Type::Set(set) => Type::Set(substitute_set(set, replacements)),
        Type::Scalar(_) | Type::None | Type::Dynamic => ty.clone(),
    }
}

/// 对数组形状递归应用当前替换。
fn apply_array(
    array: &ArrayType,
    context: &Substitution,
    seen: &mut BTreeSet<TypeVarId>,
) -> ArrayType {
    match array {
        ArrayType::Homogeneous { element, length } => ArrayType::Homogeneous {
            element: Box::new(context.apply_with_seen(element, seen)),
            length: *length,
        },
        ArrayType::Heterogeneous { elements } => ArrayType::Heterogeneous {
            elements: elements
                .iter()
                .map(|element| context.apply_with_seen(element, seen))
                .collect(),
        },
        ArrayType::Unknown => ArrayType::Unknown,
    }
}

/// 对字典条目值递归应用当前替换。
fn apply_dictionary(
    dictionary: &DictType,
    context: &Substitution,
    seen: &mut BTreeSet<TypeVarId>,
) -> DictType {
    DictType::new(
        dictionary
            .entries
            .iter()
            .map(|entry| DictEntryType {
                key: entry.key.clone(),
                value: Box::new(context.apply_with_seen(&entry.value, seen)),
            })
            .collect::<Vec<_>>(),
    )
}

/// 统一两个数组形状。
fn unify_arrays(
    context: &mut Substitution,
    left: &ArrayType,
    right: &ArrayType,
) -> Result<Type, UnifyError> {
    match (left, right) {
        (ArrayType::Unknown, other) | (other, ArrayType::Unknown) => Ok(Type::Array(other.clone())),
        (
            ArrayType::Homogeneous {
                element: left,
                length: left_length,
            },
            ArrayType::Homogeneous {
                element: right,
                length: right_length,
            },
        ) => {
            if left_length.is_some() && right_length.is_some() && left_length != right_length {
                return Err(UnifyError::ArityMismatch {
                    left: left_length.unwrap_or_default(),
                    right: right_length.unwrap_or_default(),
                });
            }
            Ok(Type::Array(ArrayType::Homogeneous {
                element: Box::new(context.unify(left, right)?),
                length: left_length.or(*right_length),
            }))
        }
        (
            ArrayType::Heterogeneous { elements: left },
            ArrayType::Heterogeneous { elements: right },
        ) => {
            if left.len() != right.len() {
                return Err(UnifyError::ArityMismatch {
                    left: left.len(),
                    right: right.len(),
                });
            }
            let elements = left
                .iter()
                .zip(right)
                .map(|(left, right)| context.unify(left, right))
                .collect::<Result<Vec<_>, _>>()?;
            Ok(Type::Array(ArrayType::Heterogeneous { elements }))
        }
        (ArrayType::Homogeneous { element, length }, ArrayType::Heterogeneous { elements })
        | (ArrayType::Heterogeneous { elements }, ArrayType::Homogeneous { element, length }) => {
            if length.is_some_and(|length| length != elements.len()) {
                return Err(UnifyError::ArityMismatch {
                    left: length.unwrap_or_default(),
                    right: elements.len(),
                });
            }
            let unified = elements
                .iter()
                .map(|item| context.unify(element, item))
                .collect::<Result<Vec<_>, _>>()?;
            Ok(Type::Array(ArrayType::Heterogeneous { elements: unified }))
        }
    }
}

/// 统一两个字典结构；字典表按键集合匹配，字典列还要求顺序一致。
fn unify_dictionaries(
    context: &mut Substitution,
    left: &DictType,
    right: &DictType,
    ordered: bool,
) -> Result<DictType, UnifyError> {
    if left.entries.len() != right.entries.len() {
        return Err(UnifyError::ArityMismatch {
            left: left.entries.len(),
            right: right.entries.len(),
        });
    }
    let mut entries = Vec::with_capacity(left.entries.len());
    for (index, left_entry) in left.entries.iter().enumerate() {
        let right_entry = if ordered {
            right
                .entries
                .get(index)
                .filter(|entry| entry.key == left_entry.key)
        } else {
            right
                .entries
                .iter()
                .find(|entry| entry.key == left_entry.key)
        }
        .ok_or_else(|| UnifyError::Mismatch {
            left: if ordered {
                Type::DictColumn(left.clone())
            } else {
                Type::DictTable(left.clone())
            },
            right: if ordered {
                Type::DictColumn(right.clone())
            } else {
                Type::DictTable(right.clone())
            },
        })?;
        entries.push(DictEntryType {
            key: left_entry.key.clone(),
            value: Box::new(context.unify(&left_entry.value, &right_entry.value)?),
        });
    }
    Ok(DictType::new(entries))
}

/// 对方案中的数组结构递归替换量化变量。
fn substitute_array(array: &ArrayType, replacements: &BTreeMap<TypeVarId, Type>) -> ArrayType {
    match array {
        ArrayType::Homogeneous { element, length } => ArrayType::Homogeneous {
            element: Box::new(substitute_quantified(element, replacements)),
            length: *length,
        },
        ArrayType::Heterogeneous { elements } => ArrayType::Heterogeneous {
            elements: elements
                .iter()
                .map(|element| substitute_quantified(element, replacements))
                .collect(),
        },
        ArrayType::Unknown => ArrayType::Unknown,
    }
}

/// 对方案中的字典结构递归替换量化变量。
fn substitute_dictionary(
    dictionary: &DictType,
    replacements: &BTreeMap<TypeVarId, Type>,
) -> DictType {
    DictType::new(
        dictionary
            .entries
            .iter()
            .map(|entry| DictEntryType {
                key: entry.key.clone(),
                value: Box::new(substitute_quantified(&entry.value, replacements)),
            })
            .collect::<Vec<_>>(),
    )
}

/// 对集合元素类型递归应用当前替换。
fn apply_set(
    set: &SetType,
    substitution: &Substitution,
    seen: &mut BTreeSet<TypeVarId>,
) -> SetType {
    match set {
        SetType::Homogeneous { element } => {
            SetType::homogeneous(substitution.apply_with_seen(element, seen))
        }
        SetType::Heterogeneous {
            members,
            allows_dynamic,
        } => SetType::heterogeneous_with_dynamic(
            members
                .iter()
                .map(|member| substitution.apply_with_seen(member, seen))
                .collect::<Vec<_>>(),
            *allows_dynamic,
        ),
        SetType::Unknown => SetType::Unknown,
    }
}

/// 统一两个集合的元素约束；未知集合接受另一侧的已知约束。
fn unify_sets(
    context: &mut Substitution,
    left: &SetType,
    right: &SetType,
) -> Result<Type, UnifyError> {
    if left.is_unknown() {
        return Ok(Type::Set(right.clone()));
    }
    if right.is_unknown() {
        return Ok(Type::Set(left.clone()));
    }
    let mut members = left.to_member_types();
    members.extend(right.member_types().cloned());
    let allows_dynamic = left.allows_dynamic() || right.allows_dynamic();
    // 集合类型的统一是成员并集，而不是把不同静态成员强行统一成一个
    // 标量。这样 `set<int>` 与 `set<str>` 可以在 C2-B 中形成稳定的
    // `set<int | str>`，同时仍由赋值规则决定方向性兼容。
    let members = members
        .into_iter()
        .map(|member| context.apply(&member))
        .collect::<Vec<_>>();
    Ok(Type::Set(SetType::heterogeneous_with_dynamic(
        members,
        allows_dynamic,
    )))
}

/// 对量化方案中的集合元素类型递归替换。
fn substitute_set(set: &SetType, replacements: &BTreeMap<TypeVarId, Type>) -> SetType {
    match set {
        SetType::Homogeneous { element } => {
            SetType::homogeneous(substitute_quantified(element, replacements))
        }
        SetType::Heterogeneous {
            members,
            allows_dynamic,
        } => SetType::heterogeneous_with_dynamic(
            members
                .iter()
                .map(|member| substitute_quantified(member, replacements))
                .collect::<Vec<_>>(),
            *allows_dynamic,
        ),
        SetType::Unknown => SetType::Unknown,
    }
}

#[cfg(test)]
/// 覆盖统一、occurs-check、泛化和实例化的单元测试。
mod tests {
    use super::{Substitution, TypeContext, UnifyError};
    use crate::containers::ArrayType;
    use crate::environment::TypeEnvironment;
    use crate::types::{Type, TypeVarId};
    use xiao_syntax::ScalarType;

    #[test]
    /// 验证变量统一、occurs-check 和替换应用。
    fn unifies_variables_and_rejects_recursive_types() {
        let variable = Type::variable(TypeVarId::new(0));
        let mut substitution = Substitution::new();
        assert_eq!(
            substitution.unify(&variable, &Type::scalar(ScalarType::Int)),
            Ok(Type::scalar(ScalarType::Int))
        );
        assert_eq!(substitution.apply(&variable), Type::scalar(ScalarType::Int));
        let recursive_variable = Type::variable(TypeVarId::new(1));
        let recursive = Type::array(recursive_variable.clone());
        assert!(matches!(
            substitution.unify(&recursive_variable, &recursive),
            Err(UnifyError::OccursCheck { .. })
        ));
    }

    #[test]
    /// 验证泛化方案在每次实例化时取得独立变量。
    fn generalizes_and_instantiates() {
        let mut context = TypeContext::new();
        let variable = context.fresh_type();
        let environment = TypeEnvironment::new();
        let scheme = context.generalize(&environment, &variable);
        let first = context.instantiate(&scheme);
        let second = context.instantiate(&scheme);
        assert_ne!(first, second);
    }

    #[test]
    /// 固定长度同构数组与异构数组统一时必须保留长度约束。
    fn preserves_array_length_during_unification() {
        let fixed = Type::Array(ArrayType::homogeneous_with_length(
            Type::scalar(ScalarType::Int),
            2,
        ));
        let matching = Type::array_literal(vec![
            Type::scalar(ScalarType::Int),
            Type::scalar(ScalarType::Int),
        ]);
        let mismatched = Type::array_literal(vec![Type::scalar(ScalarType::Int)]);
        let mut substitution = Substitution::new();
        assert!(substitution.unify(&fixed, &matching).is_ok());
        assert!(matches!(
            substitution.unify(&fixed, &mismatched),
            Err(UnifyError::ArityMismatch { left: 2, right: 1 })
        ));
    }

    #[test]
    /// 集合统一合并静态成员并集，并保留动态尾标。
    fn unifies_set_member_unions() {
        let left = Type::Set(crate::SetType::homogeneous(Type::scalar(ScalarType::Int)));
        let right = Type::Set(crate::SetType::heterogeneous_with_dynamic(
            vec![Type::scalar(ScalarType::Str)],
            true,
        ));
        let mut substitution = Substitution::new();
        let unified = substitution.unify(&left, &right).expect("集合并集应可统一");
        assert_eq!(unified.to_string(), "set<int | str | dynamic>");
    }
}
