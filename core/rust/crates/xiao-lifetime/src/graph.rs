//! 强/弱所有权图和确定性排序。
//!
//! 图算法不依赖 AST 或类型层。它只接受稳定值身份，因此可以被 08 阶段
//! 直接复用，也可以在测试中单独注入损坏输入验证错误处理。

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::{self, Display, Formatter};

use crate::model::{OwnershipEdge, OwnershipKind, ValueId};

/// 所有权图输入或排序失败的原因。
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GraphError {
    /// 边引用了尚未登记的值。
    MissingNode(ValueId),
    /// 同一强边形成自环。
    SelfCycle(ValueId),
    /// 强边形成环；向量按稳定编号排列。
    StrongCycle(Vec<ValueId>),
    /// 请求的节点列表包含重复身份。
    DuplicateNode(ValueId),
}

impl Display for GraphError {
    /// 输出稳定的开发者诊断文本。
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingNode(id) => {
                write!(formatter, "ownership graph is missing value {}", id.get())
            }
            Self::SelfCycle(id) => write!(
                formatter,
                "ownership graph contains self-cycle at {}",
                id.get()
            ),
            Self::StrongCycle(ids) => {
                write!(formatter, "ownership graph contains strong cycle: {ids:?}")
            }
            Self::DuplicateNode(id) => write!(
                formatter,
                "ownership graph received duplicate value {}",
                id.get()
            ),
        }
    }
}

impl std::error::Error for GraphError {}

/// 只保存值身份和声明顺序的所有权图。
#[derive(Clone, Debug, Eq, PartialEq, Default)]
pub struct OwnershipGraph {
    declaration_order: BTreeMap<ValueId, usize>,
    strong_edges: Vec<OwnershipEdge>,
    weak_edges: Vec<OwnershipEdge>,
}

impl OwnershipGraph {
    /// 创建空所有权图。
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// 登记一个节点及其声明序号。
    pub fn add_node(&mut self, id: ValueId, declaration_order: usize) -> Result<(), GraphError> {
        if self.declaration_order.contains_key(&id) {
            return Err(GraphError::DuplicateNode(id));
        }
        self.declaration_order.insert(id, declaration_order);
        Ok(())
    }

    /// 以零声明序号登记一个节点的便捷别名。
    pub fn insert_node(&mut self, id: ValueId) -> Result<(), GraphError> {
        self.add_node(id, id.get() as usize)
    }

    /// 从节点身份和边集合构建图。
    pub fn from_edges(
        nodes: impl IntoIterator<Item = (ValueId, usize)>,
        edges: impl IntoIterator<Item = OwnershipEdge>,
    ) -> Result<Self, GraphError> {
        let mut graph = Self::new();
        for (id, order) in nodes {
            graph.add_node(id, order)?;
        }
        for edge in edges {
            graph.add_edge(edge)?;
        }
        Ok(graph)
    }

    /// 登记一条强或弱所有权边。
    pub fn add_edge(&mut self, edge: OwnershipEdge) -> Result<(), GraphError> {
        if !self.declaration_order.contains_key(&edge.from) {
            return Err(GraphError::MissingNode(edge.from));
        }
        if !self.declaration_order.contains_key(&edge.to) {
            return Err(GraphError::MissingNode(edge.to));
        }
        if edge.kind == OwnershipKind::Strong && edge.from == edge.to {
            return Err(GraphError::SelfCycle(edge.from));
        }
        let edges = if edge.kind == OwnershipKind::Strong {
            &mut self.strong_edges
        } else {
            &mut self.weak_edges
        };
        if !edges.contains(&edge) {
            edges.push(edge);
            edges.sort_by_key(|item| (item.from, item.to, item.reason, item.span));
        }
        Ok(())
    }

    /// 登记一条强拥有边的便捷接口。
    pub fn add_strong_edge(
        &mut self,
        from: ValueId,
        to: ValueId,
        reason: crate::model::OwnershipEdgeReason,
    ) -> Result<(), GraphError> {
        self.add_edge(OwnershipEdge::strong(from, to, reason))
    }

    /// 登记一条弱引用边的便捷接口。
    pub fn add_weak_edge(
        &mut self,
        from: ValueId,
        to: ValueId,
        reason: crate::model::OwnershipEdgeReason,
    ) -> Result<(), GraphError> {
        self.add_edge(OwnershipEdge::weak(from, to, reason))
    }

    /// 返回已登记节点及其声明序号。
    #[must_use]
    pub fn nodes(&self) -> &BTreeMap<ValueId, usize> {
        &self.declaration_order
    }

    /// 返回强边切片。
    #[must_use]
    pub fn strong_edges(&self) -> &[OwnershipEdge] {
        &self.strong_edges
    }

    /// 返回弱边切片。
    #[must_use]
    pub fn weak_edges(&self) -> &[OwnershipEdge] {
        &self.weak_edges
    }

    /// 检查强图是否存在环，并返回稳定的环节点集合。
    #[must_use]
    pub fn strong_cycle(&self) -> Option<Vec<ValueId>> {
        let mut state = BTreeMap::<ValueId, u8>::new();
        let mut stack = Vec::new();
        for id in self.declaration_order.keys().copied() {
            if state.get(&id).copied().unwrap_or(0) != 0 {
                continue;
            }
            if let Some(mut cycle) = self.find_cycle(id, &mut state, &mut stack) {
                cycle.sort_unstable();
                cycle.dedup();
                return Some(cycle);
            }
        }
        None
    }

    /// 对指定节点生成确定性的强释放顺序。
    ///
    /// 边 `A -> B` 表示 A 先于 B。每一层内按声明序号逆序；声明序号相同
    /// 时按值身份逆序，确保损坏或合成输入仍然可复现。
    pub fn release_order(&self, nodes: &[ValueId]) -> Result<Vec<ValueId>, GraphError> {
        let mut selected = BTreeSet::new();
        for id in nodes {
            if !self.declaration_order.contains_key(id) {
                return Err(GraphError::MissingNode(*id));
            }
            if !selected.insert(*id) {
                return Err(GraphError::DuplicateNode(*id));
            }
        }
        let mut indegree = selected
            .iter()
            .copied()
            .map(|id| (id, 0usize))
            .collect::<BTreeMap<_, _>>();
        for edge in &self.strong_edges {
            if selected.contains(&edge.from) && selected.contains(&edge.to) {
                if let Some(degree) = indegree.get_mut(&edge.to) {
                    *degree = degree.saturating_add(1);
                }
            }
        }
        let mut layer = indegree
            .iter()
            .filter_map(|(id, degree)| (*degree == 0).then_some(*id))
            .collect::<Vec<_>>();
        let mut output = Vec::with_capacity(selected.len());
        while !layer.is_empty() {
            layer.sort_by_key(|id| self.sort_key(*id));
            let current = std::mem::take(&mut layer);
            for id in current {
                output.push(id);
                for edge in self
                    .strong_edges
                    .iter()
                    .filter(|edge| edge.from == id && selected.contains(&edge.to))
                {
                    if let Some(degree) = indegree.get_mut(&edge.to) {
                        *degree = degree.saturating_sub(1);
                        if *degree == 0 {
                            layer.push(edge.to);
                        }
                    }
                }
            }
        }
        if output.len() != selected.len() {
            let cycle = selected
                .iter()
                .copied()
                .filter(|id| !output.contains(id))
                .collect::<Vec<_>>();
            return Err(GraphError::StrongCycle(cycle));
        }
        Ok(output)
    }

    /// `release_order` 的通用拓扑排序别名。
    pub fn topological_order(&self, nodes: &[ValueId]) -> Result<Vec<ValueId>, GraphError> {
        self.release_order(nodes)
    }

    /// 对全部节点生成释放顺序。
    pub fn release_order_all(&self) -> Result<Vec<ValueId>, GraphError> {
        self.release_order(&self.declaration_order.keys().copied().collect::<Vec<_>>())
    }

    /// 按声明序号和稳定身份生成拓扑层排序键。
    fn sort_key(&self, id: ValueId) -> (std::cmp::Reverse<usize>, std::cmp::Reverse<ValueId>) {
        (
            std::cmp::Reverse(*self.declaration_order.get(&id).unwrap_or(&0)),
            std::cmp::Reverse(id),
        )
    }

    /// 深度优先查找第一条稳定强环。
    fn find_cycle(
        &self,
        id: ValueId,
        state: &mut BTreeMap<ValueId, u8>,
        stack: &mut Vec<ValueId>,
    ) -> Option<Vec<ValueId>> {
        state.insert(id, 1);
        stack.push(id);
        let mut targets = self
            .strong_edges
            .iter()
            .filter(|edge| edge.from == id)
            .map(|edge| edge.to)
            .collect::<Vec<_>>();
        targets.sort_unstable();
        targets.dedup();
        for target in targets {
            match state.get(&target).copied().unwrap_or(0) {
                0 => {
                    if let Some(cycle) = self.find_cycle(target, state, stack) {
                        return Some(cycle);
                    }
                }
                1 => {
                    let start = stack.iter().position(|candidate| *candidate == target)?;
                    return Some(stack[start..].to_vec());
                }
                _ => {}
            }
        }
        let _ = stack.pop();
        state.insert(id, 2);
        None
    }
}
