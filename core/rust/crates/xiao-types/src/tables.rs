//! 05-C 表类型、成员签名和生命周期静态契约。
//!
//! 本模块只保存表的静态身份。它不创建实例、不执行 `init`/`drop`，也不
//! 持有运行时对象；类型检查器通过这些值向后续 IR、Runtime 和模块层传递
//! 可验证的只读信息。

use std::collections::BTreeMap;
use std::fmt::{self, Display, Formatter};

use xiao_source::SourceSpan;
use xiao_syntax::TableKind;

use crate::functions::FunctionSignature;
use crate::types::Type;

/// 表名称在表达式中的静态身份。
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum TableValueKind {
    /// `[Table]` 单例表值。
    Singleton,
    /// `[[Table]]` 可实例化表的构造目标。
    Constructor,
    /// `new Table(...)` 产生的实例值。
    Instance,
}

impl TableValueKind {
    /// 判断该值是否可以作为 `new` 的构造目标。
    #[must_use]
    pub const fn is_constructor(self) -> bool {
        matches!(self, Self::Constructor)
    }

    /// 判断该值是否为已构造实例。
    #[must_use]
    pub const fn is_instance(self) -> bool {
        matches!(self, Self::Instance)
    }
}

/// 一个表值的轻量类型身份。
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct TableType {
    /// 规范化表名称。
    pub name: String,
    /// 表值在当前表达式中的形态。
    pub kind: TableValueKind,
}

impl TableType {
    /// 创建一个指定形态的表类型。
    #[must_use]
    pub fn new(name: impl Into<String>, kind: TableValueKind) -> Self {
        Self {
            name: name.into(),
            kind,
        }
    }

    /// 创建单例表类型。
    #[must_use]
    pub fn singleton(name: impl Into<String>) -> Self {
        Self::new(name, TableValueKind::Singleton)
    }

    /// 创建可实例化表的构造目标类型。
    #[must_use]
    pub fn constructor(name: impl Into<String>) -> Self {
        Self::new(name, TableValueKind::Constructor)
    }

    /// 创建表实例类型。
    #[must_use]
    pub fn instance(name: impl Into<String>) -> Self {
        Self::new(name, TableValueKind::Instance)
    }

    /// 返回该表值的名称。
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// 返回该表值的静态形态。
    #[must_use]
    pub const fn kind(&self) -> TableValueKind {
        self.kind
    }
}

impl Display for TableType {
    /// 以稳定的 Xiao 风格展示表值类型。
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self.kind {
            TableValueKind::Singleton => write!(formatter, "table {}", self.name),
            TableValueKind::Constructor => write!(formatter, "table {} constructor", self.name),
            TableValueKind::Instance => write!(formatter, "{} instance", self.name),
        }
    }
}

/// 表成员的静态类别。
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum TableMemberKind {
    /// 由赋值或声明形成的字段。
    Field,
    /// 由 `def` 形成的方法。
    Method,
}

/// 表成员的可见性。
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Visibility {
    /// 可从表外访问并参与模块导出。
    Public,
    /// 仅允许表内部访问。
    Private,
}

impl Visibility {
    /// 根据名称约定计算可见性；下划线开头的名称为私有。
    #[must_use]
    pub fn from_name(name: &str) -> Self {
        if name.starts_with('_') {
            Self::Private
        } else {
            Self::Public
        }
    }

    /// 判断成员是否公开。
    #[must_use]
    pub const fn is_public(self) -> bool {
        matches!(self, Self::Public)
    }
}

/// 表中一个字段或方法的静态签名。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TableMemberSignature {
    /// 规范化成员名称。
    pub name: String,
    /// 成员类别。
    pub kind: TableMemberKind,
    /// 成员可见性。
    pub visibility: Visibility,
    /// 字段类型或方法函数类型。
    pub ty: Type,
    /// 方法的完整参数/返回签名；字段为 `None`。
    pub function: Option<FunctionSignature>,
    /// 成员声明源码区间。
    pub span: SourceSpan,
}

impl TableMemberSignature {
    /// 创建字段签名。
    #[must_use]
    pub fn field(name: impl Into<String>, ty: Type, span: SourceSpan) -> Self {
        let name = name.into();
        Self {
            visibility: Visibility::from_name(method_display_name(&name)),
            name,
            kind: TableMemberKind::Field,
            ty,
            function: None,
            span,
        }
    }

    /// 创建方法签名。
    #[must_use]
    pub fn method(signature: FunctionSignature) -> Self {
        let name = signature.name.clone();
        Self {
            visibility: Visibility::from_name(method_display_name(&name)),
            name,
            kind: TableMemberKind::Method,
            ty: Type::Function {
                parameters: signature
                    .parameters
                    .iter()
                    .map(|parameter| parameter.ty.clone())
                    .collect(),
                return_type: Box::new(signature.return_type.clone()),
            },
            span: signature.span,
            function: Some(signature),
        }
    }

    /// 判断成员是否允许从表外访问。
    #[must_use]
    pub const fn is_public(&self) -> bool {
        self.visibility.is_public()
    }

    /// 判断成员是否为方法。
    #[must_use]
    pub const fn is_method(&self) -> bool {
        matches!(self.kind, TableMemberKind::Method)
    }
}

/// 一个表的完整静态成员接口。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TableSignature {
    /// 规范化表名称。
    pub name: String,
    /// 源码表头形态。
    pub declaration_kind: TableKind,
    /// 按规范化名称排序的成员。
    pub members: BTreeMap<String, TableMemberSignature>,
    /// 表头源码区间。
    pub span: SourceSpan,
}

impl TableSignature {
    /// 创建空的表签名。
    #[must_use]
    pub fn new(name: impl Into<String>, declaration_kind: TableKind, span: SourceSpan) -> Self {
        Self {
            name: name.into(),
            declaration_kind,
            members: BTreeMap::new(),
            span,
        }
    }

    /// 返回指定名称的成员。
    #[must_use]
    pub fn member(&self, name: &str) -> Option<&TableMemberSignature> {
        self.members.get(name).or_else(|| {
            if name.starts_with("ascii:") || name.starts_with("backtick:") {
                None
            } else {
                self.members.get(&format!("ascii:{name}"))
            }
        })
    }

    /// 判断表是否可实例化。
    #[must_use]
    pub const fn is_instantiable(&self) -> bool {
        self.declaration_kind.is_instantiable()
    }

    /// 返回公开成员的只读迭代器。
    pub fn public_members(&self) -> impl Iterator<Item = &TableMemberSignature> {
        self.members.values().filter(|member| member.is_public())
    }
}

/// 方法签名名称可能携带 `ascii:`/`backtick:` 作用域前缀；提取其展示名。
fn method_display_name(name: &str) -> &str {
    name.strip_prefix("ascii:")
        .or_else(|| name.strip_prefix("backtick:"))
        .unwrap_or(name)
}

#[cfg(test)]
/// 覆盖表类型和可见性模型的基础行为。
mod tests {
    use super::{TableKind, TableType, Visibility};

    #[test]
    /// 验证表值形态、构造判定和下划线可见性规则。
    fn models_table_value_kinds_and_visibility() {
        let constructor = TableType::constructor("User");
        assert!(constructor.kind().is_constructor());
        assert!(!constructor.kind().is_instance());
        assert_eq!(constructor.to_string(), "table User constructor");
        assert_eq!(Visibility::from_name("_private"), Visibility::Private);
        assert_eq!(Visibility::from_name("public"), Visibility::Public);
        assert!(TableKind::Instance.is_instantiable());
    }
}
