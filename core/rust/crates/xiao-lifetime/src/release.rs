//! 作用域退出边的确定性释放计划构建。
//!
//! 本模块把 `escape` 收集的值、边和转移集合交给纯图算法处理。它只生成
//! 数据，不调用 `drop`、不修改 Runtime 状态；同一作用域/退出边中的值最多
//! 出现一次，强环则保留诊断并使用声明逆序作为可观测的错误路径兜底。

use std::collections::BTreeSet;

use crate::diagnostics;
use crate::escape::TransferFacts;
use crate::graph::{GraphError, OwnershipGraph};
use crate::model::{ExitKind, LifetimeResult, ReleaseAction, ReleaseActionKind, ReleasePlan};

/// 所有退出边种类的稳定顺序。
const ALL_EXITS: [ExitKind; 11] = [
    ExitKind::Normal,
    ExitKind::Return,
    ExitKind::Break,
    ExitKind::Continue,
    ExitKind::Error,
    ExitKind::Raise,
    ExitKind::Catch,
    ExitKind::UnmatchedError,
    ExitKind::ConstructFailure,
    ExitKind::DynamicCheckFailure,
    ExitKind::Fatal,
];

/// 为分析结果建立所有权图和每个作用域的释放计划。
pub(crate) fn finalize(mut result: LifetimeResult, transfers: TransferFacts) -> LifetimeResult {
    let mut graph = OwnershipGraph::new();
    for value in result.values.values() {
        if let Err(error) = graph.add_node(value.id, value.declaration_order) {
            result
                .diagnostics
                .push(diagnostics::invalid_edge(Some(value.span), &error));
        }
    }
    for edge in result.strong_edges.iter().chain(result.weak_edges.iter()) {
        if let Err(error) = graph.add_edge(edge.clone()) {
            if let GraphError::SelfCycle(id) = error {
                result
                    .diagnostics
                    .push(diagnostics::strong_cycle(edge.span, &[id]));
            } else {
                result
                    .diagnostics
                    .push(diagnostics::invalid_edge(edge.span, &error));
            }
        }
    }

    if let Some(cycle) = graph.strong_cycle() {
        let span = cycle
            .iter()
            .filter_map(|id| result.values.get(id).map(|value| value.span))
            .next();
        result
            .diagnostics
            .push(diagnostics::strong_cycle(span, &cycle));
    }

    let scope_ids = result.scopes.keys().copied().collect::<Vec<_>>();
    for scope_id in scope_ids {
        let Some(scope) = result.scopes.get(&scope_id).cloned() else {
            continue;
        };
        let scope_values = scope
            .values
            .iter()
            .copied()
            .filter(|id| result.values.get(id).is_some_and(|value| !value.temporary))
            .collect::<Vec<_>>();
        for exit in ALL_EXITS {
            let transferred = transfers
                .get(&(scope_id, exit))
                .cloned()
                .unwrap_or_default();
            let candidates = scope_values
                .iter()
                .copied()
                .filter(|id| {
                    result
                        .values
                        .get(id)
                        .is_some_and(|value| value.needs_release() && !transferred.contains(id))
                })
                .collect::<Vec<_>>();
            let order = match graph.release_order(&candidates) {
                Ok(order) => order,
                Err(GraphError::StrongCycle(cycle)) => {
                    // 全局诊断已在上面写入；这里保留可复现的声明逆序计划，
                    // 让后续错误展示和 IR 快照仍有确定输入。
                    let mut fallback = candidates.clone();
                    fallback.sort_by_key(|id| {
                        result
                            .values
                            .get(id)
                            .map(|value| {
                                (
                                    std::cmp::Reverse(value.declaration_order),
                                    std::cmp::Reverse(*id),
                                )
                            })
                            .unwrap_or((std::cmp::Reverse(0), std::cmp::Reverse(*id)))
                    });
                    let _ = cycle;
                    fallback
                }
                Err(_) => {
                    let mut fallback = candidates.clone();
                    fallback.sort_by_key(|id| {
                        result
                            .values
                            .get(id)
                            .map(|value| {
                                (
                                    std::cmp::Reverse(value.declaration_order),
                                    std::cmp::Reverse(*id),
                                )
                            })
                            .unwrap_or((std::cmp::Reverse(0), std::cmp::Reverse(*id)))
                    });
                    fallback
                }
            };
            let actions = order
                .into_iter()
                .enumerate()
                .filter_map(|(index, id)| {
                    let value = result.values.get(&id)?;
                    let kind = match value.storage.ownership() {
                        crate::model::OwnershipKind::Weak => ReleaseActionKind::Weak,
                        crate::model::OwnershipKind::Strong => ReleaseActionKind::Strong,
                    };
                    Some(ReleaseAction {
                        value: id,
                        order: index,
                        kind,
                    })
                })
                .collect::<Vec<_>>();
            let mut transferred = transferred.into_iter().collect::<Vec<_>>();
            transferred.sort_unstable();
            result.release_plans.insert(
                (scope_id, exit),
                ReleasePlan {
                    scope: scope_id,
                    exit,
                    actions,
                    transferred,
                },
            );
        }
    }
    result
}

/// 返回一个计划中不会重复出现的值集合，供后续验证器复用。
#[allow(dead_code)]
pub(crate) fn unique_values(plan: &ReleasePlan) -> bool {
    let mut values = BTreeSet::new();
    plan.actions
        .iter()
        .all(|action| values.insert(action.value))
}

/// 验证所有计划的动作顺序字段连续且从零开始。
#[allow(dead_code)]
pub(crate) fn plans_have_contiguous_orders(result: &LifetimeResult) -> bool {
    result.release_plans.values().all(|plan| {
        plan.actions
            .iter()
            .enumerate()
            .all(|(index, action)| action.order == index)
    })
}
