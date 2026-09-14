//! 作用域、绑定状态与类型方案环境。
//!
//! 环境只管理名称到类型方案的关系，不知道名称来自普通标识符还是反引号
//! 语法；调用方负责在传入前完成名称规范化。这样语法模块不会依赖类型层。

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::{self, Display, Formatter};

use crate::containers::PathConstraintTree;
use crate::types::{Type, TypeScheme, TypeVarId};

/// 一个名称绑定及其初始化/可变状态。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Binding {
    /// 名称对应的 HM 类型方案。
    pub scheme: TypeScheme,
    /// 当前路径上是否已经写入值。
    pub initialized: bool,
    /// 是否允许后续普通赋值。
    pub mutable: bool,
    /// 是否为编译期常量。
    pub constant: bool,
    /// 绑定上的数组路径约束；普通标量绑定为空。
    pub container_constraints: PathConstraintTree,
}

impl Binding {
    /// 创建一个普通可变绑定。
    #[must_use]
    pub fn mutable(scheme: TypeScheme, initialized: bool) -> Self {
        Self {
            scheme,
            initialized,
            mutable: true,
            constant: false,
            container_constraints: PathConstraintTree::new(),
        }
    }

    /// 创建一个不可变常量绑定。
    #[must_use]
    pub fn constant(scheme: TypeScheme) -> Self {
        Self {
            scheme,
            initialized: true,
            mutable: false,
            constant: true,
            container_constraints: PathConstraintTree::new(),
        }
    }

    /// 返回绑定的实例化前方案。
    #[must_use]
    pub const fn scheme(&self) -> &TypeScheme {
        &self.scheme
    }
}

/// 作用域操作失败的结构化原因。
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EnvironmentError {
    /// 当前作用域已有同名绑定。
    Duplicate(String),
    /// 所有作用域都找不到该名称。
    Unknown(String),
    /// 尝试修改不可变常量。
    Immutable(String),
}

impl Display for EnvironmentError {
    /// 生成开发者可读的环境错误文本。
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Duplicate(name) => write!(formatter, "duplicate binding: {name}"),
            Self::Unknown(name) => write!(formatter, "unknown binding: {name}"),
            Self::Immutable(name) => write!(formatter, "immutable binding: {name}"),
        }
    }
}

impl std::error::Error for EnvironmentError {}

/// 按内层优先查找的类型环境。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TypeEnvironment {
    scopes: Vec<BTreeMap<String, Binding>>,
}

impl Default for TypeEnvironment {
    /// 默认建立一个全局作用域。
    fn default() -> Self {
        Self::new()
    }
}

impl TypeEnvironment {
    /// 创建只含全局作用域的环境。
    #[must_use]
    pub fn new() -> Self {
        Self {
            scopes: vec![BTreeMap::new()],
        }
    }

    /// 返回当前作用域层数。
    #[must_use]
    pub fn depth(&self) -> usize {
        self.scopes.len()
    }

    /// 开启一个嵌套作用域。
    pub fn push_scope(&mut self) {
        self.scopes.push(BTreeMap::new());
    }

    /// 退出最近作用域；全局作用域不能被弹出。
    pub fn pop_scope(&mut self) -> Option<BTreeMap<String, Binding>> {
        (self.scopes.len() > 1).then(|| self.scopes.pop().expect("嵌套作用域存在"))
    }

    /// 在当前作用域声明一个绑定。
    pub fn declare(
        &mut self,
        name: impl Into<String>,
        binding: Binding,
    ) -> Result<(), EnvironmentError> {
        let name = name.into();
        let scope = self.scopes.last_mut().expect("至少有全局作用域");
        if scope.contains_key(&name) {
            return Err(EnvironmentError::Duplicate(name));
        }
        scope.insert(name, binding);
        Ok(())
    }

    /// 声明一个普通可变名称。
    pub fn declare_mutable(
        &mut self,
        name: impl Into<String>,
        ty: Type,
        initialized: bool,
    ) -> Result<(), EnvironmentError> {
        self.declare(
            name,
            Binding::mutable(TypeScheme::monomorphic(ty), initialized),
        )
    }

    /// 声明带数组路径约束的普通可变名称。
    pub fn declare_mutable_with_constraints(
        &mut self,
        name: impl Into<String>,
        ty: Type,
        initialized: bool,
        constraints: PathConstraintTree,
    ) -> Result<(), EnvironmentError> {
        self.declare(
            name,
            Binding {
                scheme: TypeScheme::monomorphic(ty),
                initialized,
                mutable: true,
                constant: false,
                container_constraints: constraints,
            },
        )
    }

    /// 声明一个编译期常量方案。
    pub fn declare_constant(
        &mut self,
        name: impl Into<String>,
        scheme: TypeScheme,
    ) -> Result<(), EnvironmentError> {
        self.declare(name, Binding::constant(scheme))
    }

    /// 在内层到外层作用域中查找绑定。
    #[must_use]
    pub fn lookup(&self, name: &str) -> Option<&Binding> {
        self.scopes.iter().rev().find_map(|scope| scope.get(name))
    }

    /// 只在当前最内层作用域查找绑定，不穿透外层作用域。
    #[must_use]
    pub fn lookup_current(&self, name: &str) -> Option<&Binding> {
        self.scopes.last().and_then(|scope| scope.get(name))
    }

    /// 判断任一可见作用域是否包含名称。
    #[must_use]
    pub fn contains(&self, name: &str) -> bool {
        self.lookup(name).is_some()
    }

    /// 判断当前最内层作用域是否已经声明名称。
    #[must_use]
    pub fn contains_current(&self, name: &str) -> bool {
        self.lookup_current(name).is_some()
    }

    /// 返回名称绑定的方案；名称不存在时返回 `None`。
    #[must_use]
    pub fn scheme(&self, name: &str) -> Option<&TypeScheme> {
        self.lookup(name).map(Binding::scheme)
    }

    /// 可变地查找绑定，用于赋值状态更新。
    pub fn lookup_mut(&mut self, name: &str) -> Option<&mut Binding> {
        self.scopes
            .iter_mut()
            .rev()
            .find_map(|scope| scope.get_mut(name))
    }

    /// 标记一个已存在的绑定已经初始化。
    pub fn mark_initialized(&mut self, name: &str) -> Result<(), EnvironmentError> {
        let binding = self
            .lookup_mut(name)
            .ok_or_else(|| EnvironmentError::Unknown(name.to_owned()))?;
        binding.initialized = true;
        Ok(())
    }

    /// 检查并更新普通赋值的初始化状态。
    pub fn assign(&mut self, name: &str) -> Result<(), EnvironmentError> {
        let binding = self
            .lookup_mut(name)
            .ok_or_else(|| EnvironmentError::Unknown(name.to_owned()))?;
        if !binding.mutable {
            return Err(EnvironmentError::Immutable(name.to_owned()));
        }
        binding.initialized = true;
        Ok(())
    }

    /// 收集环境中所有方案主体的自由类型变量。
    #[must_use]
    pub fn free_type_vars(&self) -> BTreeSet<TypeVarId> {
        let mut variables = BTreeSet::new();
        for scope in &self.scopes {
            for binding in scope.values() {
                let mut binding_variables = binding.scheme.ty.free_vars();
                for quantified in &binding.scheme.quantified {
                    binding_variables.remove(quantified);
                }
                variables.extend(binding_variables);
            }
        }
        variables
    }
}

#[cfg(test)]
/// 覆盖作用域遮蔽和绑定状态的单元测试。
mod tests {
    use super::{Binding, EnvironmentError, TypeEnvironment};
    use crate::types::{Type, TypeScheme};
    use xiao_syntax::ScalarType;

    #[test]
    /// 验证嵌套作用域遮蔽、重复声明和常量不可变约束。
    fn manages_scopes_and_mutability() {
        let mut environment = TypeEnvironment::new();
        environment
            .declare_mutable("value", Type::scalar(ScalarType::Int), false)
            .expect("首次声明应成功");
        assert!(matches!(
            environment.declare_mutable("value", Type::scalar(ScalarType::Int), true),
            Err(EnvironmentError::Duplicate(_))
        ));
        environment.push_scope();
        environment
            .declare(
                "value",
                Binding::constant(TypeScheme::monomorphic(Type::scalar(ScalarType::Str))),
            )
            .expect("内层遮蔽应成功");
        assert!(environment.contains_current("value"));
        assert_eq!(environment.depth(), 2);
        assert!(matches!(
            environment.assign("value"),
            Err(EnvironmentError::Immutable(_))
        ));
        environment.pop_scope();
        assert!(environment.assign("value").is_ok());
    }
}
