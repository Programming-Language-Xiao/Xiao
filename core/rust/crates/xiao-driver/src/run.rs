//! 09-B0-C 前端到生产 VM 的内部驱动器。
//!
//! 本模块只编排阶段边界：前端产出已验证的 `IrProgram`，降低器生成统一 TAC，
//! `xiao-vm::run_request` 负责执行前验证和栈式生产入口。这里不重新解析源码、
//! 不重新推断类型或生命周期，也不搬运研究基准的入口重排逻辑。

use std::fmt::{self, Display, Formatter};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::time::{Duration, Instant};

use xiao_bytecode::{TacProgram, lower_program};
use xiao_diagnostics::{Diagnostic, DiagnosticParam, DiagnosticParams, ReportRecord};
use xiao_vm::{
    DEFAULT_EVENT_CAPACITY, RunOutcome as VmRunOutcome, RunRequest as VmRunRequest, RunResult,
    VmEvent, VmOptions, run_request as run_vm_request,
};

use crate::frontend::{FrontendArtifact, FrontendCompiler, FrontendError, FrontendRequest};

/// 当前内部驱动器请求/结果字段的版本。
pub const DRIVER_VERSION: u32 = 1;
/// 运行在进入前端或 VM 前观察到取消时的稳定编号。
pub const DRIVER_CANCELLED_CODE: &str = "X09-DRIVER-001";
/// 运行在驱动器边界观察到超时时的稳定编号。
pub const DRIVER_TIMEOUT_CODE: &str = "X09-DRIVER-002";
/// 取消/超时控制字段自身无法建立时的稳定编号。
pub const DRIVER_CONTROL_CODE: &str = "X09-DRIVER-003";

/// 驱动器对外稳定表达的五种终局。
///
/// 这些语义值属于 B0 的库边界；第 11/X0 阶段只负责把
/// [`ExitCode::as_process_code`] 的结果映射到宿主进程，并处理 CLI 自身错误。
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ExitCode {
    /// 执行成功，包括由 Xiao 程序自身 `catch` 消费的可恢复错误。
    Success,
    /// 前端源码检查失败，未产生可执行产物。
    SourceRejected,
    /// 产物、请求或控制边界拒绝执行，包括取消和超时。
    ArtifactRejected,
    /// 已进入 VM，但以未捕获的可恢复运行时错误结束。
    RuntimeError,
    /// 已进入 VM，但以不可恢复的致命故障结束。
    Fatal,
}

impl ExitCode {
    /// 返回冻结的宿主进程退出码（`0..=4`）。
    #[must_use]
    pub const fn as_process_code(self) -> u8 {
        match self {
            Self::Success => 0,
            Self::SourceRejected => 1,
            Self::ArtifactRejected => 2,
            Self::RuntimeError => 3,
            Self::Fatal => 4,
        }
    }
}

/// 可跨线程共享的取消信号。
///
/// 本批只在驱动器阶段边界采样该信号。VM 指令循环尚未接入中途检查点，
/// 具体检查点作为 `B0-C-CANCEL-001` 债项留给 11/X0 前的后续批次。
#[derive(Clone, Debug, Default)]
pub struct CancellationToken {
    cancelled: Arc<AtomicBool>,
}

impl CancellationToken {
    /// 创建一个未取消的信号。
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// 设置取消标记；重复设置不会改变语义。
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::Release);
    }

    /// 查询当前是否已经取消。
    #[must_use]
    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Acquire)
    }
}

/// 驱动器层的取消与超时控制字段。
///
/// `timeout` 从一次驱动调用开始计时。控制只在前端完成、降低完成和 VM 调用
/// 前后采样；它不会伪装成已经能中断正在执行的 VM。
#[derive(Clone, Debug, Default)]
pub struct RunControl {
    cancellation: Option<CancellationToken>,
    timeout: Option<Duration>,
}

impl RunControl {
    /// 创建不带取消和超时的控制对象。
    #[must_use]
    pub const fn new() -> Self {
        Self {
            cancellation: None,
            timeout: None,
        }
    }

    /// 设置共享取消信号。
    #[must_use]
    pub fn with_cancellation(mut self, token: CancellationToken) -> Self {
        self.cancellation = Some(token);
        self
    }

    /// 设置从驱动调用开始计时的超时。
    #[must_use]
    pub const fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// 返回共享取消信号（若配置）。
    #[must_use]
    pub const fn cancellation(&self) -> Option<&CancellationToken> {
        self.cancellation.as_ref()
    }

    /// 返回超时配置（若配置）。
    #[must_use]
    pub const fn timeout(&self) -> Option<Duration> {
        self.timeout
    }
}

/// 一次前端到 VM 的内部运行请求。
///
/// 请求没有入口函数字段：脚本和 `[main]` 工程模式都由降低器固定到函数 0，
/// 生产 VM 的入口 ABI 由 `xiao-vm::RunRequest` 消费。模块名和源码名可显式提供，
/// 缺省时从源文件路径推导或使用稳定的内存来源名。
#[derive(Clone, Debug)]
pub struct DriverRequest {
    /// 前端源码及其已规范化的上下文。
    pub frontend: FrontendRequest,
    /// 传给生产 VM 的规范化参数。
    pub options: VmOptions,
    /// 事件和调用栈使用的逻辑模块名；为空时由驱动器推导。
    pub module_name: Option<String>,
    /// 事件和调用栈使用的源码名；为空时由驱动器推导。
    pub source_name: Option<String>,
    /// 生产事件接收器容量。
    pub event_capacity: usize,
    /// 驱动器层取消/超时控制。
    pub control: RunControl,
}

impl DriverRequest {
    /// 从一份前端请求创建默认驱动请求。
    #[must_use]
    pub fn new(frontend: FrontendRequest) -> Self {
        Self {
            frontend,
            options: VmOptions::default(),
            module_name: None,
            source_name: None,
            event_capacity: DEFAULT_EVENT_CAPACITY,
            control: RunControl::default(),
        }
    }

    /// 替换 VM 运行参数。
    #[must_use]
    pub const fn with_options(mut self, options: VmOptions) -> Self {
        self.options = options;
        self
    }

    /// 设置逻辑模块名。
    #[must_use]
    pub fn with_module_name(mut self, module_name: impl Into<String>) -> Self {
        self.module_name = Some(module_name.into());
        self
    }

    /// 设置源码路径或内存来源名。
    #[must_use]
    pub fn with_source_name(mut self, source_name: impl Into<String>) -> Self {
        self.source_name = Some(source_name.into());
        self
    }

    /// 设置生产事件容量。
    #[must_use]
    pub const fn with_event_capacity(mut self, event_capacity: usize) -> Self {
        self.event_capacity = event_capacity;
        self
    }

    /// 设置完整的取消/超时控制对象。
    #[must_use]
    pub fn with_control(mut self, control: RunControl) -> Self {
        self.control = control;
        self
    }

    /// 设置取消信号。
    #[must_use]
    pub fn with_cancellation(mut self, token: CancellationToken) -> Self {
        self.control = self.control.with_cancellation(token);
        self
    }

    /// 设置超时。
    #[must_use]
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.control = self.control.with_timeout(timeout);
        self
    }

    /// 推导模块名；不参与语言语义或入口选择。
    fn resolved_module_name(&self) -> String {
        self.module_name.clone().unwrap_or_else(|| {
            self.frontend
                .source_path
                .as_ref()
                .and_then(|path| path.file_stem())
                .map(|stem| stem.to_string_lossy().into_owned())
                .filter(|name| !name.is_empty())
                .unwrap_or_else(|| "main".to_owned())
        })
    }

    /// 推导源码名；内存请求使用稳定占位名。
    fn resolved_source_name(&self) -> String {
        self.source_name.clone().unwrap_or_else(|| {
            self.frontend
                .source_path
                .as_ref()
                .map(|path| path.display().to_string())
                .unwrap_or_else(|| "<memory>".to_owned())
        })
    }
}

/// 驱动器拒绝发生的阶段。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DriverPhase {
    /// 取消或超时控制边界。
    Control,
    /// TAC/IR 执行前验证边界。
    Verification,
    /// VM 请求字段验证边界。
    Request,
}

/// 前端成功后、VM 执行前被拒绝的结构化错误。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DriverError {
    phase: DriverPhase,
    code: String,
    path: Option<String>,
    message: String,
    report: Option<ReportRecord>,
}

impl DriverError {
    /// 返回错误所属阶段。
    #[must_use]
    pub const fn phase(&self) -> DriverPhase {
        self.phase
    }

    /// 返回稳定机器编号。
    #[must_use]
    pub fn code(&self) -> &str {
        &self.code
    }

    /// 返回结构化字段路径（若有）。
    #[must_use]
    pub fn path(&self) -> Option<&str> {
        self.path.as_deref()
    }

    /// 返回开发者原因摘要。
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }

    /// 返回 VM 产生的统一报告（控制边界错误没有 VM 报告）。
    #[must_use]
    pub fn report(&self) -> Option<&ReportRecord> {
        self.report.as_ref()
    }

    /// 从 B0-B 的结构化运行报告识别执行前拒绝。
    fn from_vm_outcome(outcome: &VmRunOutcome) -> Option<Self> {
        let report = outcome.report.as_ref()?;
        let (phase, code_key, path_key, message_key) =
            if report.params.contains_key("verification_code") {
                (
                    DriverPhase::Verification,
                    "verification_code",
                    "verification_path",
                    "verification_message",
                )
            } else if report.params.contains_key("request_code") {
                (
                    DriverPhase::Request,
                    "request_code",
                    "request_field",
                    "request_message",
                )
            } else {
                return None;
            };
        let code = text_param(&report.params, code_key).unwrap_or_else(|| report.code.clone());
        let path = text_param(&report.params, path_key);
        let message =
            text_param(&report.context, message_key).unwrap_or_else(|| report.message.clone());
        Some(Self {
            phase,
            code,
            path,
            message,
            report: Some(report.clone()),
        })
    }

    /// 创建取消/超时控制错误。
    fn control(code: &str, message: &str) -> Self {
        Self {
            phase: DriverPhase::Control,
            code: code.to_owned(),
            path: None,
            message: message.to_owned(),
            report: None,
        }
    }
}

impl Display for DriverError {
    /// 输出阶段、稳定编号和原因，不供程序逻辑解析。
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        if let Some(path) = &self.path {
            write!(formatter, "{} at {}: {}", self.code, path, self.message)
        } else {
            write!(formatter, "{}: {}", self.code, self.message)
        }
    }
}

impl std::error::Error for DriverError {}

/// 一次已经进入 VM 的执行结果及前端非错误诊断。
#[derive(Debug)]
pub struct DriverExecution {
    /// B0-B 生产 VM 的完整结构化结果。
    pub outcome: VmRunOutcome,
    /// 前端成功阶段保留的警告/信息诊断。
    pub diagnostics: Vec<Diagnostic>,
}

impl DriverExecution {
    /// 返回 VM 结果的只读视图。
    #[must_use]
    pub const fn run_outcome(&self) -> &VmRunOutcome {
        &self.outcome
    }

    /// 返回前端保留诊断的只读视图。
    #[must_use]
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }

    /// 返回 VM 事件的只读视图。
    #[must_use]
    pub fn events(&self) -> &[VmEvent] {
        &self.outcome.events
    }

    /// 返回 VM 的统一报告（成功时为空）。
    #[must_use]
    pub fn report(&self) -> Option<&ReportRecord> {
        self.outcome.report.as_ref()
    }
}

/// 覆盖前端失败、执行前拒绝和已执行结果的单一全链结果类型。
///
/// 选择一个枚举而不是 `Result<RunOutcome, DriverError>`，是因为前端失败本身
/// 不是一个可以压缩成单个错误的字符串：它必须保留完整诊断列表；同时执行后的
/// `Error`/`Fatal` 仍需保留 VM 事件、指标和报告。调用方只需在一个位置区分三段。
#[derive(Debug)]
pub enum DriverOutcome {
    /// 前端失败，没有产生可消费 IR。
    Frontend(FrontendError),
    /// 已有前端产物，但降低/验证/请求控制边界拒绝执行。
    Rejected(DriverError),
    /// 已进入 VM；其中的 `RunResult` 继续区分成功、可恢复错误和致命故障。
    Executed(DriverExecution),
}

impl DriverOutcome {
    /// 从结构化驱动结果派生稳定退出语义。
    ///
    /// 派生只读取 `DriverOutcome` 的阶段和已执行结果的 `RunResult` 分支，
    /// 不读取诊断编号、消息文本或本地化展示内容。被 `catch` 消费的错误
    /// 会使 VM 返回 `RunResult::Success`，因此映射为 [`ExitCode::Success`]。
    #[must_use]
    pub fn exit_code(&self) -> ExitCode {
        match self {
            Self::Frontend(_error) => ExitCode::SourceRejected,
            Self::Rejected(_error) => ExitCode::ArtifactRejected,
            Self::Executed(execution) => match &execution.outcome.result {
                RunResult::Success => ExitCode::Success,
                RunResult::Error(_error) => ExitCode::RuntimeError,
                RunResult::Fatal(_error) => ExitCode::Fatal,
            },
        }
    }

    /// 判断是否是成功执行。
    #[must_use]
    pub fn is_success(&self) -> bool {
        matches!(self, Self::Executed(execution) if execution.outcome.result.is_success())
    }

    /// 返回当前结果的稳定错误编号；成功时为空。
    #[must_use]
    pub fn code(&self) -> Option<&str> {
        match self {
            Self::Frontend(error) => error
                .diagnostics()
                .iter()
                .find(|diagnostic| diagnostic.is_error())
                .or_else(|| error.diagnostics().first())
                .map(|diagnostic| diagnostic.code()),
            Self::Rejected(error) => Some(error.code()),
            Self::Executed(execution) => execution.outcome.result.error_code(),
        }
    }

    /// 返回统一报告；前端诊断和控制边界错误没有 VM 报告。
    #[must_use]
    pub fn report(&self) -> Option<&ReportRecord> {
        match self {
            Self::Frontend(_) => None,
            Self::Rejected(error) => error.report(),
            Self::Executed(execution) => execution.report(),
        }
    }

    /// 返回已执行结果的只读视图。
    #[must_use]
    pub fn as_executed(&self) -> Option<&DriverExecution> {
        match self {
            Self::Executed(execution) => Some(execution),
            Self::Frontend(_) | Self::Rejected(_) => None,
        }
    }
}

/// 前端到 VM 的无状态内部驱动器。
#[derive(Clone, Copy, Debug, Default)]
pub struct FrontendVmDriver;

impl FrontendVmDriver {
    /// 创建一个无状态驱动器。
    #[must_use]
    pub const fn new() -> Self {
        Self
    }

    /// 编译并执行一次内部运行请求。
    #[must_use]
    pub fn run(&self, request: &DriverRequest) -> DriverOutcome {
        let control = match ControlWindow::start(&request.control) {
            Ok(control) => control,
            Err(error) => return DriverOutcome::Rejected(*error),
        };
        if let Some(error) = control.check() {
            return DriverOutcome::Rejected(error);
        }
        let artifact = match FrontendCompiler::new().compile(&request.frontend) {
            Ok(artifact) => artifact,
            Err(error) => return DriverOutcome::Frontend(error),
        };
        if let Some(error) = control.check() {
            return DriverOutcome::Rejected(error);
        }
        self.run_artifact_with_control(&artifact, request, &control)
    }

    /// 执行一份已经由前端产出的 IR，不重新解析或重新推断语义。
    #[must_use]
    pub fn run_artifact(
        &self,
        artifact: &FrontendArtifact,
        request: &DriverRequest,
    ) -> DriverOutcome {
        let control = match ControlWindow::start(&request.control) {
            Ok(control) => control,
            Err(error) => return DriverOutcome::Rejected(*error),
        };
        if let Some(error) = control.check() {
            return DriverOutcome::Rejected(error);
        }
        self.run_artifact_with_control(artifact, request, &control)
    }

    /// 在同一控制窗口内降低并执行产物。
    fn run_artifact_with_control(
        &self,
        artifact: &FrontendArtifact,
        request: &DriverRequest,
        control: &ControlWindow,
    ) -> DriverOutcome {
        let ir = artifact.ir();
        let program = lower_program(ir);
        if let Some(error) = control.check() {
            return DriverOutcome::Rejected(error);
        }
        self.execute_program(
            ir,
            artifact.diagnostics().to_vec(),
            program,
            request,
            control,
        )
    }

    /// 交给 B0-B 生产入口；该函数不改变入口函数或载体。
    fn execute_program(
        &self,
        ir: &xiao_ir::IrProgram,
        diagnostics: Vec<Diagnostic>,
        program: TacProgram,
        request: &DriverRequest,
        control: &ControlWindow,
    ) -> DriverOutcome {
        if let Some(error) = control.check() {
            return DriverOutcome::Rejected(error);
        }
        let vm_request = VmRunRequest::new(ir, &program)
            .with_options(request.options)
            .with_module_name(request.resolved_module_name())
            .with_source_name(request.resolved_source_name())
            .with_event_capacity(request.event_capacity);
        let outcome = run_vm_request(&vm_request);
        // 方案 A 仍需在 VM 返回边界采样；VM 已经结束后发现控制信号时，控制结果
        // 优先于已完成的 VM 结果，避免把超时/取消伪装成成功。
        if let Some(error) = control.check() {
            return DriverOutcome::Rejected(error);
        }
        if let Some(error) = DriverError::from_vm_outcome(&outcome) {
            return DriverOutcome::Rejected(error);
        }
        DriverOutcome::Executed(DriverExecution {
            outcome,
            diagnostics,
        })
    }
}

/// 使用无状态驱动器编译并执行一次请求。
#[must_use]
pub fn run(request: &DriverRequest) -> DriverOutcome {
    FrontendVmDriver::new().run(request)
}

/// `run` 的语义别名，供编排代码按请求动作命名。
#[must_use]
pub fn run_request(request: &DriverRequest) -> DriverOutcome {
    run(request)
}

/// 驱动器一次调用的控制窗口。
struct ControlWindow {
    cancellation: Option<CancellationToken>,
    deadline: Option<Instant>,
}

impl ControlWindow {
    /// 从运行控制字段建立一次调用的截止时间窗口。
    fn start(control: &RunControl) -> Result<Self, Box<DriverError>> {
        let deadline = match control.timeout {
            Some(timeout) => Some(Instant::now().checked_add(timeout).ok_or_else(|| {
                Box::new(DriverError::control(
                    DRIVER_CONTROL_CODE,
                    "超时期限超出宿主时钟可表示范围",
                ))
            })?),
            None => None,
        };
        Ok(Self {
            cancellation: control.cancellation.clone(),
            deadline,
        })
    }

    /// 在当前驱动器阶段边界采样取消和超时状态。
    fn check(&self) -> Option<DriverError> {
        if self
            .cancellation
            .as_ref()
            .is_some_and(CancellationToken::is_cancelled)
        {
            return Some(DriverError::control(
                DRIVER_CANCELLED_CODE,
                "运行在驱动器边界被取消",
            ));
        }
        if self
            .deadline
            .is_some_and(|deadline| Instant::now() >= deadline)
        {
            return Some(DriverError::control(
                DRIVER_TIMEOUT_CODE,
                "运行在驱动器边界超过超时期限",
            ));
        }
        None
    }
}

/// 从诊断参数表读取不参与本地化的文本字段。
fn text_param(params: &DiagnosticParams, key: &str) -> Option<String> {
    match params.get(key) {
        Some(DiagnosticParam::Text(value)) => Some(value.clone()),
        Some(DiagnosticParam::Integer(_) | DiagnosticParam::Boolean(_)) | None => None,
    }
}

/// 驱动器内部阶段边界和结构化拒绝的回归夹具。
#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use xiao_bytecode::{TAC_INTERNAL_CONSISTENCY_CODE, lower_program};

    /// 构造一份使用稳定内存来源名的驱动请求。
    fn request(source: &str) -> DriverRequest {
        DriverRequest::new(FrontendRequest::from_text(source))
    }

    #[test]
    /// 真实源码应直接经过前端、降低和生产 VM，且没有入口覆盖参数。
    fn runs_frontend_to_vm_without_an_entry_override() {
        let outcome = run(&request("value = 1 + 2\n"));
        assert!(outcome.is_success());
        assert!(matches!(outcome, DriverOutcome::Executed(_)));
    }

    #[test]
    /// 前端错误应保留结构化诊断，并且不产生 VM 报告。
    fn frontend_failure_keeps_structured_diagnostics() {
        let outcome = run(&request("if 1\n    value = 1\n"));
        assert!(matches!(outcome, DriverOutcome::Frontend(_)));
        assert!(outcome.code().is_some());
        assert!(outcome.report().is_none());
    }

    #[test]
    /// 未降低 TAC 应在生产入口前被识别为内部一致性拒绝。
    fn corrupted_tac_is_rejected_before_execution() {
        let request = request("value = 1\n");
        let artifact = FrontendCompiler::new()
            .compile(&request.frontend)
            .expect("前端应成功");
        let mut program = lower_program(artifact.ir());
        program.unsupported.push("测试注入".to_owned());
        let control = ControlWindow::start(&request.control).expect("控制窗口");
        let outcome = FrontendVmDriver::new().execute_program(
            artifact.ir(),
            artifact.diagnostics().to_vec(),
            program,
            &request,
            &control,
        );
        let DriverOutcome::Rejected(error) = outcome else {
            panic!("损坏 TAC 必须被拒绝");
        };
        assert_eq!(error.phase(), DriverPhase::Verification);
        assert_eq!(error.code(), TAC_INTERNAL_CONSISTENCY_CODE);
        assert!(error.report().is_some());
    }

    #[test]
    /// 取消和零期限超时应返回稳定的驱动器控制错误。
    fn cancellation_and_zero_timeout_are_structured_control_errors() {
        let token = CancellationToken::new();
        token.cancel();
        let cancelled = run(&request("value = 1\n").with_cancellation(token));
        assert_eq!(cancelled.code(), Some(DRIVER_CANCELLED_CODE));
        assert!(matches!(cancelled, DriverOutcome::Rejected(_)));

        let timed_out = run(&request("value = 1\n").with_timeout(Duration::ZERO));
        assert_eq!(timed_out.code(), Some(DRIVER_TIMEOUT_CODE));
        assert!(matches!(timed_out, DriverOutcome::Rejected(_)));
    }
}
