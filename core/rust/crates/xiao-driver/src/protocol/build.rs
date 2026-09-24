//! 真实源码到原生产物的构建路径。
//!
//! 构建层只负责把协议输入编排给 `FrontendNativeDriver`，并整理调试组件、
//! 激活位和运行时配置等产物附属文件。工具链路径仍由调用方显式注入。

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde_json::json;
use xiao_codegen_llvm::{CodegenOptions, TargetDescription, Toolchain, ToolchainVersions};
use xiao_diagnostics::window::{
    DIAGNOSTIC_START_CODE, DiagnosticActivation, activation_path, write_activation,
};

use super::config::{
    FrozenRuntimeConfig, freeze_runtime_config, runtime_config_path, write_runtime_config,
};
use super::mapping::{exit_name, protocol_diagnostic, protocol_error_body};
use super::message::{
    ProtocolArtifact, ProtocolDiagnosticActivation, ProtocolResponse, ProtocolRuntimeConfig,
};
use super::request::{
    BUILD_ERROR_CODE, OptimizationConfig, ProtocolError, ProtocolTarget, SourceIdentity,
    ToolchainSpec, UNSUPPORTED_OPERATION_CODE,
};
use super::run::{cancelled_error_response, frontend_request, protocol_error_response};
use crate::native::{
    FrontendNativeDriver, NativeBuildRequest, NativeBuildResult, NativeDriverError,
};
use crate::run::{CancellationToken, ExitCode};

/// `build_response` 的已拥有输入，便于把准备阶段与执行阶段分开。
struct BuildInputs {
    language_version: String,
    target: ProtocolTarget,
    optimization: OptimizationConfig,
    source: SourceIdentity,
    output: String,
    llvm_ir_output: Option<String>,
    toolchain: ToolchainSpec,
    config_text: Option<String>,
}

/// 原生驱动器执行前已经完成校验和配置固化的构建计划。
struct BuildPlan {
    request: NativeBuildRequest,
    source: SourceIdentity,
    optimization: OptimizationConfig,
    llvm_ir_output: Option<String>,
    diagnostics_source: Option<String>,
    diagnostics_path: Option<String>,
    frozen_config: Option<FrozenRuntimeConfig>,
}

/// 将调用方注入的工具链字段转换为 LLVM 驱动器对象。
pub(super) fn build_toolchain(
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

/// 调用原生驱动器并转换构建结果或结构化后端错误。
#[allow(clippy::too_many_arguments)]
pub(super) fn build_response(
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
        return unsupported_optimization_response(request_id, optimization.level);
    }
    if output.trim().is_empty() {
        return protocol_error_response(
            Some(request_id),
            &ProtocolError::request("output", "原生构建必须提供输出路径"),
        );
    }

    let inputs = BuildInputs {
        language_version,
        target,
        optimization,
        source,
        output,
        llvm_ir_output,
        toolchain,
        config_text,
    };
    let plan = match prepare_build(&request_id, inputs) {
        Ok(plan) => plan,
        Err(response) => return response,
    };
    if cancellation.is_cancelled() {
        return cancelled_error_response(request_id);
    }

    match FrontendNativeDriver::new().build(&plan.request) {
        Ok(result) => finalize_build(request_id, result, plan, cancellation),
        Err(error) => native_build_error_response(request_id, error),
    }
}

/// 清理旧激活位、准备目标/工具链，并生成前端到原生的请求。
#[allow(clippy::result_large_err)]
fn prepare_build(request_id: &str, inputs: BuildInputs) -> Result<BuildPlan, ProtocolResponse> {
    let BuildInputs {
        language_version,
        target,
        optimization,
        source,
        output,
        llvm_ir_output,
        toolchain: toolchain_spec,
        config_text,
    } = inputs;

    clear_activation(&output, request_id)?;
    let target_description = target
        .to_target()
        .map_err(|error| protocol_error_response(Some(request_id.to_owned()), &error))?;
    let diagnostics_source = toolchain_spec.diagnostics_path.clone();
    let diagnostics_path = diagnostics_source
        .as_deref()
        .map(|path| diagnostics_component_path(&output, path));
    let toolchain = build_toolchain(&toolchain_spec, &target_description)
        .map_err(|error| protocol_error_response(Some(request_id.to_owned()), &error))?;
    let frozen_config = freeze_runtime_config(&output, config_text.as_deref())
        .map_err(|error| protocol_error_response(Some(request_id.to_owned()), &error))?;
    let frontend = frontend_request(&source, &language_version, &target);
    let mut request =
        NativeBuildRequest::new(frontend, target_description.clone(), toolchain, output)
            .with_codegen_options(build_codegen_options(
                &optimization,
                diagnostics_path.as_deref(),
                request_id,
                target_description,
            )?);
    if let Some(path) = &llvm_ir_output {
        request = request.with_llvm_ir_output(path);
    }

    Ok(BuildPlan {
        request,
        source,
        optimization,
        llvm_ir_output,
        diagnostics_source,
        diagnostics_path,
        frozen_config,
    })
}

/// 生成目标相关代码生成选项，并在调试构建缺少 shim 时提前拒绝。
#[allow(clippy::result_large_err)]
fn build_codegen_options(
    optimization: &OptimizationConfig,
    diagnostics_path: Option<&str>,
    request_id: &str,
    target: TargetDescription,
) -> Result<CodegenOptions, ProtocolResponse> {
    let options = CodegenOptions::for_target(target);
    if !optimization.debug {
        return Ok(options);
    }
    let Some(path) = diagnostics_path else {
        return Err(protocol_error_response(
            Some(request_id.to_owned()),
            &ProtocolError::build("调试构建缺少 xiao-diagnostics 路径；请随分发包携带诊断组件"),
        ));
    };
    Ok(options.with_debug_startup(path))
}

/// 清理上一次调试构建的激活位，防止失败重建沿用旧状态。
#[allow(clippy::result_large_err)]
fn clear_activation(output: &str, request_id: &str) -> Result<(), ProtocolResponse> {
    let activation_file = activation_path(output);
    if let Err(error) = fs::remove_file(&activation_file)
        && error.kind() != std::io::ErrorKind::NotFound
    {
        return Err(ProtocolResponse::Error {
            request_id: Some(request_id.to_owned()),
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
        });
    }
    Ok(())
}

/// 将原生驱动器错误转换为稳定协议响应。
fn native_build_error_response(request_id: String, error: NativeDriverError) -> ProtocolResponse {
    match error {
        NativeDriverError::Frontend(error) => ProtocolResponse::Result {
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
        NativeDriverError::Backend(error) => ProtocolResponse::Error {
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

/// 完成原生产物的调试组件、激活位和运行时配置整理。
fn finalize_build(
    request_id: String,
    result: NativeBuildResult,
    plan: BuildPlan,
    cancellation: &CancellationToken,
) -> ProtocolResponse {
    let BuildPlan {
        request: _,
        source,
        optimization,
        llvm_ir_output,
        diagnostics_source,
        diagnostics_path,
        frozen_config,
    } = plan;
    if cancellation.is_cancelled() {
        cleanup_native_outputs(&result.native.executable, llvm_ir_output.as_deref());
        return cancelled_error_response(request_id);
    }

    let (diagnostics_component, staged_diagnostics_component) = match stage_debug_component(
        optimization.debug,
        diagnostics_source.as_deref(),
        diagnostics_path.as_deref(),
        &request_id,
    ) {
        Ok(value) => value,
        Err(response) => {
            cleanup_native_outputs(&result.native.executable, llvm_ir_output.as_deref());
            return response;
        }
    };
    let diagnostic_activation = match write_debug_activation(
        optimization.debug,
        &source,
        &result.native.executable,
        &request_id,
    ) {
        Ok(value) => value,
        Err(response) => {
            cleanup_after_failure(
                &result,
                llvm_ir_output.as_deref(),
                staged_diagnostics_component,
                diagnostics_component.as_deref(),
                None,
            );
            return response;
        }
    };
    let runtime_config = match write_or_clear_runtime_config(
        frozen_config,
        &result.native.executable,
        &request_id,
    ) {
        Ok(value) => value,
        Err(response) => {
            cleanup_after_failure(
                &result,
                llvm_ir_output.as_deref(),
                staged_diagnostics_component,
                diagnostics_component.as_deref(),
                diagnostic_activation.as_ref(),
            );
            return response;
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

/// 为调试构建复制独立诊断进程到产物目录。
#[allow(clippy::result_large_err)]
fn stage_debug_component(
    debug: bool,
    source: Option<&str>,
    destination: Option<&str>,
    request_id: &str,
) -> Result<(Option<String>, bool), ProtocolResponse> {
    if !debug {
        return Ok((None, false));
    }
    let Some(source) = source else {
        return Err(protocol_error_response(
            Some(request_id.to_owned()),
            &ProtocolError::build("调试构建缺少 xiao-diagnostics 源文件路径"),
        ));
    };
    let destination = destination.expect("调试构建已计算诊断组件目标路径");
    match stage_diagnostics_component(source, destination) {
        Ok(staged) => Ok((Some(destination.to_owned()), staged)),
        Err(error) => Err(ProtocolResponse::Error {
            request_id: Some(request_id.to_owned()),
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
        }),
    }
}

/// 写入调试激活位，并返回协议摘要。
#[allow(clippy::result_large_err)]
fn write_debug_activation(
    debug: bool,
    source: &SourceIdentity,
    executable: &Path,
    request_id: &str,
) -> Result<Option<ProtocolDiagnosticActivation>, ProtocolResponse> {
    if !debug {
        return Ok(None);
    }
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
    match write_activation(executable, &activation) {
        Ok(path) => Ok(Some(ProtocolDiagnosticActivation {
            path: path.display().to_string(),
            enabled: activation.enabled,
            source_map: activation.source_map.is_some(),
            hooks: activation.hooks,
        })),
        Err(error) => Err(ProtocolResponse::Error {
            request_id: Some(request_id.to_owned()),
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
        }),
    }
}

/// 写入配置摘要；没有配置时清理旧旁置文件。
#[allow(clippy::result_large_err)]
fn write_or_clear_runtime_config(
    frozen_config: Option<FrozenRuntimeConfig>,
    executable: &Path,
    request_id: &str,
) -> Result<Option<ProtocolRuntimeConfig>, ProtocolResponse> {
    match frozen_config {
        Some(config) => write_runtime_config(executable, &config)
            .map(Some)
            .map_err(|error| protocol_error_response(Some(request_id.to_owned()), &error)),
        None => {
            let path = runtime_config_path(executable);
            if let Err(error) = fs::remove_file(&path)
                && error.kind() != std::io::ErrorKind::NotFound
            {
                return Err(protocol_error_response(
                    Some(request_id.to_owned()),
                    &ProtocolError::build(format!("无法清理旧的运行时配置：{error}")),
                ));
            }
            Ok(None)
        }
    }
}

/// 构建后续整理失败时撤销已经提交的附属产物。
fn cleanup_after_failure(
    result: &NativeBuildResult,
    llvm_ir_output: Option<&str>,
    staged_component: bool,
    diagnostics_component: Option<&str>,
    activation: Option<&ProtocolDiagnosticActivation>,
) {
    cleanup_native_outputs(&result.native.executable, llvm_ir_output);
    if staged_component {
        if let Some(component) = diagnostics_component {
            let _ = fs::remove_file(component);
        }
    }
    if let Some(activation) = activation {
        let _ = fs::remove_file(&activation.path);
    }
}

/// 为尚未支持的非零优化级别构造稳定协议错误。
fn unsupported_optimization_response(request_id: String, level: u8) -> ProtocolResponse {
    ProtocolResponse::Error {
        request_id: Some(request_id),
        error: protocol_error_body(
            UNSUPPORTED_OPERATION_CODE,
            "x11.protocol.optimization_unavailable",
            "X0-A 只接受优化级别 0",
            Some("build".to_owned()),
            Some("使用 level=0，优化接线留给后续阶段".to_owned()),
            BTreeMap::from([("level".to_owned(), json!(level))]),
        ),
        report: None,
        exit_code: ExitCode::ArtifactRejected.as_process_code(),
    }
}

/// 计算调试组件在最终产物目录中的携带路径。
pub(super) fn diagnostics_component_path(output: &str, source: &str) -> String {
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
        .unwrap_or_else(|| Path::new("."))
        .join(name)
        .display()
        .to_string()
}

/// 把独立诊断进程复制到原生构建目录，使调试产物不依赖 `xiao` 启动器。
pub(super) fn stage_diagnostics_component(
    source: &str,
    destination: &str,
) -> std::io::Result<bool> {
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
fn cleanup_native_outputs(executable: &Path, llvm_ir_output: Option<&str>) {
    let _ = fs::remove_file(executable);
    if let Some(path) = llvm_ir_output {
        let _ = fs::remove_file(path);
    }
}
