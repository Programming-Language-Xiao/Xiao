//! 真实源码到 VM 的运行路径与运行响应。

use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use serde_json::Value;
use xiao_diagnostics::{Diagnostic, DiagnosticParam, Severity};

use super::mapping::{
    exit_name, protocol_diagnostic, protocol_error_body, protocol_error_from_error, protocol_event,
    protocol_metrics, protocol_report, protocol_value,
};
use super::message::ProtocolResponse;
use super::request::{
    CANCELLED_ERROR_CODE, DiagnosticConfig, OptimizationConfig, ProtocolError, ProtocolTarget,
    RunOptions, SourceIdentity,
};
use super::validate::{validate_source, validate_target, validate_versions};
use crate::diagnostics::{DiagnosticOptions, DiagnosticSession, start_error_details};
use crate::frontend::{FrontendContext, FrontendRequest};
use crate::packages::{PackageRegistry, PackageRegistryFingerprint};
use crate::run::{
    CancellationToken, DRIVER_TIMEOUT_CODE, DriverError, DriverExecution, DriverOutcome,
    DriverPhase, DriverRequest, ExitCode, FrontendVmDriver,
};
use xiao_xar::{
    ARCHIVE_LANGUAGE_FALLBACK_CODE, ARCHIVE_MISSING_OBJECT_CODE, ArchiveAuditRecord,
    XarLanguageResolution, XarRunError, XarRunOptions, audit_archive, decode_xar,
    resolve_language_locale, run_archive_with_control,
};

/// 同一核心进程里可以安全复用模块实例的项目与环境身份。
#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct RunSessionFingerprint {
    project_root: Option<PathBuf>,
    packages: PackageRegistryFingerprint,
}

/// 将协议源码字段转换为既有前端请求。
pub(super) fn frontend_request(
    source: &SourceIdentity,
    language_version: &str,
    target: &ProtocolTarget,
    locale: Option<&str>,
) -> FrontendRequest {
    let mut context = FrontendContext::host();
    context.language_version = language_version.to_owned();
    context.target = target.triple.clone();
    context.locale = locale.unwrap_or("zh-CN").to_owned();
    let request = match &source.path {
        Some(path) => FrontendRequest::from_text_at(source.text.clone(), path.clone()),
        None => FrontendRequest::from_text(source.text.clone()),
    };
    request.with_context(context)
}

fn frontend_run_request(
    source: &SourceIdentity,
    language_version: &str,
    target: &ProtocolTarget,
    locale: Option<&str>,
) -> Result<(FrontendRequest, RunSessionFingerprint), ProtocolError> {
    let mut request = frontend_request(source, language_version, target, locale);
    if let Some(root) = source
        .path
        .as_ref()
        .and_then(|path| {
            let path = std::path::Path::new(path);
            (path.is_absolute() && path.is_file()).then(|| {
                let directory = path.parent()?;
                Some(
                    path.ancestors()
                        .skip(1)
                        .find(|parent| parent.join("config.xiao").is_file())
                        .unwrap_or(directory)
                        .to_path_buf(),
                )
            })
        })
        .flatten()
    {
        request.context.project_root = Some(root);
    }
    let active_environment = std::env::var_os("XIAO_ACTIVE_ENV");
    let registry =
        PackageRegistry::from_environment(active_environment.as_deref().map(std::path::Path::new))
            .map_err(|error| ProtocolError::request("environment", error))?;
    let fingerprint = RunSessionFingerprint {
        project_root: request.context.project_root.clone(),
        packages: registry.session_fingerprint(),
    };
    request.context.package_registry = Some(registry);
    Ok((request, fingerprint))
}

/// 将协议 VM 参数交给生产 VM 自身的范围校验。
pub(super) fn run_options(
    options: &RunOptions,
) -> Result<(xiao_vm::VmOptions, usize, Option<Duration>), ProtocolError> {
    let vm_options = xiao_vm::VmOptions {
        max_call_depth: options.max_call_depth,
        checkpoints_enabled: options.checkpoints_enabled,
        checkpoint_interval: options.checkpoint_interval,
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

#[allow(clippy::too_many_arguments)]
/// 校验运行请求并交给前端到 VM 的驱动路径。
pub(super) fn run_request_response(
    request_id: String,
    protocol_version: u16,
    core_version: u32,
    language_version: String,
    locale: Option<String>,
    target: ProtocolTarget,
    optimization: OptimizationConfig,
    source: SourceIdentity,
    options: RunOptions,
    cancellation: CancellationToken,
) -> ProtocolResponse {
    let mut driver = FrontendVmDriver::new();
    let mut fingerprint = None;
    run_request_response_with_driver(
        request_id,
        protocol_version,
        core_version,
        language_version,
        locale,
        target,
        optimization,
        source,
        options,
        cancellation,
        &mut driver,
        &mut fingerprint,
    )
}

/// 执行 `.xar`：先完成容器、索引、对象、Runtime 和平台验证，再进入统一 VM。
#[allow(clippy::too_many_arguments)]
pub(super) fn run_archive_request_response(
    request_id: String,
    protocol_version: u16,
    core_version: u32,
    locale: Option<String>,
    path: String,
    options: RunOptions,
    debug: bool,
    cancellation: CancellationToken,
) -> ProtocolResponse {
    let effective_locale = fs::read(&path)
        .ok()
        .and_then(|bytes| decode_xar(&bytes).ok())
        .map(|archive| {
            resolve_language_locale(&archive.index().language_locale, locale.as_deref()).effective
        })
        .or_else(|| locale.clone());
    if effective_locale != locale {
        return super::localize::with_locale(request_id.clone(), effective_locale.clone(), || {
            run_archive_request_response_inner(
                request_id,
                protocol_version,
                core_version,
                effective_locale,
                path,
                options,
                debug,
                cancellation,
            )
        });
    }
    run_archive_request_response_inner(
        request_id,
        protocol_version,
        core_version,
        locale,
        path,
        options,
        debug,
        cancellation,
    )
}

#[allow(clippy::too_many_arguments)]
fn run_archive_request_response_inner(
    request_id: String,
    protocol_version: u16,
    core_version: u32,
    locale: Option<String>,
    path: String,
    options: RunOptions,
    debug: bool,
    cancellation: CancellationToken,
) -> ProtocolResponse {
    if let Err(error) = validate_versions(protocol_version, core_version) {
        return protocol_error_response(Some(request_id), &error);
    }
    if cancellation.is_cancelled() {
        return cancelled_error_response(request_id);
    }
    let (vm_options, event_capacity, timeout) = match run_options(&options) {
        Ok(value) => value,
        Err(error) => return protocol_error_response(Some(request_id), &error),
    };
    let deadline = match timeout {
        Some(timeout) => match Instant::now().checked_add(timeout) {
            Some(deadline) => Some(deadline),
            None => {
                return protocol_error_response(
                    Some(request_id),
                    &ProtocolError::request("options.timeout_ms", "超时期限超出宿主时钟可表示范围"),
                );
            }
        },
        None => None,
    };
    if deadline.is_some_and(|deadline| Instant::now() >= deadline) {
        return archive_timeout_response(request_id);
    }
    if path.trim().is_empty() {
        return archive_error_response(
            request_id,
            ARCHIVE_MISSING_OBJECT_CODE,
            "x17.xar.archive_missing_object",
            "归档路径不能为空".to_owned(),
            path,
        );
    }
    let bytes = match fs::read(&path) {
        Ok(bytes) => bytes,
        Err(error) => {
            return archive_error_response(
                request_id,
                ARCHIVE_MISSING_OBJECT_CODE,
                "x17.xar.archive_missing_object",
                format!("无法读取归档：{error}"),
                path,
            );
        }
    };
    let archive = match decode_xar(&bytes) {
        Ok(archive) => archive,
        Err(error) => {
            return xar_run_error_response(request_id, path, classify_xar_error(error), None);
        }
    };
    if cancellation.is_cancelled() {
        return cancelled_error_response(request_id);
    }
    if deadline.is_some_and(|deadline| Instant::now() >= deadline) {
        return archive_timeout_response(request_id);
    }
    let language = resolve_language_locale(&archive.index().language_locale, locale.as_deref());
    let mut cancellation_source =
        xiao_vm::CancellationSource::new().with_token(cancellation.clone());
    if let Some(deadline) = deadline {
        cancellation_source = cancellation_source.with_deadline(deadline);
    }
    let xar_options = XarRunOptions {
        runtime_abi: xiao_runtime_abi::ABI_ENCODED_VERSION,
        platform: ProtocolTarget::host().triple,
        language_locale: Some(language.effective.clone()),
        vm_options,
        event_capacity,
        debug,
    };
    let audit = match audit_archive(&bytes, &xar_options) {
        Ok(audit) => audit,
        Err(error) => return xar_run_error_response(request_id, path, error, None),
    };
    let debug_active = debug || archive.index().debug_activation;
    let diagnostic_options = crate::diagnostics::DiagnosticOptions {
        locale: Some(language.effective.clone()),
        ..Default::default()
    };
    let mut diagnostic_session = if debug_active {
        match crate::diagnostics::DiagnosticSession::start(
            archive.index().entry.clone(),
            Some(path.clone()),
            &diagnostic_options,
        ) {
            Ok(session) => Some(session),
            Err(error) => return diagnostic_start_response(request_id, error),
        }
    } else {
        None
    };
    if cancellation.is_cancelled() {
        if let Some(session) = diagnostic_session.take() {
            session.finish();
        }
        return cancelled_error_response(request_id);
    }
    if deadline.is_some_and(|deadline| Instant::now() >= deadline) {
        if let Some(session) = diagnostic_session.take() {
            session.finish();
        }
        return archive_timeout_response(request_id);
    }
    let measurement = xiao_runtime::start_memory_measurement();
    let result = run_archive_with_control(&bytes, xar_options, Some(cancellation_source));
    let peak_live_bytes = measurement.peak_live_bytes();
    drop(measurement);
    if cancellation.is_cancelled() {
        if let Some(session) = diagnostic_session.take() {
            session.finish();
        }
        return cancelled_error_response(request_id);
    }
    if deadline.is_some_and(|deadline| Instant::now() >= deadline) {
        if let Some(session) = diagnostic_session.take() {
            session.finish();
        }
        return archive_timeout_response(request_id);
    }
    let result = match result {
        Ok(outcome) => {
            let mut diagnostics = Vec::new();
            if language.fallback {
                diagnostics.push(archive_language_fallback_diagnostic(&language));
            }
            let execution = DriverExecution {
                outcome,
                diagnostics,
            };
            DriverOutcome::Executed(execution)
        }
        Err(error) => {
            if let Some(session) = diagnostic_session.take() {
                session.finish();
            }
            return xar_run_error_response(request_id, path, error, Some(audit));
        }
    };
    if cancellation.is_cancelled() {
        if let Some(session) = diagnostic_session.take() {
            session.finish();
        }
        return cancelled_error_response(request_id);
    }
    if deadline.is_some_and(|deadline| Instant::now() >= deadline) {
        if let Some(session) = diagnostic_session.take() {
            session.finish();
        }
        return archive_timeout_response(request_id);
    }
    if let Some(mut session) = diagnostic_session.take() {
        if let DriverOutcome::Executed(execution) = &result {
            for event in execution.events() {
                session.record(event);
            }
        }
        session.finish();
    }
    run_response_for_operation_with_audit(
        request_id,
        result,
        peak_live_bytes,
        "run_archive",
        Some(audit),
    )
}

fn classify_xar_error(error: xiao_xar::XarError) -> XarRunError {
    match error {
        xiao_xar::XarError::MissingMember(path) => XarRunError::MissingObject(path),
        xiao_xar::XarError::Artifact(xiao_artifacts::ArtifactError::UnsupportedIndexVersion {
            major,
            ..
        }) => XarRunError::VersionIncompatible(format!("不支持的归档索引主版本 {major}")),
        xiao_xar::XarError::Artifact(xiao_artifacts::ArtifactError::Index(message))
            if message.contains("未知必需") =>
        {
            XarRunError::VersionIncompatible(message)
        }
        xiao_xar::XarError::Unsupported(message) if message.contains("版本") => {
            XarRunError::VersionIncompatible(message)
        }
        xiao_xar::XarError::InvalidIndex(message) if message.contains("版本") => {
            XarRunError::VersionIncompatible(message)
        }
        other => XarRunError::Validation(other.to_string()),
    }
}

fn xar_run_error_response(
    request_id: String,
    path: String,
    error: XarRunError,
    audit: Option<ArchiveAuditRecord>,
) -> ProtocolResponse {
    let mut response = archive_error_response(
        request_id,
        error.code(),
        error.message_id(),
        error.message().to_owned(),
        path,
    );
    if let Some(audit) = audit
        && let Ok(value) = serde_json::to_value(audit)
        && let ProtocolResponse::Error { error, .. } = &mut response
    {
        error.details.insert("audit".to_owned(), value);
    }
    response
}

fn archive_error_response(
    request_id: String,
    code: &str,
    message_id: &str,
    message: String,
    path: String,
) -> ProtocolResponse {
    let mut details = BTreeMap::new();
    details.insert("path".to_owned(), Value::String(path));
    ProtocolResponse::Error {
        request_id: Some(request_id),
        error: protocol_error_body(
            code,
            message_id,
            message,
            Some("archive_validation".to_owned()),
            Some("检查归档、Runtime 和目标平台后重试".to_owned()),
            details,
        ),
        report: None,
        exit_code: ExitCode::ArtifactRejected.as_process_code(),
    }
}

/// 归档运行在驱动器控制边界超时时返回与源码路径一致的稳定错误。
fn archive_timeout_response(request_id: String) -> ProtocolResponse {
    ProtocolResponse::Error {
        request_id: Some(request_id),
        error: protocol_error_body(
            DRIVER_TIMEOUT_CODE,
            "x11.driver.rejected",
            "运行在驱动器边界超过超时期限",
            Some("control".to_owned()),
            Some("增大 timeout_ms 后重试".to_owned()),
            BTreeMap::from([(
                "code".to_owned(),
                Value::String(DRIVER_TIMEOUT_CODE.to_owned()),
            )]),
        ),
        report: None,
        exit_code: ExitCode::ArtifactRejected.as_process_code(),
    }
}

fn archive_language_fallback_diagnostic(resolution: &XarLanguageResolution) -> Diagnostic {
    Diagnostic::new(
        ARCHIVE_LANGUAGE_FALLBACK_CODE,
        "x17.xar.language_fallback",
        Severity::Warning,
        None,
        format!(
            "归档请求语言 {:?} 没有可用内置目录，已回落到 {:?}",
            resolution.requested, resolution.effective
        ),
    )
    .with_params([
        (
            "requested".to_owned(),
            DiagnosticParam::Text(resolution.requested.clone()),
        ),
        (
            "effective".to_owned(),
            DiagnosticParam::Text(resolution.effective.clone()),
        ),
    ])
}

#[allow(clippy::too_many_arguments)]
/// 在长驻核心会话的驱动器上执行一次运行请求。
pub(super) fn run_request_response_with_driver(
    request_id: String,
    protocol_version: u16,
    core_version: u32,
    language_version: String,
    locale: Option<String>,
    target: ProtocolTarget,
    optimization: OptimizationConfig,
    source: SourceIdentity,
    options: RunOptions,
    cancellation: CancellationToken,
    driver: &mut FrontendVmDriver,
    session_fingerprint: &mut Option<RunSessionFingerprint>,
) -> ProtocolResponse {
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
    let (frontend, fingerprint) =
        match frontend_run_request(&source, &language_version, &target, locale.as_deref()) {
            Ok(frontend) => frontend,
            Err(error) => {
                driver.reset_session();
                *session_fingerprint = None;
                return protocol_error_response(Some(request_id), &error);
            }
        };
    if session_fingerprint.as_ref() != Some(&fingerprint) {
        driver.reset_session();
        *session_fingerprint = Some(fingerprint);
    }
    let mut driver_request = DriverRequest::new(frontend)
        .with_options(vm_options)
        .with_module_name(module_name.clone())
        .with_event_capacity(event_capacity)
        .with_cancellation(cancellation);
    if let Some(path) = source_name.clone() {
        driver_request = driver_request.with_source_name(path);
    }
    if let Some(timeout) = timeout {
        driver_request = driver_request.with_timeout(timeout);
    }
    let diagnostics = diagnostic_options(diagnostic_config, locale);
    run_with_diagnostics(
        request_id,
        debug,
        module_name,
        source_name,
        diagnostics,
        driver,
        &driver_request,
    )
}

/// 将三段驱动器结果转换为运行响应。
fn run_response(
    request_id: String,
    outcome: DriverOutcome,
    peak_live_bytes: u64,
) -> ProtocolResponse {
    run_response_for_operation(request_id, outcome, peak_live_bytes, "run")
}

/// 将驱动器结果转换成指定协议操作的统一运行响应。
fn run_response_for_operation(
    request_id: String,
    outcome: DriverOutcome,
    peak_live_bytes: u64,
    operation: &str,
) -> ProtocolResponse {
    run_response_for_operation_with_audit(request_id, outcome, peak_live_bytes, operation, None)
}

/// 将运行结果转换为协议响应，并在归档路径附带校验审计记录。
fn run_response_for_operation_with_audit(
    request_id: String,
    outcome: DriverOutcome,
    peak_live_bytes: u64,
    operation: &str,
    audit: Option<ArchiveAuditRecord>,
) -> ProtocolResponse {
    let exit_code = outcome.exit_code();
    match outcome {
        DriverOutcome::Frontend(error) => ProtocolResponse::Result {
            request_id,
            operation: operation.to_owned(),
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
            audit: None,
        },
        DriverOutcome::Rejected(error) => rejected_response(request_id, exit_code, &error),
        DriverOutcome::Executed(execution) => executed_response(
            request_id,
            exit_code,
            &execution,
            peak_live_bytes,
            operation,
            audit,
        ),
    }
}

/// 把协议诊断配置转换成 Runtime 会话配置。
fn diagnostic_options(
    config: Option<DiagnosticConfig>,
    locale: Option<String>,
) -> DiagnosticOptions {
    let Some(config) = config else {
        return DiagnosticOptions {
            locale,
            ..DiagnosticOptions::default()
        };
    };
    DiagnosticOptions {
        terminal_level: config.terminal_level,
        file_level: config.file_level,
        log_dir: config.log_dir.map(PathBuf::from),
        log_file: config.log_file.map(PathBuf::from),
        stacktrace: config.stacktrace,
        locale,
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
    diagnostics: DiagnosticOptions,
    driver: &mut FrontendVmDriver,
    request: &DriverRequest,
) -> ProtocolResponse {
    let mut session = if debug {
        match DiagnosticSession::start(module.clone(), source.clone(), &diagnostics) {
            Ok(session) => Some(session),
            Err(error) => return diagnostic_start_response(request_id, error),
        }
    } else {
        None
    };
    let measurement = xiao_runtime::start_memory_measurement();
    let outcome = driver.run(request);
    let peak_live_bytes = measurement.peak_live_bytes();
    drop(measurement);
    if let Some(mut session) = session.take() {
        if let DriverOutcome::Executed(execution) = &outcome {
            for event in execution.events() {
                session.record(event);
            }
        }
        session.finish();
    }
    run_response(request_id, outcome, peak_live_bytes)
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
    peak_live_bytes: u64,
    operation: &str,
    audit: Option<ArchiveAuditRecord>,
) -> ProtocolResponse {
    let outcome = &execution.outcome;
    let value = outcome.value.as_ref().map(protocol_value);
    ProtocolResponse::Result {
        request_id,
        operation: operation.to_owned(),
        exit_code: exit_code.as_process_code(),
        exit_name: exit_name(exit_code).to_owned(),
        diagnostics: execution
            .diagnostics()
            .iter()
            .map(protocol_diagnostic)
            .collect(),
        report: outcome.report.as_ref().map(protocol_report),
        events: outcome.events.iter().map(protocol_event).collect(),
        metrics: Some(protocol_metrics(
            outcome.metrics,
            outcome.dropped_events,
            peak_live_bytes,
        )),
        value,
        artifact: None,
        audit,
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
