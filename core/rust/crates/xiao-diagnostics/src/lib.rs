//! Xiao 统一结构化诊断、可恢复错误和致命故障模型。
//!
//! 本 crate 只保存语言无关的机器字段与报告边界：消息目录、CLI、日志和
//! 调试窗口可以在此基础上渲染，但不能复制另一套错误身份或改变传播语义。

use std::collections::BTreeMap;
use std::fmt::{self, Display, Formatter};
use std::sync::atomic::{AtomicU64, Ordering};

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
/// 整数除法或取模的除数为零。
pub const DIVISION_BY_ZERO_CODE: &str = "X06-RUNTIME-013";
/// 运行时容器索引超出长度。
pub const CONTAINER_INDEX_CODE: &str = "X06-RUNTIME-014";
/// 运行时字典键不存在。
pub const CONTAINER_KEY_CODE: &str = "X06-RUNTIME-015";
/// 运行时判定元素不可哈希，不能进入集合或字典键位置。
pub const CONTAINER_HASHABILITY_CODE: &str = "X06-RUNTIME-016";
/// 选择器运行时边界或路径失败。
pub const SELECTOR_BOUNDS_CODE: &str = "X06-RUNTIME-017";
/// 选择器步长非法（包括零步长）。
pub const SELECTOR_STEP_CODE: &str = "X06-RUNTIME-018";
/// 随机选择数量非法或超出候选范围。
pub const RANDOM_COUNT_CODE: &str = "X06-RUNTIME-019";
/// 随机种子非法。
pub const RANDOM_SEED_CODE: &str = "X06-RUNTIME-020";
/// 集合代数运算的操作数在运行期不是集合。
pub const SET_OPERATION_CODE: &str = "X06-RUNTIME-021";
/// 集合关系比较的操作数在运行期不是集合。
pub const SET_COMPARISON_CODE: &str = "X06-RUNTIME-022";
/// 集合成员判定的操作数在运行期不可哈希。
pub const SET_MEMBERSHIP_CODE: &str = "X06-RUNTIME-023";
/// 动态值不能作为 `for in` 的可迭代对象。
pub const ITERABLE_CODE: &str = "X06-RUNTIME-024";

/// 虚拟机不变量损坏。
pub const FATAL_RUNTIME_INVARIANT_CODE: &str = "X07-FATAL-001";
/// 字节码或其他执行产物损坏。
pub const FATAL_CORRUPT_ARTIFACT_CODE: &str = "X07-FATAL-002";
/// 无法安全建立错误对象的内存耗尽。
pub const FATAL_OUT_OF_MEMORY_CODE: &str = "X07-FATAL-003";
/// 执行调用栈耗尽。
pub const FATAL_STACK_OVERFLOW_CODE: &str = "X07-FATAL-004";
/// 硬件异常。
pub const FATAL_HARDWARE_CODE: &str = "X07-FATAL-005";
/// 未分类的内部故障。
pub const FATAL_INTERNAL_CODE: &str = "X07-FATAL-006";

/// 诊断严重级别。
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Severity {
    /// 必须修复、会阻止当前阶段继续的错误。
    Error,
    /// 不阻止继续处理但需要用户注意的警告。
    Warning,
    /// 仅供开发者查看的信息。
    Info,
}

/// 未经本地化渲染的诊断插值参数。
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DiagnosticParam {
    /// 类型名、路径、标识符或其他应保持原文的文本。
    Text(String),
    /// 数量、位置或其他整数语义值。
    Integer(i128),
    /// 布尔语义值，不使用本地化文本保存。
    Boolean(bool),
}

/// 按稳定参数名排列的诊断参数；不包含翻译后的句子。
pub type DiagnosticParams = BTreeMap<String, DiagnosticParam>;

/// 一条不可变的结构化前端诊断。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Diagnostic {
    code: String,
    message_id: String,
    params: DiagnosticParams,
    severity: Severity,
    span: Option<SourceSpan>,
    message: String,
}

impl Diagnostic {
    /// 创建一条结构化诊断。
    #[must_use]
    pub fn new(
        code: impl Into<String>,
        message_id: impl Into<String>,
        severity: Severity,
        span: Option<SourceSpan>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            code: code.into(),
            message_id: message_id.into(),
            params: DiagnosticParams::new(),
            severity,
            span,
            message: message.into(),
        }
    }

    /// 创建一条带源码区间的错误诊断。
    #[must_use]
    pub fn error_at(
        code: impl Into<String>,
        message_id: impl Into<String>,
        span: SourceSpan,
        message: impl Into<String>,
    ) -> Self {
        Self::new(code, message_id, Severity::Error, Some(span), message)
    }

    /// 为诊断附加结构化参数并返回新的完整记录。
    #[must_use]
    pub fn with_params(
        mut self,
        params: impl IntoIterator<Item = (String, DiagnosticParam)>,
    ) -> Self {
        self.params.extend(params);
        self
    }

    /// 判断诊断是否为错误级别。
    #[must_use]
    pub const fn is_error(&self) -> bool {
        matches!(self.severity, Severity::Error)
    }

    /// 返回稳定的机器诊断编号。
    #[must_use]
    pub fn code(&self) -> &str {
        &self.code
    }
    /// 返回可翻译的稳定消息键。
    #[must_use]
    pub fn message_id(&self) -> &str {
        &self.message_id
    }
    /// 返回原始插值参数。
    #[must_use]
    pub const fn params(&self) -> &DiagnosticParams {
        &self.params
    }
    /// 返回诊断严重级别。
    #[must_use]
    pub const fn severity(&self) -> Severity {
        self.severity
    }
    /// 返回关联的源码区间。
    #[must_use]
    pub const fn span(&self) -> Option<SourceSpan> {
        self.span
    }
    /// 返回当前语言下的展示文本。
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }
}

/// Runtime 错误的稳定类别。
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum XiaoErrorKind {
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
    /// 其他可恢复执行错误。
    Other,
}

impl XiaoErrorKind {
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
            Self::Other => "other",
        }
    }
}

/// `catch` 类型名称的权威分类。
///
/// 这张表是前端和 Runtime 共用的唯一来源。`AnyRecoverable` 对应
/// `Error`/`XiaoError`，`Fatal` 只用于明确拒绝普通 `catch`，不会被路由到
/// 可恢复错误处理器。
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum CatchTypeKind {
    /// 匹配任意可恢复错误。
    AnyRecoverable,
    /// 匹配指定的可恢复错误类别。
    Recoverable(XiaoErrorKind),
    /// 不可由普通 `catch` 捕获的致命故障。
    Fatal,
}

/// 首版错误类型名称的**唯一**来源。
///
/// 名称与类别的对应关系只在这里写一次；[`error_kind_of`] 从它派生，不再另写
/// 一份 `match`。此前两者是平行列举：加一个名字而忘了改另一处，静态检查与
/// Runtime 路由就会漂移，而这类漂移没有门禁能发现。
pub const ERROR_TYPE_NAMES: &[(&str, CatchTypeKind)] = &[
    ("Error", CatchTypeKind::AnyRecoverable),
    ("XiaoError", CatchTypeKind::AnyRecoverable),
    (
        "ArithmeticError",
        CatchTypeKind::Recoverable(XiaoErrorKind::Arithmetic),
    ),
    (
        "MemoryError",
        CatchTypeKind::Recoverable(XiaoErrorKind::Memory),
    ),
    (
        "TableError",
        CatchTypeKind::Recoverable(XiaoErrorKind::Table),
    ),
    (
        "ConcurrencyError",
        CatchTypeKind::Recoverable(XiaoErrorKind::Concurrency),
    ),
    (
        "ResourceError",
        CatchTypeKind::Recoverable(XiaoErrorKind::Resource),
    ),
    ("TypeError", CatchTypeKind::Recoverable(XiaoErrorKind::Type)),
    ("FatalError", CatchTypeKind::Fatal),
];

impl CatchTypeKind {
    /// 判断一个可恢复错误类别是否被该捕获类型覆盖。
    #[must_use]
    pub fn matches(self, actual: XiaoErrorKind) -> bool {
        match self {
            Self::AnyRecoverable => true,
            Self::Recoverable(expected) => expected == actual,
            Self::Fatal => false,
        }
    }

    /// 判断该名称是否可以作为普通 `catch` 类型。
    #[must_use]
    pub fn is_catchable(self) -> bool {
        matches!(self, Self::AnyRecoverable | Self::Recoverable(_))
    }
}

/// 查询一个源码错误类型名称的权威分类。
///
/// 从 [`ERROR_TYPE_NAMES`] 派生，不另写映射表。
#[must_use]
pub fn error_kind_of(name: &str) -> Option<CatchTypeKind> {
    ERROR_TYPE_NAMES
        .iter()
        .find(|(candidate, _)| *candidate == name)
        .map(|(_, kind)| *kind)
}

/// 查询一个名称是否属于首版错误类型集合。
#[must_use]
pub fn is_error_type_name(name: &str) -> bool {
    error_kind_of(name).is_some()
}

/// 查询一个名称是否可以出现在普通 `catch` 中。
#[must_use]
pub fn is_catchable_error_type_name(name: &str) -> bool {
    error_kind_of(name).is_some_and(CatchTypeKind::is_catchable)
}

/// 致命故障的稳定类别。
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum FatalKind {
    /// Runtime 不变量损坏。
    RuntimeInvariant,
    /// 产物内容无法执行。
    CorruptArtifact,
    /// 内存耗尽。
    OutOfMemory,
    /// 调用栈耗尽。
    StackOverflow,
    /// 硬件故障。
    Hardware,
    /// 未分类内部故障。
    Internal,
}

impl FatalKind {
    /// 返回稳定的小写类别名。
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::RuntimeInvariant => "runtime_invariant",
            Self::CorruptArtifact => "corrupt_artifact",
            Self::OutOfMemory => "out_of_memory",
            Self::StackOverflow => "stack_overflow",
            Self::Hardware => "hardware",
            Self::Internal => "internal",
        }
    }
}

/// 后端执行位置，为字节码、原生代码和内联信息预留统一字段。
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct BackendLocation {
    /// 可选字节码指令偏移。
    pub bytecode_offset: Option<u64>,
    /// 可选原生机器地址。
    pub native_address: Option<u64>,
    /// 内联展开深度；零表示非内联帧或未知。
    pub inline_depth: Option<u32>,
}

impl BackendLocation {
    /// 创建一个空的后端位置。
    #[must_use]
    pub const fn empty() -> Self {
        Self {
            bytecode_offset: None,
            native_address: None,
            inline_depth: None,
        }
    }

    /// 设置字节码指令偏移。
    #[must_use]
    pub const fn with_bytecode_offset(mut self, offset: u64) -> Self {
        self.bytecode_offset = Some(offset);
        self
    }

    /// 设置原生机器地址。
    #[must_use]
    pub const fn with_native_address(mut self, address: u64) -> Self {
        self.native_address = Some(address);
        self
    }

    /// 设置内联展开深度。
    #[must_use]
    pub const fn with_inline_depth(mut self, depth: u32) -> Self {
        self.inline_depth = Some(depth);
        self
    }
}

/// 调用栈帧属于用户代码还是 Runtime 内部。
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum FrameKind {
    /// 用户源码或用户模块帧。
    User,
    /// Runtime、VM 或系统内部帧。
    Runtime,
}

/// 可由字节码和 LLVM 后端共同填充的统一调用栈帧。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StackFrame {
    /// 所属模块名。
    pub module: String,
    /// 函数名。
    pub function: String,
    /// 逻辑源文件路径。
    pub source: Option<String>,
    /// 源码字节区间。
    pub span: Option<SourceSpan>,
    /// 后端偏移和内联信息。
    pub backend: BackendLocation,
    /// 用户帧或 Runtime 帧。
    pub kind: FrameKind,
}

impl StackFrame {
    /// 创建一个用户源码帧。
    #[must_use]
    pub fn user(module: impl Into<String>, function: impl Into<String>) -> Self {
        Self::new(module, function, FrameKind::User)
    }

    /// 创建一个 Runtime 内部帧。
    #[must_use]
    pub fn runtime(module: impl Into<String>, function: impl Into<String>) -> Self {
        Self::new(module, function, FrameKind::Runtime)
    }

    /// 创建指定帧类型的空位置帧。
    #[must_use]
    pub fn new(module: impl Into<String>, function: impl Into<String>, kind: FrameKind) -> Self {
        Self {
            module: module.into(),
            function: function.into(),
            source: None,
            span: None,
            backend: BackendLocation::empty(),
            kind,
        }
    }

    /// 附加源文件和源码区间。
    #[must_use]
    pub fn with_source(mut self, source: impl Into<String>, span: Option<SourceSpan>) -> Self {
        self.source = Some(source.into());
        self.span = span;
        self
    }

    /// 以更明确的名称附加源文件和源码区间。
    #[must_use]
    pub fn with_source_file(self, source: impl Into<String>, span: Option<SourceSpan>) -> Self {
        self.with_source(source, span)
    }

    /// 附加后端位置。
    #[must_use]
    pub fn with_backend(mut self, backend: BackendLocation) -> Self {
        self.backend = backend;
        self
    }

    /// 直接附加字节码偏移。
    #[must_use]
    pub fn with_bytecode_offset(self, offset: u64) -> Self {
        let backend = self.backend.with_bytecode_offset(offset);
        self.with_backend(backend)
    }

    /// 直接附加原生地址。
    #[must_use]
    pub fn with_native_address(self, address: u64) -> Self {
        let backend = self.backend.with_native_address(address);
        self.with_backend(backend)
    }

    /// 直接附加内联深度。
    #[must_use]
    pub fn with_inline_depth(self, depth: u32) -> Self {
        let backend = self.backend.with_inline_depth(depth);
        self.with_backend(backend)
    }

    /// 返回模块名。
    #[must_use]
    pub fn module(&self) -> &str {
        &self.module
    }

    /// 返回函数名。
    #[must_use]
    pub fn function(&self) -> &str {
        &self.function
    }

    /// 返回可选源文件。
    #[must_use]
    pub fn source_file(&self) -> Option<&str> {
        self.source.as_deref()
    }

    /// 返回源码区间。
    #[must_use]
    pub const fn span(&self) -> Option<SourceSpan> {
        self.span
    }

    /// 返回后端位置。
    #[must_use]
    pub const fn backend(&self) -> BackendLocation {
        self.backend
    }

    /// 返回帧类别。
    #[must_use]
    pub const fn kind(&self) -> FrameKind {
        self.kind
    }
}

/// 可恢复错误的结构化身份和传播载荷。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct XiaoError(Box<XiaoErrorData>);

/// 可恢复错误的堆上详细载荷，避免错误句柄放大 Runtime 热路径上的 `Result`。
#[derive(Clone, Debug, Eq, PartialEq)]
struct XiaoErrorData {
    code: String,
    message_id: String,
    params: DiagnosticParams,
    kind: XiaoErrorKind,
    message: String,
    location: Option<SourceSpan>,
    context: DiagnosticParams,
    stack: Vec<StackFrame>,
    cause: Option<Box<XiaoError>>,
    suppressed: Vec<XiaoError>,
    error_id: u64,
}

/// 进程内错误事件编号分配器。
static NEXT_ERROR_ID: AtomicU64 = AtomicU64::new(1);

impl XiaoError {
    /// 创建一条可恢复错误。
    #[must_use]
    pub fn new(
        kind: XiaoErrorKind,
        code: impl Into<String>,
        message_id: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self(Box::new(XiaoErrorData {
            code: code.into(),
            message_id: message_id.into(),
            params: BTreeMap::new(),
            kind,
            message: message.into(),
            location: None,
            context: BTreeMap::new(),
            stack: Vec::new(),
            cause: None,
            suppressed: Vec::new(),
            error_id: NEXT_ERROR_ID.fetch_add(1, Ordering::Relaxed),
        }))
    }

    /// 按语言层错误类型名称构造可恢复错误对象。
    ///
    /// `code` 和 `message` 由 `raise TypeError(...)` 的调用参数提供；省略时
    /// 使用稳定的通用身份，具体类别仍由错误类型名单决定。`FatalError` 与
    /// 未知名称返回 `None`，避免把致命故障伪装成可恢复错误。
    #[must_use]
    pub fn from_type_name(
        type_name: &str,
        code: Option<&str>,
        message: Option<&str>,
    ) -> Option<Self> {
        let kind = match error_kind_of(type_name)? {
            CatchTypeKind::Recoverable(kind) => kind,
            CatchTypeKind::AnyRecoverable => XiaoErrorKind::Other,
            CatchTypeKind::Fatal => return None,
        };
        Some(Self::new(
            kind,
            code.unwrap_or("X07-RUNTIME-ERROR"),
            "runtime.user_error",
            message.unwrap_or(type_name),
        ))
    }

    /// 创建句柄无效错误。
    #[must_use]
    pub fn invalid_handle(message: impl Into<String>) -> Self {
        Self::new(
            XiaoErrorKind::Memory,
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
            XiaoErrorKind::Type,
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
            XiaoErrorKind::Memory,
            REFCOUNT_INVARIANT_CODE,
            "runtime.refcount_invariant",
            message,
        )
    }
    /// 创建已释放对象访问错误。
    #[must_use]
    pub fn use_after_release() -> Self {
        Self::new(
            XiaoErrorKind::Memory,
            USE_AFTER_RELEASE_CODE,
            "runtime.use_after_release",
            "对象已经释放，不能继续访问",
        )
    }
    /// 创建弱引用升级失败错误。
    #[must_use]
    pub fn weak_upgrade() -> Self {
        Self::new(
            XiaoErrorKind::Memory,
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
            XiaoErrorKind::Table,
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
            XiaoErrorKind::Table,
            TABLE_INIT_CODE,
            "runtime.table_init",
            message,
        )
    }
    /// 创建表释放钩子失败错误。
    #[must_use]
    pub fn table_drop(message: impl Into<String>) -> Self {
        Self::new(
            XiaoErrorKind::Table,
            TABLE_DROP_CODE,
            "runtime.table_drop",
            message,
        )
    }
    /// 创建数值溢出错误。
    #[must_use]
    pub fn numeric_overflow(message: impl Into<String>) -> Self {
        Self::new(
            XiaoErrorKind::Arithmetic,
            NUMERIC_OVERFLOW_CODE,
            "runtime.numeric_overflow",
            message,
        )
    }
    /// 创建除数为零错误。
    #[must_use]
    pub fn division_by_zero(operator: impl Into<String>) -> Self {
        Self::new(
            XiaoErrorKind::Arithmetic,
            DIVISION_BY_ZERO_CODE,
            "runtime.division_by_zero",
            "除数不能为零",
        )
        .with_param("operator", DiagnosticParam::Text(operator.into()))
    }
    /// 创建运行时容器索引越界错误。
    #[must_use]
    pub fn index_out_of_bounds(container: impl Into<String>, length: usize, index: i128) -> Self {
        Self::new(
            XiaoErrorKind::Type,
            CONTAINER_INDEX_CODE,
            "runtime.index_out_of_bounds",
            "容器索引超出长度",
        )
        .with_param("container", DiagnosticParam::Text(container.into()))
        .with_param("length", DiagnosticParam::Integer(length as i128))
        .with_param("index", DiagnosticParam::Integer(index))
    }

    /// 创建运行时字典键不存在错误。
    #[must_use]
    pub fn key_not_found(container: impl Into<String>, key: impl Into<String>) -> Self {
        Self::new(
            XiaoErrorKind::Type,
            CONTAINER_KEY_CODE,
            "runtime.key_not_found",
            "字典中不存在该键",
        )
        .with_param("container", DiagnosticParam::Text(container.into()))
        .with_param("key", DiagnosticParam::Text(key.into()))
    }

    /// 创建元素不可哈希错误。
    #[must_use]
    pub fn unhashable_element(type_name: impl Into<String>) -> Self {
        Self::new(
            XiaoErrorKind::Type,
            CONTAINER_HASHABILITY_CODE,
            "runtime.unhashable_element",
            "该类型的值不能作为集合元素或字典键",
        )
        .with_param("type_name", DiagnosticParam::Text(type_name.into()))
    }

    /// 集合代数运算的操作数在运行期不是集合。
    pub fn set_operation_requires_sets(type_name: impl Into<String>) -> Self {
        Self::new(
            XiaoErrorKind::Type,
            SET_OPERATION_CODE,
            "runtime.set_operation_requires_sets",
            "集合运算要求两侧都是集合",
        )
        .with_param("type_name", DiagnosticParam::Text(type_name.into()))
    }

    /// 集合关系比较的操作数在运行期不是集合。
    pub fn set_comparison_requires_sets(type_name: impl Into<String>) -> Self {
        Self::new(
            XiaoErrorKind::Type,
            SET_COMPARISON_CODE,
            "runtime.set_comparison_requires_sets",
            "集合比较要求两侧都是集合",
        )
        .with_param("type_name", DiagnosticParam::Text(type_name.into()))
    }

    /// 集合成员判定的操作数在运行期不可哈希。
    pub fn set_membership_requires_hashable(type_name: impl Into<String>) -> Self {
        Self::new(
            XiaoErrorKind::Type,
            SET_MEMBERSHIP_CODE,
            "runtime.set_membership_requires_hashable",
            "成员判定的左操作数必须是可哈希值",
        )
        .with_param("type_name", DiagnosticParam::Text(type_name.into()))
    }

    /// 动态 `for in` 的右侧值不可迭代。
    #[must_use]
    pub fn iterable_required(type_name: impl Into<String>) -> Self {
        Self::new(
            XiaoErrorKind::Type,
            ITERABLE_CODE,
            "runtime.iterable_required",
            "for 的右侧必须是可迭代容器",
        )
        .with_param("type_name", DiagnosticParam::Text(type_name.into()))
    }

    /// 创建选择器边界或路径错误。
    #[must_use]
    pub fn selector_bounds(message: impl Into<String>) -> Self {
        Self::new(
            XiaoErrorKind::Type,
            SELECTOR_BOUNDS_CODE,
            "runtime.selector_bounds",
            message,
        )
    }

    /// 创建选择器步长错误。
    #[must_use]
    pub fn selector_step(message: impl Into<String>) -> Self {
        Self::new(
            XiaoErrorKind::Type,
            SELECTOR_STEP_CODE,
            "runtime.selector_step",
            message,
        )
    }

    /// 创建随机选择数量错误。
    #[must_use]
    pub fn random_count(message: impl Into<String>) -> Self {
        Self::new(
            XiaoErrorKind::Type,
            RANDOM_COUNT_CODE,
            "runtime.random_count",
            message,
        )
    }

    /// 创建随机种子错误。
    #[must_use]
    pub fn random_seed(message: impl Into<String>) -> Self {
        Self::new(
            XiaoErrorKind::Type,
            RANDOM_SEED_CODE,
            "runtime.random_seed",
            message,
        )
    }

    /// 创建首版跨线程错误。
    #[must_use]
    pub fn cross_thread() -> Self {
        Self::new(
            XiaoErrorKind::Concurrency,
            CROSS_THREAD_CODE,
            "runtime.cross_thread",
            "首版 Runtime 对象不能跨线程传递",
        )
    }
    /// 创建一般值错误。
    #[must_use]
    pub fn invalid_value(message: impl Into<String>) -> Self {
        Self::new(
            XiaoErrorKind::Type,
            INVALID_VALUE_CODE,
            "runtime.invalid_value",
            message,
        )
    }
    /// 附加结构化参数。
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
    /// 附加操作上下文，不覆盖错误身份。
    #[must_use]
    pub fn with_context(mut self, name: impl Into<String>, value: DiagnosticParam) -> Self {
        self.0.context.insert(name.into(), value);
        self
    }
    /// 追加一个调用栈帧。
    #[must_use]
    pub fn with_stack_frame(mut self, frame: StackFrame) -> Self {
        self.0.stack.push(frame);
        self
    }
    /// 包装原始错误并保留原因链。
    #[must_use]
    pub fn with_cause(mut self, cause: Self) -> Self {
        self.0.cause = Some(Box::new(cause));
        self
    }
    /// 将次生错误加入 suppressed 列表。
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
    pub const fn kind(&self) -> XiaoErrorKind {
        self.0.kind
    }
    /// 返回当前展示文本。
    #[must_use]
    pub fn message(&self) -> &str {
        &self.0.message
    }
    /// 返回可选源码位置。
    #[must_use]
    pub const fn location(&self) -> Option<SourceSpan> {
        self.0.location
    }
    /// 返回上下文参数。
    #[must_use]
    pub const fn context(&self) -> &DiagnosticParams {
        &self.0.context
    }
    /// 返回调用栈帧。
    #[must_use]
    pub fn stack(&self) -> &[StackFrame] {
        &self.0.stack
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
    /// 返回错误事件编号。
    #[must_use]
    pub const fn error_id(&self) -> u64 {
        self.0.error_id
    }
    /// 可恢复错误始终允许进入普通捕获路径。
    #[must_use]
    pub const fn is_recoverable(&self) -> bool {
        true
    }
    /// 生成与本错误关联的结构化报告记录。
    #[must_use]
    pub fn report(&self) -> ReportRecord {
        ReportRecord::from_error(self)
    }
}

impl Display for XiaoError {
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

impl std::error::Error for XiaoError {}

/// 致命故障；普通 `catch` 不得将其转为成功状态。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FatalError(Box<FatalErrorData>);

/// 致命故障的堆上详细载荷，保持 `Result` 错误句柄轻量。
#[derive(Clone, Debug, Eq, PartialEq)]
struct FatalErrorData {
    code: String,
    message_id: String,
    params: DiagnosticParams,
    kind: FatalKind,
    message: String,
    location: Option<SourceSpan>,
    context: DiagnosticParams,
    stack: Vec<StackFrame>,
    cause: Option<Box<FatalError>>,
    suppressed: Vec<FatalError>,
    error_id: u64,
}

impl FatalError {
    /// 创建一条致命故障。
    #[must_use]
    pub fn new(
        kind: FatalKind,
        code: impl Into<String>,
        message_id: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self(Box::new(FatalErrorData {
            code: code.into(),
            message_id: message_id.into(),
            params: BTreeMap::new(),
            kind,
            message: message.into(),
            location: None,
            context: BTreeMap::new(),
            stack: Vec::new(),
            cause: None,
            suppressed: Vec::new(),
            error_id: NEXT_ERROR_ID.fetch_add(1, Ordering::Relaxed),
        }))
    }
    /// 创建 Runtime 不变量故障。
    #[must_use]
    pub fn runtime_invariant(message: impl Into<String>) -> Self {
        Self::new(
            FatalKind::RuntimeInvariant,
            FATAL_RUNTIME_INVARIANT_CODE,
            "fatal.runtime_invariant",
            message,
        )
    }
    /// 创建损坏产物故障。
    #[must_use]
    pub fn corrupt_artifact(message: impl Into<String>) -> Self {
        Self::new(
            FatalKind::CorruptArtifact,
            FATAL_CORRUPT_ARTIFACT_CODE,
            "fatal.corrupt_artifact",
            message,
        )
    }
    /// 创建内存耗尽故障。
    #[must_use]
    pub fn out_of_memory(message: impl Into<String>) -> Self {
        Self::new(
            FatalKind::OutOfMemory,
            FATAL_OUT_OF_MEMORY_CODE,
            "fatal.out_of_memory",
            message,
        )
    }
    /// 创建调用栈耗尽故障。
    #[must_use]
    pub fn stack_overflow(message: impl Into<String>) -> Self {
        Self::new(
            FatalKind::StackOverflow,
            FATAL_STACK_OVERFLOW_CODE,
            "fatal.stack_overflow",
            message,
        )
    }
    /// 创建硬件故障。
    #[must_use]
    pub fn hardware(message: impl Into<String>) -> Self {
        Self::new(
            FatalKind::Hardware,
            FATAL_HARDWARE_CODE,
            "fatal.hardware",
            message,
        )
    }
    /// 创建内部故障。
    #[must_use]
    pub fn internal(message: impl Into<String>) -> Self {
        Self::new(
            FatalKind::Internal,
            FATAL_INTERNAL_CODE,
            "fatal.internal",
            message,
        )
    }
    /// 附加结构化参数。
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
    /// 附加操作上下文，不改变致命故障身份。
    #[must_use]
    pub fn with_context(mut self, name: impl Into<String>, value: DiagnosticParam) -> Self {
        self.0.context.insert(name.into(), value);
        self
    }
    /// 附加调用栈帧。
    #[must_use]
    pub fn with_stack_frame(mut self, frame: StackFrame) -> Self {
        self.0.stack.push(frame);
        self
    }
    /// 包装致命原因。
    #[must_use]
    pub fn with_cause(mut self, cause: Self) -> Self {
        self.0.cause = Some(Box::new(cause));
        self
    }
    /// 附加清理阶段的致命故障。
    pub fn push_suppressed(&mut self, error: Self) {
        self.0.suppressed.push(error);
    }
    /// 返回稳定错误码。
    #[must_use]
    pub fn code(&self) -> &str {
        &self.0.code
    }
    /// 返回消息键。
    #[must_use]
    pub fn message_id(&self) -> &str {
        &self.0.message_id
    }
    /// 返回参数。
    #[must_use]
    pub const fn params(&self) -> &DiagnosticParams {
        &self.0.params
    }
    /// 返回致命类别。
    #[must_use]
    pub const fn kind(&self) -> FatalKind {
        self.0.kind
    }
    /// 返回当前展示文本。
    #[must_use]
    pub fn message(&self) -> &str {
        &self.0.message
    }
    /// 返回源码位置。
    #[must_use]
    pub const fn location(&self) -> Option<SourceSpan> {
        self.0.location
    }
    /// 返回上下文参数。
    #[must_use]
    pub const fn context(&self) -> &DiagnosticParams {
        &self.0.context
    }
    /// 返回调用栈。
    #[must_use]
    pub fn stack(&self) -> &[StackFrame] {
        &self.0.stack
    }
    /// 返回直接原因。
    #[must_use]
    pub fn cause(&self) -> Option<&Self> {
        self.0.cause.as_deref()
    }
    /// 返回清理阶段故障。
    #[must_use]
    pub fn suppressed(&self) -> &[Self] {
        &self.0.suppressed
    }
    /// 返回故障事件编号，供日志和诊断窗口关联。
    #[must_use]
    pub const fn error_id(&self) -> u64 {
        self.0.error_id
    }
    /// 致命故障不能进入普通可恢复捕获路径。
    #[must_use]
    pub const fn is_recoverable(&self) -> bool {
        false
    }
    /// 生成结构化报告记录。
    #[must_use]
    pub fn report(&self) -> ReportRecord {
        ReportRecord::from_fatal(self)
    }
}

impl Display for FatalError {
    /// 生成致命故障摘要。
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "fatal {} [{}]: {}",
            self.kind().as_str(),
            self.code(),
            self.message()
        )
    }
}

impl std::error::Error for FatalError {}

/// 错误报告所属类别。
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ReportClass {
    /// 可以交给普通错误处理路径的错误。
    Recoverable,
    /// 必须终止当前执行的故障。
    Fatal,
}

/// 与具体 JSON/二进制格式无关的结构化报告记录。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReportRecord {
    /// 可恢复或致命类别。
    pub class: ReportClass,
    /// 稳定错误码。
    pub code: String,
    /// 错误事件唯一编号；用于日志与调试会话关联。
    pub error_id: u64,
    /// 消息目录键。
    pub message_id: String,
    /// 结构化消息参数。
    pub params: DiagnosticParams,
    /// 当前语言预览文本。
    pub message: String,
    /// 直接源码位置。
    pub location: Option<SourceSpan>,
    /// 操作上下文。
    pub context: DiagnosticParams,
    /// 统一调用栈。
    pub stack: Vec<StackFrame>,
    /// 递归原因链报告。
    pub cause: Option<Box<ReportRecord>>,
    /// 次生错误报告。
    pub suppressed: Vec<ReportRecord>,
}

impl ReportRecord {
    /// 从可恢复错误建立报告记录。
    #[must_use]
    pub fn from_error(error: &XiaoError) -> Self {
        Self {
            class: ReportClass::Recoverable,
            code: error.code().to_owned(),
            error_id: error.error_id(),
            message_id: error.message_id().to_owned(),
            params: error.params().clone(),
            message: error.message().to_owned(),
            location: error.location(),
            context: error.context().clone(),
            stack: error.stack().to_vec(),
            cause: error.cause().map(|cause| Box::new(Self::from_error(cause))),
            suppressed: error.suppressed().iter().map(Self::from_error).collect(),
        }
    }

    /// 从致命故障建立报告记录。
    #[must_use]
    pub fn from_fatal(error: &FatalError) -> Self {
        Self {
            class: ReportClass::Fatal,
            code: error.code().to_owned(),
            error_id: error.error_id(),
            message_id: error.message_id().to_owned(),
            params: error.params().clone(),
            message: error.message().to_owned(),
            location: error.location(),
            context: error.context().clone(),
            stack: error.stack().to_vec(),
            cause: error.cause().map(|cause| Box::new(Self::from_fatal(cause))),
            suppressed: error.suppressed().iter().map(Self::from_fatal).collect(),
        }
    }
}

/// 将消息身份和参数渲染为人类可读文本的接口。
pub trait MessageRenderer {
    /// 渲染一个消息键；返回 `None` 表示目录没有该消息。
    fn render(&self, message_id: &str, params: &DiagnosticParams) -> Option<String>;
}

/// 不依赖 `xiao-i18n` 的最小回退渲染器。
#[derive(Clone, Copy, Debug, Default)]
pub struct PreviewRenderer;

impl MessageRenderer for PreviewRenderer {
    /// 没有目录时返回消息键和参数的可读摘要。
    fn render(&self, message_id: &str, params: &DiagnosticParams) -> Option<String> {
        let values = params
            .iter()
            .map(|(key, value)| format!("{key}={value:?}"))
            .collect::<Vec<_>>()
            .join(", ");
        Some(if values.is_empty() {
            message_id.to_owned()
        } else {
            format!("{message_id} ({values})")
        })
    }
}

/// 将报告渲染为默认多行文本。
#[must_use]
pub fn render_text(report: &ReportRecord, renderer: &impl MessageRenderer) -> String {
    /// 递归渲染主错误、原因链和清理阶段错误。
    fn render_one(
        report: &ReportRecord,
        renderer: &impl MessageRenderer,
        indent: usize,
        output: &mut String,
    ) {
        let prefix = "  ".repeat(indent);
        let title = match report.class {
            ReportClass::Recoverable => "error",
            ReportClass::Fatal => "fatal",
        };
        let message = renderer
            .render(&report.message_id, &report.params)
            .unwrap_or_else(|| report.message.clone());
        output.push_str(&format!("{prefix}{title} [{}]: {message}\n", report.code));
        if let Some(location) = report.location {
            output.push_str(&format!(
                "{prefix}at bytes {}..{}\n",
                location.start(),
                location.end()
            ));
        }
        for frame in &report.stack {
            output.push_str(&format!(
                "{prefix}at {}::{} ({:?})\n",
                frame.module, frame.function, frame.kind
            ));
        }
        if let Some(cause) = &report.cause {
            output.push_str(&format!("{prefix}caused by:\n"));
            render_one(cause, renderer, indent + 1, output);
        }
        for suppressed in &report.suppressed {
            output.push_str(&format!("{prefix}suppressed:\n"));
            render_one(suppressed, renderer, indent + 1, output);
        }
    }
    let mut output = String::new();
    render_one(report, renderer, 0, &mut output);
    output
}

/// 使用内置回退渲染器生成报告文本。
#[must_use]
pub fn render_preview(report: &ReportRecord) -> String {
    render_text(report, &PreviewRenderer)
}

/// 使用自定义消息渲染器生成报告文本。
#[must_use]
pub fn render_text_with_renderer(report: &ReportRecord, renderer: &impl MessageRenderer) -> String {
    render_text(report, renderer)
}

/// 可恢复错误结果别名。
pub type XiaoResult<T> = Result<T, XiaoError>;
/// 兼容 Runtime 调用点的结果别名；错误本体已经统一为 `XiaoError`。
pub type RuntimeResult<T> = XiaoResult<T>;
/// 兼容旧 Runtime 命名的错误类别别名。
pub type RuntimeErrorKind = XiaoErrorKind;
/// 兼容旧 Runtime 命名的错误别名；不再维护第二套结构。
pub type RuntimeError = XiaoError;

/// 作用域展开期间收集主错误和清理阶段的次生错误。
#[derive(Clone, Debug, Default)]
pub struct ErrorAccumulator {
    primary: Option<XiaoError>,
}

impl ErrorAccumulator {
    /// 创建一个可选主错误的展开累加器。
    #[must_use]
    pub fn new(primary: Option<XiaoError>) -> Self {
        Self { primary }
    }
    /// 记录一个错误；已有主错误时将其作为次生错误保存。
    pub fn record(&mut self, error: XiaoError) {
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
    pub fn primary(&self) -> Option<&XiaoError> {
        self.primary.as_ref()
    }
    /// 消耗累加器并取出最终主错误。
    #[must_use]
    pub fn finish(self) -> Option<XiaoError> {
        self.primary
    }
}

#[cfg(test)]
/// 覆盖诊断、统一错误、致命故障和报告器边界。
mod tests {
    use super::*;
    use xiao_source::SourceSpan;

    #[test]
    /// 确认旧诊断构造器保留机器字段与源码区间。
    fn builds_error_with_span() {
        let span = SourceSpan::new(2, 3).expect("区间应有效");
        let diagnostic = Diagnostic::error_at("X01-LEX-001", "x01.lex.invalid", span, "bad");
        assert!(diagnostic.is_error());
        assert_eq!(diagnostic.span(), Some(span));
    }

    #[test]
    /// 确认参数不依赖当前语言文本。
    fn preserves_language_independent_params() {
        let diagnostic = Diagnostic::new("E", "type.mismatch", Severity::Error, None, "预览")
            .with_params([(
                "expected".to_owned(),
                DiagnosticParam::Text("bool".to_owned()),
            )]);
        assert_eq!(
            diagnostic.params().get("expected"),
            Some(&DiagnosticParam::Text("bool".to_owned()))
        );
    }

    #[test]
    /// 确认可恢复错误保留原因、上下文和堆栈。
    fn keeps_error_context_and_stack() {
        let frame = StackFrame::user("app", "main").with_source("main.xiao", SourceSpan::new(1, 2));
        let cause = XiaoError::invalid_handle("底层句柄为空");
        let error = XiaoError::type_mismatch("str", "int")
            .with_context("attempt", DiagnosticParam::Integer(1))
            .with_stack_frame(frame.clone())
            .with_cause(cause);
        assert!(error.is_recoverable());
        assert_eq!(error.stack(), &[frame]);
        assert!(error.cause().is_some());
    }

    #[test]
    /// 确认除零是可恢复算术错误，且保留触发它的算子身份。
    fn division_by_zero_keeps_operator_identity() {
        let error = XiaoError::division_by_zero("//");
        assert_eq!(error.code(), DIVISION_BY_ZERO_CODE);
        assert_eq!(error.kind(), XiaoErrorKind::Arithmetic);
        assert!(error.is_recoverable());
        assert_eq!(
            error.params().get("operator"),
            Some(&DiagnosticParam::Text("//".to_owned()))
        );
    }

    #[test]
    /// 确认三个容器错误保留稳定身份与结构化参数。
    fn container_errors_keep_stable_identity() {
        let bounds = XiaoError::index_out_of_bounds("array", 3, -5);
        assert_eq!(bounds.code(), CONTAINER_INDEX_CODE);
        assert_eq!(bounds.kind(), XiaoErrorKind::Type);
        assert_eq!(
            bounds.params().get("length"),
            Some(&DiagnosticParam::Integer(3))
        );
        assert_eq!(
            bounds.params().get("index"),
            Some(&DiagnosticParam::Integer(-5))
        );
        let key = XiaoError::key_not_found("dict table", "missing");
        assert_eq!(key.code(), CONTAINER_KEY_CODE);
        assert_eq!(
            key.params().get("key"),
            Some(&DiagnosticParam::Text("missing".to_owned()))
        );
        let hash = XiaoError::unhashable_element("array");
        assert_eq!(hash.code(), CONTAINER_HASHABILITY_CODE);
        assert_eq!(hash.kind(), XiaoErrorKind::Type);
    }

    #[test]
    /// 确认清理错误进入 suppressed 且不替换主错误。
    fn reports_suppressed_without_replacing_primary() {
        let mut error = XiaoError::invalid_handle("主错误");
        error.push_suppressed(XiaoError::table_drop("清理失败"));
        let report = error.report();
        let text = render_text(&report, &PreviewRenderer);
        assert_eq!(report.code, INVALID_HANDLE_CODE);
        assert_eq!(report.suppressed.len(), 1);
        assert!(text.contains("suppressed"));
    }

    #[test]
    /// 确认致命故障独立于可恢复错误并保留 fatal 类别。
    fn distinguishes_fatal_report() {
        let fatal = FatalError::corrupt_artifact("字节码损坏");
        assert_eq!(fatal.kind(), FatalKind::CorruptArtifact);
        assert!(!fatal.is_recoverable());
        assert_eq!(fatal.report().class, ReportClass::Fatal);
        assert_eq!(fatal.report().error_id, fatal.error_id());
    }

    #[test]
    /// 确认后端位置可以同时保存字节码、原生地址和内联深度。
    fn preserves_backend_location() {
        let backend = BackendLocation {
            bytecode_offset: Some(4),
            native_address: Some(8),
            inline_depth: Some(2),
        };
        let frame = StackFrame::runtime("vm", "dispatch").with_backend(backend);
        assert_eq!(frame.backend, backend);
    }

    #[test]
    /// 确认没有主错误时首个清理错误成为主错误。
    fn accumulator_uses_first_error_as_primary() {
        let mut errors = ErrorAccumulator::new(None);
        errors.record(XiaoError::table_drop("清理失败"));
        assert_eq!(errors.primary().map(XiaoError::code), Some(TABLE_DROP_CODE));
    }

    #[test]
    /// 前端与 Runtime 使用同一张有限错误类型表，未知后缀不得隐式通过。
    fn error_type_registry_is_finite_and_shared() {
        assert!(is_error_type_name("ArithmeticError"));
        assert!(is_catchable_error_type_name("Error"));
        assert!(!is_catchable_error_type_name("FatalError"));
        assert!(error_kind_of("FooError").is_none());
        assert_eq!(ERROR_TYPE_NAMES.len(), 9);
        // 表本身是唯一来源：每个条目都必须能被查询还原，且没有重名。
        let mut seen = std::collections::BTreeSet::new();
        for (name, kind) in ERROR_TYPE_NAMES {
            assert_eq!(error_kind_of(name), Some(*kind), "条目 {name} 未能还原");
            assert!(seen.insert(*name), "错误类型名 {name} 重复");
            if !matches!(kind, CatchTypeKind::Fatal) {
                assert!(is_catchable_error_type_name(name));
            }
        }
    }
}
