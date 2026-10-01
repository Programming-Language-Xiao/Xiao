//! 优化器可读取的 IR 效果、所有权和别名事实。

use serde::{Deserialize, Serialize};
use xiao_ir::IrProgram;

/// 一份只读的程序事实摘要。
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct ProgramFacts {
    /// 控制流基本块数量。
    pub control_flow_blocks: usize,
    /// 源码映射区间数量。
    pub source_mapped_spans: usize,
    /// 运行时检查数量。
    pub runtime_checks: usize,
    /// 选择计划数量。
    pub selection_plans: usize,
    /// 随机种子计划数量。
    pub random_seed_plans: usize,
    /// 广播赋值计划数量。
    pub broadcast_plans: usize,
    /// 只读所有权摘要。
    pub ownership: OwnershipFacts,
    /// 只读别名和逃逸摘要。
    pub aliases: AliasFacts,
    /// 只读效果摘要。
    pub effects: EffectFacts,
}

impl ProgramFacts {
    /// 从已验证 IR 建立只读事实摘要。
    #[must_use]
    pub fn from_program(program: &IrProgram) -> Self {
        let ownership = OwnershipFacts {
            values: program.ownership.values.len(),
            strong_edges: program.ownership.strong_edges.len(),
            weak_edges: program.ownership.weak_edges.len(),
            release_plans: program.ownership.release_plans.len(),
            escaped_values: program
                .ownership
                .values
                .iter()
                .filter(|value| !value.escapes.is_empty())
                .count(),
            dynamic_checks: program.ownership.dynamic_checks.len(),
        };
        let effects = EffectFacts {
            has_runtime_checks: !program.runtime_checks.is_empty(),
            has_randomness: !program.random_seed_plans.is_empty()
                || program.selection_plans.iter().any(|plan| {
                    plan.items
                        .iter()
                        .any(|item| matches!(item, xiao_ir::IrSelectionItemPlan::Random { .. }))
                }),
            has_io_boundary: false,
            has_dynamic_values: program
                .ownership
                .values
                .iter()
                .any(|value| value.ty.as_ref().is_some_and(is_dynamic_type)),
            has_container_access: !program.selection_plans.is_empty()
                || !program.broadcast_assignment_plans.is_empty(),
        };
        let aliases = AliasFacts {
            possible_aliases: program.ownership.strong_edges.len()
                + program.ownership.weak_edges.len(),
            escaped_values: ownership.escaped_values,
            weak_references: ownership.weak_edges,
        };
        Self {
            control_flow_blocks: program.control_flow.blocks.len(),
            source_mapped_spans: program
                .control_flow
                .blocks
                .iter()
                .flat_map(|block| block.statements.iter())
                .count(),
            runtime_checks: program.runtime_checks.len(),
            selection_plans: program.selection_plans.len(),
            random_seed_plans: program.random_seed_plans.len(),
            broadcast_plans: program.broadcast_assignment_plans.len(),
            ownership,
            aliases,
            effects,
        }
    }
}

/// 所有权、逃逸和释放计划的只读摘要。
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct OwnershipFacts {
    /// 值槽数量。
    pub values: usize,
    /// 强拥有边数量。
    pub strong_edges: usize,
    /// 弱引用边数量。
    pub weak_edges: usize,
    /// 释放计划数量。
    pub release_plans: usize,
    /// 发生逃逸的值数量。
    pub escaped_values: usize,
    /// 动态生命周期检查数量。
    pub dynamic_checks: usize,
}

/// 别名、逃逸和弱引用的只读摘要。
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct AliasFacts {
    /// 可能产生别名关系的强/弱边数量。
    pub possible_aliases: usize,
    /// 发生逃逸的值数量。
    pub escaped_values: usize,
    /// 弱引用边数量。
    pub weak_references: usize,
}

/// 可能阻止重排的效果事实摘要。
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct EffectFacts {
    /// 是否包含运行时检查。
    pub has_runtime_checks: bool,
    /// 是否包含随机选择或随机种子。
    pub has_randomness: bool,
    /// 是否包含输入输出边界；当前 IR 不登记时保持 false。
    pub has_io_boundary: bool,
    /// 是否包含动态值。
    pub has_dynamic_values: bool,
    /// 是否包含容器选择或广播访问。
    pub has_container_access: bool,
}

fn is_dynamic_type(ty: &xiao_ir::IrType) -> bool {
    matches!(
        ty,
        xiao_ir::IrType::Dynamic
            | xiao_ir::IrType::Array { .. }
            | xiao_ir::IrType::Tuple { .. }
            | xiao_ir::IrType::DictTable { .. }
            | xiao_ir::IrType::DictColumn { .. }
            | xiao_ir::IrType::Set { .. }
            | xiao_ir::IrType::Table { .. }
    )
}
