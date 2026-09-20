//! 运行入口、结构化结果与运行指标。

use std::fmt::{Display, Formatter};

use xiao_bytecode::{TacProgram, TacVerificationError, verify_for_execution};
use xiao_diagnostics::{DiagnosticParam, FatalError, ReportRecord, XiaoError};
use xiao_ir::{IrEntryMode, IrProgram};
use xiao_runtime::RuntimeValue;

use crate::carrier::Carrier;
use crate::machine::hybrid::HybridCarrier;
use crate::machine::register::RegisterCarrier;
use crate::machine::stack::StackCarrier;
use crate::semantics::VmMetadata;
use crate::semantics::{BoundArgument, Vm};
use crate::sink::{
    BoundedSink, DEFAULT_EVENT_CAPACITY, MAX_EVENT_CAPACITY, RecordingSink, VmEvent, VmEventSink,
};

/// 默认的最大调用深度。
pub const DEFAULT_MAX_CALL_DEPTH: usize = 1024;
/// 生产入口允许的最大调用深度。
pub const MAX_MAX_CALL_DEPTH: usize = 1_000_000;
/// 运行参数非法时使用的稳定诊断编号。
pub const VM_OPTIONS_CODE: &str = "X09-VM-001";
/// 运行请求字段非法时使用的稳定诊断编号。
pub const VM_REQUEST_CODE: &str = "X09-VM-002";

/// 规范化运行参数的验证错误。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VmOptionsError {
    /// 出错字段。
    pub field: &'static str,
    /// 收到的数值。
    pub value: usize,
    /// 稳定的开发者原因。
    pub reason: &'static str,
}

impl VmOptionsError {
    /// 返回稳定诊断编号。
    #[must_use]
    pub const fn code(&self) -> &'static str {
        VM_OPTIONS_CODE
    }
}

impl Display for VmOptionsError {
    /// 输出稳定编号和字段原因。
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "{}: {}={} ({})",
            self.code(),
            self.field,
            self.value,
            self.reason
        )
    }
}

impl std::error::Error for VmOptionsError {}

/// 生产请求的字段验证错误。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RunRequestError {
    /// 稳定诊断编号。
    pub code: &'static str,
    /// 出错字段。
    pub field: &'static str,
    /// 面向开发者的原因。
    pub message: String,
}

impl Display for RunRequestError {
    /// 输出稳定编号、字段和原因。
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "{}: {} ({})",
            self.code, self.field, self.message
        )
    }
}

impl std::error::Error for RunRequestError {}

/// 一次运行的结构化结果。
///
/// 可恢复错误与致命故障分开表达：`Fatal` 不是「更严重的错误」，它不可被普通
/// `catch` 恢复，也不执行释放计划，因此不能与 `Error` 共用一个分支。生产入口
/// 将未捕获的两类故障复制到 [`RunOutcome::report`]，供报告器消费；本类型不冻结
/// CLI 整数退出码。
#[derive(Debug)]
pub enum RunResult {
    /// 正常结束。
    Success,
    /// 以可恢复错误结束。
    Error(XiaoError),
    /// 以不可恢复故障结束。
    Fatal(FatalError),
}

impl RunResult {
    /// 判断运行是否成功。
    #[must_use]
    pub const fn is_success(&self) -> bool {
        matches!(self, Self::Success)
    }

    /// 返回可恢复错误码。
    #[must_use]
    pub fn error_code(&self) -> Option<&str> {
        match self {
            Self::Error(error) => Some(error.code()),
            Self::Fatal(error) => Some(error.code()),
            Self::Success => None,
        }
    }
}

/// 运行参数。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct VmOptions {
    /// 最大调用深度；超限产生不可恢复的栈溢出故障。
    pub max_call_depth: usize,
}

impl VmOptions {
    /// 创建默认参数。
    #[must_use]
    pub const fn new() -> Self {
        Self {
            max_call_depth: DEFAULT_MAX_CALL_DEPTH,
        }
    }

    /// 验证规范化参数，拒绝零值和超过实现上限的调用深度。
    pub fn validate(&self) -> Result<(), VmOptionsError> {
        if self.max_call_depth == 0 {
            return Err(VmOptionsError {
                field: "max_call_depth",
                value: self.max_call_depth,
                reason: "必须大于零",
            });
        }
        if self.max_call_depth > MAX_MAX_CALL_DEPTH {
            return Err(VmOptionsError {
                field: "max_call_depth",
                value: self.max_call_depth,
                reason: "超过实现上限",
            });
        }
        Ok(())
    }
}

impl Default for VmOptions {
    /// 使用默认调用深度上限。
    fn default() -> Self {
        Self::new()
    }
}

/// 一次生产 VM 执行请求。
///
/// 请求同时携带前端 IR 和对应 TAC，生产入口据此在创建 VM 前执行唯一的
/// `verify_for_execution` 结构关卡。模块名、源码名和事件容量属于运行上下文，
/// 不写入冻结的 TAC 或字节码布局。
#[derive(Debug)]
pub struct RunRequest<'a> {
    /// 已通过前端验证的输入 IR。
    pub ir: &'a IrProgram,
    /// 与 [`Self::ir`] 一一对应的三地址程序。
    pub program: &'a TacProgram,
    /// 规范化 VM 参数。
    pub options: VmOptions,
    /// 供事件和调用栈使用的逻辑模块身份。
    pub module_name: String,
    /// 供调用栈使用的源码路径或稳定内存来源名。
    pub source_name: String,
    /// 生产事件接收器容量；超限事件按丢弃新事件策略记账。
    pub event_capacity: usize,
}

impl<'a> RunRequest<'a> {
    /// 创建使用默认模块名、源码名和事件容量的请求。
    #[must_use]
    pub fn new(ir: &'a IrProgram, program: &'a TacProgram) -> Self {
        Self {
            ir,
            program,
            options: VmOptions::new(),
            module_name: "main".to_owned(),
            source_name: "<memory>".to_owned(),
            event_capacity: DEFAULT_EVENT_CAPACITY,
        }
    }

    /// 替换运行参数。
    #[must_use]
    pub const fn with_options(mut self, options: VmOptions) -> Self {
        self.options = options;
        self
    }

    /// 替换逻辑模块名。
    #[must_use]
    pub fn with_module_name(mut self, module_name: impl Into<String>) -> Self {
        self.module_name = module_name.into();
        self
    }

    /// 替换源码路径或内存来源名。
    #[must_use]
    pub fn with_source_name(mut self, source_name: impl Into<String>) -> Self {
        self.source_name = source_name.into();
        self
    }

    /// 替换事件接收器容量。
    #[must_use]
    pub const fn with_event_capacity(mut self, event_capacity: usize) -> Self {
        self.event_capacity = event_capacity;
        self
    }

    /// 验证请求自身的规范化字段。
    pub fn validate(&self) -> Result<(), RunRequestError> {
        self.options.validate().map_err(|error| RunRequestError {
            code: error.code(),
            field: error.field,
            message: error.to_string(),
        })?;
        if self.event_capacity == 0 || self.event_capacity > MAX_EVENT_CAPACITY {
            return Err(RunRequestError {
                code: VM_REQUEST_CODE,
                field: "event_capacity",
                message: format!(
                    "必须位于 1..={}（收到 {}）",
                    MAX_EVENT_CAPACITY, self.event_capacity
                ),
            });
        }
        if self.module_name.trim().is_empty() {
            return Err(RunRequestError {
                code: VM_REQUEST_CODE,
                field: "module_name",
                message: "不能为空".to_owned(),
            });
        }
        if self.source_name.trim().is_empty() {
            return Err(RunRequestError {
                code: VM_REQUEST_CODE,
                field: "source_name",
                message: "不能为空".to_owned(),
            });
        }
        Ok(())
    }

    /// 返回入口模式对应的诊断跨度。
    #[must_use]
    pub(crate) fn entry_span(&self) -> xiao_ir::IrSpan {
        match self.ir.entry_mode {
            IrEntryMode::Script => self.ir.span,
            IrEntryMode::Project { span } => span,
        }
    }
}

/// 运行指标。
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct VmMetrics {
    /// 执行的指令条数。
    pub instructions: u64,
    /// 达到过的最大调用深度。
    pub max_call_depth: usize,
    /// 载体占用的历史峰值。
    pub max_stack_depth: usize,
    /// 按冻结计划释放的值的数量。
    pub releases: usize,
    /// 写入独立帧槽的次数。
    ///
    /// 它与 [`Self::stack_map_entries`] 量纲不同：这里数的是**落帧槽的次数**，
    /// 那里数的是**需要栈映射的程序点**。两者不可合并，否则三种机型在 09R3
    /// 的对比中不再可比。
    pub spill_count: u64,
    /// 需要建立栈映射的程序点数量（R1-F 的机型分界）。
    ///
    /// 栈式数跳转目标、混合式数调用点与帧尾、分类型寄存器式恒为 0；
    /// 详见 [`crate::carrier::CarrierMetrics::stack_map_entries`]。
    pub stack_map_entries: usize,
    /// 跨调用保存值的次数。
    pub call_save_count: u64,
}

/// 一次运行的完整结果。
#[derive(Debug)]
pub struct RunOutcome {
    /// 结构化结果。
    pub result: RunResult,
    /// 入口显式返回的值；脚本和 `[main]` 工程入口遵循同一规则，没有显式返回值时为 `None`。
    ///
    /// 生产入口与研究兼容入口都保留这一结构化观察面。可恢复错误或致命故障结束时
    /// 该字段始终为 `None`。
    pub value: Option<RuntimeValue>,
    /// 运行指标。
    pub metrics: VmMetrics,
    /// 记录到的调试事件。
    pub events: Vec<VmEvent>,
    /// 未能放入有界生产接收器的事件数量；研究入口恒为零。
    pub dropped_events: usize,
    /// 未捕获错误或致命故障的统一报告；成功时为空。
    pub report: Option<ReportRecord>,
}

/// 从执行结果建立统一报告。
fn report_for_result(result: &RunResult) -> Option<ReportRecord> {
    match result {
        RunResult::Success => None,
        RunResult::Error(error) => Some(error.report()),
        RunResult::Fatal(error) => Some(error.report()),
    }
}

/// 组装研究兼容入口的结果。
fn research_outcome(
    result: RunResult,
    value: Option<RuntimeValue>,
    metrics: VmMetrics,
    events: Vec<VmEvent>,
) -> RunOutcome {
    RunOutcome {
        report: report_for_result(&result),
        result,
        value,
        metrics,
        events,
        dropped_events: 0,
    }
}

/// 将验证错误转换为不可恢复的损坏产物故障。
fn verification_fatal(error: &TacVerificationError, entry_span: xiao_ir::IrSpan) -> FatalError {
    let location = xiao_source::SourceSpan::new(entry_span.start, entry_span.end);
    let mut fatal = FatalError::corrupt_artifact(format!("生产执行前验证失败：{}", error))
        .with_param(
            "verification_code",
            DiagnosticParam::Text(error.code.to_owned()),
        )
        .with_param(
            "verification_path",
            DiagnosticParam::Text(error.path.clone()),
        )
        .with_context(
            "verification_message",
            DiagnosticParam::Text(error.message.clone()),
        );
    if let Some(location) = location {
        fatal = fatal.with_location(location);
    }
    fatal
}

/// 将请求字段错误转换为不可恢复的参数故障。
fn request_fatal(error: &RunRequestError, entry_span: xiao_ir::IrSpan) -> FatalError {
    let location = xiao_source::SourceSpan::new(entry_span.start, entry_span.end);
    let mut fatal = FatalError::runtime_invariant(format!("生产运行请求无效：{}", error.message))
        .with_param("request_code", DiagnosticParam::Text(error.code.to_owned()))
        .with_param(
            "request_field",
            DiagnosticParam::Text(error.field.to_owned()),
        )
        .with_context(
            "request_message",
            DiagnosticParam::Text(error.message.clone()),
        );
    if let Some(location) = location {
        fatal = fatal.with_location(location);
    }
    fatal
}

/// 用栈式载体运行一份三地址产物并记录全部事件。
#[must_use]
pub fn run(program: &xiao_bytecode::TacProgram, options: VmOptions) -> RunOutcome {
    run_with::<StackCarrier>(program, options)
}

/// 使用指定静态载体运行一份三地址产物并记录全部事件。
#[must_use]
pub fn run_with<C: Carrier>(program: &xiao_bytecode::TacProgram, options: VmOptions) -> RunOutcome {
    let mut vm = Vm::<C, RecordingSink>::new(program, options, RecordingSink::new());
    let (result, value) = vm.run_with_value();
    let metrics = vm.metrics();
    research_outcome(result, value, metrics, vm.into_sink().into_events())
}

/// 使用位置实参和指定载体运行一份三地址产物。
///
/// 该入口只供研究 VM 的动态边界向量注入入口形参；实参按入口函数的形参
/// 声明顺序绑定，不承诺生产 VM 的脚本调用 ABI。
#[must_use]
pub fn run_with_values<C: Carrier>(
    program: &xiao_bytecode::TacProgram,
    options: VmOptions,
    arguments: &[RuntimeValue],
) -> RunOutcome {
    let bound_arguments = arguments
        .iter()
        .cloned()
        .map(|value| BoundArgument {
            keyword: None,
            value,
        })
        .collect::<Vec<_>>();
    let mut vm = Vm::<C, RecordingSink>::new(program, options, RecordingSink::new());
    let (result, value) = vm.run_with_arguments(&bound_arguments);
    let metrics = vm.metrics();
    research_outcome(result, value, metrics, vm.into_sink().into_events())
}

/// 使用指定种子的栈式载体运行一份三地址产物。
#[must_use]
pub fn run_with_seed(
    program: &xiao_bytecode::TacProgram,
    options: VmOptions,
    seed: u128,
) -> RunOutcome {
    let mut vm = Vm::<StackCarrier, RecordingSink>::new_with_seed(
        program,
        options,
        RecordingSink::new(),
        seed,
    );
    let (result, value) = vm.run_with_value();
    let metrics = vm.metrics();
    research_outcome(result, value, metrics, vm.into_sink().into_events())
}

/// 使用指定种子的任意静态载体运行一份三地址产物。
#[must_use]
pub fn run_with_machine_seed<C: Carrier>(
    program: &xiao_bytecode::TacProgram,
    options: VmOptions,
    seed: u128,
) -> RunOutcome {
    let mut vm =
        Vm::<C, RecordingSink>::new_with_seed(program, options, RecordingSink::new(), seed);
    let (result, value) = vm.run_with_value();
    let metrics = vm.metrics();
    research_outcome(result, value, metrics, vm.into_sink().into_events())
}

/// 用分类型寄存器载体运行一份三地址产物。
#[must_use]
pub fn run_register(program: &xiao_bytecode::TacProgram, options: VmOptions) -> RunOutcome {
    run_with::<RegisterCarrier>(program, options)
}

/// 用混合式窗口/求值栈载体运行一份三地址产物。
#[must_use]
pub fn run_hybrid(program: &xiao_bytecode::TacProgram, options: VmOptions) -> RunOutcome {
    run_with::<HybridCarrier>(program, options)
}

/// 使用生产契约执行一份前端 IR 对应的 TAC。
///
/// 该入口固定使用栈式载体，并在创建解释器前无条件验证 IR/TAC 对应关系。
/// 失败会返回带验证路径和源码入口位置的 `Fatal`，不会进入 VM 内部循环；
/// 未捕获的普通错误和 Fatal 都通过 [`RunOutcome::report`] 提供统一报告。
#[must_use]
pub fn run_request(request: &RunRequest<'_>) -> RunOutcome {
    let mut sink = BoundedSink::new(request.event_capacity);
    let entry_span = request.entry_span();
    if let Err(error) = verify_for_execution(request.ir, request.program) {
        let fatal = verification_fatal(&error, entry_span);
        sink.record(VmEvent::FatalRaised {
            code: fatal.code().to_owned(),
        });
        return production_outcome(RunResult::Fatal(fatal), None, VmMetrics::default(), sink);
    }
    if let Err(error) = request.validate() {
        let fatal = request_fatal(&error, entry_span);
        sink.record(VmEvent::FatalRaised {
            code: fatal.code().to_owned(),
        });
        return production_outcome(RunResult::Fatal(fatal), None, VmMetrics::default(), sink);
    }

    let metadata = VmMetadata::new(
        request.module_name.clone(),
        request.source_name.clone(),
        Some(entry_span),
    );
    let mut vm = Vm::<StackCarrier, BoundedSink>::new_with_metadata(
        request.program,
        request.options,
        sink,
        metadata,
    );
    let (result, value) = vm.run_with_value();
    let metrics = vm.metrics();
    sink = vm.into_sink();
    production_outcome(result, value, metrics, sink)
}

/// `run_request` 的生产入口别名，供驱动器按动作命名调用。
#[must_use]
pub fn run_production(request: &RunRequest<'_>) -> RunOutcome {
    run_request(request)
}

/// 从 IR/TAC 借用对创建默认生产请求并执行。
#[must_use]
pub fn run_checked(ir: &IrProgram, program: &TacProgram, options: VmOptions) -> RunOutcome {
    run_request(&RunRequest::new(ir, program).with_options(options))
}

/// 组装有界生产接收器的结果，并把保留事件转为值对象。
fn production_outcome(
    result: RunResult,
    value: Option<RuntimeValue>,
    metrics: VmMetrics,
    sink: BoundedSink,
) -> RunOutcome {
    let mut sink = sink;
    sink.record_metrics(metrics);
    let dropped_events = sink.dropped_events();
    RunOutcome {
        report: report_for_result(&result),
        result,
        value,
        metrics,
        events: sink.into_events(),
        dropped_events,
    }
}
