//! C0 容器路径的语义化转换与静态解析。
//!
//! 语法层只保存 [`xiao_syntax::IndexPath`]；本模块把它转换为不依赖 AST 的
//! `ContainerPathSegment`，并在已知容器形状上执行精确路径检查。范围、多选、
//! 步长和随机选择不会在这里被偷偷降级，它们由检查器以稳定诊断拒绝。

use std::fmt::{self, Display, Formatter};

use xiao_source::SourceFile;
use xiao_syntax::{IndexPath, PathSegment};

use crate::containers::{ArrayType, ContainerPathSegment, DictType};
use crate::types::Type;

/// 将语法路径转换失败的原因。
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PathConversionErrorKind {
    /// C0 不接受负索引。
    NegativeIndex,
    /// 数字索引无法解析为平台无关的非负整数。
    InvalidIndex,
    /// 路径段为空；仅用于防御性检查。
    EmptySegment,
}

/// 一个带路径段序号的转换错误。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PathConversionError {
    /// 出错的零基路径段序号。
    pub segment: usize,
    /// 具体转换原因。
    pub kind: PathConversionErrorKind,
}

impl Display for PathConversionError {
    /// 生成供诊断预览使用的稳定文本。
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self.kind {
            PathConversionErrorKind::NegativeIndex => {
                write!(
                    formatter,
                    "path segment {} cannot be negative",
                    self.segment
                )
            }
            PathConversionErrorKind::InvalidIndex => {
                write!(
                    formatter,
                    "path segment {} is not a valid index",
                    self.segment
                )
            }
            PathConversionErrorKind::EmptySegment => {
                write!(formatter, "path segment {} is empty", self.segment)
            }
        }
    }
}

impl std::error::Error for PathConversionError {}

/// 将静态语法索引路径转换成类型层路径。
pub fn lower_index_path(
    source: &SourceFile,
    path: &IndexPath,
) -> Result<Vec<ContainerPathSegment>, PathConversionError> {
    path.segments
        .iter()
        .enumerate()
        .map(|(segment, value)| match value {
            PathSegment::Integer { span, negative } => {
                if *negative {
                    return Err(PathConversionError {
                        segment,
                        kind: PathConversionErrorKind::NegativeIndex,
                    });
                }
                let text = source.slice(*span);
                let value = text.parse::<usize>().map_err(|_| PathConversionError {
                    segment,
                    kind: PathConversionErrorKind::InvalidIndex,
                })?;
                Ok(ContainerPathSegment::Index(value))
            }
            PathSegment::Name(name) => {
                let text = name.unquoted_text(source);
                if text.is_empty() {
                    Err(PathConversionError {
                        segment,
                        kind: PathConversionErrorKind::EmptySegment,
                    })
                } else {
                    Ok(ContainerPathSegment::Key(text.to_owned()))
                }
            }
        })
        .collect()
}

/// 精确路径解析失败的原因。
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PathResolutionErrorKind {
    /// 当前容器不接受此种路径段。
    WrongSegment,
    /// 静态已知的数字索引越界。
    OutOfBounds { length: usize },
    /// 静态已知的字典键不存在。
    MissingKey,
}

/// 一个带路径段序号的精确路径解析错误。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PathResolutionError {
    /// 出错的零基路径段序号。
    pub segment: usize,
    /// 具体解析原因。
    pub kind: PathResolutionErrorKind,
}

impl Display for PathResolutionError {
    /// 生成供诊断预览使用的稳定文本。
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self.kind {
            PathResolutionErrorKind::WrongSegment => {
                write!(
                    formatter,
                    "path segment {} does not match the container",
                    self.segment
                )
            }
            PathResolutionErrorKind::OutOfBounds { length } => write!(
                formatter,
                "path segment {} is outside container length {}",
                self.segment, length
            ),
            PathResolutionErrorKind::MissingKey => {
                write!(
                    formatter,
                    "path key at segment {} does not exist",
                    self.segment
                )
            }
        }
    }
}

impl std::error::Error for PathResolutionError {}

/// 在已知容器类型上解析一条精确路径。
///
/// 未知数组边界和 `dynamic` 值返回 `Type::Dynamic`，表示需要后续运行时
/// 检查；只有编译期能够证明错误时才返回 `Err`。
pub fn resolve_exact_path(
    root: &Type,
    path: &[ContainerPathSegment],
) -> Result<Type, PathResolutionError> {
    let mut current = root.clone();
    for (segment, part) in path.iter().enumerate() {
        current = resolve_segment(&current, part, segment)?;
    }
    Ok(current)
}

/// 解析一层路径段。
fn resolve_segment(
    current: &Type,
    part: &ContainerPathSegment,
    segment: usize,
) -> Result<Type, PathResolutionError> {
    match current {
        Type::Dynamic | Type::Variable(_) => Ok(Type::Dynamic),
        Type::Array(array) => match part {
            ContainerPathSegment::Index(index) => array_element(array, *index, segment),
            ContainerPathSegment::Key(_) => Err(PathResolutionError {
                segment,
                kind: PathResolutionErrorKind::WrongSegment,
            }),
        },
        Type::Tuple(items) => match part {
            ContainerPathSegment::Index(index) => {
                items.get(*index).cloned().ok_or(PathResolutionError {
                    segment,
                    kind: PathResolutionErrorKind::OutOfBounds {
                        length: items.len(),
                    },
                })
            }
            ContainerPathSegment::Key(_) => Err(PathResolutionError {
                segment,
                kind: PathResolutionErrorKind::WrongSegment,
            }),
        },
        Type::DictTable(dictionary) => resolve_dictionary(dictionary, part, segment, false),
        Type::DictColumn(dictionary) => resolve_dictionary(dictionary, part, segment, true),
        Type::Function { .. } | Type::Scalar(_) | Type::None => Err(PathResolutionError {
            segment,
            kind: PathResolutionErrorKind::WrongSegment,
        }),
    }
}

/// 解析数组一层索引。
fn array_element(
    array: &ArrayType,
    index: usize,
    segment: usize,
) -> Result<Type, PathResolutionError> {
    match array {
        ArrayType::Homogeneous { element, length } => {
            if let Some(length) = length
                && index >= *length
            {
                return Err(PathResolutionError {
                    segment,
                    kind: PathResolutionErrorKind::OutOfBounds { length: *length },
                });
            }
            Ok(element.as_ref().clone())
        }
        ArrayType::Heterogeneous { elements } => {
            elements.get(index).cloned().ok_or(PathResolutionError {
                segment,
                kind: PathResolutionErrorKind::OutOfBounds {
                    length: elements.len(),
                },
            })
        }
        ArrayType::Unknown => Ok(Type::Dynamic),
    }
}

/// 解析字典表/字典列的一层路径。
fn resolve_dictionary(
    dictionary: &DictType,
    part: &ContainerPathSegment,
    segment: usize,
    allow_numeric: bool,
) -> Result<Type, PathResolutionError> {
    match part {
        ContainerPathSegment::Key(key) => {
            dictionary
                .value_type(key)
                .cloned()
                .ok_or(PathResolutionError {
                    segment,
                    kind: PathResolutionErrorKind::MissingKey,
                })
        }
        ContainerPathSegment::Index(index) if allow_numeric => dictionary
            .entries
            .get(*index)
            .map(|entry| entry.value.as_ref().clone())
            .ok_or(PathResolutionError {
                segment,
                kind: PathResolutionErrorKind::OutOfBounds {
                    length: dictionary.entries.len(),
                },
            }),
        ContainerPathSegment::Index(_) => Err(PathResolutionError {
            segment,
            kind: PathResolutionErrorKind::WrongSegment,
        }),
    }
}

#[cfg(test)]
/// 覆盖路径下降和静态错误分类。
mod tests {
    use super::{PathResolutionErrorKind, resolve_exact_path};
    use crate::{ArrayType, ContainerPathSegment, DictEntryType, DictType, Type};
    use xiao_syntax::ScalarType;

    #[test]
    /// 确认数组和字典列可以沿精确路径解析到叶子类型。
    fn resolves_nested_paths() {
        let root = Type::Array(ArrayType::heterogeneous(vec![
            Type::scalar(ScalarType::Str),
            Type::DictColumn(DictType::new(vec![DictEntryType {
                key: "name".to_owned(),
                value: Box::new(Type::scalar(ScalarType::Int)),
            }])),
        ]));
        let path = vec![
            ContainerPathSegment::Index(1),
            ContainerPathSegment::Key("name".to_owned()),
        ];
        assert_eq!(
            resolve_exact_path(&root, &path),
            Ok(Type::scalar(ScalarType::Int))
        );
    }

    #[test]
    /// 确认静态数组越界不会被误报为动态值。
    fn reports_static_bounds() {
        let root = Type::array_literal(vec![Type::scalar(ScalarType::Int)]);
        let error =
            resolve_exact_path(&root, &[ContainerPathSegment::Index(1)]).expect_err("索引应越界");
        assert!(matches!(
            error.kind,
            PathResolutionErrorKind::OutOfBounds { .. }
        ));
    }
}
