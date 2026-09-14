//! C1 选择结果的静态形状重建。
//!
//! 本模块根据已经解析的路径投影 `Type`，不创建运行时值。重复路径和
//! 字典列重复键会显式转为元组，确保类型表示不会伪造重复键的字典列。

use crate::containers::{ArrayType, ContainerPathSegment, DictEntryType, DictType};
use crate::selection_model::{SelectionPath, SelectionPathSegment};
use crate::types::Type;

/// 枚举一个已知有序容器的直接路径和元素类型。
#[must_use]
pub fn direct_selection_children(root: &Type) -> Option<Vec<(ContainerPathSegment, Type)>> {
    match root {
        Type::Array(ArrayType::Heterogeneous { elements }) => Some(
            elements
                .iter()
                .enumerate()
                .map(|(index, ty)| (ContainerPathSegment::Index(index), ty.clone()))
                .collect(),
        ),
        Type::Array(ArrayType::Homogeneous {
            element,
            length: Some(length),
        }) => Some(
            (0..*length)
                .map(|index| (ContainerPathSegment::Index(index), element.as_ref().clone()))
                .collect(),
        ),
        Type::Tuple(elements) => Some(
            elements
                .iter()
                .enumerate()
                .map(|(index, ty)| (ContainerPathSegment::Index(index), ty.clone()))
                .collect(),
        ),
        Type::DictColumn(dictionary) => Some(
            dictionary
                .entries
                .iter()
                .enumerate()
                .map(|(index, entry)| {
                    (
                        ContainerPathSegment::Index(index),
                        entry.value.as_ref().clone(),
                    )
                })
                .collect(),
        ),
        Type::Array(ArrayType::Homogeneous { length: None, .. })
        | Type::Array(ArrayType::Unknown)
        | Type::Scalar(_)
        | Type::Set(_)
        | Type::DictTable(_)
        | Type::Dynamic
        | Type::Variable(_)
        | Type::Function { .. }
        | Type::None => None,
    }
}

/// 返回一个来源容器对应的空选择结果。
#[must_use]
pub fn empty_selection_type(source: &Type) -> Type {
    match source {
        Type::Array(ArrayType::Homogeneous { element, .. }) => Type::Array(
            ArrayType::homogeneous_with_length(element.as_ref().clone(), 0),
        ),
        Type::Array(ArrayType::Heterogeneous { .. } | ArrayType::Unknown) => {
            Type::Array(ArrayType::heterogeneous(Vec::<Type>::new()))
        }
        Type::Tuple(_) => Type::Tuple(Vec::new()),
        Type::DictColumn(_) => Type::DictColumn(DictType::new(Vec::<DictEntryType>::new())),
        Type::Scalar(xiao_syntax::ScalarType::Str) => source.clone(),
        _ => Type::Dynamic,
    }
}

/// 根据静态路径投影来源类型。
#[must_use]
pub fn project_selection_type(
    source: &Type,
    paths: &[SelectionPath],
    dictionary_repetition_as_tuple: bool,
) -> Type {
    if paths.is_empty() {
        return empty_selection_type(source);
    }
    if matches!(source, Type::DictColumn(_))
        && (dictionary_repetition_as_tuple || has_repeated_direct_key(source, paths))
    {
        return Type::Tuple(
            paths
                .iter()
                .filter_map(|path| project_path_type(source, path))
                .collect(),
        );
    }
    project_root(source, paths)
}

/// 递归投影一层来源容器。
fn project_root(source: &Type, paths: &[SelectionPath]) -> Type {
    let groups = group_direct_paths(source, paths);
    match source {
        Type::Array(array) => {
            let children = groups
                .iter()
                .filter_map(|group| project_group_type(source, group))
                .collect::<Vec<_>>();
            if children.is_empty() {
                return empty_selection_type(source);
            }
            match array {
                ArrayType::Homogeneous { element, .. }
                    if children.iter().all(|child| child == element.as_ref()) =>
                {
                    Type::Array(ArrayType::homogeneous_with_length(
                        element.as_ref().clone(),
                        children.len(),
                    ))
                }
                _ => Type::Array(ArrayType::heterogeneous(children)),
            }
        }
        Type::Tuple(_) => Type::Tuple(
            groups
                .iter()
                .filter_map(|group| project_group_type(source, group))
                .collect(),
        ),
        Type::DictColumn(dictionary) => {
            let mut entries = Vec::new();
            for group in &groups {
                let Some(entry) = dictionary.entries.get(group.index) else {
                    continue;
                };
                let Some(value) = project_group_type(source, group) else {
                    continue;
                };
                entries.push(DictEntryType {
                    key: entry.key.clone(),
                    value: Box::new(value),
                });
            }
            Type::DictColumn(DictType::new(entries))
        }
        Type::Scalar(xiao_syntax::ScalarType::Str) => source.clone(),
        _ => Type::Dynamic,
    }
}

/// 一组共享直接父节点的后缀路径。
#[derive(Clone, Debug)]
struct DirectPathGroup {
    /// 直接子节点的规范化位置。
    index: usize,
    /// 该子节点下的后缀；空后缀表示直接选择整个子节点。
    suffixes: Vec<SelectionPath>,
}

/// 按直接位置分组路径，同时保留重复的直接选择项。
fn group_direct_paths(source: &Type, paths: &[SelectionPath]) -> Vec<DirectPathGroup> {
    let mut groups = Vec::new();
    for path in paths {
        let Some(index) = direct_index_for_path(source, path) else {
            continue;
        };
        let suffix = path.iter().skip(1).cloned().collect::<SelectionPath>();
        if suffix.is_empty() {
            // 直接选择同一位置两次必须保留两项；若已有更深路径，
            // 也不要把显式的父节点选择悄悄吞掉。
            groups.push(DirectPathGroup {
                index,
                suffixes: vec![suffix],
            });
        } else if let Some(group) = groups.iter_mut().find(|group| {
            group.index == index && !group.suffixes.iter().any(|suffix| suffix.is_empty())
        }) {
            group.suffixes.push(suffix);
        } else {
            groups.push(DirectPathGroup {
                index,
                suffixes: vec![suffix],
            });
        }
    }
    groups
}

/// 计算一组共享父节点的投影类型。
fn project_group_type(source: &Type, group: &DirectPathGroup) -> Option<Type> {
    let child = direct_selection_children(source)?
        .get(group.index)
        .map(|(_, ty)| ty.clone())?;
    if group.suffixes.iter().any(SelectionPath::is_empty) {
        return Some(child);
    }
    Some(project_root(&child, &group.suffixes))
}

/// 获取路径的直接数组/元组/字典列位置。
fn direct_index_for_path(source: &Type, path: &SelectionPath) -> Option<usize> {
    let segment = path.first()?;
    match source {
        Type::Array(_) | Type::Tuple(_) => match segment {
            SelectionPathSegment::Index {
                resolved: Some(index),
                ..
            } => Some(*index),
            _ => None,
        },
        Type::DictColumn(dictionary) => match segment {
            SelectionPathSegment::Index {
                resolved: Some(index),
                ..
            } => Some(*index),
            SelectionPathSegment::Key(key) => dictionary
                .entries
                .iter()
                .position(|entry| entry.key == *key),
            SelectionPathSegment::Index { resolved: None, .. } => None,
        },
        _ => None,
    }
}

/// 解析一条路径的叶子类型，并在有后缀时递归投影。
fn project_path_type(source: &Type, path: &SelectionPath) -> Option<Type> {
    let (segment, suffix) = path.split_first()?;
    let child =
        if let (Type::DictColumn(dictionary), SelectionPathSegment::Key(key)) = (source, segment) {
            dictionary
                .entries
                .iter()
                .find(|entry| entry.key == *key)
                .map(|entry| entry.value.as_ref().clone())
        } else {
            direct_selection_children(source)?
                .into_iter()
                .find_map(|(candidate, ty)| match (candidate, segment) {
                    (
                        ContainerPathSegment::Index(candidate),
                        SelectionPathSegment::Index {
                            resolved: Some(index),
                            ..
                        },
                    ) if candidate == *index => Some(ty),
                    (
                        ContainerPathSegment::Index(_),
                        SelectionPathSegment::Index { resolved: None, .. },
                    ) => None,
                    (ContainerPathSegment::Index(_), SelectionPathSegment::Key(_)) => None,
                    _ => None,
                })
        }?;
    if suffix.is_empty() {
        Some(child)
    } else {
        Some(project_root(&child, &[suffix.to_vec()]))
    }
}

/// 判断字典列路径是否重复选择同一个直接键。
fn has_repeated_direct_key(source: &Type, paths: &[SelectionPath]) -> bool {
    if !matches!(source, Type::DictColumn(_)) {
        return false;
    }
    let mut seen = Vec::new();
    for path in paths {
        let Some(index) = direct_index_for_path(source, path) else {
            continue;
        };
        if seen.contains(&index) {
            return true;
        }
        seen.push(index);
    }
    false
}

#[cfg(test)]
/// 覆盖空结果、异构投影和字典列重复键规则。
mod tests {
    use super::{empty_selection_type, project_selection_type};
    use crate::{ArrayType, DictEntryType, DictType, SelectionPathSegment, Type};
    use xiao_syntax::ScalarType;

    #[test]
    /// 多选异构数组应按源码顺序形成异构结果。
    fn projects_heterogeneous_array() {
        let source = Type::array_literal(vec![
            Type::scalar(ScalarType::Int),
            Type::scalar(ScalarType::Str),
        ]);
        let paths = vec![
            vec![SelectionPathSegment::resolved_index(1, 1)],
            vec![SelectionPathSegment::resolved_index(0, 0)],
        ];
        assert_eq!(
            project_selection_type(&source, &paths, false),
            Type::Array(ArrayType::heterogeneous(vec![
                Type::scalar(ScalarType::Str),
                Type::scalar(ScalarType::Int),
            ]))
        );
    }

    #[test]
    /// 字典列重复选择不能伪造重复键，必须转为元组。
    fn repeats_dictionary_key_as_tuple() {
        let source = Type::DictColumn(DictType::new(vec![DictEntryType {
            key: "name".to_owned(),
            value: Box::new(Type::scalar(ScalarType::Str)),
        }]));
        let paths = vec![
            vec![SelectionPathSegment::resolved_index(0, 0)],
            vec![SelectionPathSegment::key("name")],
        ];
        assert!(matches!(
            project_selection_type(&source, &paths, true),
            Type::Tuple(items) if items.len() == 2
        ));
        assert!(matches!(empty_selection_type(&source), Type::DictColumn(_)));
    }
}
