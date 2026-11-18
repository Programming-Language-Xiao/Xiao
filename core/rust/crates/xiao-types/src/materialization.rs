//! C0 空数组的静态物化计划辅助。
//!
//! 这里不创建运行时数组，也不填充默认值。它只把声明中的路径约束汇总为
//! 后端可消费的最小形状计划；真正的 `""`、`0`、`false` 等默认值属于
//! Runtime 阶段的职责。

use crate::containers::{ContainerMaterializationPlan, PathConstraintTree};

/// 根据一组路径约束建立空数组/未知容器的物化计划。
#[must_use]
pub fn build_plan(
    binding: impl Into<String>,
    constraints: &PathConstraintTree,
) -> ContainerMaterializationPlan {
    ContainerMaterializationPlan::new(binding, constraints.clone())
}

/// 将后声明的约束合并到旧约束中；相同路径由后者覆盖。
#[must_use]
pub fn merge_constraints(
    base: &PathConstraintTree,
    additional: &PathConstraintTree,
) -> PathConstraintTree {
    let mut merged = base.clone();
    for constraint in additional.constraints() {
        merged.insert(constraint.path.clone(), constraint.ty.as_ref().clone());
    }
    merged
}

#[cfg(test)]
/// 覆盖相同路径覆盖和最小形状计划。
mod tests {
    use super::{build_plan, merge_constraints};
    use crate::{ContainerPathSegment, PathConstraintTree, Type};
    use xiao_syntax::ScalarType;

    #[test]
    /// 确认后声明的精确路径覆盖同一路径，而不丢失其他路径。
    fn merges_constraints_by_path() {
        let mut first = PathConstraintTree::new();
        first.insert(
            vec![ContainerPathSegment::Index(2)],
            Type::scalar(ScalarType::Str),
        );
        let mut second = PathConstraintTree::new();
        second.insert(
            vec![ContainerPathSegment::Index(2)],
            Type::scalar(ScalarType::Int),
        );
        let merged = merge_constraints(&first, &second);
        assert_eq!(
            merged.get(&[ContainerPathSegment::Index(2)]),
            Some(&Type::scalar(ScalarType::Int))
        );
        assert_eq!(build_plan("items", &merged).binding, "items");
    }
}
