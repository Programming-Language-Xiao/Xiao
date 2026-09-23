//! X0-A 子进程协议、长度前缀帧和 Rust 核心入口。
//!
//! 协议层只负责传输、版本协商和结果映射；源码解析、类型检查、生命周期分析、
//! 字节码降低、VM 执行和 LLVM 构建仍分别由已有驱动器负责。帧的长度字段是 8 字节
//! 大端无符号整数，只计算 UTF-8 JSON 负载；解码器在分配前检查 16 MiB 上限。

mod frame;
mod message;
mod request;

pub use frame::{
    FRAME_ERROR_CODE, FRAME_LENGTH_BYTES, FrameError, MAX_FRAME_BYTES, decode_frame, encode_frame,
    read_frame, write_frame,
};
pub use message::*;
pub use request::*;
use std::collections::BTreeMap;
use std::fs;
use std::io::{self, BufReader, BufWriter, Read, Write};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use serde_json::{Value, json};
use xiao_codegen_llvm::{CodegenOptions, TargetDescription, Toolchain, ToolchainVersions};
use xiao_config::{ConfigDocument, ConfigValue, parse_config_text};
use xiao_diagnostics::window::{
    DIAGNOSTIC_START_CODE, DiagnosticActivation, activation_path, write_activation,
};
use xiao_diagnostics::{
    Diagnostic, DiagnosticParam, FrameKind, ReportClass, ReportRecord, Severity, StackFrame,
};
use xiao_runtime::RuntimeValue;
use xiao_vm::{VmEvent, VmOptions};

use crate::diagnostics::{DiagnosticOptions, DiagnosticSession, start_error_details};
use crate::frontend::{FrontendContext, FrontendRequest};
use crate::native::{FrontendNativeDriver, NativeBuildRequest, NativeDriverError};
use crate::run::{
    CancellationToken, DriverError, DriverExecution, DriverOutcome, DriverPhase, DriverRequest,
    ExitCode, FrontendVmDriver,
};

/// 当前协议版本。协议字段和帧布局变化时必须递增。
/// 核心处理请求时发生 panic。
pub const CORE_CRASH_CODE: &str = "X11-PROTOCOL-003";
/// 协议请求主动取消。
pub const CANCELLED_ERROR_CODE: &str = "X11-PROTOCOL-005";
/// X0-A 尚未支持的协议操作或配置。
pub const UNSUPPORTED_OPERATION_CODE: &str = "X11-PROTOCOL-006";

/// 从流读取并解码一条请求。
pub fn read_request<R: Read>(reader: &mut R) -> Result<Option<ProtocolRequest>, FrameError> {
    let Some(payload) = read_frame(reader)? else {
        return Ok(None);
    };
    decode_frame(&payload).map(Some)
}

/// 将诊断参数映射到协议类型。
#[must_use]
pub fn protocol_param(value: &DiagnosticParam) -> ProtocolParam {
    match value {
        DiagnosticParam::Text(value) => ProtocolParam::Text(value.clone()),
        DiagnosticParam::Integer(value) => ProtocolParam::Integer(*value),
        DiagnosticParam::Boolean(value) => ProtocolParam::Boolean(*value),
    }
}

/// 转换一张诊断参数表，并保持稳定的键排序。
fn protocol_params(values: &BTreeMap<String, DiagnosticParam>) -> BTreeMap<String, ProtocolParam> {
    values
        .iter()
        .map(|(key, value)| (key.clone(), protocol_param(value)))
        .collect()
}

/// 将内部源码区间转换为协议区间。
fn protocol_span(span: Option<xiao_source::SourceSpan>) -> Option<ProtocolSpan> {
    span.map(|span| ProtocolSpan {
        start: span.start(),
        end: span.end(),
    })
}

/// 将前端诊断转换为协议诊断。
#[must_use]
pub fn protocol_diagnostic(diagnostic: &Diagnostic) -> ProtocolDiagnostic {
    ProtocolDiagnostic {
        code: diagnostic.code().to_owned(),
        message_id: diagnostic.message_id().to_owned(),
        severity: match diagnostic.severity() {
            Severity::Error => "error",
            Severity::Warning => "warning",
            Severity::Info => "info",
        }
        .to_owned(),
        span: protocol_span(diagnostic.span()),
        params: protocol_params(diagnostic.params()),
        message: diagnostic.message().to_owned(),
    }
}

/// 将统一调用栈帧转换为跨进程摘要。
fn protocol_stack_frame(frame: &StackFrame) -> ProtocolStackFrame {
    ProtocolStackFrame {
        module: frame.module.clone(),
        function: frame.function.clone(),
        source: frame.source.clone(),
        span: protocol_span(frame.span),
        backend: ProtocolBackendLocation {
            bytecode_offset: frame.backend.bytecode_offset,
            native_address: frame.backend.native_address,
            inline_depth: frame.backend.inline_depth,
        },
        kind: match frame.kind {
            FrameKind::User => "user",
            FrameKind::Runtime => "runtime",
        }
        .to_owned(),
    }
}

/// 递归转换统一错误报告及其原因链。
fn protocol_report(report: &ReportRecord) -> ProtocolReport {
    ProtocolReport {
        class: match report.class {
            ReportClass::Recoverable => "recoverable",
            ReportClass::Fatal => "fatal",
        }
        .to_owned(),
        code: report.code.clone(),
        error_id: report.error_id,
        message_id: report.message_id.clone(),
        params: protocol_params(&report.params),
        message: report.message.clone(),
        location: protocol_span(report.location),
        context: protocol_params(&report.context),
        stack: report.stack.iter().map(protocol_stack_frame).collect(),
        cause: report.cause.as_deref().map(protocol_report).map(Box::new),
        suppressed: report.suppressed.iter().map(protocol_report).collect(),
    }
}

/// 将 VM 指标和事件丢弃计数转换为协议结构。
fn protocol_metrics(metrics: xiao_vm::VmMetrics, dropped_events: usize) -> ProtocolMetrics {
    ProtocolMetrics {
        instructions: metrics.instructions,
        max_call_depth: metrics.max_call_depth,
        max_stack_depth: metrics.max_stack_depth,
        releases: metrics.releases,
        spill_count: metrics.spill_count,
        stack_map_entries: metrics.stack_map_entries,
        call_save_count: metrics.call_save_count,
        dropped_events,
    }
}

/// 将 VM 事件转换为稳定类型名和机器字段。
fn protocol_event(event: &VmEvent) -> ProtocolEvent {
    let (kind, data) = match event {
        VmEvent::ModuleLoaded { module } => ("module_loaded", json!({ "module": module })),
        VmEvent::FunctionEntered { function, depth } => (
            "function_entered",
            json!({ "function": function, "depth": depth }),
        ),
        VmEvent::FunctionReturned { function, depth } => (
            "function_returned",
            json!({ "function": function, "depth": depth }),
        ),
        VmEvent::ScopeEntered { scope } => ("scope_entered", json!({ "scope": scope })),
        VmEvent::ScopeExited { scope, exit } => {
            ("scope_exited", json!({ "scope": scope, "exit": exit }))
        }
        VmEvent::HandlerEntered { scope, handler } => (
            "handler_entered",
            json!({ "scope": scope, "handler": handler }),
        ),
        VmEvent::HandlerMatched {
            scope,
            handler,
            catch_type,
        } => (
            "handler_matched",
            json!({ "scope": scope, "handler": handler, "catch_type": catch_type }),
        ),
        VmEvent::HandlerUnmatched { scope } => ("handler_unmatched", json!({ "scope": scope })),
        VmEvent::ValueReleased {
            scope,
            exit,
            value,
            kind,
        } => (
            "value_released",
            json!({ "scope": scope, "exit": exit, "value": value, "kind": kind }),
        ),
        VmEvent::ErrorRaised { code, message_id } => (
            "error_raised",
            json!({ "code": code, "message_id": message_id }),
        ),
        VmEvent::FatalRaised { code } => ("fatal_raised", json!({ "code": code })),
        VmEvent::StackFrame {
            function,
            depth,
            return_to,
        } => (
            "stack_frame",
            json!({
                "function": function,
                "depth": depth,
                "return_to": return_to.map(|value| value.get()),
            }),
        ),
        VmEvent::BackendLocationMissing {
            function,
            block,
            instruction,
        } => (
            "backend_location_missing",
            json!({ "function": function, "block": block, "instruction": instruction }),
        ),
        VmEvent::Metrics {
            instructions,
            max_call_depth,
            max_stack_depth,
            releases,
            spill_count,
            stack_map_entries,
            call_save_count,
            dropped_events,
        } => (
            "metrics",
            json!({
                "instructions": instructions,
                "max_call_depth": max_call_depth,
                "max_stack_depth": max_stack_depth,
                "releases": releases,
                "spill_count": spill_count,
                "stack_map_entries": stack_map_entries,
                "call_save_count": call_save_count,
                "dropped_events": dropped_events,
            }),
        ),
    };
    let data = data
        .as_object()
        .map(|object| {
            object
                .iter()
                .map(|(key, value)| (key.clone(), value.clone()))
                .collect()
        })
        .unwrap_or_default();
    ProtocolEvent {
        kind: kind.to_owned(),
        data,
    }
}

/// 将入口值转换为不泄露 Runtime 句柄的稳定摘要。
fn protocol_value(value: &RuntimeValue) -> ProtocolValue {
    let kind = value.type_name();
    let text = match value {
        RuntimeValue::Int(value) => value.to_string(),
        RuntimeValue::Sint(value) => value.to_string(),
        RuntimeValue::Lint(value) | RuntimeValue::Lfloat(value) => value.clone(),
        RuntimeValue::Float(value) => value.to_string(),
        RuntimeValue::Sfloat(value) => value.to_string(),
        RuntimeValue::Bool(value) => value.to_string(),
        RuntimeValue::Str(value) => value
            .to_string()
            .unwrap_or_else(|_| "<invalid-string-handle>".to_owned()),
        RuntimeValue::None => "none".to_owned(),
        RuntimeValue::Table(_)
        | RuntimeValue::TableDropView(_)
        | RuntimeValue::Array(_)
        | RuntimeValue::Tuple(_)
        | RuntimeValue::DictTable(_)
        | RuntimeValue::DictColumn(_)
        | RuntimeValue::Set(_)
        | RuntimeValue::Error(_) => format!("<{kind}>"),
    };
    ProtocolValue { kind, value: text }
}

/// 返回冻结退出码对应的稳定语义名称。
fn exit_name(code: ExitCode) -> &'static str {
    match code {
        ExitCode::Success => "success",
        ExitCode::SourceRejected => "source_rejected",
        ExitCode::ArtifactRejected => "artifact_rejected",
        ExitCode::RuntimeError => "runtime_error",
        ExitCode::Fatal => "fatal",
    }
}

/// 创建带下一步建议和结构化字段的协议错误体。
fn protocol_error_body(
    code: impl Into<String>,
    message_id: impl Into<String>,
    message: impl Into<String>,
    phase: Option<String>,
    next_step: Option<String>,
    details: BTreeMap<String, Value>,
) -> ProtocolErrorBody {
    ProtocolErrorBody {
        code: code.into(),
        message_id: message_id.into(),
        message: message.into(),
        phase,
        next_step,
        details,
    }
}

/// 将内部协议验证错误转换为跨语言错误体。
fn protocol_error_from_error(error: &ProtocolError) -> ProtocolErrorBody {
    let mut details = BTreeMap::new();
    if let Some(field) = error.field() {
        details.insert("field".to_owned(), Value::String(field.to_owned()));
    }
    protocol_error_body(
        error.code(),
        "x11.protocol.request",
        error.message(),
        Some("protocol".to_owned()),
        Some("修正请求字段后重试".to_owned()),
        details,
    )
}

/// 校验协议版本和统一核心版本。
fn validate_versions(protocol_version: u16, core_version: u32) -> Result<(), ProtocolError> {
    if protocol_version != PROTOCOL_VERSION {
        return Err(ProtocolError::version(format!(
            "协议版本不兼容：需要 {}，收到 {}",
            PROTOCOL_VERSION, protocol_version
        )));
    }
    if core_version != CORE_VERSION {
        return Err(ProtocolError::version(format!(
            "核心版本不兼容：需要 {}，收到 {}",
            CORE_VERSION, core_version
        )));
    }
    Ok(())
}

/// 将协议源码字段转换为既有前端请求。
fn frontend_request(
    source: &SourceIdentity,
    language_version: &str,
    target: &ProtocolTarget,
) -> FrontendRequest {
    let mut context = FrontendContext::host();
    context.language_version = language_version.to_owned();
    context.target = target.triple.clone();
    let request = match &source.path {
        Some(path) => FrontendRequest::from_text_at(source.text.clone(), path.clone()),
        None => FrontendRequest::from_text(source.text.clone()),
    };
    request.with_context(context)
}

/// 校验源码和模块身份的最小边界。
fn validate_source(source: &SourceIdentity) -> Result<(), ProtocolError> {
    if source.module.trim().is_empty() {
        return Err(ProtocolError::request("source.module", "模块名不能为空"));
    }
    Ok(())
}

/// 校验目标字段而不复制 LLVM 目标语义。
fn validate_target(target: &ProtocolTarget) -> Result<(), ProtocolError> {
    target.to_target().map(|_| ())
}

/// 将协议 VM 参数交给生产 VM 自身的范围校验。
fn run_options(
    options: &RunOptions,
) -> Result<(VmOptions, usize, Option<Duration>), ProtocolError> {
    let vm_options = VmOptions {
        max_call_depth: options.max_call_depth,
    };
    vm_options
        .validate()
        .map_err(|error| ProtocolError::request("options.max_call_depth", error.to_string()))?;
    if options.event_capacity == 0 || options.event_capacity > xiao_vm::MAX_EVENT_CAPACITY {
        return Err(ProtocolError::request(
            "options.event_capacity",
            format!(
                "必须位于 1..={}（收到 {}）",
                xiao_vm::MAX_EVENT_CAPACITY,
                options.event_capacity
            ),
        ));
    }
    let timeout = options.timeout_ms.map(Duration::from_millis);
    Ok((vm_options, options.event_capacity, timeout))
}

/// 将三段驱动器结果转换为运行响应。
fn run_response(request_id: String, outcome: DriverOutcome) -> ProtocolResponse {
    let exit_code = outcome.exit_code();
    match outcome {
        DriverOutcome::Frontend(error) => ProtocolResponse::Result {
            request_id,
            operation: "run".to_owned(),
            exit_code: exit_code.as_process_code(),
            exit_name: exit_name(exit_code).to_owned(),
            diagnostics: error
                .diagnostics()
                .iter()
                .map(protocol_diagnostic)
                .collect(),
            report: None,
            events: Vec::new(),
            metrics: None,
            value: None,
            artifact: None,
        },
        DriverOutcome::Rejected(error) => rejected_response(request_id, exit_code, &error),
        DriverOutcome::Executed(execution) => executed_response(request_id, exit_code, &execution),
    }
}

/// 把协议诊断配置转换成 Runtime 会话配置。
fn diagnostic_options(config: Option<DiagnosticConfig>) -> DiagnosticOptions {
    let Some(config) = config else {
        return DiagnosticOptions::default();
    };
    DiagnosticOptions {
        terminal_level: config.terminal_level,
        file_level: config.file_level,
        log_dir: config.log_dir.map(PathBuf::from),
        log_file: config.log_file.map(PathBuf::from),
        stacktrace: config.stacktrace,
        focus: config
            .focus
            .into_iter()
            .map(|focus| crate::diagnostics::DiagnosticFocus {
                module: focus.module,
                source: focus.source,
                output: PathBuf::from(focus.output),
                level: focus.level,
                mirror: focus.mirror,
            })
            .collect(),
    }
}

/// 将 Runtime 侧诊断启动失败转换为稳定协议响应。
fn diagnostic_start_response(
    request_id: String,
    error: crate::diagnostics::DiagnosticStartError,
) -> ProtocolResponse {
    let details = start_error_details(&error);
    let code = error.code;
    let message = error.message;
    ProtocolResponse::Error {
        request_id: Some(request_id),
        error: protocol_error_body(
            code,
            "x11.diagnostics.start_failed",
            message,
            Some("diagnostic_startup".to_owned()),
            Some("安装可用终端并重试，或检查 XIAO_DIAGNOSTICS_PATH".to_owned()),
            details,
        ),
        report: None,
        exit_code: ExitCode::ArtifactRejected.as_process_code(),
    }
}

/// 在用户代码开始前建立诊断会话，执行后投递所有 VM 事件。
fn run_with_diagnostics(
    request_id: String,
    debug: bool,
    module: String,
    source: Option<String>,
    config: Option<DiagnosticConfig>,
    request: &DriverRequest,
) -> ProtocolResponse {
    let mut session = if debug {
        match DiagnosticSession::start(module.clone(), source.clone(), &diagnostic_options(config))
        {
            Ok(session) => Some(session),
            Err(error) => return diagnostic_start_response(request_id, error),
        }
    } else {
        None
    };
    let outcome = FrontendVmDriver::new().run(request);
    if let Some(mut session) = session.take() {
        if let DriverOutcome::Executed(execution) = &outcome {
            for event in execution.events() {
                session.record(event);
            }
        }
        session.finish();
    }
    run_response(request_id, outcome)
}

/// 将执行前拒绝转换为稳定协议错误。
fn rejected_response(
    request_id: String,
    exit_code: ExitCode,
    error: &DriverError,
) -> ProtocolResponse {
    let mut details = BTreeMap::new();
    details.insert("code".to_owned(), Value::String(error.code().to_owned()));
    if let Some(path) = error.path() {
        details.insert("path".to_owned(), Value::String(path.to_owned()));
    }
    ProtocolResponse::Error {
        request_id: Some(request_id),
        error: protocol_error_body(
            error.code(),
            "x11.driver.rejected",
            error.message(),
            Some(driver_phase_name(error.phase()).to_owned()),
            Some("检查源码、产物或取消状态后重试".to_owned()),
            details,
        ),
        report: error.report().map(protocol_report),
        exit_code: exit_code.as_process_code(),
    }
}

/// 返回驱动器阶段的稳定名称。
fn driver_phase_name(phase: DriverPhase) -> &'static str {
    match phase {
        DriverPhase::Control => "control",
        DriverPhase::Verification => "verification",
        DriverPhase::Request => "request",
    }
}

/// 将已进入 VM 的执行结果转换为结构化响应。
fn executed_response(
    request_id: String,
    exit_code: ExitCode,
    execution: &DriverExecution,
) -> ProtocolResponse {
    let outcome = &execution.outcome;
    let value = outcome.value.as_ref().map(protocol_value);
    ProtocolResponse::Result {
        request_id,
        operation: "run".to_owned(),
        exit_code: exit_code.as_process_code(),
        exit_name: exit_name(exit_code).to_owned(),
        diagnostics: execution
            .diagnostics()
            .iter()
            .map(protocol_diagnostic)
            .collect(),
        report: outcome.report.as_ref().map(protocol_report),
        events: outcome.events.iter().map(protocol_event).collect(),
        metrics: Some(protocol_metrics(outcome.metrics, outcome.dropped_events)),
        value,
        artifact: None,
    }
}

/// 将调用方注入的工具链字段转换为 LLVM 驱动器对象。
fn build_toolchain(
    spec: &ToolchainSpec,
    target: &TargetDescription,
) -> Result<Toolchain, ProtocolError> {
    if spec.clang.trim().is_empty() {
        return Err(ProtocolError::request(
            "toolchain.clang",
            "原生构建必须显式提供 clang 路径",
        ));
    }
    let versions = ToolchainVersions {
        clang: spec.versions.clang.clone(),
        llvm_as: spec.versions.llvm_as.clone(),
        llc: spec.versions.llc.clone(),
        rustc: spec.versions.rustc.clone(),
    };
    let mut toolchain = Toolchain::new(PathBuf::from(&spec.clang)).with_versions(versions);
    if let Some(path) = &spec.llvm_as {
        toolchain = toolchain.with_llvm_as(path);
    }
    if let Some(path) = &spec.llc {
        toolchain = toolchain.with_llc(path);
    }
    if let Some(path) = &spec.runtime_library {
        toolchain = toolchain.with_runtime_library(path);
    }
    let mut toolchain =
        toolchain.with_native_static_libraries(spec.native_static_libraries.clone());
    if toolchain.runtime_library.is_some() && toolchain.native_static_libraries.is_empty() {
        let Some(rustc) = spec.rustc.as_deref() else {
            return Err(ProtocolError::build(
                "动态 Runtime 构建需要 rustc 路径以查询 native-static-libs",
            ));
        };
        toolchain = toolchain
            .probe_native_static_libraries(rustc, target)
            .map_err(|error| ProtocolError::build(format!("无法查询 Rust 原生库清单：{error}")))?;
    }
    Ok(toolchain)
}

#[allow(clippy::too_many_arguments)]
/// 调用原生驱动器并转换构建结果或结构化后端错误。
fn build_response(
    request_id: String,
    language_version: String,
    target: ProtocolTarget,
    optimization: OptimizationConfig,
    source: SourceIdentity,
    output: String,
    llvm_ir_output: Option<String>,
    toolchain: ToolchainSpec,
    config_text: Option<String>,
    cancellation: &CancellationToken,
) -> ProtocolResponse {
    if cancellation.is_cancelled() {
        return cancelled_error_response(request_id);
    }
    if optimization.level != 0 {
        return ProtocolResponse::Error {
            request_id: Some(request_id),
            error: protocol_error_body(
                UNSUPPORTED_OPERATION_CODE,
                "x11.protocol.optimization_unavailable",
                "X0-A 只接受优化级别 0",
                Some("build".to_owned()),
                Some("使用 level=0，优化接线留给后续阶段".to_owned()),
                BTreeMap::from([("level".to_owned(), json!(optimization.level))]),
            ),
            report: None,
            exit_code: ExitCode::ArtifactRejected.as_process_code(),
        };
    }
    if output.trim().is_empty() {
        return protocol_error_response(
            Some(request_id),
            &ProtocolError::request("output", "原生构建必须提供输出路径"),
        );
    }
    // 先移除旧激活位，避免失败的普通/调试重建继续误启用上一次的诊断配置。
    let activation_file = activation_path(&output);
    if let Err(error) = fs::remove_file(&activation_file)
        && error.kind() != std::io::ErrorKind::NotFound
    {
        return ProtocolResponse::Error {
            request_id: Some(request_id),
            error: protocol_error_body(
                DIAGNOSTIC_START_CODE,
                "x11.diagnostics.activation_cleanup_failed",
                format!("无法清理旧的调试产物激活位：{error}"),
                Some("build".to_owned()),
                Some("检查产物目录权限后重试".to_owned()),
                BTreeMap::from([(
                    "path".to_owned(),
                    json!(activation_file.display().to_string()),
                )]),
            ),
            report: None,
            exit_code: ExitCode::ArtifactRejected.as_process_code(),
        };
    }
    let target_description = match target.to_target() {
        Ok(target) => target,
        Err(error) => return protocol_error_response(Some(request_id), &error),
    };
    let diagnostics_source = toolchain.diagnostics_path.clone();
    let diagnostics_path = diagnostics_source
        .as_deref()
        .map(|path| diagnostics_component_path(&output, path));
    let toolchain = match build_toolchain(&toolchain, &target_description) {
        Ok(toolchain) => toolchain,
        Err(error) => return protocol_error_response(Some(request_id), &error),
    };
    let frozen_config = match freeze_runtime_config(&output, config_text.as_deref()) {
        Ok(config) => config,
        Err(error) => return protocol_error_response(Some(request_id), &error),
    };
    let frontend = frontend_request(&source, &language_version, &target);
    let mut request = NativeBuildRequest::new(
        frontend,
        target_description.clone(),
        toolchain,
        output.clone(),
    )
    .with_codegen_options({
        let options = CodegenOptions::for_target(target_description);
        if optimization.debug {
            let Some(path) = diagnostics_path.as_deref() else {
                return protocol_error_response(
                    Some(request_id),
                    &ProtocolError::build(
                        "调试构建缺少 xiao-diagnostics 路径；请随分发包携带诊断组件",
                    ),
                );
            };
            options.with_debug_startup(path)
        } else {
            options
        }
    });
    if let Some(path) = &llvm_ir_output {
        request = request.with_llvm_ir_output(path);
    }
    if cancellation.is_cancelled() {
        return cancelled_error_response(request_id);
    }
    match FrontendNativeDriver::new().build(&request) {
        Ok(result) => {
            if cancellation.is_cancelled() {
                cleanup_native_outputs(&result.native.executable, llvm_ir_output.as_deref());
                return cancelled_error_response(request_id);
            }
            let mut staged_diagnostics_component = false;
            let diagnostics_component = if optimization.debug {
                let Some(source) = diagnostics_source.as_deref() else {
                    return protocol_error_response(
                        Some(request_id),
                        &ProtocolError::build("调试构建缺少 xiao-diagnostics 源文件路径"),
                    );
                };
                let destination = diagnostics_path
                    .as_deref()
                    .expect("调试构建已计算诊断组件目标路径");
                match stage_diagnostics_component(source, destination) {
                    Ok(staged) => staged_diagnostics_component = staged,
                    Err(error) => {
                        cleanup_native_outputs(
                            &result.native.executable,
                            llvm_ir_output.as_deref(),
                        );
                        return ProtocolResponse::Error {
                            request_id: Some(request_id),
                            error: protocol_error_body(
                                DIAGNOSTIC_START_CODE,
                                "x11.diagnostics.component_copy_failed",
                                format!("无法把 xiao-diagnostics 随产物携带：{error}"),
                                Some("build".to_owned()),
                                Some("检查诊断组件路径和产物目录权限后重试".to_owned()),
                                BTreeMap::from([
                                    ("source".to_owned(), json!(source)),
                                    ("destination".to_owned(), json!(destination)),
                                ]),
                            ),
                            report: None,
                            exit_code: ExitCode::ArtifactRejected.as_process_code(),
                        };
                    }
                }
                Some(destination.to_owned())
            } else {
                None
            };
            let diagnostic_activation = if optimization.debug {
                let activation = DiagnosticActivation {
                    format_version: 1,
                    enabled: true,
                    source_map: Some(
                        source
                            .path
                            .clone()
                            .unwrap_or_else(|| "<embedded>".to_owned()),
                    ),
                    metadata_version: 1,
                    hooks: true,
                };
                match write_activation(&result.native.executable, &activation) {
                    Ok(path) => Some(ProtocolDiagnosticActivation {
                        path: path.display().to_string(),
                        enabled: activation.enabled,
                        source_map: activation.source_map.is_some(),
                        hooks: activation.hooks,
                    }),
                    Err(error) => {
                        cleanup_native_outputs(
                            &result.native.executable,
                            llvm_ir_output.as_deref(),
                        );
                        if staged_diagnostics_component
                            && let Some(component) = &diagnostics_component
                        {
                            let _ = fs::remove_file(component);
                        }
                        return ProtocolResponse::Error {
                            request_id: Some(request_id),
                            error: protocol_error_body(
                                DIAGNOSTIC_START_CODE,
                                "x11.diagnostics.activation_write_failed",
                                format!("无法写入调试产物激活位：{error}"),
                                Some("build".to_owned()),
                                Some("检查产物目录权限后重试".to_owned()),
                                BTreeMap::new(),
                            ),
                            report: None,
                            exit_code: ExitCode::ArtifactRejected.as_process_code(),
                        };
                    }
                }
            } else {
                None
            };
            let runtime_config = match frozen_config {
                Some(config) => match write_runtime_config(&result.native.executable, &config) {
                    Ok(summary) => Some(summary),
                    Err(error) => {
                        cleanup_native_outputs(
                            &result.native.executable,
                            llvm_ir_output.as_deref(),
                        );
                        if staged_diagnostics_component
                            && let Some(component) = &diagnostics_component
                        {
                            let _ = fs::remove_file(component);
                        }
                        if let Some(activation) = &diagnostic_activation {
                            let _ = fs::remove_file(&activation.path);
                        }
                        return protocol_error_response(Some(request_id), &error);
                    }
                },
                None => {
                    let path = runtime_config_path(&result.native.executable);
                    if let Err(error) = fs::remove_file(&path)
                        && error.kind() != std::io::ErrorKind::NotFound
                    {
                        cleanup_native_outputs(
                            &result.native.executable,
                            llvm_ir_output.as_deref(),
                        );
                        if staged_diagnostics_component
                            && let Some(component) = &diagnostics_component
                        {
                            let _ = fs::remove_file(component);
                        }
                        if let Some(activation) = &diagnostic_activation {
                            let _ = fs::remove_file(&activation.path);
                        }
                        return protocol_error_response(
                            Some(request_id),
                            &ProtocolError::build(format!("无法清理旧的运行时配置：{error}")),
                        );
                    }
                    None
                }
            };
            ProtocolResponse::Result {
                request_id,
                operation: "build".to_owned(),
                exit_code: ExitCode::Success.as_process_code(),
                exit_name: exit_name(ExitCode::Success).to_owned(),
                diagnostics: result
                    .frontend
                    .diagnostics()
                    .iter()
                    .map(protocol_diagnostic)
                    .collect(),
                report: None,
                events: Vec::new(),
                metrics: None,
                value: None,
                artifact: Some(ProtocolArtifact {
                    executable: result.native.executable.display().to_string(),
                    llvm_ir_output,
                    toolchain_fingerprint: result.native.toolchain_fingerprint.to_string(),
                    uses_runtime: result.native.module.uses_runtime,
                    runtime_components: result.native.module.runtime_components,
                    diagnostic_activation,
                    diagnostics_component,
                    runtime_config,
                }),
            }
        }
        Err(NativeDriverError::Frontend(error)) => ProtocolResponse::Result {
            request_id,
            operation: "build".to_owned(),
            exit_code: ExitCode::SourceRejected.as_process_code(),
            exit_name: exit_name(ExitCode::SourceRejected).to_owned(),
            diagnostics: error
                .diagnostics()
                .iter()
                .map(protocol_diagnostic)
                .collect(),
            report: None,
            events: Vec::new(),
            metrics: None,
            value: None,
            artifact: None,
        },
        Err(NativeDriverError::Backend(error)) => ProtocolResponse::Error {
            request_id: Some(request_id),
            error: protocol_error_body(
                BUILD_ERROR_CODE,
                "x11.driver.native_build",
                error.to_string(),
                Some("build".to_owned()),
                Some("检查目标描述、输出路径和外部 LLVM 工具链".to_owned()),
                BTreeMap::new(),
            ),
            report: None,
            exit_code: ExitCode::ArtifactRejected.as_process_code(),
        },
    }
}

/// 构建时已验证的配置摘要；写文件延迟到原生链接成功之后。
struct FrozenRuntimeConfig {
    value: Value,
}

/// 计算调试组件在最终产物目录中的携带路径。
fn diagnostics_component_path(output: &str, source: &str) -> String {
    let output_path = PathBuf::from(output);
    // 协议路径可能来自另一种宿主格式（例如 Windows 上收到 `C:/...`），不能只依赖
    // 当前平台的 PathBuf 解析，否则盘符或反斜杠会被误当成文件名的一部分。
    let name = source
        .rsplit(['/', '\\'])
        .find(|part| !part.is_empty())
        .map(std::ffi::OsString::from)
        .unwrap_or_else(|| {
            if cfg!(windows) {
                std::ffi::OsString::from("xiao-diagnostics.exe")
            } else {
                std::ffi::OsString::from("xiao-diagnostics")
            }
        });
    output_path
        .parent()
        .unwrap_or_else(|| std::path::Path::new("."))
        .join(name)
        .display()
        .to_string()
}

/// 把独立诊断进程复制到原生构建目录，使调试产物不依赖 `xiao` 启动器。
fn stage_diagnostics_component(source: &str, destination: &str) -> std::io::Result<bool> {
    let source_path = PathBuf::from(source);
    let destination_path = PathBuf::from(destination);
    let same_file = source_path == destination_path
        || (source_path
            .canonicalize()
            .ok()
            .zip(destination_path.canonicalize().ok())
            .is_some_and(|(source, destination)| source == destination));
    if same_file {
        return Ok(false);
    }
    if let Some(parent) = destination_path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent)?;
    }
    let temporary = PathBuf::from(format!(
        "{}.tmp-{}",
        destination_path.display(),
        std::process::id()
    ));
    if let Err(error) = fs::copy(source_path, &temporary) {
        let _ = fs::remove_file(&temporary);
        return Err(error);
    }
    if cfg!(windows)
        && let Err(error) = fs::remove_file(&destination_path)
        && error.kind() != std::io::ErrorKind::NotFound
    {
        let _ = fs::remove_file(&temporary);
        return Err(error);
    }
    if let Err(error) = fs::rename(&temporary, &destination_path) {
        let _ = fs::remove_file(&temporary);
        return Err(error);
    }
    Ok(true)
}

/// 清理一次失败或已取消构建已经提交的原生与 LLVM 输出。
fn cleanup_native_outputs(executable: &std::path::Path, llvm_ir_output: Option<&str>) {
    let _ = fs::remove_file(executable);
    if let Some(path) = llvm_ir_output {
        let _ = fs::remove_file(path);
    }
}

/// 解析并校验 `config.xiao`，只保留静态配置树，不执行用户代码。
fn freeze_runtime_config(
    _output: &str,
    text: Option<&str>,
) -> Result<Option<FrozenRuntimeConfig>, ProtocolError> {
    let Some(text) = text else {
        return Ok(None);
    };
    let document = parse_config_text(text).map_err(|diagnostics| {
        let first = diagnostics
            .first()
            .map(|diagnostic| format!("{}: {}", diagnostic.code(), diagnostic.message()))
            .unwrap_or_else(|| "配置解析失败".to_owned());
        ProtocolError::build(format!("config.xiao 校验失败：{first}"))
    })?;
    Ok(Some(FrozenRuntimeConfig {
        value: config_document_value(&document),
    }))
}

/// 把不可执行配置模型转换为确定性 JSON 值。
fn config_document_value(document: &ConfigDocument) -> Value {
    let tables = document
        .tables
        .iter()
        .map(|(name, table)| {
            let entries = table
                .entries
                .iter()
                .map(|(key, entry)| (key.clone(), config_value(&entry.value)))
                .collect::<serde_json::Map<_, _>>();
            (name.clone(), Value::Object(entries))
        })
        .collect::<serde_json::Map<_, _>>();
    Value::Object(serde_json::Map::from_iter([
        ("format_version".to_owned(), json!(1)),
        ("cli_overrides".to_owned(), json!(true)),
        ("tables".to_owned(), Value::Object(tables)),
    ]))
}

/// 递归转换配置字面量。
fn config_value(value: &ConfigValue) -> Value {
    match value {
        ConfigValue::String(value) => Value::String(value.clone()),
        ConfigValue::Integer(value) => json!(value),
        ConfigValue::Float(value) => json!(value),
        ConfigValue::Boolean(value) => json!(value),
        ConfigValue::Array(values) => Value::Array(values.iter().map(config_value).collect()),
        ConfigValue::Dictionary(values) => Value::Object(
            values
                .iter()
                .map(|(key, value)| (key.clone(), config_value(value)))
                .collect(),
        ),
    }
}

/// 将配置摘要原子地写到可执行文件旁边。
fn write_runtime_config(
    executable: &std::path::Path,
    config: &FrozenRuntimeConfig,
) -> Result<ProtocolRuntimeConfig, ProtocolError> {
    let path = runtime_config_path(executable);
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent)
            .map_err(|error| ProtocolError::build(format!("无法创建配置目录：{error}")))?;
    }
    let bytes = serde_json::to_vec_pretty(&config.value)
        .map_err(|error| ProtocolError::build(format!("无法编码运行时配置：{error}")))?;
    let temporary = PathBuf::from(format!("{}.tmp-{}", path.display(), std::process::id()));
    fs::write(&temporary, [bytes.as_slice(), b"\n"].concat())
        .map_err(|error| ProtocolError::build(format!("无法写入运行时配置：{error}")))?;
    if cfg!(windows)
        && let Err(error) = fs::remove_file(&path)
        && error.kind() != std::io::ErrorKind::NotFound
    {
        let _ = fs::remove_file(&temporary);
        return Err(ProtocolError::build(format!("无法替换运行时配置：{error}")));
    }
    if let Err(error) = fs::rename(&temporary, &path) {
        let _ = fs::remove_file(&temporary);
        return Err(ProtocolError::build(format!("无法提交运行时配置：{error}")));
    }
    Ok(ProtocolRuntimeConfig {
        path: path.display().to_string(),
        format_version: 1,
        cli_overrides: true,
    })
}

/// 返回原生可执行文件旁的运行时配置路径。
fn runtime_config_path(executable: &std::path::Path) -> PathBuf {
    PathBuf::from(format!("{}.xiao-runtime.json", executable.display()))
}

/// 创建统一取消响应，使用 `ArtifactRejected` 进程码。
fn cancelled_error_response(request_id: String) -> ProtocolResponse {
    ProtocolResponse::Error {
        request_id: Some(request_id),
        error: protocol_error_body(
            CANCELLED_ERROR_CODE,
            "x11.protocol.cancelled",
            "请求已取消",
            Some("control".to_owned()),
            Some("重新提交请求或继续等待其他请求".to_owned()),
            BTreeMap::new(),
        ),
        report: None,
        exit_code: ExitCode::ArtifactRejected.as_process_code(),
    }
}

/// 将内部协议验证错误包装为响应。
fn protocol_error_response(request_id: Option<String>, error: &ProtocolError) -> ProtocolResponse {
    ProtocolResponse::Error {
        request_id,
        error: protocol_error_from_error(error),
        report: None,
        exit_code: ExitCode::ArtifactRejected.as_process_code(),
    }
}

/// 直接处理一条请求；适合契约测试和不需要并发取消的调用方。
pub fn dispatch(request: ProtocolRequest) -> ProtocolResponse {
    match request {
        ProtocolRequest::Hello {
            request_id,
            protocol_version,
            core_version,
        } => {
            let versions = CoreVersions::current();
            match validate_versions(protocol_version, core_version) {
                Ok(()) => ProtocolResponse::Hello {
                    request_id,
                    accepted: true,
                    protocol_version: PROTOCOL_VERSION,
                    core_version: CORE_VERSION,
                    versions,
                    capabilities: vec!["run".to_owned(), "build".to_owned(), "cancel".to_owned()],
                    error: None,
                },
                Err(error) => ProtocolResponse::Hello {
                    request_id,
                    accepted: false,
                    protocol_version: PROTOCOL_VERSION,
                    core_version: CORE_VERSION,
                    versions,
                    capabilities: Vec::new(),
                    error: Some(protocol_error_from_error(&error)),
                },
            }
        }
        ProtocolRequest::Run {
            request_id,
            protocol_version,
            core_version,
            language_version,
            runtime_version: _,
            target,
            optimization,
            source,
            options,
        } => {
            if let Err(error) = validate_versions(protocol_version, core_version) {
                return protocol_error_response(Some(request_id), &error);
            }
            if let Err(error) = validate_source(&source).and_then(|_| validate_target(&target)) {
                return protocol_error_response(Some(request_id), &error);
            }
            if optimization.level != 0 {
                return protocol_error_response(
                    Some(request_id),
                    &ProtocolError::request("optimization.level", "X0-A 只接受优化级别 0"),
                );
            }
            let (vm_options, event_capacity, timeout) = match run_options(&options) {
                Ok(value) => value,
                Err(error) => return protocol_error_response(Some(request_id), &error),
            };
            let debug = optimization.debug;
            let diagnostic_config = optimization.diagnostics.clone();
            let module_name = source.module.clone();
            let source_name = source.path.clone();
            let token = CancellationToken::new();
            let mut driver_request =
                DriverRequest::new(frontend_request(&source, &language_version, &target))
                    .with_options(vm_options)
                    .with_module_name(module_name.clone())
                    .with_event_capacity(event_capacity)
                    .with_cancellation(token);
            if let Some(path) = source_name.clone() {
                driver_request = driver_request.with_source_name(path);
            }
            if let Some(timeout) = timeout {
                driver_request = driver_request.with_timeout(timeout);
            }
            run_with_diagnostics(
                request_id,
                debug,
                module_name,
                source_name,
                diagnostic_config,
                &driver_request,
            )
        }
        ProtocolRequest::Build {
            request_id,
            protocol_version,
            core_version,
            language_version,
            runtime_version: _,
            target,
            optimization,
            source,
            output,
            llvm_ir_output,
            toolchain,
            config_text,
        } => {
            if let Err(error) = validate_versions(protocol_version, core_version) {
                return protocol_error_response(Some(request_id), &error);
            }
            if let Err(error) = validate_source(&source).and_then(|_| validate_target(&target)) {
                return protocol_error_response(Some(request_id), &error);
            }
            build_response(
                request_id,
                language_version,
                target,
                optimization,
                source,
                output,
                llvm_ir_output,
                toolchain,
                config_text,
                &CancellationToken::new(),
            )
        }
        ProtocolRequest::Cancel {
            request_id,
            protocol_version,
            core_version,
            target_request_id,
        } => {
            if let Err(error) = validate_versions(protocol_version, core_version) {
                return protocol_error_response(Some(request_id), &error);
            }
            ProtocolResponse::Cancelled {
                request_id,
                target_request_id,
                accepted: false,
                exit_code: ExitCode::ArtifactRejected.as_process_code(),
            }
        }
        ProtocolRequest::Shutdown {
            request_id,
            protocol_version,
            core_version,
        } => {
            if let Err(error) = validate_versions(protocol_version, core_version) {
                return protocol_error_response(Some(request_id), &error);
            }
            ProtocolResponse::Shutdown { request_id }
        }
    }
}

/// 服务线程共享的串行输出锁。
type SharedWriter<W> = Arc<Mutex<W>>;
/// 请求 ID 到取消令牌的登记表。
type CancellationMap = Arc<Mutex<BTreeMap<String, CancellationToken>>>;

/// 在线程安全的输出锁上写入一条响应。
fn write_response<W: Write>(writer: &SharedWriter<W>, response: &ProtocolResponse) {
    if let Ok(mut writer) = writer.lock() {
        let _ = write_frame(&mut *writer, response);
    }
}

/// 在线程中执行运行/构建请求，并把 panic 转为稳定响应。
fn worker_response(request: ProtocolRequest, token: CancellationToken) -> ProtocolResponse {
    match request {
        ProtocolRequest::Run {
            request_id,
            protocol_version,
            core_version,
            language_version,
            runtime_version: _,
            target,
            optimization,
            source,
            options,
        } => {
            if let Err(error) = validate_versions(protocol_version, core_version) {
                return protocol_error_response(Some(request_id), &error);
            }
            if let Err(error) = validate_source(&source).and_then(|_| validate_target(&target)) {
                return protocol_error_response(Some(request_id), &error);
            }
            if optimization.level != 0 {
                return protocol_error_response(
                    Some(request_id),
                    &ProtocolError::request("optimization.level", "X0-A 只接受优化级别 0"),
                );
            }
            let (vm_options, event_capacity, timeout) = match run_options(&options) {
                Ok(value) => value,
                Err(error) => return protocol_error_response(Some(request_id), &error),
            };
            let debug = optimization.debug;
            let diagnostic_config = optimization.diagnostics.clone();
            let module_name = source.module.clone();
            let source_name = source.path.clone();
            let mut driver_request =
                DriverRequest::new(frontend_request(&source, &language_version, &target))
                    .with_options(vm_options)
                    .with_module_name(module_name.clone())
                    .with_event_capacity(event_capacity)
                    .with_cancellation(token);
            if let Some(path) = source_name.clone() {
                driver_request = driver_request.with_source_name(path);
            }
            if let Some(timeout) = timeout {
                driver_request = driver_request.with_timeout(timeout);
            }
            run_with_diagnostics(
                request_id,
                debug,
                module_name,
                source_name,
                diagnostic_config,
                &driver_request,
            )
        }
        ProtocolRequest::Build {
            request_id,
            protocol_version,
            core_version,
            language_version,
            runtime_version: _,
            target,
            optimization,
            source,
            output,
            llvm_ir_output,
            toolchain,
            config_text,
        } => {
            if let Err(error) = validate_versions(protocol_version, core_version) {
                return protocol_error_response(Some(request_id), &error);
            }
            if let Err(error) = validate_source(&source).and_then(|_| validate_target(&target)) {
                return protocol_error_response(Some(request_id), &error);
            }
            build_response(
                request_id,
                language_version,
                target,
                optimization,
                source,
                output,
                llvm_ir_output,
                toolchain,
                config_text,
                &token,
            )
        }
        other => dispatch(other),
    }
}

/// 在拥有的输入/输出流上运行可取消的协议服务。
pub fn serve<R, W>(reader: R, writer: W) -> Result<(), FrameError>
where
    R: Read,
    W: Write + Send + 'static,
{
    let mut reader = BufReader::new(reader);
    let writer = Arc::new(Mutex::new(BufWriter::new(writer)));
    let cancellations: CancellationMap = Arc::new(Mutex::new(BTreeMap::new()));
    let mut workers: Vec<JoinHandle<()>> = Vec::new();
    let mut first_frame = true;
    let mut negotiated = false;
    while let Some(request) = read_request(&mut reader)? {
        if first_frame {
            first_frame = false;
            if !matches!(request, ProtocolRequest::Hello { .. }) {
                let request_id = match &request {
                    ProtocolRequest::Run { request_id, .. }
                    | ProtocolRequest::Build { request_id, .. }
                    | ProtocolRequest::Cancel { request_id, .. }
                    | ProtocolRequest::Shutdown { request_id, .. } => Some(request_id.clone()),
                    ProtocolRequest::Hello { .. } => None,
                };
                let error = ProtocolError::version("首帧必须是 hello 版本协商");
                write_response(&writer, &protocol_error_response(request_id, &error));
                break;
            }
        }
        match request {
            ProtocolRequest::Hello { .. } => {
                let response = dispatch(request);
                negotiated = matches!(&response, ProtocolResponse::Hello { accepted: true, .. });
                write_response(&writer, &response);
            }
            ProtocolRequest::Cancel {
                request_id,
                protocol_version,
                core_version,
                target_request_id,
            } => {
                if !negotiated {
                    let error = ProtocolError::version("必须先完成 hello 版本协商");
                    write_response(&writer, &protocol_error_response(Some(request_id), &error));
                    continue;
                }
                if let Err(error) = validate_versions(protocol_version, core_version) {
                    write_response(&writer, &protocol_error_response(Some(request_id), &error));
                    continue;
                }
                let accepted = cancellations
                    .lock()
                    .ok()
                    .and_then(|map| map.get(&target_request_id).cloned())
                    .map(|token| {
                        token.cancel();
                        true
                    })
                    .unwrap_or(false);
                write_response(
                    &writer,
                    &ProtocolResponse::Cancelled {
                        request_id,
                        target_request_id,
                        accepted,
                        exit_code: ExitCode::ArtifactRejected.as_process_code(),
                    },
                );
            }
            ProtocolRequest::Shutdown {
                request_id,
                protocol_version,
                core_version,
            } => {
                if !negotiated {
                    let error = ProtocolError::version("必须先完成 hello 版本协商");
                    write_response(&writer, &protocol_error_response(Some(request_id), &error));
                    continue;
                }
                if let Err(error) = validate_versions(protocol_version, core_version) {
                    write_response(&writer, &protocol_error_response(Some(request_id), &error));
                } else {
                    write_response(&writer, &ProtocolResponse::Shutdown { request_id });
                    break;
                }
            }
            request @ (ProtocolRequest::Run { .. } | ProtocolRequest::Build { .. }) => {
                if !negotiated {
                    let request_id = match &request {
                        ProtocolRequest::Run { request_id, .. }
                        | ProtocolRequest::Build { request_id, .. } => request_id.clone(),
                        _ => unreachable!(),
                    };
                    let error = ProtocolError::version("必须先完成 hello 版本协商");
                    write_response(&writer, &protocol_error_response(Some(request_id), &error));
                    continue;
                }
                let request_id = match &request {
                    ProtocolRequest::Run { request_id, .. }
                    | ProtocolRequest::Build { request_id, .. } => request_id.clone(),
                    _ => unreachable!(),
                };
                let token = CancellationToken::new();
                if let Ok(mut map) = cancellations.lock() {
                    map.insert(request_id.clone(), token.clone());
                }
                let writer_clone = Arc::clone(&writer);
                let cancellations_clone = Arc::clone(&cancellations);
                workers.push(thread::spawn(move || {
                    let response = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                        worker_response(request, token)
                    }))
                    .unwrap_or_else(|_| ProtocolResponse::Error {
                        request_id: Some(request_id.clone()),
                        error: protocol_error_body(
                            CORE_CRASH_CODE,
                            "x11.protocol.core_crash",
                            "Rust 核心处理请求时发生内部崩溃",
                            Some("core".to_owned()),
                            Some("收集核心日志并重启核心进程".to_owned()),
                            BTreeMap::from([
                                ("status".to_owned(), json!("panic")),
                                (
                                    "process_exit_code".to_owned(),
                                    json!(ExitCode::Fatal.as_process_code()),
                                ),
                            ]),
                        ),
                        report: None,
                        exit_code: ExitCode::Fatal.as_process_code(),
                    });
                    if let Ok(mut map) = cancellations_clone.lock() {
                        map.remove(&request_id);
                    }
                    write_response(&writer_clone, &response);
                }));
            }
        }
    }
    for worker in workers {
        let _ = worker.join();
    }
    Ok(())
}

/// 使用标准输入/输出启动 Rust 核心协议服务。
pub fn serve_stdio() -> Result<(), FrameError> {
    let stdin = io::stdin();
    let stdout = io::stdout();
    serve(stdin, stdout)
}

/// 将任意 panic 统一包装为核心崩溃响应。
#[must_use]
pub fn core_crash_response(request_id: Option<String>) -> ProtocolResponse {
    ProtocolResponse::Error {
        request_id,
        error: protocol_error_body(
            CORE_CRASH_CODE,
            "x11.protocol.core_crash",
            "Rust 核心处理请求时发生内部崩溃",
            Some("core".to_owned()),
            Some("收集核心日志并重启核心进程".to_owned()),
            BTreeMap::from([
                ("status".to_owned(), json!("panic")),
                (
                    "process_exit_code".to_owned(),
                    json!(ExitCode::Fatal.as_process_code()),
                ),
            ]),
        ),
        report: None,
        exit_code: ExitCode::Fatal.as_process_code(),
    }
}

#[cfg(test)]
#[path = "protocol_tests.rs"]
/// 覆盖帧边界、版本协商、取消和真实前端运行路径。
mod tests;
