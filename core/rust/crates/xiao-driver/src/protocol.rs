//! X0-A 子进程协议、长度前缀帧和 Rust 核心入口。
//!
//! 协议层只负责传输、版本协商和结果映射；源码解析、类型检查、生命周期分析、
//! 字节码降低、VM 执行和 LLVM 构建仍分别由已有驱动器负责。帧的长度字段是 8 字节
//! 大端无符号整数，只计算 UTF-8 JSON 负载；解码器在分配前检查 16 MiB 上限。

mod build;
mod config;
mod frame;
mod mapping;
mod message;
mod request;
mod run;
mod validate;

#[allow(unused_imports)]
use build::{diagnostics_component_path, stage_diagnostics_component};
#[allow(unused_imports)]
use config::FrozenRuntimeConfig;
#[allow(unused_imports)]
use config::{freeze_runtime_config, write_runtime_config};
pub use frame::{
    FRAME_ERROR_CODE, FRAME_LENGTH_BYTES, FrameError, MAX_FRAME_BYTES, decode_frame, encode_frame,
    read_frame, write_frame,
};
pub use mapping::{protocol_diagnostic, protocol_param};
use mapping::{protocol_error_body, protocol_error_from_error};
pub use message::*;
pub use request::*;
use run::{frontend_request, protocol_error_response, run_options, run_with_diagnostics};
use std::collections::BTreeMap;
#[allow(unused_imports)]
use std::fs;
use std::io::{self, BufReader, BufWriter, Read, Write};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use validate::{validate_source, validate_target, validate_versions};

use crate::run::{CancellationToken, DriverRequest, ExitCode};
#[allow(unused_imports)]
use serde_json::Value;
use serde_json::json;

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
            build::build_response(
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
            build::build_response(
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
