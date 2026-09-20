//! AST 控制流遍历与逃逸事实收集。
//!
//! 这里不生成最终释放顺序，而是把名称绑定、表达式引用、闭包捕获、容器持有
//! 和每条退出边上的转移事实记录到 `LifetimeResult`。`release` 模块随后消费
//! 这些事实。即使类型检查结果不完整，也必须采用保守的动态堆策略而不能 panic。

use std::collections::{BTreeMap, BTreeSet};

use xiao_source::{SourceFile, SourceSpan};
use xiao_syntax::{
    CatchClause, DeclaredType, ElifBranch, Expression, FunctionParameter, LiteralKind, Name,
    Program, ScalarType, Statement, TableKind, TypeTerm,
};
use xiao_types::{SetType, TableType, Type, TypeCheckResult};

use crate::diagnostics;
use crate::model::{
    BasicBlock, BlockId, ControlFlowEdgeKind, EscapeReason, ExitKind, LifetimeResult,
    OwnershipEdge, OwnershipEdgeReason, OwnershipKind, ScopeId, ScopeInfo, ScopeKind, StorageClass,
    ValueId, ValueInfo,
};

/// 一个退出边上的值转移事实。
pub(crate) type TransferFacts = BTreeMap<(ScopeId, ExitKind), BTreeSet<ValueId>>;

/// 表达式遍历产生的中间事实。
#[derive(Default)]
struct ExpressionFacts {
    /// 表达式中读取的已有值。
    references: Vec<ValueId>,
    /// 表达式求值后实际指向的堆对象或栈值。
    referents: Vec<ValueId>,
    /// 表达式产生的匿名堆值。
    produced: Option<ValueId>,
    /// 类型层给出的表达式类型。
    ty: Option<Type>,
    /// 是否包含动态边界。
    dynamic: bool,
    /// 是否包含 `new` 构造调用。
    construct: bool,
}

impl ExpressionFacts {
    /// 按源码出现顺序合并另一个事实集合并去重引用。
    fn merge(&mut self, other: Self) {
        for id in other.references {
            if !self.references.contains(&id) {
                self.references.push(id);
            }
        }
        if self.produced.is_none() {
            self.produced = other.produced;
        }
        for id in other.referents {
            if !self.referents.contains(&id) {
                self.referents.push(id);
            }
        }
        self.dynamic |= other.dynamic;
        self.construct |= other.construct;
        if self.ty.is_none() {
            self.ty = other.ty;
        }
    }
}

/// 一段语句序列的控制流摘要。
#[derive(Default)]
struct FlowSummary {
    /// 是否仍存在到达序列末尾的路径。
    normal: bool,
    /// 已观察到的非正常退出类别。
    exits: BTreeSet<ExitKind>,
}

/// 一个受保护语句片段在分析期间产生的错误出口。
///
/// `try`、`catch` 和 `finally` 各自消费自己的收集器，避免一个处理器
/// 把自己产生的错误错误地送回同一个 `catch` 链。片段结束后，未匹配出口
/// 才会显式登记到外层收集器。
#[derive(Default)]
struct TryContext {
    /// 该片段中由动态检查或 `raise` 创建的错误块。
    error_blocks: Vec<BlockId>,
    /// 该片段中尚未经过所属 `finally` 的控制转移源。
    control_exits: Vec<ControlExit>,
}

/// 尚未经过所属 `finally` 的控制转移及其最终目标。
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
struct ControlExit {
    /// 产生转移的基本块。
    source: BlockId,
    /// 转移类别。
    kind: ExitKind,
    /// 已解析的目标块；循环作用域结束后仍需保留该身份。
    target: Option<BlockId>,
}

impl FlowSummary {
    /// 创建仍可正常到达的空摘要。
    fn reachable() -> Self {
        Self {
            normal: true,
            exits: BTreeSet::new(),
        }
    }

    /// 创建终止于指定退出边的摘要。
    fn terminal(exit: ExitKind) -> Self {
        let mut exits = BTreeSet::new();
        exits.insert(exit);
        Self {
            normal: false,
            exits,
        }
    }
}

/// 当前函数的闭包捕获上下文。
struct FunctionContext {
    /// 函数作用域。
    scope: ScopeId,
    /// 函数值本身；捕获边从该值指向外层值。
    value: ValueId,
    /// 函数内所有 return 边汇合的退出块。
    exit: BlockId,
}

/// 当前循环上下文。
struct LoopContext {
    /// 循环体作用域。
    scope: ScopeId,
    /// 条件/迭代入口块。
    header: BlockId,
    /// break 和条件为假时到达的循环后继块。
    exit: BlockId,
}

/// 负责一次 AST 生命周期事实收集的内部分析器。
pub(crate) struct EscapeAnalyzer<'source> {
    source: &'source SourceFile,
    types: &'source TypeCheckResult,
    result: LifetimeResult,
    transfers: TransferFacts,
    scopes: Vec<ScopeId>,
    bindings: Vec<BTreeMap<String, ValueId>>,
    functions: Vec<FunctionContext>,
    loops: Vec<LoopContext>,
    next_scope: u32,
    next_value: u32,
    next_block: u32,
    next_order: BTreeMap<ScopeId, usize>,
    current_block: Option<BlockId>,
    try_contexts: Vec<TryContext>,
}

impl<'source> EscapeAnalyzer<'source> {
    /// 创建一个尚未运行的事实收集器。
    pub(crate) fn new(source: &'source SourceFile, types: &'source TypeCheckResult) -> Self {
        Self {
            source,
            types,
            result: LifetimeResult::default(),
            transfers: BTreeMap::new(),
            scopes: Vec::new(),
            bindings: Vec::new(),
            functions: Vec::new(),
            loops: Vec::new(),
            next_scope: 0,
            next_value: 0,
            next_block: 0,
            next_order: BTreeMap::new(),
            current_block: None,
            try_contexts: Vec::new(),
        }
    }

    /// 收集完整程序的事实，并返回结果和退出边转移表。
    pub(crate) fn run(mut self, program: &Program) -> (LifetimeResult, TransferFacts) {
        let root = self.push_scope(ScopeKind::Program, program.span, None);
        let entry = self.new_block(root, Some(program.span));
        self.result.control_flow.entry = Some(entry);
        self.current_block = Some(entry);
        let _ = self.analyze_statements(&program.statements);
        self.pop_scope();
        (self.result, self.transfers)
    }

    /// 创建一个新的词法作用域。
    fn push_scope(
        &mut self,
        kind: ScopeKind,
        span: SourceSpan,
        parent_override: Option<ScopeId>,
    ) -> ScopeId {
        let id = ScopeId::new(self.next_scope);
        self.next_scope = self.next_scope.saturating_add(1);
        let parent = parent_override.or_else(|| self.scopes.last().copied());
        let depth = self.scopes.len();
        self.result
            .scopes
            .insert(id, ScopeInfo::new(id, parent, kind, span, depth));
        self.next_order.insert(id, 0);
        self.scopes.push(id);
        self.bindings.push(BTreeMap::new());
        id
    }

    /// 安全退出当前作用域；损坏的内部状态不会导致用户程序 panic。
    fn pop_scope(&mut self) -> Option<ScopeId> {
        let scope = self.scopes.pop();
        let _ = self.bindings.pop();
        scope
    }

    /// 返回当前作用域；正常 AST 遍历始终至少有程序根。
    fn current_scope(&self) -> Option<ScopeId> {
        self.scopes.last().copied()
    }

    /// 创建基本块并登记到控制流图。
    fn new_block(&mut self, scope: ScopeId, span: Option<SourceSpan>) -> BlockId {
        let id = BlockId::new(self.next_block);
        self.next_block = self.next_block.saturating_add(1);
        self.result.control_flow.blocks.insert(
            id,
            BasicBlock {
                id,
                scope,
                span,
                statements: Vec::new(),
                successors: Vec::new(),
                exits: BTreeSet::new(),
            },
        );
        id
    }

    /// 登记一条控制流后继边。
    fn connect_blocks(&mut self, from: Option<BlockId>, to: BlockId, kind: ControlFlowEdgeKind) {
        let Some(from) = from else { return };
        let Some(block) = self.result.control_flow.blocks.get_mut(&from) else {
            return;
        };
        if !block.successors.contains(&(to, kind)) {
            block.successors.push((to, kind));
        }
    }

    /// 记录一个语句区间到当前块。
    fn record_statement_block(&mut self, span: SourceSpan) {
        if let Some(block) = self
            .current_block
            .and_then(|id| self.result.control_flow.blocks.get_mut(&id))
        {
            block.statements.push(span);
        }
    }

    /// 在当前基本块登记一条可能退出边。
    fn record_block_exit(&mut self, exit: ExitKind) {
        if let Some(current) = self.current_block {
            if let Some(block) = self.result.control_flow.blocks.get_mut(&current) {
                block.exits.insert(exit);
            }
            if matches!(
                exit,
                ExitKind::Return | ExitKind::Break | ExitKind::Continue
            ) {
                self.register_control_exit(current, exit);
            }
        }
    }

    /// 为当前块建立一条错误后继边，但保持成功路径仍在当前块继续。
    fn connect_error_block(&mut self, span: SourceSpan, exit: ExitKind) {
        let Some(scope) = self.current_scope() else {
            return;
        };
        let from = self.current_block;
        let block = self.new_block(scope, Some(span));
        if let Some(info) = self.result.control_flow.blocks.get_mut(&block) {
            info.exits.insert(exit);
        }
        self.connect_blocks(from, block, ControlFlowEdgeKind::Error);
        self.register_error_block(block);
    }

    /// 把错误块放入当前片段；没有片段时表示它已经到达程序外层。
    fn register_error_block(&mut self, block: BlockId) {
        if let Some(context) = self.try_contexts.last_mut() {
            if !context.error_blocks.contains(&block) {
                context.error_blocks.push(block);
            }
        }
    }

    /// 向一个合成块连接一组退出来源并保持边去重。
    fn connect_exit_sources(
        &mut self,
        sources: &[BlockId],
        target: BlockId,
        kind: ControlFlowEdgeKind,
    ) {
        for source in sources {
            self.connect_blocks(Some(*source), target, kind);
        }
    }

    /// 移除控制转移在进入 `finally` 前建立的直达边，禁止绕过清理块。
    fn disconnect_control_exit(&mut self, source: BlockId, exit: ExitKind) {
        let edge = control_edge(exit);
        if let Some(block) = self.result.control_flow.blocks.get_mut(&source) {
            block.successors.retain(|(_, kind)| *kind != edge);
        }
    }

    /// 将尚未清理的控制退出登记到当前错误控制片段。
    fn register_control_exit(&mut self, source: BlockId, exit: ExitKind) {
        let target = match exit {
            ExitKind::Return => self.functions.last().map(|context| context.exit),
            ExitKind::Break => self.loops.last().map(|context| context.exit),
            ExitKind::Continue => self.loops.last().map(|context| context.header),
            _ => None,
        };
        self.register_control_exit_with_target(source, exit, target);
    }

    /// 登记带固定目标的控制退出。
    fn register_control_exit_with_target(
        &mut self,
        source: BlockId,
        exit: ExitKind,
        target: Option<BlockId>,
    ) {
        if let Some(context) = self.try_contexts.last_mut() {
            let control = ControlExit {
                source,
                kind: exit,
                target,
            };
            if !context.control_exits.contains(&control) {
                context.control_exits.push(control);
            }
        }
    }

    /// 把控制退出转发给外层片段；没有外层片段时保留原有直达边。
    fn forward_control_exits(&mut self, exits: &[ControlExit]) {
        if self.try_contexts.is_empty() {
            return;
        }
        for exit in exits {
            self.disconnect_control_exit(exit.source, exit.kind);
            self.register_control_exit_with_target(exit.source, exit.kind, exit.target);
        }
    }

    /// 从清理完成块恢复一个已登记的非正常控制流目标。
    fn connect_control_exit(
        &mut self,
        from: Option<BlockId>,
        exit: ExitKind,
        target: Option<BlockId>,
    ) {
        if let Some(target) = target {
            self.connect_blocks(from, target, control_edge(exit));
            return;
        }
        match exit {
            ExitKind::Return => {
                if let Some(target) = self.functions.last().map(|context| context.exit) {
                    self.connect_blocks(from, target, ControlFlowEdgeKind::Return);
                }
            }
            ExitKind::Break => {
                if let Some(target) = self.loops.last().map(|context| context.exit) {
                    self.connect_blocks(from, target, ControlFlowEdgeKind::Break);
                }
            }
            ExitKind::Continue => {
                if let Some(target) = self.loops.last().map(|context| context.header) {
                    self.connect_blocks(from, target, ControlFlowEdgeKind::Continue);
                }
            }
            _ => {}
        }
    }

    /// 规范化普通和反引号名称，格式与类型层环境一致。
    fn name_key(&self, name: Name) -> String {
        if name.backticked {
            format!("backtick:{}", name.unquoted_text(self.source))
        } else {
            format!("ascii:{}", name.text(self.source))
        }
    }

    /// 建立一个值记录并绑定到当前作用域。
    #[allow(clippy::too_many_arguments)]
    fn create_value(
        &mut self,
        name: Option<String>,
        span: SourceSpan,
        ty: Option<Type>,
        storage: StorageClass,
        parameter: bool,
        constant: bool,
        temporary: bool,
    ) -> Option<ValueId> {
        let scope = self.current_scope()?;
        let order = self.next_order.entry(scope).or_insert(0);
        let declaration_order = *order;
        *order = order.saturating_add(1);
        let id = ValueId::new(self.next_value);
        self.next_value = self.next_value.saturating_add(1);
        let info = ValueInfo {
            id,
            name: name.clone(),
            scope,
            span,
            ty,
            storage,
            declaration_order,
            parameter,
            constant,
            temporary,
            escapes: BTreeSet::new(),
        };
        self.result.values.insert(id, info);
        if let Some(scope_info) = self.result.scopes.get_mut(&scope) {
            scope_info.values.push(id);
        }
        if let Some(name) = name {
            if let Some(bindings) = self.bindings.last_mut() {
                bindings.insert(name, id);
            }
        }
        Some(id)
    }

    /// 为声明/函数名建立值，并返回当前可见绑定。
    fn declare_binding(
        &mut self,
        name: Name,
        span: SourceSpan,
        ty: Option<Type>,
        storage: StorageClass,
        parameter: bool,
        constant: bool,
    ) -> Option<ValueId> {
        let key = self.name_key(name);
        self.create_value(Some(key), span, ty, storage, parameter, constant, false)
    }

    /// 查找从内层到外层可见的值。
    fn resolve(&self, key: &str) -> Option<(ValueId, usize)> {
        for (depth, bindings) in self.bindings.iter().enumerate().rev() {
            if let Some(id) = bindings.get(key) {
                return Some((*id, depth));
            }
        }
        None
    }

    /// 返回一个绑定当前指向的对象；没有独立对象时返回绑定自身。
    fn referents_of(&self, id: ValueId) -> Vec<ValueId> {
        let targets = self
            .result
            .strong_edges
            .iter()
            .filter(|edge| edge.from == id && edge.reason == OwnershipEdgeReason::Alias)
            .map(|edge| edge.to)
            .collect::<Vec<_>>();
        if targets.is_empty() {
            vec![id]
        } else {
            targets
        }
    }

    /// 在重赋值前移除绑定到旧对象的根引用边。
    fn clear_binding_target(&mut self, id: ValueId) {
        self.result
            .strong_edges
            .retain(|edge| !(edge.from == id && edge.reason == OwnershipEdgeReason::Alias));
    }

    /// 从选择器/成员左值中找到根绑定当前指向的对象。
    fn mutation_owner(&self, expression: &Expression) -> Option<ValueId> {
        let name = match expression {
            Expression::Name(name) => Some(*name),
            Expression::Group { expression, .. }
            | Expression::Member {
                object: expression, ..
            }
            | Expression::Selector {
                source: expression, ..
            } => return self.mutation_owner(expression),
            _ => None,
        }?;
        let binding = self.resolve(&self.name_key(name))?.0;
        self.referents_of(binding).into_iter().next()
    }

    /// 返回值所属作用域是否是另一个作用域的后代。
    fn is_descendant(&self, child: ScopeId, ancestor: ScopeId) -> bool {
        let mut current = Some(child);
        while let Some(id) = current {
            if id == ancestor {
                return true;
            }
            current = self.result.scopes.get(&id).and_then(|scope| scope.parent);
        }
        false
    }

    /// 标记值逃逸；动态值和未知类型一律提升到强拥有堆。
    fn mark_escape(&mut self, id: ValueId, reason: EscapeReason) {
        if let Some(value) = self.result.values.get_mut(&id) {
            value.escapes.insert(reason);
            // 标量/栈值跨作用域被读取时仍按值传递，不应因为保守的
            // 逃逸边界被伪装成堆句柄。只有已经需要堆管理的值才需
            // 在逃逸后提升为强拥有类别。
            if value.storage.is_heap() && value.storage != StorageClass::HeapWeak {
                value.storage = StorageClass::HeapStrong;
            }
        }
    }

    /// 标记一个值及其强拥有闭包在指定退出边上转移。
    fn mark_escape_tree(&mut self, id: ValueId, reason: EscapeReason, exit: ExitKind) {
        let mut pending = vec![id];
        let mut visited = BTreeSet::new();
        while let Some(value) = pending.pop() {
            if !visited.insert(value) {
                continue;
            }
            self.mark_escape(value, reason);
            self.mark_transfer(exit, value);
            pending.extend(
                self.result
                    .strong_edges
                    .iter()
                    .filter(|edge| edge.from == value)
                    .map(|edge| edge.to),
            );
        }
    }

    /// 在从当前作用域向根展开的路径上登记值转移。
    fn mark_transfer(&mut self, exit: ExitKind, id: ValueId) {
        let Some(value_scope) = self.result.values.get(&id).map(|value| value.scope) else {
            self.result
                .diagnostics
                .push(diagnostics::unknown_id(None, id));
            return;
        };
        for scope in self.scopes.iter().rev().copied() {
            self.transfers.entry((scope, exit)).or_default().insert(id);
            if scope == value_scope {
                break;
            }
        }
    }

    /// 记录动态生命周期检查及其结构化警告。
    fn record_dynamic(&mut self, span: SourceSpan, value: Option<ValueId>, reason: EscapeReason) {
        if !self
            .result
            .dynamic_checks
            .iter()
            .any(|check| check.span == span && check.value == value && check.reason == reason)
        {
            self.result
                .dynamic_checks
                .push(crate::model::DynamicLifetimeCheck {
                    span,
                    value,
                    reason,
                });
            self.result
                .diagnostics
                .push(diagnostics::dynamic_check(span, value, reason));
            self.record_block_exit(ExitKind::DynamicCheckFailure);
            self.connect_error_block(span, ExitKind::DynamicCheckFailure);
        }
        if let Some(id) = value {
            self.mark_escape(id, reason);
        }
    }

    /// 安全加入所有权边；无效边被记录为诊断而不会 panic。
    fn add_edge(
        &mut self,
        from: ValueId,
        to: ValueId,
        kind: OwnershipKind,
        reason: OwnershipEdgeReason,
        span: Option<SourceSpan>,
    ) {
        // 普通名称给自身赋值只是保留现有绑定，不是对象环；容器或表对象
        // 直接持有自身则必须保留为强自环，交给图验证器报告。
        if from == to && kind == OwnershipKind::Strong && reason == OwnershipEdgeReason::Alias {
            return;
        }
        if !self.result.values.contains_key(&from) {
            self.result
                .diagnostics
                .push(diagnostics::unknown_id(span, from));
            return;
        }
        if !self.result.values.contains_key(&to) {
            self.result
                .diagnostics
                .push(diagnostics::unknown_id(span, to));
            return;
        }
        let edge = OwnershipEdge::new(from, to, kind, reason, span);
        let edges = if kind == OwnershipKind::Strong {
            &mut self.result.strong_edges
        } else {
            &mut self.result.weak_edges
        };
        if !edges.contains(&edge) {
            edges.push(edge);
        }
    }

    /// 将表达式事实挂接到一个拥有者值。
    fn attach_facts(
        &mut self,
        owner: Option<ValueId>,
        facts: &ExpressionFacts,
        span: SourceSpan,
        reason: OwnershipEdgeReason,
    ) {
        if facts.construct {
            self.record_block_exit(ExitKind::ConstructFailure);
            self.connect_error_block(span, ExitKind::ConstructFailure);
        }
        let Some(owner) = owner else {
            if facts.dynamic {
                self.record_dynamic(span, None, EscapeReason::DynamicValue);
            }
            return;
        };
        if facts.dynamic {
            self.record_dynamic(span, Some(owner), EscapeReason::DynamicValue);
        }
        for referent in &facts.referents {
            if *referent == owner && reason == OwnershipEdgeReason::Alias {
                continue;
            }
            let needs_edge = self
                .result
                .values
                .get(referent)
                .is_some_and(|value| value.storage.is_heap())
                || self
                    .result
                    .values
                    .get(&owner)
                    .is_some_and(|value| value.storage.is_heap());
            if needs_edge {
                self.add_edge(owner, *referent, OwnershipKind::Strong, reason, Some(span));
            }
        }
        for source in &facts.references {
            if *source == owner {
                continue;
            }
            let Some(source_scope) = self.result.values.get(source).map(|value| value.scope) else {
                continue;
            };
            let Some(owner_scope) = self.result.values.get(&owner).map(|value| value.scope) else {
                continue;
            };
            if source_scope != owner_scope && self.is_descendant(source_scope, owner_scope) {
                self.mark_escape_tree(
                    *source,
                    EscapeReason::StoredInLongerLivedContainer,
                    ExitKind::Normal,
                );
            }
        }
    }

    /// 递归分析一段语句序列。
    fn analyze_statements(&mut self, statements: &[Statement]) -> FlowSummary {
        let mut summary = FlowSummary::reachable();
        for statement in statements {
            self.record_statement_block(statement.span());
            let flow = self.analyze_statement(statement);
            if summary.normal {
                summary.exits.extend(flow.exits.iter().copied());
                summary.normal = flow.normal;
            }
        }
        summary
    }

    /// 分派并分析一条语句。
    fn analyze_statement(&mut self, statement: &Statement) -> FlowSummary {
        match statement {
            Statement::Expression { expression, .. } => {
                let facts = self.analyze_expression(expression);
                self.attach_facts(None, &facts, expression.span(), OwnershipEdgeReason::Alias);
                let mut flow = FlowSummary::reachable();
                if facts.dynamic {
                    flow.exits.insert(ExitKind::DynamicCheckFailure);
                }
                if facts.construct {
                    flow.exits.insert(ExitKind::ConstructFailure);
                }
                flow
            }
            Statement::Assignment { target, value, .. } => {
                self.analyze_binding_assignment(*target, value, false, false)
            }
            Statement::ExtendedAssignment {
                target,
                value,
                span,
                ..
            } => {
                let target_facts = self.analyze_expression(target);
                let value_facts = self.analyze_expression(value);
                let (owner, reason) = match target {
                    Expression::Name(name) => (
                        self.resolve(&self.name_key(*name)).map(|item| item.0),
                        OwnershipEdgeReason::Alias,
                    ),
                    _ => (
                        self.mutation_owner(target),
                        OwnershipEdgeReason::ContainerElement,
                    ),
                };
                self.attach_facts(owner, &value_facts, *span, reason);
                let mut flow = FlowSummary::reachable();
                if target_facts.dynamic || value_facts.dynamic {
                    flow.exits.insert(ExitKind::DynamicCheckFailure);
                }
                flow
            }
            Statement::Declaration {
                target,
                declared_type,
                value,
                span,
                ..
            } => self.analyze_declaration(*target, declared_type, value.as_ref(), *span, false),
            Statement::ConstDeclaration {
                target,
                declared_type,
                value,
                span,
                ..
            } => {
                let ty = declared_type.map(Type::scalar);
                let resolved_type = ty.or_else(|| self.type_for_span(value.span()));
                let id = self.declare_binding(
                    *target,
                    *span,
                    resolved_type.clone(),
                    storage_for_type(resolved_type.as_ref()),
                    false,
                    true,
                );
                let facts = self.analyze_expression(value);
                self.attach_facts(id, &facts, value.span(), OwnershipEdgeReason::Alias);
                let mut flow = FlowSummary::reachable();
                if facts.dynamic {
                    flow.exits.insert(ExitKind::DynamicCheckFailure);
                }
                flow
            }
            Statement::Import { .. } => FlowSummary::reachable(),
            Statement::Table {
                name,
                kind,
                body,
                span,
                ..
            } => self.analyze_table(*name, *kind, body, *span),
            Statement::Function {
                name,
                parameters,
                body,
                span,
                ..
            } => self.analyze_function(*name, parameters, body, *span),
            Statement::If {
                condition,
                body,
                elif_branches,
                else_body,
                span,
                ..
            } => self.analyze_if(condition, body, elif_branches, else_body.as_deref(), *span),
            Statement::For {
                target,
                iterable,
                body,
                span,
                ..
            } => self.analyze_for(*target, iterable, body, *span),
            Statement::While {
                condition,
                body,
                span,
                ..
            } => self.analyze_while(condition, body, *span),
            Statement::Return { value, span, .. } => self.analyze_return(value.as_ref(), *span),
            Statement::Break { .. } => {
                if let Some(loop_context) = self.loops.last() {
                    let exit = loop_context.exit;
                    let _loop_scope = loop_context.scope;
                    self.connect_blocks(self.current_block, exit, ControlFlowEdgeKind::Break);
                }
                self.record_block_exit(ExitKind::Break);
                FlowSummary::terminal(ExitKind::Break)
            }
            Statement::Continue { .. } => {
                if let Some(loop_context) = self.loops.last() {
                    let header = loop_context.header;
                    let _loop_scope = loop_context.scope;
                    self.connect_blocks(self.current_block, header, ControlFlowEdgeKind::Continue);
                }
                self.record_block_exit(ExitKind::Continue);
                FlowSummary::terminal(ExitKind::Continue)
            }
            Statement::Raise { value, span, .. } => self.analyze_raise(value, *span),
            Statement::Try {
                body,
                catches,
                finally_body,
                span,
                ..
            } => self.analyze_try(body, catches, finally_body.as_deref(), *span),
        }
    }

    /// 分析主动 `raise`，将可恢复错误接入统一错误退出边。
    fn analyze_raise(&mut self, value: &Expression, span: SourceSpan) -> FlowSummary {
        let facts = self.analyze_expression(value);
        self.attach_facts(None, &facts, span, OwnershipEdgeReason::Alias);
        self.record_block_exit(ExitKind::Raise);
        self.connect_error_block(span, ExitKind::Raise);
        FlowSummary::terminal(ExitKind::Raise)
    }

    /// 分析 `try`/`catch`/`finally`，为每个子块建立独立作用域和错误边。
    fn analyze_try(
        &mut self,
        body: &[Statement],
        catches: &[CatchClause],
        finally_body: Option<&[Statement]>,
        span: SourceSpan,
    ) -> FlowSummary {
        let Some(parent_scope) = self.current_scope() else {
            return FlowSummary::reachable();
        };
        let parent_block = self.current_block;
        let join = self.new_block(parent_scope, Some(span));
        let try_scope = self.push_scope(ScopeKind::Try, span, Some(parent_scope));
        let try_block = self.new_block(try_scope, Some(span));
        self.connect_blocks(parent_block, try_block, ControlFlowEdgeKind::Next);
        self.current_block = Some(try_block);
        self.try_contexts.push(TryContext::default());
        let try_flow = self.analyze_statements(body);
        let try_errors = self
            .try_contexts
            .pop()
            .map(|context| (context.error_blocks, context.control_exits))
            .unwrap_or_default();
        let (try_errors, try_control_sources) = try_errors;
        let try_normal_sources = if try_flow.normal {
            self.current_block.into_iter().collect::<Vec<_>>()
        } else {
            Vec::new()
        };
        self.pop_scope();

        let mut summary = FlowSummary {
            normal: false,
            exits: BTreeSet::new(),
        };
        for exit in try_flow.exits.iter().copied() {
            if catches.is_empty()
                || !matches!(
                    exit,
                    ExitKind::Error
                        | ExitKind::Raise
                        | ExitKind::ConstructFailure
                        | ExitKind::DynamicCheckFailure
                        | ExitKind::Fatal
                        | ExitKind::UnmatchedError
                )
            {
                summary.exits.insert(exit);
            }
        }

        let mut normal_sources = try_normal_sources;
        let mut control_sources = try_control_sources;
        let mut propagated_sources = Vec::new();
        let mut catch_error_sources = Vec::new();
        if catches.is_empty() {
            propagated_sources.extend(try_errors.iter().copied());
        }
        for catch in catches {
            let catch_scope = self.push_scope(ScopeKind::Catch, catch.span, Some(parent_scope));
            let catch_block = self.new_block(catch_scope, Some(catch.span));
            self.connect_exit_sources(&try_errors, catch_block, ControlFlowEdgeKind::Error);
            self.current_block = Some(catch_block);
            let _ = self.declare_binding(
                catch.binding,
                catch.binding.span,
                Some(Type::Dynamic),
                StorageClass::HeapStrong,
                false,
                false,
            );
            self.try_contexts.push(TryContext::default());
            let catch_flow = self.analyze_statements(&catch.body);
            let catch_errors = self
                .try_contexts
                .pop()
                .map(|context| (context.error_blocks, context.control_exits))
                .unwrap_or_default();
            let (catch_errors, catch_controls) = catch_errors;
            let catch_normal = if catch_flow.normal {
                self.current_block.into_iter().collect::<Vec<_>>()
            } else {
                Vec::new()
            };
            if catch_flow.normal {
                self.record_block_exit(ExitKind::Catch);
                normal_sources.extend(catch_normal);
            }
            control_sources.extend(catch_controls);
            catch_error_sources.extend(catch_errors);
            summary.exits.extend(
                catch_flow
                    .exits
                    .iter()
                    .copied()
                    .filter(|exit| *exit != ExitKind::Catch),
            );
            self.pop_scope();
        }

        let unmatched_block = if catches.is_empty() {
            None
        } else {
            let block = self.new_block(parent_scope, Some(span));
            if let Some(info) = self.result.control_flow.blocks.get_mut(&block) {
                info.exits.insert(ExitKind::UnmatchedError);
            }
            self.connect_exit_sources(&try_errors, block, ControlFlowEdgeKind::Error);
            summary.exits.insert(ExitKind::Catch);
            summary.exits.insert(ExitKind::UnmatchedError);
            Some(block)
        };

        propagated_sources.extend(unmatched_block);
        propagated_sources.extend(catch_error_sources.iter().copied());
        propagated_sources.sort_unstable();
        propagated_sources.dedup();

        let final_errors;
        let mut final_normal = true;

        if let Some(finally_body) = finally_body {
            let finally_scope = self.push_scope(ScopeKind::Finally, span, Some(parent_scope));
            let finally_block = self.new_block(finally_scope, Some(span));
            self.connect_exit_sources(&normal_sources, finally_block, ControlFlowEdgeKind::Next);
            self.connect_exit_sources(
                &propagated_sources,
                finally_block,
                ControlFlowEdgeKind::Error,
            );
            for control in &control_sources {
                self.disconnect_control_exit(control.source, control.kind);
                self.connect_blocks(
                    Some(control.source),
                    finally_block,
                    control_edge(control.kind),
                );
            }
            self.current_block = Some(finally_block);
            self.try_contexts.push(TryContext::default());
            let finally_flow = self.analyze_statements(finally_body);
            final_errors = self
                .try_contexts
                .pop()
                .map(|context| (context.error_blocks, context.control_exits))
                .unwrap_or_default();
            let (final_errors, final_controls) = final_errors;
            summary.exits.extend(finally_flow.exits.iter().copied());
            final_normal = finally_flow.normal;
            if finally_flow.normal {
                if !normal_sources.is_empty() {
                    self.connect_blocks(self.current_block, join, ControlFlowEdgeKind::Next);
                }
                for control in &control_sources {
                    self.connect_control_exit(self.current_block, control.kind, control.target);
                }
                if !propagated_sources.is_empty() {
                    let propagate = self.new_block(parent_scope, Some(span));
                    if let Some(info) = self.result.control_flow.blocks.get_mut(&propagate) {
                        info.exits.insert(ExitKind::UnmatchedError);
                    }
                    self.connect_blocks(self.current_block, propagate, ControlFlowEdgeKind::Error);
                    self.register_error_block(propagate);
                }
                // 先把已经经过本层 finally 的控制转移交给外层；没有
                // 外层 try 时，保留它们到函数/循环目标的直达边。
                let completed_controls = control_sources
                    .iter()
                    .map(|control| ControlExit {
                        source: finally_block,
                        kind: control.kind,
                        target: control.target,
                    })
                    .collect::<Vec<_>>();
                self.forward_control_exits(&completed_controls);
                self.forward_control_exits(&final_controls);
            } else {
                // `finally` 自身的控制转移覆盖之前的路径；只有它们继续
                // 向外传播，内层 try 的旧出口不能复活。
                self.forward_control_exits(&final_controls);
            }
            for block in final_errors.iter().copied() {
                self.register_error_block(block);
            }
            self.pop_scope();
        } else {
            self.connect_exit_sources(&normal_sources, join, ControlFlowEdgeKind::Next);
            for control in &control_sources {
                if self.try_contexts.is_empty() {
                    self.connect_control_exit(Some(control.source), control.kind, control.target);
                }
            }
            self.forward_control_exits(&control_sources);
            for block in propagated_sources.iter().copied() {
                self.register_error_block(block);
            }
        }

        summary.normal = !normal_sources.is_empty() && final_normal;
        self.current_block = summary.normal.then_some(join).or(parent_block);
        summary
    }

    /// 分析普通声明。
    fn analyze_declaration(
        &mut self,
        target: Name,
        declared_type: &DeclaredType,
        value: Option<&Expression>,
        span: SourceSpan,
        parameter: bool,
    ) -> FlowSummary {
        let declared = self.declared_type(declared_type);
        let inferred = value.and_then(|expression| self.type_for_span(expression.span()));
        let ty = declared.or(inferred);
        let storage = storage_for_type(ty.as_ref());
        let id = self.declare_binding(target, span, ty, storage, parameter, false);
        let Some(value) = value else {
            return FlowSummary::reachable();
        };
        let facts = self.analyze_expression(value);
        self.attach_facts(id, &facts, value.span(), OwnershipEdgeReason::Alias);
        let mut flow = FlowSummary::reachable();
        if facts.dynamic {
            flow.exits.insert(ExitKind::DynamicCheckFailure);
        }
        if facts.construct {
            flow.exits.insert(ExitKind::ConstructFailure);
        }
        flow
    }

    /// 分析无显式类型的赋值；已有绑定会保留同一值身份。
    fn analyze_binding_assignment(
        &mut self,
        target: Name,
        expression: &Expression,
        _parameter: bool,
        constant: bool,
    ) -> FlowSummary {
        let key = self.name_key(target);
        let existing = self.resolve(&key).map(|item| item.0);
        let facts = self.analyze_expression(expression);
        let owner = existing.or_else(|| {
            self.declare_binding(
                target,
                target.span,
                self.type_for_span(expression.span()),
                storage_for_type(self.type_for_span(expression.span()).as_ref()),
                false,
                constant,
            )
        });
        if let Some(existing) = existing {
            self.clear_binding_target(existing);
        }
        self.attach_facts(owner, &facts, expression.span(), OwnershipEdgeReason::Alias);
        let mut flow = FlowSummary::reachable();
        if facts.dynamic {
            flow.exits.insert(ExitKind::DynamicCheckFailure);
        }
        if facts.construct {
            flow.exits.insert(ExitKind::ConstructFailure);
        }
        flow
    }

    /// 分析表值与表体字段拥有关系。
    fn analyze_table(
        &mut self,
        name: Name,
        kind: TableKind,
        body: &[Statement],
        span: SourceSpan,
    ) -> FlowSummary {
        let table_key = self.name_key(name);
        let table_name = name.unquoted_text(self.source).to_owned();
        let fallback_type = match kind {
            TableKind::Singleton => Type::Table(TableType::singleton(table_name.clone())),
            TableKind::Instance => Type::Table(TableType::constructor(table_name)),
        };
        let table_value = self.declare_binding(
            name,
            span,
            Some(self.binding_type(&table_key).unwrap_or(fallback_type)),
            StorageClass::HeapStrong,
            false,
            false,
        );
        let Some(parent) = self.current_scope() else {
            return FlowSummary::reachable();
        };
        let table_scope = self.push_scope(ScopeKind::Table, span, Some(parent));
        let previous_block = self.current_block;
        self.current_block = Some(self.new_block(table_scope, Some(span)));
        let flow = self.analyze_statements(body);
        if let Some(owner) = table_value {
            let fields = self
                .result
                .scopes
                .get(&table_scope)
                .map(|info| info.values.clone())
                .unwrap_or_default();
            for field in &fields {
                if self
                    .result
                    .values
                    .get(field)
                    .is_some_and(|value| value.storage.is_heap())
                {
                    self.mark_escape(*field, EscapeReason::StoredInLongerLivedContainer);
                    self.mark_transfer(ExitKind::Normal, *field);
                    self.add_edge(
                        owner,
                        *field,
                        OwnershipKind::Strong,
                        OwnershipEdgeReason::TableMember,
                        Some(span),
                    );
                }
            }
        }
        self.current_block = previous_block;
        self.pop_scope();
        flow
    }

    /// 分析函数、参数、嵌套函数捕获和返回边。
    fn analyze_function(
        &mut self,
        name: Name,
        parameters: &[FunctionParameter],
        body: &[Statement],
        span: SourceSpan,
    ) -> FlowSummary {
        let function_key = self.name_key(name);
        let function_value = self.declare_binding(
            name,
            span,
            self.binding_type(&function_key)
                .or_else(|| self.function_type(&function_key)),
            StorageClass::HeapStrong,
            false,
            true,
        );
        let Some(parent) = self.current_scope() else {
            return FlowSummary::reachable();
        };
        let function_scope = self.push_scope(ScopeKind::Function, span, Some(parent));
        let previous_block = self.current_block;
        let previous_loops = std::mem::take(&mut self.loops);
        let previous_try_contexts = std::mem::take(&mut self.try_contexts);
        let entry = self.new_block(function_scope, Some(span));
        let exit = self.new_block(function_scope, None);
        self.current_block = Some(entry);
        let signature_parameters = self
            .types
            .function_signatures()
            .get(&function_key)
            .filter(|signature| signature.span == span)
            .or_else(|| {
                self.types
                    .table_signatures
                    .values()
                    .flat_map(|table| table.members.values())
                    .filter_map(|member| member.function.as_ref())
                    .find(|signature| signature.span == span)
            })
            .map(|signature| signature.parameters.clone());
        for (index, parameter) in parameters.iter().enumerate() {
            let ty = signature_parameters
                .as_ref()
                .and_then(|signatures| signatures.get(index).map(|parameter| parameter.ty.clone()))
                .or_else(|| {
                    parameter.annotation.map(|annotation| match annotation {
                        xiao_syntax::FunctionTypeAnnotation::Scalar(scalar) => Type::scalar(scalar),
                        xiao_syntax::FunctionTypeAnnotation::None => Type::None,
                    })
                });
            let _ = self.declare_binding(
                parameter.name,
                parameter.span,
                ty.clone(),
                storage_for_type(ty.as_ref()),
                true,
                false,
            );
            if let Some(default) = &parameter.default {
                let facts = self.analyze_expression(default);
                if facts.dynamic {
                    self.record_dynamic(default.span(), None, EscapeReason::DynamicValue);
                }
            }
        }
        let function_context = function_value.map(|value| FunctionContext {
            scope: function_scope,
            value,
            exit,
        });
        if let Some(context) = function_context {
            self.functions.push(context);
        }
        let body_flow = self.analyze_statements(body);
        if body_flow.normal {
            self.connect_blocks(self.current_block, exit, ControlFlowEdgeKind::Next);
        }
        let _ = self.functions.pop();
        self.current_block = previous_block;
        self.loops = previous_loops;
        self.try_contexts = previous_try_contexts;
        self.pop_scope();
        // 函数体的 return/break/continue 只属于函数内部；声明函数本身不会
        // 终止外围语句序列。
        FlowSummary::reachable()
    }

    /// 分析条件链并为每个分支建立独立作用域。
    fn analyze_if(
        &mut self,
        condition: &Expression,
        body: &[Statement],
        elif_branches: &[ElifBranch],
        else_body: Option<&[Statement]>,
        span: SourceSpan,
    ) -> FlowSummary {
        let condition_facts = self.analyze_expression(condition);
        let mut condition_dynamic = condition_facts.dynamic;
        for branch in elif_branches {
            let branch_facts = self.analyze_expression(&branch.condition);
            condition_dynamic |= branch_facts.dynamic;
        }
        let parent_block = self.current_block;
        let join = self
            .current_scope()
            .map(|scope| self.new_block(scope, Some(span)));
        let mut summary = FlowSummary {
            normal: false,
            exits: BTreeSet::new(),
        };
        let mut branches: Vec<(&[Statement], SourceSpan)> = vec![(body, body_span(body, span))];
        for branch in elif_branches {
            branches.push((&branch.body, branch.span));
        }
        if let Some(body) = else_body {
            branches.push((body, body_span(body, span)));
        } else {
            // 没有 else 时条件为假可直接合流。
            summary.normal = true;
            if let Some(join) = join {
                self.connect_blocks(parent_block, join, ControlFlowEdgeKind::BranchFalse);
            }
        }
        for (index, (branch_body, branch_span)) in branches.into_iter().enumerate() {
            let Some(parent) = self.current_scope() else {
                continue;
            };
            let branch_scope = self.push_scope(ScopeKind::Branch, branch_span, Some(parent));
            let branch_block = self.new_block(branch_scope, Some(branch_span));
            self.connect_blocks(
                parent_block,
                branch_block,
                if index == 0 {
                    ControlFlowEdgeKind::BranchTrue
                } else {
                    ControlFlowEdgeKind::BranchFalse
                },
            );
            self.current_block = Some(branch_block);
            let flow = self.analyze_statements(branch_body);
            summary.exits.extend(flow.exits.iter().copied());
            if flow.normal {
                summary.normal = true;
                if let Some(join) = join {
                    self.connect_blocks(self.current_block, join, ControlFlowEdgeKind::Next);
                }
            }
            self.pop_scope();
            self.current_block = parent_block;
        }
        if let Some(join) = join {
            self.current_block = Some(join);
        }
        if condition_dynamic {
            summary.exits.insert(ExitKind::DynamicCheckFailure);
        }
        summary
    }

    /// 分析 `for` 循环；break/continue 在循环边界被消费。
    fn analyze_for(
        &mut self,
        target: Name,
        iterable: &Expression,
        body: &[Statement],
        span: SourceSpan,
    ) -> FlowSummary {
        let iterable_facts = self.analyze_expression(iterable);
        let Some(parent) = self.current_scope() else {
            return FlowSummary::reachable();
        };
        let header = self.new_block(parent, Some(span));
        let exit = self.new_block(parent, None);
        self.connect_blocks(self.current_block, header, ControlFlowEdgeKind::Next);
        let loop_scope = self.push_scope(ScopeKind::Loop, span, Some(parent));
        let body_block = self.new_block(loop_scope, Some(span));
        self.connect_blocks(Some(header), body_block, ControlFlowEdgeKind::BranchTrue);
        self.connect_blocks(Some(header), exit, ControlFlowEdgeKind::BranchFalse);
        self.current_block = Some(body_block);
        self.loops.push(LoopContext {
            scope: loop_scope,
            header,
            exit,
        });
        let element_type = iterable_element_type(iterable_facts.ty.as_ref());
        let element_storage = storage_for_type(element_type.as_ref());
        let _ = self.declare_binding(
            target,
            target.span,
            element_type,
            element_storage,
            false,
            false,
        );
        let body_flow = self.analyze_statements(body);
        if body_flow.normal {
            self.connect_blocks(self.current_block, header, ControlFlowEdgeKind::LoopBack);
        }
        self.loops.pop();
        self.pop_scope();
        self.current_block = Some(exit);
        let mut summary = FlowSummary::reachable();
        summary.exits.extend(
            body_flow
                .exits
                .into_iter()
                .filter(|exit| !matches!(exit, ExitKind::Break | ExitKind::Continue)),
        );
        if iterable_facts.dynamic {
            summary.exits.insert(ExitKind::DynamicCheckFailure);
        }
        summary
    }

    /// 分析 `while` 循环。
    fn analyze_while(
        &mut self,
        condition: &Expression,
        body: &[Statement],
        span: SourceSpan,
    ) -> FlowSummary {
        let condition_facts = self.analyze_expression(condition);
        let Some(parent) = self.current_scope() else {
            return FlowSummary::reachable();
        };
        let header = self.new_block(parent, Some(span));
        let exit = self.new_block(parent, None);
        self.connect_blocks(self.current_block, header, ControlFlowEdgeKind::Next);
        let loop_scope = self.push_scope(ScopeKind::Loop, span, Some(parent));
        let body_block = self.new_block(loop_scope, Some(span));
        self.connect_blocks(Some(header), body_block, ControlFlowEdgeKind::BranchTrue);
        self.connect_blocks(Some(header), exit, ControlFlowEdgeKind::BranchFalse);
        self.current_block = Some(body_block);
        self.loops.push(LoopContext {
            scope: loop_scope,
            header,
            exit,
        });
        let body_flow = self.analyze_statements(body);
        if body_flow.normal {
            self.connect_blocks(self.current_block, header, ControlFlowEdgeKind::LoopBack);
        }
        self.loops.pop();
        self.pop_scope();
        self.current_block = Some(exit);
        let mut summary = FlowSummary::reachable();
        summary.exits.extend(
            body_flow
                .exits
                .into_iter()
                .filter(|exit| !matches!(exit, ExitKind::Break | ExitKind::Continue)),
        );
        if condition_facts.dynamic {
            summary.exits.insert(ExitKind::DynamicCheckFailure);
        }
        summary
    }

    /// 分析返回值并把值转移到函数调用方。
    fn analyze_return(&mut self, value: Option<&Expression>, _span: SourceSpan) -> FlowSummary {
        if let Some(value) = value {
            let facts = self.analyze_expression(value);
            let roots = if is_direct_name(value) {
                // 直接返回名称时，绑定本身就是调用方接管的句柄；其 Alias
                // 边会继续把底层对象和强拥有子对象一并转移。
                facts.references.to_vec()
            } else {
                // 构造、运算、成员访问等表达式的 references 只是求值输入，
                // 不能因为结果返回就把这些局部输入误标记为已转移。
                facts.produced.into_iter().collect::<Vec<_>>()
            };
            for id in roots {
                self.mark_escape_tree(id, EscapeReason::Returned, ExitKind::Return);
            }
            if facts.dynamic {
                self.record_dynamic(value.span(), facts.produced, EscapeReason::Returned);
            }
        }
        self.record_block_exit(ExitKind::Return);
        if let Some(exit) = self.functions.last().map(|context| context.exit) {
            self.connect_blocks(self.current_block, exit, ControlFlowEdgeKind::Return);
        }
        FlowSummary::terminal(ExitKind::Return)
    }

    /// 递归分析表达式，并在必要时创建匿名临时值。
    fn analyze_expression(&mut self, expression: &Expression) -> ExpressionFacts {
        let ty = self.type_for_span(expression.span());
        let mut facts = match expression {
            Expression::Literal { kind, span } => {
                let inferred = ty.or_else(|| literal_type(*kind));
                let dynamic = inferred
                    .as_ref()
                    .is_none_or(|value| matches!(value, Type::Dynamic | Type::Variable(_)));
                let produced = self.make_temporary(*span, inferred.clone());
                if dynamic && !matches!(kind, LiteralKind::None) {
                    self.record_dynamic(*span, produced, EscapeReason::DynamicValue);
                }
                ExpressionFacts {
                    referents: produced.into_iter().collect(),
                    produced,
                    ty: inferred,
                    dynamic,
                    construct: false,
                    references: Vec::new(),
                }
            }
            Expression::Name(name) => {
                let key = self.name_key(*name);
                let mut facts = ExpressionFacts {
                    ty,
                    ..ExpressionFacts::default()
                };
                if let Some((id, binding_depth)) = self.resolve(&key) {
                    facts.references.push(id);
                    facts.referents = self.referents_of(id);
                    facts.dynamic = self
                        .result
                        .values
                        .get(&id)
                        .and_then(|value| value.ty.as_ref())
                        .is_none_or(|value| matches!(value, Type::Dynamic | Type::Variable(_)));
                    self.record_capture(id, binding_depth, name.span);
                    if facts.dynamic {
                        self.record_dynamic(name.span, Some(id), EscapeReason::DynamicValue);
                    }
                } else if let Some(known_type) = self.binding_type(&key) {
                    // 类型层可能已登记前向函数/表声明；没有本地值身份时
                    // 保留类型事实，但不虚构所有权边。
                    facts.ty = Some(known_type);
                    facts.dynamic = false;
                } else {
                    facts.dynamic = true;
                    self.record_dynamic(name.span, None, EscapeReason::Unknown);
                }
                facts
            }
            Expression::ArrayLiteral { elements, .. }
            | Expression::TupleLiteral { elements, .. }
            | Expression::SetLiteral { elements, .. } => {
                let mut facts = ExpressionFacts {
                    ty: ty.clone(),
                    ..ExpressionFacts::default()
                };
                let mut children = Vec::new();
                for element in elements {
                    let child = self.analyze_expression(element);
                    children.extend(child.referents.iter().copied());
                    facts.merge(child);
                }
                facts.produced = self.make_temporary(expression.span(), ty.clone());
                if let Some(container) = facts.produced {
                    for child in children {
                        if self
                            .result
                            .values
                            .get(&child)
                            .is_some_and(|value| value.storage.is_heap())
                        {
                            self.add_edge(
                                container,
                                child,
                                OwnershipKind::Strong,
                                OwnershipEdgeReason::ContainerElement,
                                Some(expression.span()),
                            );
                        }
                    }
                }
                facts.referents = facts.produced.into_iter().collect();
                facts.dynamic |= ty
                    .as_ref()
                    .is_none_or(|value| matches!(value, Type::Dynamic | Type::Variable(_)));
                facts
            }
            Expression::DictTableLiteral { entries, .. }
            | Expression::DictColumnLiteral { entries, .. } => {
                let mut facts = ExpressionFacts {
                    ty: ty.clone(),
                    ..ExpressionFacts::default()
                };
                let mut children = Vec::new();
                for entry in entries {
                    let child = self.analyze_expression(&entry.value);
                    children.extend(child.referents.iter().copied());
                    facts.merge(child);
                }
                facts.produced = self.make_temporary(expression.span(), ty.clone());
                if let Some(container) = facts.produced {
                    for child in children {
                        if self
                            .result
                            .values
                            .get(&child)
                            .is_some_and(|value| value.storage.is_heap())
                        {
                            self.add_edge(
                                container,
                                child,
                                OwnershipKind::Strong,
                                OwnershipEdgeReason::ContainerElement,
                                Some(expression.span()),
                            );
                        }
                    }
                }
                facts.referents = facts.produced.into_iter().collect();
                facts.dynamic |= ty
                    .as_ref()
                    .is_none_or(|value| matches!(value, Type::Dynamic | Type::Variable(_)));
                facts
            }
            Expression::Group {
                expression: inner, ..
            } => self.analyze_expression(inner),
            Expression::Cast {
                expression: inner, ..
            } => {
                let mut facts = self.analyze_expression(inner);
                facts.ty = ty.clone().or(facts.ty);
                facts.produced = self.make_temporary(expression.span(), ty.clone());
                facts.referents = facts.produced.into_iter().collect();
                facts.dynamic |= ty
                    .as_ref()
                    .is_none_or(|value| matches!(value, Type::Dynamic | Type::Variable(_)));
                facts
            }
            Expression::Unary { operand, .. } => {
                let mut facts = self.analyze_expression(operand);
                facts.ty = ty.clone().or(facts.ty);
                facts.produced = self.make_temporary(expression.span(), ty.clone());
                facts.referents = facts.produced.into_iter().collect();
                facts
            }
            Expression::Binary { left, right, .. } => {
                let mut facts = self.analyze_expression(left);
                facts.merge(self.analyze_expression(right));
                facts.ty = ty.clone().or(facts.ty);
                facts.produced = self.make_temporary(expression.span(), ty.clone());
                facts.referents = facts.produced.into_iter().collect();
                facts
            }
            Expression::Call {
                callee, arguments, ..
            }
            | Expression::NewCall {
                callee, arguments, ..
            } => {
                let mut facts = self.analyze_expression(callee);
                for argument in arguments {
                    facts.merge(self.analyze_expression(&argument.value));
                }
                facts.ty = ty.clone().or(facts.ty);
                facts.produced = self.make_temporary(expression.span(), ty.clone());
                facts.referents = facts.produced.into_iter().collect();
                if matches!(expression, Expression::NewCall { .. }) {
                    facts.construct = true;
                }
                facts.dynamic |= ty
                    .as_ref()
                    .is_none_or(|value| matches!(value, Type::Dynamic | Type::Variable(_)));
                facts
            }
            Expression::Member { object, .. } => {
                let mut facts = self.analyze_expression(object);
                facts.ty = ty.clone().or(facts.ty);
                facts.produced = self.make_temporary(expression.span(), ty.clone());
                facts.referents = facts.produced.into_iter().collect();
                facts
            }
            Expression::Selector { source, step, .. } => {
                let mut facts = self.analyze_expression(source);
                if let Some(step) = step {
                    facts.merge(self.analyze_expression(step));
                }
                facts.ty = ty.clone().or(facts.ty);
                facts.produced = self.make_temporary(expression.span(), ty.clone());
                facts.referents = facts.produced.into_iter().collect();
                facts
            }
        };
        if self
            .types
            .runtime_checks()
            .iter()
            .any(|check| check.span == expression.span())
        {
            facts.dynamic = true;
            let value = facts.produced.or_else(|| facts.references.first().copied());
            self.record_dynamic(expression.span(), value, EscapeReason::DynamicValue);
        }
        facts
    }

    /// 创建匿名表达式临时值；标量无需独立值。
    fn make_temporary(&mut self, span: SourceSpan, ty: Option<Type>) -> Option<ValueId> {
        if !storage_for_type(ty.as_ref()).is_heap() {
            return None;
        }
        self.create_value(None, span, ty, StorageClass::HeapStrong, false, false, true)
    }

    /// 处理名称读取引起的嵌套函数捕获。
    fn record_capture(&mut self, id: ValueId, binding_depth: usize, span: SourceSpan) {
        let Some((function_scope, function_value)) = self
            .functions
            .last()
            .map(|context| (context.scope, context.value))
        else {
            return;
        };
        let Some(current_depth) = self.scopes.len().checked_sub(1) else {
            return;
        };
        if binding_depth >= current_depth {
            return;
        }
        let Some(captured_scope) = self.result.values.get(&id).map(|value| value.scope) else {
            return;
        };
        // 分支和循环作用域仍属于当前函数。只有名称来自当前函数作用域
        // 之外、且该作用域确实包围当前函数时，才形成闭包捕获。
        if captured_scope == function_scope || !self.is_descendant(function_scope, captured_scope) {
            return;
        }
        if self
            .result
            .scopes
            .get(&captured_scope)
            .is_some_and(|scope| scope.kind == ScopeKind::Program)
        {
            return;
        }
        if id == function_value {
            return;
        }
        self.mark_escape(id, EscapeReason::CapturedByClosure);
        self.add_edge(
            function_value,
            id,
            OwnershipKind::Strong,
            OwnershipEdgeReason::ClosureCapture,
            Some(span),
        );
    }

    /// 从类型结果读取精确区间类型。
    fn type_for_span(&self, span: SourceSpan) -> Option<Type> {
        self.types.type_at(span).cloned()
    }

    /// 从类型环境读取一个已经登记的名称类型。
    fn binding_type(&self, key: &str) -> Option<Type> {
        self.types
            .binding(key)
            .map(|binding| binding.scheme.ty.clone())
    }

    /// 从函数签名表组装一个函数值类型。
    fn function_type(&self, key: &str) -> Option<Type> {
        let signature = self.types.function_signatures().get(key)?;
        Some(Type::Function {
            parameters: signature
                .parameters
                .iter()
                .map(|parameter| parameter.ty.clone())
                .collect(),
            return_type: Box::new(signature.return_type.clone()),
        })
    }

    /// 将显式声明转换为类型层表示。
    fn declared_type(&self, declared: &DeclaredType) -> Option<Type> {
        match declared {
            DeclaredType::Scalar(scalar) => Some(Type::scalar(*scalar)),
            DeclaredType::Set(annotation) => {
                let members = annotation
                    .members
                    .iter()
                    .map(|term| match term {
                        TypeTerm::Scalar(scalar) => Type::scalar(*scalar),
                        TypeTerm::None => Type::None,
                    })
                    .collect::<Vec<_>>();
                Some(Type::Set(SetType::heterogeneous(members)))
            }
        }
    }
}

/// 依据类型决定默认存储类别。
pub(crate) fn storage_for_type(ty: Option<&Type>) -> StorageClass {
    match ty {
        Some(Type::Scalar(
            ScalarType::Int
            | ScalarType::Sint
            | ScalarType::Lint
            | ScalarType::Float
            | ScalarType::Sfloat
            | ScalarType::Lfloat
            | ScalarType::Bool,
        ))
        | Some(Type::None) => StorageClass::Stack,
        Some(Type::Tuple(items)) if items.iter().all(is_stack_type) => StorageClass::Stack,
        Some(Type::Variable(_) | Type::Dynamic)
        | Some(Type::Scalar(ScalarType::Str))
        | Some(Type::Function { .. })
        | Some(Type::Array(_))
        | Some(Type::Tuple(_))
        | Some(Type::DictTable(_))
        | Some(Type::DictColumn(_))
        | Some(Type::Set(_))
        | Some(Type::Table(_))
        | None => StorageClass::HeapStrong,
    }
}

/// 判断一个递归类型是否可以直接放在栈/寄存器中。
fn is_stack_type(ty: &Type) -> bool {
    matches!(
        ty,
        Type::Scalar(
            ScalarType::Int
                | ScalarType::Sint
                | ScalarType::Lint
                | ScalarType::Float
                | ScalarType::Sfloat
                | ScalarType::Lfloat
                | ScalarType::Bool
        ) | Type::None
    ) || matches!(ty, Type::Tuple(items) if items.iter().all(is_stack_type))
}

/// 从可迭代类型提取循环绑定的元素类型。
fn iterable_element_type(ty: Option<&Type>) -> Option<Type> {
    match ty {
        Some(Type::Scalar(ScalarType::Str)) => Some(Type::scalar(ScalarType::Str)),
        Some(Type::Array(array)) => match array {
            xiao_types::ArrayType::Homogeneous { element, .. } => Some((**element).clone()),
            xiao_types::ArrayType::Heterogeneous { elements } => elements.first().cloned(),
            xiao_types::ArrayType::Unknown => Some(Type::Dynamic),
        },
        Some(Type::Tuple(items)) => items.first().cloned(),
        Some(Type::Set(set)) => set.member_types().next().cloned(),
        Some(Type::DictTable(dictionary)) | Some(Type::DictColumn(dictionary)) => dictionary
            .entries
            .first()
            .map(|entry| (*entry.value).clone()),
        Some(Type::Dynamic | Type::Variable(_)) | None => Some(Type::Dynamic),
        _ => Some(Type::Dynamic),
    }
}

/// 在类型层没有记录时，从字面量类别提供最小静态回退类型。
fn literal_type(kind: LiteralKind) -> Option<Type> {
    let scalar = match kind {
        LiteralKind::Integer => ScalarType::Int,
        LiteralKind::Float => ScalarType::Float,
        LiteralKind::String => ScalarType::Str,
        LiteralKind::Boolean => ScalarType::Bool,
        LiteralKind::None => return Some(Type::None),
    };
    Some(Type::scalar(scalar))
}

/// 使用第一个/最后一个语句区间近似一个缩进体的源码范围。
fn body_span(body: &[Statement], fallback: SourceSpan) -> SourceSpan {
    let Some(first) = body.first() else {
        return fallback;
    };
    let Some(last) = body.last() else {
        return fallback;
    };
    SourceSpan::new(first.span().start(), last.span().end()).unwrap_or(fallback)
}

/// 将结构化控制流退出类别映射为控制流图边类别。
fn control_edge(exit: ExitKind) -> ControlFlowEdgeKind {
    match exit {
        ExitKind::Return => ControlFlowEdgeKind::Return,
        ExitKind::Break => ControlFlowEdgeKind::Break,
        ExitKind::Continue => ControlFlowEdgeKind::Continue,
        _ => ControlFlowEdgeKind::Error,
    }
}

/// 判断表达式是否只是名称（允许任意层分组），用于区分句柄返回和构造结果返回。
fn is_direct_name(expression: &Expression) -> bool {
    match expression {
        Expression::Name(_) => true,
        Expression::Group { expression, .. } => is_direct_name(expression),
        _ => false,
    }
}
