//! 静态表接口的序列化镜像；转换只改变表示，不推断成员或类型。

use serde::{Deserialize, Serialize};
use xiao_source::SourceSpan;
use xiao_syntax::{ScalarType, TableKind};
use xiao_types::{
    ArrayType, DictEntryType, DictType, SetType, TableMemberKind, TableMemberSignature,
    TableSignature, TableType, TableValueKind, Type, TypeVarId, Visibility,
};

use crate::{IrArrayShape, IrSpan, IrType, ir_span, lower_type};

/// 已完成类型检查的表接口；方法的调用参数另由函数 IR 保存。
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct IrTableSignature {
    /// 表的规范名称。
    pub name: String,
    /// 声明形态：`singleton` 或 `instance`。
    pub kind: String,
    /// 按成员键排序的静态成员。
    pub members: Vec<IrTableMember>,
    /// 表声明源码区间。
    pub span: IrSpan,
}

/// 一个字段或方法的静态接口镜像。
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct IrTableMember {
    /// 含 `ascii:` / `backtick:` 前缀的成员键。
    pub name: String,
    /// 是否为方法；外部方法类型不含隐式接收者，完整参数另存于函数 IR。
    pub method: bool,
    /// 是否可从表外访问。
    pub public: bool,
    /// 前端确定的字段或函数类型。
    pub ty: IrType,
    /// 成员源码区间。
    pub span: IrSpan,
}

impl IrTableSignature {
    /// 从类型阶段的唯一签名来源复制接口事实。
    #[must_use]
    pub fn from_signature(signature: &TableSignature) -> Self {
        Self {
            name: signature.name.clone(),
            kind: match signature.declaration_kind {
                TableKind::Singleton => "singleton",
                TableKind::Instance => "instance",
            }
            .to_owned(),
            members: signature
                .members
                .values()
                .map(|member| IrTableMember {
                    name: member.name.clone(),
                    method: member.kind == TableMemberKind::Method,
                    public: member.is_public(),
                    ty: lower_type(&member.ty),
                    span: ir_span(member.span),
                })
                .collect(),
            span: ir_span(signature.span),
        }
    }

    /// 还原 Runtime 使用的成员接口；调用签名由后端函数表单独携带。
    ///
    /// 非法标签、重复成员或非法类型返回空值，不退化为 `dynamic`。
    #[must_use]
    pub fn runtime_signature(&self) -> Option<TableSignature> {
        let kind = match self.kind.as_str() {
            "singleton" => TableKind::Singleton,
            "instance" => TableKind::Instance,
            _ => return None,
        };
        let mut signature = TableSignature::new(&self.name, kind, source_span(self.span)?);
        for member in &self.members {
            if member.method && !matches!(member.ty, IrType::Function { .. }) {
                return None;
            }
            let previous = signature.members.insert(
                member.name.clone(),
                TableMemberSignature {
                    name: member.name.clone(),
                    kind: if member.method {
                        TableMemberKind::Method
                    } else {
                        TableMemberKind::Field
                    },
                    visibility: if member.public {
                        Visibility::Public
                    } else {
                        Visibility::Private
                    },
                    ty: restore_type(&member.ty)?,
                    function: None,
                    span: source_span(member.span)?,
                },
            );
            if previous.is_some() {
                return None;
            }
        }
        Some(signature)
    }
}

/// 转换已验证的半开源码区间。
fn source_span(span: IrSpan) -> Option<SourceSpan> {
    SourceSpan::new(span.start, span.end)
}

/// 把 IR 类型还原为类型层表示，供 Runtime 复用同一套赋值兼容检查。
#[must_use]
pub fn restore_type(ty: &IrType) -> Option<Type> {
    Some(match ty {
        IrType::None => Type::None,
        IrType::Dynamic => Type::Dynamic,
        IrType::Variable { id } => Type::Variable(TypeVarId::new(*id)),
        IrType::Scalar { name } => Type::scalar(match name.as_str() {
            "int" => ScalarType::Int,
            "sint" => ScalarType::Sint,
            "lint" => ScalarType::Lint,
            "float" => ScalarType::Float,
            "sfloat" => ScalarType::Sfloat,
            "lfloat" => ScalarType::Lfloat,
            "bool" => ScalarType::Bool,
            "str" => ScalarType::Str,
            _ => return None,
        }),
        IrType::Function {
            parameters,
            return_type,
        } => Type::Function {
            parameters: parameters.iter().map(restore_type).collect::<Option<_>>()?,
            return_type: Box::new(restore_type(return_type)?),
        },
        IrType::Array { shape } => Type::Array(match shape {
            IrArrayShape::Unknown => ArrayType::Unknown,
            IrArrayShape::Homogeneous { element, length } => ArrayType::Homogeneous {
                element: Box::new(restore_type(element)?),
                length: *length,
            },
            IrArrayShape::Heterogeneous { elements } => ArrayType::Heterogeneous {
                elements: elements.iter().map(restore_type).collect::<Option<_>>()?,
            },
        }),
        IrType::Tuple { elements } => {
            Type::Tuple(elements.iter().map(restore_type).collect::<Option<_>>()?)
        }
        IrType::DictTable { entries } | IrType::DictColumn { entries } => {
            let dict = DictType::new(
                entries
                    .iter()
                    .map(|entry| {
                        Some(DictEntryType {
                            key: entry.key.clone(),
                            value: Box::new(restore_type(&entry.value)?),
                        })
                    })
                    .collect::<Option<Vec<_>>>()?,
            );
            if matches!(ty, IrType::DictTable { .. }) {
                Type::DictTable(dict)
            } else {
                Type::DictColumn(dict)
            }
        }
        IrType::Set {
            members,
            allows_dynamic,
            empty,
            unknown,
        } => Type::Set(if *empty {
            SetType::Empty
        } else if *unknown {
            SetType::Unknown
        } else {
            SetType::heterogeneous_with_dynamic(
                members
                    .iter()
                    .map(restore_type)
                    .collect::<Option<Vec<_>>>()?,
                *allows_dynamic,
            )
        }),
        IrType::Table { name, kind } => Type::Table(TableType::new(
            name,
            match kind.as_str() {
                "singleton" => TableValueKind::Singleton,
                "constructor" => TableValueKind::Constructor,
                "instance" => TableValueKind::Instance,
                _ => return None,
            },
        )),
    })
}
