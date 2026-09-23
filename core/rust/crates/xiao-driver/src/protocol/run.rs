//! 真实源码到 VM 的运行路径与运行响应。

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::time::Duration;

use serde_json::Value;

use super::mapping::{
    exit_name, protocol_diagnostic, protocol_error_body, protocol_error_from_error, protocol_event,
    protocol_metrics, protocol_report, protocol_value,
};
use super::{
    CANCELLED_ERROR_CODE, DiagnosticConfig, ProtocolError, ProtocolResponse, ProtocolTarget,
    RunOptions, SourceIdentity,
};
use crate::diagnostics::{DiagnosticOptions, DiagnosticSession, start_error_details};
use crate::frontend::{FrontendContext, FrontendRequest};
use crate::run::{
    DriverError, DriverExecution, DriverOutcome, DriverPhase, DriverRequest, ExitCode,
    FrontendVmDriver,
};

/// 将协议源码字段转换为既有前端请求。
pub(super) fn frontend_request(
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

/// 将协议 VM 参数交给生产 VM 自身的范围校验。
pub(super) fn run_options(
    options: &RunOptions,
) -> Result<(xiao_vm::VmOptions, usize, Option<Duration>), ProtocolError> {
    let vm_options = xiao_vm::VmOptions {
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
pub(super) fn run_with_diagnostics(
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

/// 创建统一取消响应，使用 `ArtifactRejected` 进程码。
pub(super) fn cancelled_error_response(request_id: String) -> ProtocolResponse {
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
pub(super) fn protocol_error_response(
    request_id: Option<String>,
    error: &ProtocolError,
) -> ProtocolResponse {
    ProtocolResponse::Error {
        request_id,
        error: protocol_error_from_error(error),
        report: None,
        exit_code: ExitCode::ArtifactRejected.as_process_code(),
    }
}
