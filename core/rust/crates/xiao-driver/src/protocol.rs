//! X0-A 子进程协议、长度前缀帧和 Rust 核心入口。
//!
//! 协议层只负责传输、版本协商和结果映射；源码解析、类型检查、生命周期分析、
//! 字节码降低、VM 执行和 LLVM 构建仍分别由已有驱动器负责。帧的长度字段是 8 字节
//! 大端无符号整数，只计算 UTF-8 JSON 负载；解码器在分配前检查 16 MiB 上限。

mod config;
mod frame;
mod mapping;
mod message;
mod request;
mod run;
mod validate;

#[allow(unused_imports)]
use config::FrozenRuntimeConfig;
use config::{freeze_runtime_config, runtime_config_path, write_runtime_config};
pub use frame::{
    FRAME_ERROR_CODE, FRAME_LENGTH_BYTES, FrameError, MAX_FRAME_BYTES, decode_frame, encode_frame,
    read_frame, write_frame,
};
use mapping::{exit_name, protocol_error_body, protocol_error_from_error};
pub use mapping::{protocol_diagnostic, protocol_param};
pub use message::*;
pub use request::*;
use run::{
    cancelled_error_response, frontend_request, protocol_error_response, run_options,
    run_with_diagnostics,
};
use std::collections::BTreeMap;
use std::fs;
use std::io::{self, BufReader, BufWriter, Read, Write};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use validate::{validate_source, validate_target, validate_versions};

#[allow(unused_imports)]
use serde_json::Value;
use serde_json::json;
use xiao_codegen_llvm::{CodegenOptions, TargetDescription, Toolchain, ToolchainVersions};
use xiao_diagnostics::window::{
    DIAGNOSTIC_START_CODE, DiagnosticActivation, activation_path, write_activation,
};

use crate::native::{FrontendNativeDriver, NativeBuildRequest, NativeDriverError};
use crate::run::{CancellationToken, DriverRequest, ExitCode};

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
