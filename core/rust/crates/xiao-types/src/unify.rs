//! HM 类型变量统一、occurs-check、泛化与实例化。
//!
//! 统一器不读取 AST，也不产生诊断文本；调用方可以把 [`UnifyError`] 映射到
//! 自己的源码位置和错误编号。这样函数类型等后续语法加入时无需改动语法层。

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::{self, Display, Formatter};

use crate::environment::TypeEnvironment;
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
            Type::Array(item) => Type::Array(Box::new(self.apply_with_seen(item, seen))),
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
            (Type::Array(left_item), Type::Array(right_item)) => {
                Ok(Type::Array(Box::new(self.unify(left_item, right_item)?)))
            }
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
        Type::Array(item) => Type::Array(Box::new(substitute_quantified(item, replacements))),
        Type::Scalar(_) | Type::None | Type::Dynamic => ty.clone(),
    }
}

#[cfg(test)]
/// 覆盖统一、occurs-check、泛化和实例化的单元测试。
mod tests {
    use super::{Substitution, TypeContext, UnifyError};
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
        let recursive = Type::Array(Box::new(recursive_variable.clone()));
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
}
