//! 06-A 生命周期分析使用的稳定数据模型。
//!
//! 本模块只保存值对象和只读结果，不执行任何释放动作。字段保持公开，方便
//! 08 阶段把模型复制到统一 IR；算法实现放在 `graph`、`escape` 和 `release`。

use std::collections::{BTreeMap, BTreeSet};

use xiao_diagnostics::Diagnostic;
use xiao_source::SourceSpan;
use xiao_types::Type;

/// 一个静态作用域的稳定身份。
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ScopeId(u32);

impl ScopeId {
    /// 从原始编号创建作用域身份。
    #[must_use]
    pub const fn new(raw: u32) -> Self {
        Self(raw)
    }

    /// 返回作用域编号。
    #[must_use]
    pub const fn get(self) -> u32 {
        self.0
    }
}

/// 一个静态值槽的稳定身份。
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ValueId(u32);

impl ValueId {
    /// 从原始编号创建值身份。
    #[must_use]
    pub const fn new(raw: u32) -> Self {
        Self(raw)
    }

    /// 返回值编号。
    #[must_use]
    pub const fn get(self) -> u32 {
        self.0
    }
}

/// 一个控制流基本块的稳定身份。
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct BlockId(u32);

impl BlockId {
    /// 从原始编号创建基本块身份。
    #[must_use]
    pub const fn new(raw: u32) -> Self {
        Self(raw)
    }

    /// 返回基本块编号。
    #[must_use]
    pub const fn get(self) -> u32 {
        self.0
    }
}

/// 作用域的来源类别。
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ScopeKind {
    /// 程序顶层作用域。
    Program,
    /// 函数或嵌套函数作用域。
    Function,
    /// `if`、`elif` 或 `else` 分支作用域。
    Branch,
    /// `for` 或 `while` 循环体作用域。
    Loop,
    /// 表体作用域。
    Table,
    /// 其他由后端建立的普通块作用域。
    Block,
}

/// 值的静态所有权边类型。
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum OwnershipKind {
    /// 拥有目标并延长目标生命周期的强边。
    Strong,
    /// 不延长目标生命周期的弱边。
    Weak,
}

impl OwnershipKind {
    /// 判断边是否会延长目标生命周期。
    #[must_use]
    pub const fn is_strong(self) -> bool {
        matches!(self, Self::Strong)
    }
}

/// 编译期推导的值存储类别。
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum StorageClass {
    /// 不需要独立堆对象的值，优先放在栈或寄存器中。
    Stack,
    /// 带强引用计数的堆句柄或堆对象。
    HeapStrong,
    /// 弱引用句柄；它不拥有目标对象。
    HeapWeak,
}

impl StorageClass {
    /// 判断该类别是否需要堆上的生命周期管理。
    #[must_use]
    pub const fn is_heap(self) -> bool {
        !matches!(self, Self::Stack)
    }

    /// 返回该类别对应的默认所有权边。
    #[must_use]
    pub const fn ownership(self) -> OwnershipKind {
        match self {
            Self::HeapWeak => OwnershipKind::Weak,
            Self::Stack | Self::HeapStrong => OwnershipKind::Strong,
        }
    }
}

/// 值被提升或延长生命周期的静态原因。
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum EscapeReason {
    /// 值从当前函数返回。
    Returned,
    /// 值被嵌套函数闭包捕获。
    CapturedByClosure,
    /// 值被存入寿命更长的容器或绑定。
    StoredInLongerLivedContainer,
    /// 值可能跨线程或任务边界传递。
    CrossThread,
    /// 值的类型只能在运行时确定。
    DynamicValue,
    /// 分析输入不完整，只能采用保守策略。
    Unknown,
}

/// 控制流离开作用域的类别。
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ExitKind {
    /// 正常到达块尾或条件合流点。
    Normal,
    /// `return` 提前离开函数。
    Return,
    /// `break` 离开当前循环。
    Break,
    /// `continue` 进入下一轮循环。
    Continue,
    /// 一般错误传播边。
    Error,
    /// 构造器或初始化失败边。
    ConstructFailure,
    /// 动态检查失败边。
    DynamicCheckFailure,
    /// 不可恢复的致命退出边。
    Fatal,
}

impl ExitKind {
    /// 判断是否为正常离开。
    #[must_use]
    pub const fn is_normal(self) -> bool {
        matches!(self, Self::Normal)
    }
}

/// 所有权边产生的静态原因。
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum OwnershipEdgeReason {
    /// 普通名称别名或参数传递。
    Alias,
    /// 容器元素持有关系。
    ContainerElement,
    /// 闭包捕获关系。
    ClosureCapture,
    /// 表字段持有关系。
    TableMember,
    /// 后端显式登记的关系。
    Explicit,
}

/// 一条值到值的所有权关系。
///
/// 边方向已经冻结为 `from -> to` 表示 `from` 必须先于 `to` 释放。该方向
/// 与常见的“父节点指向子节点”画法一致，但这里的释放排序语义是明确的，
/// 后端不得自行反转。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OwnershipEdge {
    /// 必须先释放的值。
    pub from: ValueId,
    /// 依赖前者、随后释放的值。
    pub to: ValueId,
    /// 强边或弱边。
    pub kind: OwnershipKind,
    /// 关系来源。
    pub reason: OwnershipEdgeReason,
    /// 关系对应的源码区间；手工图输入可以为空。
    pub span: Option<SourceSpan>,
}

impl OwnershipEdge {
    /// 创建一条带来源信息的所有权边。
    #[must_use]
    pub const fn new(
        from: ValueId,
        to: ValueId,
        kind: OwnershipKind,
        reason: OwnershipEdgeReason,
        span: Option<SourceSpan>,
    ) -> Self {
        Self {
            from,
            to,
            kind,
            reason,
            span,
        }
    }

    /// 创建一条强拥有边。
    #[must_use]
    pub const fn strong(from: ValueId, to: ValueId, reason: OwnershipEdgeReason) -> Self {
        Self::new(from, to, OwnershipKind::Strong, reason, None)
    }

    /// 创建一条弱引用边。
    #[must_use]
    pub const fn weak(from: ValueId, to: ValueId, reason: OwnershipEdgeReason) -> Self {
        Self::new(from, to, OwnershipKind::Weak, reason, None)
    }
}

/// 值释放动作的种类。
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ReleaseActionKind {
    /// 释放一个强拥有堆值或调用其 `drop` 钩子。
    Strong,
    /// 释放一个弱句柄，不触碰目标对象。
    Weak,
}

/// 释放计划中的一个确定性动作。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReleaseAction {
    /// 要处理的值。
    pub value: ValueId,
    /// 在该计划中的从零开始顺序。
    pub order: usize,
    /// 强释放或弱句柄释放。
    pub kind: ReleaseActionKind,
}

/// 一个作用域在某种退出边上的释放计划。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReleasePlan {
    /// 计划所属作用域。
    pub scope: ScopeId,
    /// 触发该计划的退出边。
    pub exit: ExitKind,
    /// 按执行顺序排列的释放动作。
    pub actions: Vec<ReleaseAction>,
    /// 在该退出边转移给外层或调用方、不能在此处释放的值。
    pub transferred: Vec<ValueId>,
}

impl ReleasePlan {
    /// 创建空释放计划。
    #[must_use]
    pub fn empty(scope: ScopeId, exit: ExitKind) -> Self {
        Self {
            scope,
            exit,
            actions: Vec::new(),
            transferred: Vec::new(),
        }
    }

    /// 判断计划没有释放动作。
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.actions.is_empty()
    }
}

/// 一个静态作用域及其声明顺序。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ScopeInfo {
    /// 稳定作用域身份。
    pub id: ScopeId,
    /// 直接父作用域；程序根没有父作用域。
    pub parent: Option<ScopeId>,
    /// 作用域来源类别。
    pub kind: ScopeKind,
    /// 覆盖该作用域的源码区间。
    pub span: SourceSpan,
    /// 从根开始的嵌套深度。
    pub depth: usize,
    /// 按声明/参数出现顺序保存的值身份。
    pub values: Vec<ValueId>,
}

impl ScopeInfo {
    /// 创建一个作用域记录。
    #[must_use]
    pub fn new(
        id: ScopeId,
        parent: Option<ScopeId>,
        kind: ScopeKind,
        span: SourceSpan,
        depth: usize,
    ) -> Self {
        Self {
            id,
            parent,
            kind,
            span,
            depth,
            values: Vec::new(),
        }
    }

    /// 返回按声明顺序保存的值身份。
    #[must_use]
    pub fn values(&self) -> &[ValueId] {
        &self.values
    }
}

/// 一个名称绑定或表达式产生的静态值记录。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValueInfo {
    /// 稳定值身份。
    pub id: ValueId,
    /// 绑定的规范化名称；匿名临时值为空。
    pub name: Option<String>,
    /// 所属作用域。
    pub scope: ScopeId,
    /// 声明或产生位置。
    pub span: SourceSpan,
    /// 类型检查结果；输入缺失时为空。
    pub ty: Option<Type>,
    /// 静态存储类别。
    pub storage: StorageClass,
    /// 在所属作用域内的声明序号。
    pub declaration_order: usize,
    /// 是否是函数参数。
    pub parameter: bool,
    /// 是否是编译期常量。
    pub constant: bool,
    /// 值是否为匿名表达式临时结果。
    pub temporary: bool,
    /// 已知的逃逸原因，按稳定顺序保存。
    pub escapes: BTreeSet<EscapeReason>,
}

impl ValueInfo {
    /// 返回规范化绑定名称；匿名临时值返回 `None`。
    #[must_use]
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    /// 判断值是否已被标记为逃逸。
    #[must_use]
    pub fn escapes(&self) -> bool {
        !self.escapes.is_empty()
    }

    /// 判断该值是否需要独立释放动作。
    #[must_use]
    pub const fn needs_release(&self) -> bool {
        self.storage.is_heap()
    }
}

/// 动态生命周期边界需要由 Runtime 执行的检查。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DynamicLifetimeCheck {
    /// 相关源码区间。
    pub span: SourceSpan,
    /// 相关值；纯表达式检查可以为空。
    pub value: Option<ValueId>,
    /// 触发保守策略的原因。
    pub reason: EscapeReason,
}

/// 控制流边的静态类别。
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ControlFlowEdgeKind {
    /// 顺序执行。
    Next,
    /// 条件为真。
    BranchTrue,
    /// 条件为假。
    BranchFalse,
    /// 循环回边。
    LoopBack,
    /// 循环 `break`。
    Break,
    /// 循环 `continue`。
    Continue,
    /// 函数返回。
    Return,
    /// 错误或动态检查失败。
    Error,
}

/// 控制流基本块的静态记录。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BasicBlock {
    /// 基本块身份。
    pub id: BlockId,
    /// 所属作用域。
    pub scope: ScopeId,
    /// 覆盖源码区间；合成块可以为空。
    pub span: Option<SourceSpan>,
    /// 该块包含的语句区间。
    pub statements: Vec<SourceSpan>,
    /// 后继块和边类别。
    pub successors: Vec<(BlockId, ControlFlowEdgeKind)>,
    /// 该块中可触发的作用域退出边。
    pub exits: BTreeSet<ExitKind>,
}

/// 可供后端消费的最小控制流图。
#[derive(Clone, Debug, Eq, PartialEq, Default)]
pub struct ControlFlowGraph {
    /// 按稳定编号保存基本块。
    pub blocks: BTreeMap<BlockId, BasicBlock>,
    /// 图入口块。
    pub entry: Option<BlockId>,
}

impl ControlFlowGraph {
    /// 创建空控制流图。
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// 返回指定基本块。
    #[must_use]
    pub fn block(&self, id: BlockId) -> Option<&BasicBlock> {
        self.blocks.get(&id)
    }
}

/// 一次完整 06-A 生命周期分析的结果。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LifetimeResult {
    /// 按稳定身份保存的作用域。
    pub scopes: BTreeMap<ScopeId, ScopeInfo>,
    /// 按稳定身份保存的值。
    pub values: BTreeMap<ValueId, ValueInfo>,
    /// 强拥有关系。
    pub strong_edges: Vec<OwnershipEdge>,
    /// 弱引用关系；不参与强环和强释放拓扑。
    pub weak_edges: Vec<OwnershipEdge>,
    /// 每个作用域/退出边对应的释放计划。
    pub release_plans: BTreeMap<(ScopeId, ExitKind), ReleasePlan>,
    /// 控制流图。
    pub control_flow: ControlFlowGraph,
    /// 需要 Runtime 检查的动态生命周期边界。
    pub dynamic_checks: Vec<DynamicLifetimeCheck>,
    /// 生命周期诊断。
    pub diagnostics: Vec<Diagnostic>,
}

impl Default for LifetimeResult {
    /// 创建空分析结果。
    fn default() -> Self {
        Self {
            scopes: BTreeMap::new(),
            values: BTreeMap::new(),
            strong_edges: Vec::new(),
            weak_edges: Vec::new(),
            release_plans: BTreeMap::new(),
            control_flow: ControlFlowGraph::new(),
            dynamic_checks: Vec::new(),
            diagnostics: Vec::new(),
        }
    }
}

impl LifetimeResult {
    /// 判断是否包含生命周期错误诊断。
    #[must_use]
    pub fn has_errors(&self) -> bool {
        self.diagnostics.iter().any(Diagnostic::is_error)
    }

    /// 判断分析是否成功且没有错误。
    #[must_use]
    pub fn is_success(&self) -> bool {
        !self.has_errors()
    }

    /// 返回指定作用域。
    #[must_use]
    pub fn scope(&self, id: ScopeId) -> Option<&ScopeInfo> {
        self.scopes.get(&id)
    }

    /// 返回全部作用域的确定性映射。
    #[must_use]
    pub const fn scopes(&self) -> &BTreeMap<ScopeId, ScopeInfo> {
        &self.scopes
    }

    /// 返回指定值。
    #[must_use]
    pub fn value(&self, id: ValueId) -> Option<&ValueInfo> {
        self.values.get(&id)
    }

    /// 返回全部值的确定性映射。
    #[must_use]
    pub const fn values(&self) -> &BTreeMap<ValueId, ValueInfo> {
        &self.values
    }

    /// 返回指定作用域/退出边的释放计划。
    #[must_use]
    pub fn release_plan(&self, scope: ScopeId, exit: ExitKind) -> Option<&ReleasePlan> {
        self.release_plans.get(&(scope, exit))
    }

    /// 返回全部作用域/退出边释放计划。
    #[must_use]
    pub const fn release_plans(&self) -> &BTreeMap<(ScopeId, ExitKind), ReleasePlan> {
        &self.release_plans
    }

    /// 返回强拥有关系。
    #[must_use]
    pub fn strong_edges(&self) -> &[OwnershipEdge] {
        &self.strong_edges
    }

    /// 返回弱引用关系。
    #[must_use]
    pub fn weak_edges(&self) -> &[OwnershipEdge] {
        &self.weak_edges
    }

    /// 返回所有权边的迭代器；弱边位于强边之后。
    pub fn ownership_edges(&self) -> impl Iterator<Item = &OwnershipEdge> {
        self.strong_edges.iter().chain(self.weak_edges.iter())
    }

    /// 返回诊断的只读切片。
    #[must_use]
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }

    /// 返回动态检查记录的只读切片。
    #[must_use]
    pub fn dynamic_checks(&self) -> &[DynamicLifetimeCheck] {
        &self.dynamic_checks
    }

    /// 返回控制流图。
    #[must_use]
    pub const fn control_flow(&self) -> &ControlFlowGraph {
        &self.control_flow
    }
}
