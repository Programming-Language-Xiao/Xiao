//! Runtime 可恢复错误与稳定机器字段。
//!
//! 本模块不负责把错误渲染成最终的本地化文本；它保存稳定错误码、消息键、
//! 结构化参数、原因链和被抑制错误，供后续 07 阶段的统一报告器消费。

use std::collections::BTreeMap;
use std::fmt::{self, Display, Formatter};
use std::sync::atomic::{AtomicU64, Ordering};

use xiao_diagnostics::{DiagnosticParam, DiagnosticParams};
use xiao_source::SourceSpan;

/// 无效或空 Runtime 句柄。
pub const INVALID_HANDLE_CODE: &str = "X06-RUNTIME-001";
/// Runtime 类型标签与预期不匹配。
pub const TYPE_MISMATCH_CODE: &str = "X06-RUNTIME-002";
/// 引用计数下溢或溢出。
pub const REFCOUNT_INVARIANT_CODE: &str = "X06-RUNTIME-003";
/// 访问已经释放的对象。
pub const USE_AFTER_RELEASE_CODE: &str = "X06-RUNTIME-004";
/// 弱引用无法升级。
pub const WEAK_UPGRADE_CODE: &str = "X06-RUNTIME-005";
/// 表对象状态不允许当前操作。
pub const TABLE_STATE_CODE: &str = "X06-RUNTIME-006";
/// 表初始化失败。
pub const TABLE_INIT_CODE: &str = "X06-RUNTIME-007";
/// 表释放钩子失败。
pub const TABLE_DROP_CODE: &str = "X06-RUNTIME-008";
/// 数值运算溢出或产生非有限结果。
pub const NUMERIC_OVERFLOW_CODE: &str = "X06-RUNTIME-009";
/// 首版禁止跨线程传递 Runtime 对象。
pub const CROSS_THREAD_CODE: &str = "X06-RUNTIME-010";
/// Runtime 对象分配失败。
pub const ALLOCATION_CODE: &str = "X06-RUNTIME-011";
/// Runtime 值不满足操作要求。
pub const INVALID_VALUE_CODE: &str = "X06-RUNTIME-012";

/// Runtime 错误的稳定类别。
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum RuntimeErrorKind {
    /// 句柄、对象头或生命周期不变量错误。
    Memory,
    /// 值的类型或形状错误。
    Type,
    /// 数值运算错误。
    Arithmetic,
    /// 表构造、成员访问或生命周期错误。
    Table,
    /// 首版并发边界错误。
    Concurrency,
    /// 分配或系统资源错误。
    Resource,
}

impl RuntimeErrorKind {
    /// 返回稳定的小写类别名。
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Memory => "memory",
            Self::Type => "type",
            Self::Arithmetic => "arithmetic",
            Self::Table => "table",
            Self::Concurrency => "concurrency",
            Self::Resource => "resource",
        }
    }
}

/// 所有可恢复 Runtime 操作的错误结果别名。
pub type RuntimeResult<T> = Result<T, RuntimeError>;

/// 为每条 Runtime 错误分配进程内可追踪的单调标识。
static NEXT_ERROR_ID: AtomicU64 = AtomicU64::new(1);

/// 不依赖展示语言的 Runtime 结构化错误。
///
/// 错误句柄本身保持很小，详细原因链和参数放在单独的堆对象中，避免
/// `Result<T, RuntimeError>` 在 Runtime 热路径上携带巨大的错误变体。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeError(Box<RuntimeErrorData>);

/// `RuntimeError` 的堆上详细载荷。
#[derive(Clone, Debug, Eq, PartialEq)]
struct RuntimeErrorData {
    code: String,
    message_id: String,
    params: DiagnosticParams,
    kind: RuntimeErrorKind,
    message: String,
    location: Option<SourceSpan>,
    cause: Option<Box<RuntimeError>>,
    suppressed: Vec<RuntimeError>,
    error_id: u64,
}

impl RuntimeError {
    /// 创建一条带稳定身份和中文预览的 Runtime 错误。
    #[must_use]
    pub fn new(
        kind: RuntimeErrorKind,
        code: impl Into<String>,
        message_id: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self(Box::new(RuntimeErrorData {
            kind,
            code: code.into(),
            message_id: message_id.into(),
            params: BTreeMap::new(),
            message: message.into(),
            location: None,
            cause: None,
            suppressed: Vec::new(),
            error_id: NEXT_ERROR_ID.fetch_add(1, Ordering::Relaxed),
        }))
    }

    /// 创建句柄无效错误。
    #[must_use]
    pub fn invalid_handle(message: impl Into<String>) -> Self {
        Self::new(
            RuntimeErrorKind::Memory,
            INVALID_HANDLE_CODE,
            "runtime.invalid_handle",
            message,
        )
    }

    /// 创建类型不匹配错误。
    #[must_use]
    pub fn type_mismatch(expected: impl Into<String>, actual: impl Into<String>) -> Self {
        let expected = expected.into();
        let actual = actual.into();
        Self::new(
            RuntimeErrorKind::Type,
            TYPE_MISMATCH_CODE,
            "runtime.type_mismatch",
            format!("期望类型 {expected}，实际为 {actual}"),
        )
        .with_param("expected", DiagnosticParam::Text(expected))
        .with_param("actual", DiagnosticParam::Text(actual))
    }

    /// 创建引用计数不变量错误。
    #[must_use]
    pub fn refcount_invariant(message: impl Into<String>) -> Self {
        Self::new(
            RuntimeErrorKind::Memory,
            REFCOUNT_INVARIANT_CODE,
            "runtime.refcount_invariant",
            message,
        )
    }

    /// 创建已释放对象访问错误。
    #[must_use]
    pub fn use_after_release() -> Self {
        Self::new(
            RuntimeErrorKind::Memory,
            USE_AFTER_RELEASE_CODE,
            "runtime.use_after_release",
            "对象已经释放，不能继续访问",
        )
    }

    /// 创建弱引用升级失败错误。
    #[must_use]
    pub fn weak_upgrade() -> Self {
        Self::new(
            RuntimeErrorKind::Memory,
            WEAK_UPGRADE_CODE,
            "runtime.weak_upgrade",
            "弱引用指向的对象已经释放",
        )
    }

    /// 创建表状态错误。
    #[must_use]
    pub fn table_state(expected: impl Into<String>, actual: impl Into<String>) -> Self {
        let expected = expected.into();
        let actual = actual.into();
        Self::new(
            RuntimeErrorKind::Table,
            TABLE_STATE_CODE,
            "runtime.table_state",
            format!("表状态应为 {expected}，实际为 {actual}"),
        )
        .with_param("expected", DiagnosticParam::Text(expected))
        .with_param("actual", DiagnosticParam::Text(actual))
    }

    /// 创建表初始化失败错误。
    #[must_use]
    pub fn table_init(message: impl Into<String>) -> Self {
        Self::new(
            RuntimeErrorKind::Table,
            TABLE_INIT_CODE,
            "runtime.table_init",
            message,
        )
    }

    /// 创建表释放钩子失败错误。
    #[must_use]
    pub fn table_drop(message: impl Into<String>) -> Self {
        Self::new(
            RuntimeErrorKind::Table,
            TABLE_DROP_CODE,
            "runtime.table_drop",
            message,
        )
    }

    /// 创建数值溢出错误。
    #[must_use]
    pub fn numeric_overflow(message: impl Into<String>) -> Self {
        Self::new(
            RuntimeErrorKind::Arithmetic,
            NUMERIC_OVERFLOW_CODE,
            "runtime.numeric_overflow",
            message,
        )
    }

    /// 创建首版跨线程错误。
    #[must_use]
    pub fn cross_thread() -> Self {
        Self::new(
            RuntimeErrorKind::Concurrency,
            CROSS_THREAD_CODE,
            "runtime.cross_thread",
            "首版 Runtime 对象不能跨线程传递",
        )
    }

    /// 创建一般值错误。
    #[must_use]
    pub fn invalid_value(message: impl Into<String>) -> Self {
        Self::new(
            RuntimeErrorKind::Type,
            INVALID_VALUE_CODE,
            "runtime.invalid_value",
            message,
        )
    }

    /// 附加一个结构化参数。
    #[must_use]
    pub fn with_param(mut self, name: impl Into<String>, value: DiagnosticParam) -> Self {
        self.0.params.insert(name.into(), value);
        self
    }

    /// 附加源码位置。
    #[must_use]
    pub fn with_location(mut self, location: SourceSpan) -> Self {
        self.0.location = Some(location);
        self
    }

    /// 包装原始错误并保留原因链。
    #[must_use]
    pub fn with_cause(mut self, cause: Self) -> Self {
        self.0.cause = Some(Box::new(cause));
        self
    }

    /// 将次生错误加入 `suppressed` 列表。
    pub fn push_suppressed(&mut self, error: Self) {
        self.0.suppressed.push(error);
    }

    /// 返回稳定错误码。
    #[must_use]
    pub fn code(&self) -> &str {
        &self.0.code
    }

    /// 返回可翻译消息键。
    #[must_use]
    pub fn message_id(&self) -> &str {
        &self.0.message_id
    }

    /// 返回结构化插值参数。
    #[must_use]
    pub const fn params(&self) -> &DiagnosticParams {
        &self.0.params
    }

    /// 返回错误类别。
    #[must_use]
    pub const fn kind(&self) -> RuntimeErrorKind {
        self.0.kind
    }

    /// 返回中文预览文本；不应被程序作为接口匹配。
    #[must_use]
    pub fn message(&self) -> &str {
        &self.0.message
    }

    /// 返回可选源码位置。
    #[must_use]
    pub const fn location(&self) -> Option<SourceSpan> {
        self.0.location
    }

    /// 返回直接原因。
    #[must_use]
    pub fn cause(&self) -> Option<&Self> {
        self.0.cause.as_deref()
    }

    /// 返回次生错误列表。
    #[must_use]
    pub fn suppressed(&self) -> &[Self] {
        &self.0.suppressed
    }

    /// 返回本次错误事件的唯一编号。
    #[must_use]
    pub const fn error_id(&self) -> u64 {
        self.0.error_id
    }
}

impl Display for RuntimeError {
    /// 生成开发者可读的稳定错误摘要。
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{} [{}]: {}",
            self.kind().as_str(),
            self.code(),
            self.message()
        )
    }
}

impl std::error::Error for RuntimeError {}

/// 作用域展开期间收集主错误和清理阶段的次生错误。
///
/// 第一个错误成为主错误；之后的 `finally`、`drop` 或其他清理错误会被
/// 追加到主错误的 `suppressed` 列表。若开始时没有主错误，则第一个清理
/// 错误成为新的主错误。这保证资源清理不会静默覆盖原始失败原因。
#[derive(Clone, Debug, Default)]
pub struct ErrorAccumulator {
    primary: Option<RuntimeError>,
}

impl ErrorAccumulator {
    /// 创建一个可选主错误的展开累加器。
    #[must_use]
    pub fn new(primary: Option<RuntimeError>) -> Self {
        Self { primary }
    }

    /// 记录一个错误；已有主错误时将其作为次生错误保存。
    pub fn record(&mut self, error: RuntimeError) {
        if let Some(primary) = self.primary.as_mut() {
            primary.push_suppressed(error);
        } else {
            self.primary = Some(error);
        }
    }

    /// 判断当前是否已经有主错误。
    #[must_use]
    pub fn has_error(&self) -> bool {
        self.primary.is_some()
    }

    /// 借用当前主错误。
    #[must_use]
    pub fn primary(&self) -> Option<&RuntimeError> {
        self.primary.as_ref()
    }

    /// 消耗累加器并取出最终主错误。
    #[must_use]
    pub fn finish(self) -> Option<RuntimeError> {
        self.primary
    }
}

#[cfg(test)]
/// 错误身份、原因链和清理错误聚合的回归测试。
mod tests {
    use super::{ErrorAccumulator, RuntimeError, RuntimeErrorKind};
    use xiao_diagnostics::DiagnosticParam;

    #[test]
    /// 保留稳定错误身份、结构化参数和原因链。
    fn keeps_structured_identity_and_cause() {
        let cause = RuntimeError::invalid_handle("底层句柄为空");
        let error = RuntimeError::type_mismatch("str", "int")
            .with_cause(cause.clone())
            .with_param("attempt", DiagnosticParam::Integer(1));
        assert_eq!(error.kind(), RuntimeErrorKind::Type);
        assert_eq!(error.code(), "X06-RUNTIME-002");
        assert_eq!(error.message_id(), "runtime.type_mismatch");
        assert_eq!(error.cause(), Some(&cause));
        assert_eq!(error.params().len(), 3);
        assert!(error.error_id() > 0);
    }

    #[test]
    /// 清理错误进入 suppressed 而不替换主错误。
    fn cleanup_errors_are_suppressed_without_replacing_primary() {
        let primary = RuntimeError::invalid_handle("主错误");
        let mut errors = ErrorAccumulator::new(Some(primary.clone()));
        errors.record(RuntimeError::table_drop("清理失败"));
        let result = errors.finish().expect("应保留主错误");
        assert_eq!(result.code(), primary.code());
        assert_eq!(result.suppressed().len(), 1);
    }

    #[test]
    /// 没有主错误时，首个清理错误成为主错误。
    fn first_cleanup_error_becomes_primary_when_no_error_exists() {
        let mut errors = ErrorAccumulator::new(None);
        errors.record(RuntimeError::table_drop("清理失败"));
        assert_eq!(
            errors.primary().map(RuntimeError::code),
            Some("X06-RUNTIME-008")
        );
    }
}
