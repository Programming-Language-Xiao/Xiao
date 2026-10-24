//! 06-A 生命周期分析使用的稳定数据模型。
//!
//! 本模块只保存值对象和只读结果，不执行任何释放动作。字段保持公开，方便
//! 08 阶段把模型复制到统一 IR；算法实现放在 `graph`、`escape` 和 `release`。

use std::collections::{BTreeMap, BTreeSet};

use xiao_diagnostics::Diagnostic;
use xiao_source::SourceSpan;
use xiao_types::Type;

/// Runtime 的稳定身份。
///
/// Runtime ID 由显式 lowering 或后续 Runtime 分配；生命周期层只保存和
/// 校验这个不透明的数值，不负责创建或回收 Runtime。
pub type RuntimeId = u64;

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

/// 跨并发边界的事实种类。
///
/// `spawn`/`submit` 同时产生任务边界和线程边界；同线程挂起只产生任务边界。
/// 该枚举是事实的语义标签，不能从变量名或函数名推断。
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum BoundaryKind {
    /// 任务/协程生命周期边界。
    TaskBoundary,
    /// 可能迁移到其他 worker 的线程边界。
    ThreadBoundary,
}

impl BoundaryKind {
    /// 返回诊断和快照共用的稳定名称。
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::TaskBoundary => "task_boundary",
            Self::ThreadBoundary => "thread_boundary",
        }
    }
}

/// lowering 传入的值能力分类。
///
/// 该分类是显式 lowering 的输入，不会通过名称、类型显示文本或函数名猜测。
/// 类型检查器可以把自己的并发类型映射到这里，生命周期层据此拒绝不能
/// 进入边界的句柄和动态/资源值。
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum BoundaryValueKind {
    /// 已经通过候选 A Send/Sync 判定的值。
    Sendable,
    /// 只读共享句柄；允许携带源 Runtime ID。
    Shared,
    /// 共享可变句柄；不得跨 Runtime。
    SharedMut,
    /// 普通运行时句柄，包含 StrongHandle/WeakHandle。
    OrdinaryHandle,
    /// 锁守卫；只能在创建它的词法作用域中使用。
    LockGuard,
    /// Runtime 句柄。
    Runtime,
    /// 任务池句柄。
    Pool,
    /// 任务句柄。
    Task,
    /// 线程句柄。
    Thread,
    /// 动态值；应由类型层拒绝或留下运行时检查。
    Dynamic,
    /// 外部资源值。
    Resource,
}

impl BoundaryValueKind {
    /// 返回稳定的类别名，用于结构化诊断参数。
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Sendable => "sendable",
            Self::Shared => "shared",
            Self::SharedMut => "shared_mut",
            Self::OrdinaryHandle => "ordinary_handle",
            Self::LockGuard => "lock_guard",
            Self::Runtime => "runtime",
            Self::Pool => "pool",
            Self::Task => "task",
            Self::Thread => "thread",
            Self::Dynamic => "dynamic",
            Self::Resource => "resource",
        }
    }

    /// 判断该类别是否属于不可转移的普通句柄族。
    #[must_use]
    pub const fn is_non_send_handle(self) -> bool {
        matches!(
            self,
            Self::OrdinaryHandle | Self::Runtime | Self::Pool | Self::Task | Self::Thread
        )
    }
}

/// 将已经降低的语言类型映射为显式边界能力类别。
///
/// 该函数只读取类型变体和能力表，不读取名称。类型层若已经完成候选 A
/// 判定，调用方也可以直接使用 [`BoundaryValueKind::Sendable`]，无需重复映射。
#[must_use]
pub fn classify_boundary_type(ty: &Type) -> BoundaryValueKind {
    match ty {
        Type::Shared(value) if xiao_types::is_send(value) && xiao_types::is_sync(value) => {
            BoundaryValueKind::Shared
        }
        Type::SharedMut(value) if xiao_types::can_share_mutably(value) => {
            BoundaryValueKind::SharedMut
        }
        Type::Shared(_) | Type::SharedMut(_) => BoundaryValueKind::Dynamic,
        Type::LockGuard(_) => BoundaryValueKind::LockGuard,
        Type::Thread(_) => BoundaryValueKind::Thread,
        Type::Task(_) => BoundaryValueKind::Task,
        Type::Runtime => BoundaryValueKind::Runtime,
        Type::Pool => BoundaryValueKind::Pool,
        Type::OrdinaryHandle => BoundaryValueKind::OrdinaryHandle,
        Type::Dynamic | Type::Variable(_) => BoundaryValueKind::Dynamic,
        Type::Resource(_) => BoundaryValueKind::Resource,
        _ if xiao_types::is_send(ty) => BoundaryValueKind::Sendable,
        _ => BoundaryValueKind::Dynamic,
    }
}

/// 一个由 lowering 显式提交的边界参数。
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct BoundaryValue {
    /// 生命周期分析中的稳定值身份。
    pub value: ValueId,
    /// 类型层已判定的并发能力类别。
    pub kind: BoundaryValueKind,
    /// 已知的来源 Runtime；普通值可以没有来源 Runtime。
    pub source_runtime_id: Option<RuntimeId>,
}

impl BoundaryValue {
    /// 创建没有来源 Runtime 的边界参数。
    #[must_use]
    pub const fn new(value: ValueId, kind: BoundaryValueKind) -> Self {
        Self {
            value,
            kind,
            source_runtime_id: None,
        }
    }

    /// 附加该值当前所属的 Runtime ID。
    #[must_use]
    pub const fn from_runtime(mut self, runtime_id: RuntimeId) -> Self {
        self.source_runtime_id = Some(runtime_id);
        self
    }

    /// 根据类型层变体创建显式边界参数。
    #[must_use]
    pub fn from_type(value: ValueId, ty: &Type) -> Self {
        Self::new(value, classify_boundary_type(ty))
    }

    /// 根据类型层变体创建带来源 Runtime 的边界参数。
    #[must_use]
    pub fn from_type_in_runtime(value: ValueId, ty: &Type, runtime_id: RuntimeId) -> Self {
        Self::from_type(value, ty).from_runtime(runtime_id)
    }
}

/// 一条跨线程/任务边界事实。
///
/// 一个值在一次显式 lowering 中只登记一条事实。`runtime_id` 是目标 Runtime，
/// 与 `RuntimeIdFact` 中保存的来源 Runtime 可以不同。
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct CrossThreadFact {
    /// 被转移的值。
    pub value: ValueId,
    /// 目标 Runtime 的稳定身份。
    pub runtime_id: RuntimeId,
    /// 是否跨越任务边界。
    pub task_boundary: bool,
    /// 是否跨越线程边界。
    pub thread_boundary: bool,
    /// 产生事实的显式 lowering 源码区间。
    pub span: SourceSpan,
}

impl CrossThreadFact {
    /// 创建一条事实；至少要有一种边界。
    #[must_use]
    pub const fn new(
        value: ValueId,
        runtime_id: RuntimeId,
        task_boundary: bool,
        thread_boundary: bool,
        span: SourceSpan,
    ) -> Self {
        Self {
            value,
            runtime_id,
            task_boundary,
            thread_boundary,
            span,
        }
    }

    /// 创建 `spawn`/`submit` 产生的双边界事实。
    #[must_use]
    pub const fn spawn(value: ValueId, runtime_id: RuntimeId, span: SourceSpan) -> Self {
        Self::new(value, runtime_id, true, true, span)
    }

    /// 创建同线程挂起产生的单任务边界事实。
    #[must_use]
    pub const fn task(value: ValueId, runtime_id: RuntimeId, span: SourceSpan) -> Self {
        Self::new(value, runtime_id, true, false, span)
    }

    /// 判断事实是否包含指定边界。
    #[must_use]
    pub const fn contains(self, kind: BoundaryKind) -> bool {
        match kind {
            BoundaryKind::TaskBoundary => self.task_boundary,
            BoundaryKind::ThreadBoundary => self.thread_boundary,
        }
    }
}

/// `BoundaryFact` 是后端使用的通用别名。
pub type BoundaryFact = CrossThreadFact;

/// 任务边界事实的语义别名。
pub type TaskBoundary = CrossThreadFact;

/// 线程边界事实的语义别名。
pub type ThreadBoundary = CrossThreadFact;

/// Runtime ID 的来源事实。
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct RuntimeIdFact {
    /// 绑定 Runtime 身份的值。
    pub value: ValueId,
    /// 该值所属 Runtime。
    pub runtime_id: RuntimeId,
    /// 登记 Runtime 身份的源码区间。
    pub span: SourceSpan,
}

/// 按值身份保存 Runtime ID 的确定性事实容器。
#[derive(Clone, Debug, Eq, PartialEq, Default)]
pub struct RuntimeIdFacts {
    /// 每个值最多对应一个 Runtime ID。
    pub facts: BTreeMap<ValueId, RuntimeIdFact>,
}

impl RuntimeIdFacts {
    /// 创建空容器。
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// 登记 Runtime ID；相同值的不同 ID 会返回冲突。
    pub fn insert(&mut self, fact: RuntimeIdFact) -> Result<(), RuntimeIdFact> {
        if let Some(existing) = self.facts.get(&fact.value) {
            if existing.runtime_id != fact.runtime_id {
                return Err(*existing);
            }
            return Ok(());
        }
        self.facts.insert(fact.value, fact);
        Ok(())
    }

    /// 以值身份和源码区间登记 Runtime ID。
    pub fn record(
        &mut self,
        value: ValueId,
        runtime_id: RuntimeId,
        span: SourceSpan,
    ) -> Result<(), RuntimeIdFact> {
        self.insert(RuntimeIdFact {
            value,
            runtime_id,
            span,
        })
    }

    /// 返回某个值的 Runtime ID 事实。
    #[must_use]
    pub fn get(&self, value: ValueId) -> Option<&RuntimeIdFact> {
        self.facts.get(&value)
    }

    /// 返回某个值的 Runtime ID 数值。
    #[must_use]
    pub fn runtime_id(&self, value: ValueId) -> Option<RuntimeId> {
        self.get(value).map(|fact| fact.runtime_id)
    }

    /// 返回只读映射。
    #[must_use]
    pub const fn as_map(&self) -> &BTreeMap<ValueId, RuntimeIdFact> {
        &self.facts
    }

    /// 判断容器是否为空。
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.facts.is_empty()
    }
}

/// lowering 产生边界事实的操作类别。
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum BoundaryOperation {
    /// `spawn(runtime, fn, ...)`。
    Spawn,
    /// `submit(pool, fn, ...)`。
    Submit,
    /// 同线程挂起或未来协程边界。
    TaskSuspend,
}

impl BoundaryOperation {
    /// 返回稳定操作名称。
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Spawn => "spawn",
            Self::Submit => "submit",
            Self::TaskSuspend => "task_suspend",
        }
    }
}

/// 登记并发边界事实时的结构化错误。
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum BoundaryFactError {
    /// 同一个值已经有一条边界事实。
    DuplicateValue {
        /// 冲突的值身份。
        value: ValueId,
    },
    /// 同一个值被登记了不同 Runtime ID。
    RuntimeIdConflict {
        /// 冲突的值身份。
        value: ValueId,
        /// 已有 Runtime ID。
        existing: RuntimeId,
        /// 新提交的 Runtime ID。
        incoming: RuntimeId,
    },
    /// 值的类别不允许进入并发边界。
    UnsupportedValue {
        /// 冲突的值身份。
        value: ValueId,
        /// 类型层提交的能力类别。
        kind: BoundaryValueKind,
    },
    /// `lock_guard<T>` 试图逃出其词法作用域。
    LockGuardEscape {
        /// 冲突的值身份。
        value: ValueId,
    },
    /// `shared_mut<T>` 试图跨 Runtime 传播。
    SharedMutCrossRuntime {
        /// 冲突的值身份。
        value: ValueId,
        /// 来源 Runtime。
        source: RuntimeId,
        /// 目标 Runtime。
        target: RuntimeId,
    },
}

impl BoundaryFactError {
    /// 返回错误对应的边界种类；普通值分类错误没有单一边界。
    #[must_use]
    pub const fn boundary_kind(self) -> Option<BoundaryKind> {
        match self {
            Self::LockGuardEscape { .. } | Self::SharedMutCrossRuntime { .. } => {
                Some(BoundaryKind::ThreadBoundary)
            }
            Self::DuplicateValue { .. }
            | Self::RuntimeIdConflict { .. }
            | Self::UnsupportedValue { .. } => None,
        }
    }
}

impl std::fmt::Display for BoundaryFactError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DuplicateValue { value } => write!(formatter, "值 {} 已有边界事实", value.get()),
            Self::RuntimeIdConflict {
                value,
                existing,
                incoming,
            } => write!(
                formatter,
                "值 {} 的 Runtime ID 冲突：已有 {existing}，收到 {incoming}",
                value.get()
            ),
            Self::UnsupportedValue { value, kind } => write!(
                formatter,
                "值 {} 的类别 {} 不能进入并发边界",
                value.get(),
                kind.as_str()
            ),
            Self::LockGuardEscape { value } => {
                write!(
                    formatter,
                    "lock_guard 值 {} 不能逃出词法作用域",
                    value.get()
                )
            }
            Self::SharedMutCrossRuntime {
                value,
                source,
                target,
            } => write!(
                formatter,
                "shared_mut 值 {} 不能从 Runtime {source} 传到 Runtime {target}",
                value.get()
            ),
        }
    }
}

impl std::error::Error for BoundaryFactError {}

/// 显式 lowering 产生的 Runtime ID、任务边界和线程边界事实。
///
/// 该容器不访问 AST，也不识别函数/变量名称。调用方必须从已经解析并完成
/// 类型判定的 `spawn`/`submit` lowering 入口提交事实。
#[derive(Clone, Debug, Eq, PartialEq, Default)]
pub struct BoundaryFacts {
    /// 值到来源 Runtime ID 的事实。
    pub runtime_ids: RuntimeIdFacts,
    /// 每个边界值对应的一条跨线程事实。
    pub cross_thread_facts: Vec<CrossThreadFact>,
    /// 登记阶段产生的结构化诊断；结果合并时一并交给调用方。
    pub diagnostics: Vec<Diagnostic>,
}

impl BoundaryFacts {
    /// 创建空事实容器。
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// 登记一个值的来源 Runtime ID。
    pub fn register_runtime_id(
        &mut self,
        value: ValueId,
        runtime_id: RuntimeId,
        span: SourceSpan,
    ) -> Result<(), BoundaryFactError> {
        let fact = RuntimeIdFact {
            value,
            runtime_id,
            span,
        };
        if let Err(existing) = self.runtime_ids.insert(fact) {
            let error = BoundaryFactError::RuntimeIdConflict {
                value,
                existing: existing.runtime_id,
                incoming: runtime_id,
            };
            self.diagnostics
                .push(crate::diagnostics::runtime_id_conflict(span, &error));
            return Err(error);
        }
        Ok(())
    }

    /// `register_runtime_id` 的简短别名。
    pub fn record_runtime_id(
        &mut self,
        value: ValueId,
        runtime_id: RuntimeId,
        span: SourceSpan,
    ) -> Result<(), BoundaryFactError> {
        self.register_runtime_id(value, runtime_id, span)
    }

    /// 返回值的来源 Runtime ID。
    #[must_use]
    pub fn runtime_id(&self, value: ValueId) -> Option<RuntimeId> {
        self.runtime_ids.get(value).map(|fact| fact.runtime_id)
    }

    /// 返回 Runtime ID 事实容器。
    #[must_use]
    pub const fn runtime_id_facts(&self) -> &RuntimeIdFacts {
        &self.runtime_ids
    }

    /// 登记 `spawn` 的一组未分类边界值。
    ///
    /// 该入口供已经完成类型判定的 lowering 使用；需要生命周期层再次校验
    /// 值类别时使用 [`Self::record_spawn_values`]。
    pub fn record_spawn<I>(
        &mut self,
        values: I,
        runtime_id: RuntimeId,
        span: SourceSpan,
    ) -> Result<(), BoundaryFactError>
    where
        I: IntoIterator<Item = ValueId>,
    {
        self.record_operation(
            BoundaryOperation::Spawn,
            values
                .into_iter()
                .map(|value| BoundaryValue::new(value, BoundaryValueKind::Sendable)),
            runtime_id,
            span,
        )
    }

    /// 登记 `submit` 的一组未分类边界值。
    pub fn record_submit<I>(
        &mut self,
        values: I,
        runtime_id: RuntimeId,
        span: SourceSpan,
    ) -> Result<(), BoundaryFactError>
    where
        I: IntoIterator<Item = ValueId>,
    {
        self.record_operation(
            BoundaryOperation::Submit,
            values
                .into_iter()
                .map(|value| BoundaryValue::new(value, BoundaryValueKind::Sendable)),
            runtime_id,
            span,
        )
    }

    /// 登记单个 `spawn` 边界参数。
    pub fn record_spawn_value(
        &mut self,
        value: ValueId,
        runtime_id: RuntimeId,
        span: SourceSpan,
    ) -> Result<(), BoundaryFactError> {
        self.record_spawn([value], runtime_id, span)
    }

    /// 登记单个 `submit` 边界参数。
    pub fn record_submit_value(
        &mut self,
        value: ValueId,
        runtime_id: RuntimeId,
        span: SourceSpan,
    ) -> Result<(), BoundaryFactError> {
        self.record_submit([value], runtime_id, span)
    }

    /// 登记同线程挂起产生的任务边界。
    pub fn record_task_boundary<I>(
        &mut self,
        values: I,
        runtime_id: RuntimeId,
        span: SourceSpan,
    ) -> Result<(), BoundaryFactError>
    where
        I: IntoIterator<Item = ValueId>,
    {
        self.record_operation(
            BoundaryOperation::TaskSuspend,
            values
                .into_iter()
                .map(|value| BoundaryValue::new(value, BoundaryValueKind::Sendable)),
            runtime_id,
            span,
        )
    }

    /// 登记带显式能力类别的一组 `spawn` 参数。
    pub fn record_spawn_values<I>(
        &mut self,
        values: I,
        runtime_id: RuntimeId,
        span: SourceSpan,
    ) -> Result<(), BoundaryFactError>
    where
        I: IntoIterator<Item = BoundaryValue>,
    {
        self.record_operation(BoundaryOperation::Spawn, values, runtime_id, span)
    }

    /// 登记带显式能力类别的一组 `submit` 参数。
    pub fn record_submit_values<I>(
        &mut self,
        values: I,
        runtime_id: RuntimeId,
        span: SourceSpan,
    ) -> Result<(), BoundaryFactError>
    where
        I: IntoIterator<Item = BoundaryValue>,
    {
        self.record_operation(BoundaryOperation::Submit, values, runtime_id, span)
    }

    /// 登记一个显式边界操作。
    pub fn record_operation<I>(
        &mut self,
        operation: BoundaryOperation,
        values: I,
        runtime_id: RuntimeId,
        span: SourceSpan,
    ) -> Result<(), BoundaryFactError>
    where
        I: IntoIterator<Item = BoundaryValue>,
    {
        let values = values.into_iter().collect::<Vec<_>>();
        let (task_boundary, thread_boundary) = match operation {
            BoundaryOperation::Spawn | BoundaryOperation::Submit => (true, true),
            BoundaryOperation::TaskSuspend => (true, false),
        };
        let mut errors = Vec::new();
        let mut seen = BTreeSet::new();
        for item in &values {
            if !seen.insert(item.value)
                || self
                    .cross_thread_facts
                    .iter()
                    .any(|fact| fact.value == item.value)
            {
                errors.push(BoundaryFactError::DuplicateValue { value: item.value });
                continue;
            }
            if let Some(error) =
                self.validate_value(*item, runtime_id, task_boundary, thread_boundary)
            {
                errors.push(error);
            }
        }
        if let Some(error) = errors.first().copied() {
            for error in errors {
                self.diagnostics
                    .push(crate::diagnostics::boundary_fact_error(
                        span, operation, &error,
                    ));
            }
            return Err(error);
        }

        for item in values {
            self.cross_thread_facts.push(CrossThreadFact::new(
                item.value,
                runtime_id,
                task_boundary,
                thread_boundary,
                span,
            ));
        }
        Ok(())
    }

    /// 检查一个值能否离开词法作用域。
    ///
    /// 该入口专门供 `return`、闭包捕获和边界 lowering 共用；锁守卫一律拒绝，
    /// 避免它通过普通生命周期逃逸路径进入任务或线程。
    pub fn validate_escape(
        &mut self,
        value: BoundaryValue,
        span: SourceSpan,
    ) -> Result<(), BoundaryFactError> {
        if value.kind == BoundaryValueKind::LockGuard {
            let error = BoundaryFactError::LockGuardEscape { value: value.value };
            self.diagnostics
                .push(crate::diagnostics::boundary_fact_error(
                    span,
                    BoundaryOperation::TaskSuspend,
                    &error,
                ));
            return Err(error);
        }
        Ok(())
    }

    /// 将另一组显式 lowering 事实合并到当前容器。
    pub fn merge(&mut self, other: Self) -> Result<(), BoundaryFactError> {
        let mut merged = self.clone();
        for fact in other.runtime_ids.facts.values().copied() {
            merged.register_runtime_id(fact.value, fact.runtime_id, fact.span)?;
        }
        for fact in other.cross_thread_facts {
            if merged
                .cross_thread_facts
                .iter()
                .any(|existing| existing.value == fact.value)
            {
                return Err(BoundaryFactError::DuplicateValue { value: fact.value });
            }
            merged.cross_thread_facts.push(fact);
        }
        merged.diagnostics.extend(other.diagnostics);
        *self = merged;
        Ok(())
    }

    /// 返回跨线程事实的只读切片。
    #[must_use]
    pub fn cross_thread_facts(&self) -> &[CrossThreadFact] {
        &self.cross_thread_facts
    }

    /// 返回含任务边界标志的事实。
    pub fn task_boundaries(&self) -> impl Iterator<Item = &TaskBoundary> {
        self.cross_thread_facts
            .iter()
            .filter(|fact| fact.task_boundary)
    }

    /// 返回含线程边界标志的事实。
    pub fn thread_boundaries(&self) -> impl Iterator<Item = &ThreadBoundary> {
        self.cross_thread_facts
            .iter()
            .filter(|fact| fact.thread_boundary)
    }

    /// 返回事实容器是否为空。
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.runtime_ids.is_empty() && self.cross_thread_facts.is_empty()
    }

    fn validate_value(
        &self,
        value: BoundaryValue,
        target_runtime_id: RuntimeId,
        task_boundary: bool,
        thread_boundary: bool,
    ) -> Option<BoundaryFactError> {
        if value.kind == BoundaryValueKind::LockGuard {
            return Some(BoundaryFactError::LockGuardEscape { value: value.value });
        }
        if value.kind == BoundaryValueKind::SharedMut
            && value
                .source_runtime_id
                .is_some_and(|source| source != target_runtime_id)
        {
            return Some(BoundaryFactError::SharedMutCrossRuntime {
                value: value.value,
                source: value
                    .source_runtime_id
                    .expect("is_some_and 已确认来源 Runtime"),
                target: target_runtime_id,
            });
        }
        if value.kind.is_non_send_handle()
            || matches!(
                value.kind,
                BoundaryValueKind::Dynamic | BoundaryValueKind::Resource
            )
        {
            let _ = (task_boundary, thread_boundary);
            return Some(BoundaryFactError::UnsupportedValue {
                value: value.value,
                kind: value.kind,
            });
        }
        None
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
    /// `try` 受保护主体作用域。
    Try,
    /// `catch` 处理器作用域；不继承已释放的 try 局部值。
    Catch,
    /// `finally` 清理作用域。
    Finally,
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
    /// `raise` 主动抛出可恢复错误。
    Raise,
    /// 错误被某个 `catch` 处理器匹配。
    Catch,
    /// 没有匹配处理器、继续向外传播。
    UnmatchedError,
    /// 构造器或初始化失败边。
    ConstructFailure,
    /// 动态检查失败边。
    DynamicCheckFailure,
    /// 不可恢复的致命退出边。
    Fatal,
}

impl ExitKind {
    /// 所有退出边种类的稳定顺序。
    ///
    /// 释放计划按该顺序为每个作用域预建条目，后端不得依赖其他顺序。
    pub const ALL: [Self; 11] = [
        Self::Normal,
        Self::Return,
        Self::Break,
        Self::Continue,
        Self::Error,
        Self::Raise,
        Self::Catch,
        Self::UnmatchedError,
        Self::ConstructFailure,
        Self::DynamicCheckFailure,
        Self::Fatal,
    ];

    /// 判断是否为正常离开。
    #[must_use]
    pub const fn is_normal(self) -> bool {
        matches!(self, Self::Normal)
    }

    /// 返回该退出边在 IR、字节码和原生后端共享的稳定名称。
    ///
    /// 名称一旦发布就不能改写；后端用它反查释放计划，不得维护第二份映射表。
    #[must_use]
    pub const fn as_name(self) -> &'static str {
        match self {
            Self::Normal => "normal",
            Self::Return => "return",
            Self::Break => "break",
            Self::Continue => "continue",
            Self::Error => "error",
            Self::Raise => "raise",
            Self::Catch => "catch",
            Self::UnmatchedError => "unmatched_error",
            Self::ConstructFailure => "construct_failure",
            Self::DynamicCheckFailure => "dynamic_check_failure",
            Self::Fatal => "fatal",
        }
    }

    /// 按稳定名称还原退出边类别。
    ///
    /// 只接受 [`Self::as_name`] 产出的拼写；未知名称返回 `None`，由调用方
    /// 决定是拒绝产物还是继续执行，不得静默退化为某个默认类别。
    #[must_use]
    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|kind| kind.as_name() == name)
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

impl ReleaseActionKind {
    /// 所有释放动作种类的稳定顺序。
    pub const ALL: [Self; 2] = [Self::Strong, Self::Weak];

    /// 返回该动作在 IR、字节码和原生后端共享的稳定名称。
    #[must_use]
    pub const fn as_name(self) -> &'static str {
        match self {
            Self::Strong => "strong",
            Self::Weak => "weak",
        }
    }

    /// 按稳定名称还原释放动作种类；未知名称返回 `None`。
    #[must_use]
    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|kind| kind.as_name() == name)
    }
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
    /// 由显式 lowering 产生的任务/线程边界及 Runtime ID 事实。
    pub boundary_facts: BoundaryFacts,
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
            boundary_facts: BoundaryFacts::default(),
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

    /// 返回显式 lowering 产生的边界事实容器。
    #[must_use]
    pub const fn boundary_facts(&self) -> &BoundaryFacts {
        &self.boundary_facts
    }

    /// 返回跨任务/线程事实。
    #[must_use]
    pub fn cross_thread_facts(&self) -> &[CrossThreadFact] {
        self.boundary_facts.cross_thread_facts()
    }

    /// 返回 Runtime ID 事实。
    #[must_use]
    pub const fn runtime_id_facts(&self) -> &RuntimeIdFacts {
        self.boundary_facts.runtime_id_facts()
    }

    /// 返回控制流图。
    #[must_use]
    pub const fn control_flow(&self) -> &ControlFlowGraph {
        &self.control_flow
    }
}

#[cfg(test)]
mod tests {
    use super::{
        BoundaryFactError, BoundaryFacts, BoundaryOperation, BoundaryValue, BoundaryValueKind,
        CrossThreadFact, RuntimeIdFact, RuntimeIdFacts, ValueId, classify_boundary_type,
    };
    use xiao_source::SourceSpan;
    use xiao_types::{ScalarType, Type};

    fn span() -> SourceSpan {
        SourceSpan::new(0, 1).expect("测试区间有效")
    }

    #[test]
    fn explicit_spawn_and_submit_produce_both_boundary_facts() {
        let mut facts = BoundaryFacts::new();
        facts
            .record_spawn([ValueId::new(1)], 7, span())
            .expect("spawn 应登记");
        facts
            .record_submit([ValueId::new(2)], 9, span())
            .expect("submit 应登记");
        assert_eq!(facts.cross_thread_facts.len(), 2);
        assert!(facts.cross_thread_facts.iter().all(|fact| {
            fact.task_boundary
                && fact.thread_boundary
                && (fact.runtime_id == 7 || fact.runtime_id == 9)
        }));
    }

    #[test]
    fn task_suspend_only_produces_task_boundary() {
        let mut facts = BoundaryFacts::new();
        facts
            .record_task_boundary([ValueId::new(3)], 11, span())
            .expect("同线程挂起应登记");
        assert_eq!(
            facts.cross_thread_facts,
            vec![CrossThreadFact::task(ValueId::new(3), 11, span())]
        );
    }

    #[test]
    fn ordinary_handle_and_lock_guard_are_rejected_without_name_inspection() {
        let mut facts = BoundaryFacts::new();
        let ordinary = facts.record_spawn_values(
            [BoundaryValue::new(
                ValueId::new(4),
                BoundaryValueKind::OrdinaryHandle,
            )],
            1,
            span(),
        );
        assert!(matches!(
            ordinary,
            Err(BoundaryFactError::UnsupportedValue {
                kind: BoundaryValueKind::OrdinaryHandle,
                ..
            })
        ));
        let guard = facts.validate_escape(
            BoundaryValue::new(ValueId::new(5), BoundaryValueKind::LockGuard),
            span(),
        );
        assert!(matches!(
            guard,
            Err(BoundaryFactError::LockGuardEscape { .. })
        ));
        assert!(facts.cross_thread_facts.is_empty());
    }

    #[test]
    fn shared_mut_cross_runtime_and_runtime_id_conflict_are_rejected() {
        let mut facts = BoundaryFacts::new();
        facts
            .register_runtime_id(ValueId::new(6), 3, span())
            .expect("首次 Runtime ID 应登记");
        let conflict = facts.register_runtime_id(ValueId::new(6), 4, span());
        assert!(matches!(
            conflict,
            Err(BoundaryFactError::RuntimeIdConflict {
                existing: 3,
                incoming: 4,
                ..
            })
        ));
        let shared_mut = facts.record_operation(
            BoundaryOperation::Spawn,
            [BoundaryValue::new(ValueId::new(7), BoundaryValueKind::SharedMut).from_runtime(3)],
            4,
            span(),
        );
        assert!(matches!(
            shared_mut,
            Err(BoundaryFactError::SharedMutCrossRuntime {
                source: 3,
                target: 4,
                ..
            })
        ));
        assert!(
            facts
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code() == "X06-LIFETIME-008")
        );
    }

    #[test]
    fn runtime_id_facts_are_deterministic_and_idempotent() {
        let mut ids = RuntimeIdFacts::new();
        let fact = RuntimeIdFact {
            value: ValueId::new(8),
            runtime_id: 12,
            span: span(),
        };
        ids.insert(fact).expect("首次插入");
        ids.insert(fact).expect("同一事实可重复插入");
        assert_eq!(ids.as_map().len(), 1);
        assert_eq!(ids.get(ValueId::new(8)), Some(&fact));
    }

    #[test]
    fn boundary_type_classification_uses_type_identity() {
        assert_eq!(
            classify_boundary_type(&Type::shared(Type::scalar(ScalarType::Int))),
            BoundaryValueKind::Shared
        );
        assert_eq!(
            classify_boundary_type(&Type::shared_mut(Type::scalar(ScalarType::Int))),
            BoundaryValueKind::SharedMut
        );
        assert_eq!(
            classify_boundary_type(&Type::lock_guard(Type::scalar(ScalarType::Int))),
            BoundaryValueKind::LockGuard
        );
        assert_eq!(
            classify_boundary_type(&Type::OrdinaryHandle),
            BoundaryValueKind::OrdinaryHandle
        );
        assert_eq!(
            classify_boundary_type(&Type::scalar(ScalarType::Int)),
            BoundaryValueKind::Sendable
        );
        assert_eq!(
            classify_boundary_type(&Type::shared(Type::Dynamic)),
            BoundaryValueKind::Dynamic
        );
    }
}
